use std::collections::HashSet;
use std::path::Path;
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::IoResultExt;
use crate::instances::mods::{ContentKind, ModsDir};
use crate::modrinth::install::{ModTarget, install};
use crate::modrinth::service::content_loaders;
use crate::services::Services;
use crate::tasks::{TaskId, TaskKind, TaskSpec};
use crate::{CoreError, CoreResult};

const PACK_DATA: &str = include_str!("packs.json");
const RESOURCEPACKS_DIR: &str = "resourcepacks";
const MAX_SCANNED_ENTRIES: usize = 200_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum OptifineFeature {
    ConnectedTextures,
    EntityModels,
    RandomEntities,
    Emissive,
    ItemTextures,
    Sky,
    Animations,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PackMod {
    pub slug: String,
    pub name: String,
    #[serde(default)]
    pub features: Vec<OptifineFeature>,
}

#[derive(Debug, Clone, Deserialize)]
struct PackDefinition {
    id: String,
    mods: Vec<PackMod>,
}

#[derive(Debug, Deserialize)]
struct PackData {
    packs: Vec<PackDefinition>,
}

static PACKS: LazyLock<Vec<PackDefinition>> = LazyLock::new(|| {
    serde_json::from_str::<PackData>(PACK_DATA)
        .map(|data| data.packs)
        .unwrap_or_else(|err| {
            tracing::error!(error = %err, "the bundled pack list is invalid");
            Vec::new()
        })
});

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ContentPack {
    pub id: String,
    pub mods: Vec<PackMod>,
    pub installed: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PackInstallResult {
    pub task_id: Option<TaskId>,
    pub installing: Vec<String>,
    pub already_installed: Vec<String>,
    pub unavailable: Vec<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OptifineReport {
    pub file_name: String,
    pub features: Vec<OptifineFeature>,
    pub missing: Vec<PackMod>,
}

fn pack(id: &str) -> CoreResult<&'static PackDefinition> {
    PACKS
        .iter()
        .find(|pack| pack.id == id)
        .ok_or_else(|| CoreError::NotFound(format!("content pack {id}")))
}

fn feature_of(entry: &str) -> Option<OptifineFeature> {
    let entry = entry.to_ascii_lowercase().replace('\\', "/");
    let contains = |parts: &[&str]| {
        parts.iter().any(|part| {
            entry.contains(&format!("/optifine/{part}/"))
                || entry.contains(&format!("/mcpatcher/{part}/"))
        })
    };
    if !entry.starts_with("assets/") {
        return None;
    }
    if contains(&["ctm"]) {
        Some(OptifineFeature::ConnectedTextures)
    } else if contains(&["cem"]) {
        Some(OptifineFeature::EntityModels)
    } else if contains(&["cit"]) {
        Some(OptifineFeature::ItemTextures)
    } else if contains(&["random", "mob"]) {
        Some(OptifineFeature::RandomEntities)
    } else if contains(&["sky"]) {
        Some(OptifineFeature::Sky)
    } else if contains(&["anim"]) {
        Some(OptifineFeature::Animations)
    } else if entry.ends_with("/optifine/emissive.properties") {
        Some(OptifineFeature::Emissive)
    } else {
        None
    }
}

fn collect_features(names: impl Iterator<Item = String>) -> Vec<OptifineFeature> {
    let mut found: Vec<OptifineFeature> = Vec::new();
    for name in names.take(MAX_SCANNED_ENTRIES) {
        if let Some(feature) = feature_of(&name)
            && !found.contains(&feature)
        {
            found.push(feature);
        }
    }
    found.sort_by_key(|feature| *feature as u8);
    found
}

fn zip_features(path: &Path) -> Vec<OptifineFeature> {
    let Ok(file) = std::fs::File::open(path) else {
        return Vec::new();
    };
    let Ok(archive) = zip::ZipArchive::new(file) else {
        return Vec::new();
    };
    collect_features(
        archive
            .file_names()
            .filter_map(|name| name.ok().map(|name| name.into_owned())),
    )
}

fn folder_features(root: &Path) -> Vec<OptifineFeature> {
    let mut names = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if entry.file_type().is_ok_and(|t| t.is_dir()) {
                stack.push(path.clone());
            }
            if let Ok(relative) = path.strip_prefix(root) {
                let mut name = relative.to_string_lossy().replace('\\', "/");
                if path.is_dir() {
                    name.push('/');
                }
                names.push(name);
            }
            if names.len() >= MAX_SCANNED_ENTRIES {
                return collect_features(names.into_iter());
            }
        }
    }
    collect_features(names.into_iter())
}

fn scan_resourcepacks(game_dir: &Path) -> CoreResult<Vec<(String, Vec<OptifineFeature>)>> {
    let dir = game_dir.join(RESOURCEPACKS_DIR);
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err).at_path(&dir),
    };
    let mut reports = Vec::new();
    for entry in entries.flatten() {
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        let path = entry.path();
        let features = if entry.file_type().is_ok_and(|t| t.is_dir()) {
            folder_features(&path)
        } else if name.to_ascii_lowercase().ends_with(".zip") {
            zip_features(&path)
        } else {
            continue;
        };
        if !features.is_empty() {
            reports.push((name, features));
        }
    }
    reports.sort_by_key(|report| report.0.to_lowercase());
    Ok(reports)
}

async fn tag_pack(mods: &ModsDir, project_ids: &[String], pack_id: &str) -> CoreResult<()> {
    let mut file = mods.load().await?;
    for entry in &mut file.mods {
        if !entry.dependency && project_ids.contains(&entry.project_id) {
            entry.pack = Some(pack_id.to_string());
        }
    }
    mods.save(&file).await
}

impl Services {
    pub async fn content_packs(&self, instance_id: &str) -> CoreResult<Vec<ContentPack>> {
        let instance = self.instances.get(instance_id).await?;
        let installed = self.mods_dir(&instance)?.load().await?.mods;
        Ok(PACKS
            .iter()
            .map(|pack| ContentPack {
                id: pack.id.clone(),
                mods: pack.mods.clone(),
                installed: installed
                    .iter()
                    .any(|m| m.pack.as_deref() == Some(pack.id.as_str())),
            })
            .collect())
    }

    pub async fn install_content_pack(
        &self,
        instance_id: &str,
        pack_id: &str,
        only: Option<Vec<String>>,
    ) -> CoreResult<PackInstallResult> {
        let definition = pack(pack_id)?;
        let instance = self.instances.get(instance_id).await?;
        self.ensure_stopped(&instance)?;
        let loaders = content_loaders(&instance, ContentKind::Mod)?;
        let installed = self.mods_dir(&instance)?.load().await?.mods;
        let wanted: Vec<&PackMod> = definition
            .mods
            .iter()
            .filter(|m| only.as_ref().is_none_or(|only| only.contains(&m.slug)))
            .collect();

        let lookups = wanted.iter().map(|item| {
            let loaders = loaders.clone();
            let game_version = instance.minecraft_version.clone();
            let client = self.modrinth.clone();
            async move {
                let versions = client
                    .project_versions(&item.slug, &game_version, &loaders)
                    .await;
                (*item, versions)
            }
        });
        let mut result = PackInstallResult {
            task_id: None,
            installing: Vec::new(),
            already_installed: Vec::new(),
            unavailable: Vec::new(),
        };
        let mut to_install = Vec::new();
        for (item, versions) in futures::future::join_all(lookups).await {
            match versions {
                Ok(versions) if !versions.is_empty() => {
                    let project_id = &versions[0].project_id;
                    if installed.iter().any(|m| &m.project_id == project_id) {
                        result.already_installed.push(item.name.clone());
                    } else {
                        to_install.push(item.slug.clone());
                        result.installing.push(item.name.clone());
                    }
                }
                Ok(_) | Err(CoreError::NotFound(_)) => result.unavailable.push(item.name.clone()),
                Err(err) => return Err(err),
            }
        }
        if to_install.is_empty() {
            return Ok(result);
        }

        let services = self.clone();
        let pack_id = definition.id.clone();
        let task_id = self.tasks.spawn(
            TaskSpec {
                kind: TaskKind::InstallPack,
                subject: pack_id.clone(),
                instance_id: Some(instance.id.clone()),
            },
            move |progress| async move {
                let _guard = services.mods_lock.lock().await;
                let mods_dir = services.mods_dir(&instance)?;
                let target = ModTarget {
                    game_version: &instance.minecraft_version,
                    loaders: content_loaders(&instance, ContentKind::Mod)?,
                    kind: ContentKind::Mod,
                };
                let mut added = Vec::new();
                for slug in &to_install {
                    progress.check_cancelled()?;
                    let entries = install(
                        &services.http,
                        &services.modrinth,
                        &mods_dir,
                        &target,
                        slug,
                        None,
                        &progress,
                    )
                    .await?;
                    added.extend(entries.into_iter().map(|entry| entry.project_id));
                    tag_pack(&mods_dir, &added, &pack_id).await?;
                }
                Ok(())
            },
        )?;
        result.task_id = Some(task_id);
        Ok(result)
    }

    pub async fn remove_content_pack(&self, instance_id: &str, pack_id: &str) -> CoreResult<u32> {
        pack(pack_id)?;
        let instance = self.instances.get(instance_id).await?;
        self.ensure_stopped(&instance)?;
        let _guard = self.mods_lock.lock().await;
        let mods = self.mods_dir(&instance)?;
        let mut file = mods.load().await?;
        let (removed, kept): (Vec<_>, Vec<_>) = file
            .mods
            .into_iter()
            .partition(|m| m.pack.as_deref() == Some(pack_id));
        for entry in &removed {
            mods.delete_files(entry).await?;
        }
        file.mods = kept;
        mods.save(&file).await?;
        Ok(removed.len() as u32)
    }

    pub async fn scan_optifine_packs(&self, instance_id: &str) -> CoreResult<Vec<OptifineReport>> {
        let instance = self.instances.get(instance_id).await?;
        let game_dir = self.instances.game_dir(&instance.id)?;
        let scanned = tokio::task::spawn_blocking(move || scan_resourcepacks(&game_dir))
            .await
            .map_err(|err| {
                CoreError::Unsupported(format!("resource pack scan panicked: {err}"))
            })??;
        if scanned.is_empty() {
            return Ok(Vec::new());
        }
        let candidates: Vec<&PackMod> = pack("optifine")?
            .mods
            .iter()
            .filter(|m| !m.features.is_empty())
            .collect();
        let slugs: Vec<String> = candidates.iter().map(|m| m.slug.clone()).collect();
        let projects = self.modrinth.projects(&slugs).await?;
        let installed: HashSet<String> = self
            .mods_dir(&instance)?
            .load()
            .await?
            .mods
            .into_iter()
            .map(|m| m.project_id)
            .collect();
        let is_installed = |slug: &str| {
            projects
                .iter()
                .find(|p| p.slug.as_deref() == Some(slug) || p.id == slug)
                .is_some_and(|p| installed.contains(&p.id))
        };
        Ok(scanned
            .into_iter()
            .map(|(file_name, features)| OptifineReport {
                missing: candidates
                    .iter()
                    .filter(|m| m.features.iter().any(|f| features.contains(f)))
                    .filter(|m| !is_installed(&m.slug))
                    .map(|m| (*m).clone())
                    .collect(),
                file_name,
                features,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn bundled_packs_parse_and_are_unique() {
        let ids: Vec<&str> = PACKS.iter().map(|pack| pack.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "performance",
                "optifine",
                "shader_ready",
                "low_end",
                "quality_of_life"
            ]
        );
        for pack in PACKS.iter() {
            let slugs: HashSet<&str> = pack.mods.iter().map(|m| m.slug.as_str()).collect();
            assert_eq!(slugs.len(), pack.mods.len(), "{}", pack.id);
        }
        assert!(pack("missing").is_err());
    }

    #[test]
    fn detects_optifine_features() {
        let names = [
            "pack.mcmeta",
            "assets/minecraft/optifine/ctm/glass/1.png",
            "assets/minecraft/optifine/cem/creeper.jem",
            "assets/minecraft/mcpatcher/cit/sword.properties",
            "assets/minecraft/optifine/random/entity/cow2.png",
            "assets/minecraft/optifine/emissive.properties",
            "assets/minecraft/optifine/sky/world0/sky1.properties",
            "assets/minecraft/optifine/ctm/stone/2.png",
            "assets/minecraft/textures/block/stone.png",
            "optifine/ctm/outside_assets.png",
        ];
        assert_eq!(
            collect_features(names.iter().map(|n| n.to_string())),
            [
                OptifineFeature::ConnectedTextures,
                OptifineFeature::EntityModels,
                OptifineFeature::RandomEntities,
                OptifineFeature::Emissive,
                OptifineFeature::ItemTextures,
                OptifineFeature::Sky,
            ]
        );
    }

    #[test]
    fn scans_zip_and_folder_packs() {
        let dir = tempfile::tempdir().unwrap();
        let packs = dir.path().join(RESOURCEPACKS_DIR);
        std::fs::create_dir_all(packs.join("Folder Pack/assets/minecraft/optifine/anim")).unwrap();
        std::fs::write(
            packs.join("Folder Pack/assets/minecraft/optifine/anim/fire.properties"),
            "",
        )
        .unwrap();
        let mut zip = zip::ZipWriter::new(std::fs::File::create(packs.join("ctm.zip")).unwrap());
        zip.start_file(
            "assets/minecraft/optifine/ctm/glass/0.png",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zip.write_all(b"x").unwrap();
        zip.finish().unwrap();
        let mut plain =
            zip::ZipWriter::new(std::fs::File::create(packs.join("plain.zip")).unwrap());
        plain
            .start_file("pack.mcmeta", zip::write::SimpleFileOptions::default())
            .unwrap();
        plain.finish().unwrap();

        let reports = scan_resourcepacks(dir.path()).unwrap();
        assert_eq!(
            reports,
            [
                (
                    "ctm.zip".to_string(),
                    vec![OptifineFeature::ConnectedTextures]
                ),
                ("Folder Pack".to_string(), vec![OptifineFeature::Animations]),
            ]
        );
        assert!(
            scan_resourcepacks(&dir.path().join("missing"))
                .unwrap()
                .is_empty()
        );
    }
}

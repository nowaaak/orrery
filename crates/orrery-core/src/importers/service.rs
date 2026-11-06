use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::{
    CopyPlan, ForeignInstance, ForeignLoader, ImportFolder, ImportPreview, ImportSource,
    ImportTarget, ImportWarning, Locations, Selection, execute_copy, extract_archive, inspect,
    official, plan_copy,
};
use crate::error::IoResultExt;
use crate::instances::mods::ModsDir;
use crate::instances::{Instance, InstanceUpdate, LoaderSpec, NewInstance};
use crate::loaders::{self, forge, neoforge};
use crate::modpacks::index_content;
use crate::progress::{Progress, Stage};
use crate::services::Services;
use crate::tasks::{TaskId, TaskKind, TaskSpec};
use crate::{CoreError, CoreResult};

const EXTRACTED_DIR: &str = "imports";
const MAX_MEMORY_MB: u32 = 65536;
const MIN_MEMORY_MB: u32 = 512;
const STALE_EXTRACT: std::time::Duration = std::time::Duration::from_secs(24 * 60 * 60);

fn prune_extracted(root: &Path) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let stale = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age > STALE_EXTRACT);
        if stale && let Err(err) = std::fs::remove_dir_all(entry.path()) {
            tracing::warn!(error = %err, "failed to remove an old extracted import");
        }
    }
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> CoreResult<T> + Send + 'static,
) -> CoreResult<T> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|err| CoreError::Unsupported(format!("import worker failed: {err}")))?
}

fn selection(instance: &ForeignInstance) -> Selection {
    match (instance.source, instance.loader.uses_mods()) {
        (ImportSource::Official, true) => Selection::Only(&official::COPIED_WITH_MODS),
        (ImportSource::Official, false) => Selection::Only(&official::COPIED),
        _ => Selection::Everything,
    }
}

fn skip_metadata(instance: &ForeignInstance, plan: &mut CopyPlan) {
    let metadata: &[&str] = match instance.source {
        ImportSource::CurseForge => &["minecraftinstance.json"],
        ImportSource::AtLauncher => &["instance.json"],
        ImportSource::GdLauncher => &["config.json", "instance.json"],
        ImportSource::ModrinthApp => &["profile.json"],
        _ => &[],
    };
    plan.entries.retain(|entry| {
        let top = entry.relative.components().count() == 1;
        !(top
            && metadata
                .iter()
                .any(|name| entry.relative == Path::new(name)))
    });
}

fn folders(plan: &CopyPlan) -> Vec<ImportFolder> {
    let mut grouped: BTreeMap<String, (u32, u64)> = BTreeMap::new();
    for entry in &plan.entries {
        let mut parts = entry.relative.components();
        let top = parts
            .next()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .unwrap_or_default();
        let name = if parts.next().is_some() {
            top
        } else {
            String::new()
        };
        let slot = grouped.entry(name).or_default();
        slot.0 += 1;
        slot.1 += entry.size;
    }
    let mut folders: Vec<ImportFolder> = grouped
        .into_iter()
        .map(|(name, (files, bytes))| ImportFolder { name, files, bytes })
        .collect();
    folders.sort_by_key(|folder| std::cmp::Reverse(folder.bytes));
    folders
}

fn warnings(
    instance: &ForeignInstance,
    version: Option<&str>,
    plan: &CopyPlan,
) -> Vec<ImportWarning> {
    let mut warnings = Vec::new();
    if loader_spec(&instance.loader, version).is_none() {
        warnings.push(ImportWarning::LoaderNotSupported {
            loader: instance.loader.clone(),
        });
    }
    if version.is_none() {
        warnings.push(ImportWarning::UnknownVersion);
    }
    if let Some(path) = &instance.settings.java_path {
        warnings.push(ImportWarning::JavaNotTaken { path: path.clone() });
    }
    if let Some(args) = &instance.settings.jvm_args {
        warnings.push(ImportWarning::JvmArgsNotTaken { args: args.clone() });
    }
    if !instance.settings.commands.is_empty() {
        warnings.push(ImportWarning::CommandsNotTaken {
            commands: instance.settings.commands.clone(),
        });
    }
    if plan.skipped_links > 0 {
        warnings.push(ImportWarning::SkippedLinks {
            count: plan.skipped_links,
        });
    }
    if plan.skipped_unsafe > 0 {
        warnings.push(ImportWarning::SkippedUnsafe {
            count: plan.skipped_unsafe,
        });
    }
    warnings
}

fn forge_version(version: &str, minecraft: &str) -> Option<String> {
    let mut rest = version.trim();
    for prefix in [
        format!("{minecraft}-forge{minecraft}-"),
        format!("{minecraft}-forge-"),
        format!("{minecraft}-"),
        "forge-".to_string(),
    ] {
        if let Some(stripped) = rest.strip_prefix(prefix.as_str()) {
            rest = stripped;
            break;
        }
    }
    let suffix = format!("-{minecraft}");
    let rest = rest.strip_suffix(suffix.as_str()).unwrap_or(rest);
    let numeric = rest
        .split('.')
        .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()));
    numeric.then(|| rest.to_string())
}

pub(crate) fn loader_spec(loader: &ForeignLoader, minecraft: Option<&str>) -> Option<LoaderSpec> {
    match loader {
        ForeignLoader::Vanilla => Some(LoaderSpec::Vanilla),
        ForeignLoader::Fabric { version } if !version.is_empty() => {
            let loader_version = version.clone();
            Some(match minecraft {
                Some(game) if loaders::fabric_needs_legacy(game) => {
                    LoaderSpec::LegacyFabric { loader_version }
                }
                _ => LoaderSpec::Fabric { loader_version },
            })
        }
        ForeignLoader::Quilt { version } if !version.is_empty() => Some(LoaderSpec::Quilt {
            loader_version: version.clone(),
        }),
        ForeignLoader::Forge { version } => {
            let game = minecraft.filter(|game| forge::supported(game))?;
            forge_version(version, game).map(|loader_version| LoaderSpec::Forge { loader_version })
        }
        ForeignLoader::NeoForge { version } => {
            let version = version.trim();
            let version = version.strip_prefix("neoforge-").unwrap_or(version);
            (neoforge::minecraft_version(version).as_deref() == minecraft).then(|| {
                LoaderSpec::NeoForge {
                    loader_version: version.to_string(),
                }
            })
        }
        _ => None,
    }
}

impl Services {
    fn extracted_root(&self) -> PathBuf {
        self.paths.cache.join(EXTRACTED_DIR)
    }

    pub async fn scan_imports(&self) -> CoreResult<Vec<ForeignInstance>> {
        blocking(|| Ok(super::scan_defaults(&Locations::current()))).await
    }

    pub async fn inspect_import(&self, path: PathBuf) -> CoreResult<Vec<ForeignInstance>> {
        let metadata = tokio::fs::metadata(&path).await.at_path(&path)?;
        if metadata.is_dir() {
            return blocking(move || Ok(inspect(&path, &Locations::current()))).await;
        }
        let is_zip = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("zip"));
        if !is_zip {
            return Err(CoreError::InvalidInput(
                "choose an instance folder or a .zip archive".into(),
            ));
        }
        let root = self.extracted_root();
        let prune = root.clone();
        blocking(move || {
            prune_extracted(&prune);
            Ok(())
        })
        .await?;
        let target = root.join(uuid::Uuid::new_v4().simple().to_string());
        let archive = path.clone();
        let extracted = target.clone();
        blocking(move || extract_archive(&archive, &extracted)).await?;
        let root = target.clone();
        let found = blocking(move || {
            let locations = Locations::default();
            let mut found = inspect(&root, &locations);
            if found.is_empty() {
                let children: Vec<PathBuf> = std::fs::read_dir(&root)
                    .at_path(&root)?
                    .flatten()
                    .map(|entry| entry.path())
                    .filter(|p| p.is_dir())
                    .collect();
                if let [only] = children.as_slice() {
                    found = inspect(only, &locations);
                }
            }
            Ok(found)
        })
        .await?;
        if found.is_empty() {
            let _ = tokio::fs::remove_dir_all(&target).await;
            return Err(CoreError::InvalidInput(
                "the archive does not contain an instance from a known launcher".into(),
            ));
        }
        Ok(found)
    }

    async fn find_import(&self, target: &ImportTarget) -> CoreResult<ForeignInstance> {
        let path = PathBuf::from(&target.path);
        let profile = target.profile.clone();
        let found = blocking(move || Ok(inspect(&path, &Locations::current()))).await?;
        found
            .into_iter()
            .find(|instance| instance.profile == profile)
            .ok_or_else(|| CoreError::NotFound(format!("instance at {}", target.path)))
    }

    async fn resolve_version(&self, instance: &ForeignInstance) -> Option<String> {
        if let Some(version) = &instance.minecraft_version {
            return Some(version.clone());
        }
        let manifest = self.meta.manifest(false).await.ok()?;
        match instance.latest.as_deref() {
            Some("snapshot") => Some(manifest.latest.snapshot),
            Some(_) => Some(manifest.latest.release),
            None => None,
        }
    }

    async fn copy_plan(&self, instance: &ForeignInstance) -> CoreResult<CopyPlan> {
        let game_dir = PathBuf::from(&instance.game_dir);
        let chosen = selection(instance);
        let mut plan = blocking(move || plan_copy(&game_dir, chosen)).await?;
        skip_metadata(instance, &mut plan);
        Ok(plan)
    }

    pub async fn preview_import(&self, target: ImportTarget) -> CoreResult<ImportPreview> {
        let instance = self.find_import(&target).await?;
        let version = self.resolve_version(&instance).await;
        let plan = self.copy_plan(&instance).await?;
        Ok(ImportPreview {
            warnings: warnings(&instance, version.as_deref(), &plan),
            folders: folders(&plan),
            files: plan.entries.len() as u32,
            bytes: plan.bytes(),
            minecraft_version: version,
            instance,
        })
    }

    pub async fn start_import(
        &self,
        target: ImportTarget,
        name: Option<String>,
    ) -> CoreResult<TaskId> {
        let instance = self.find_import(&target).await?;
        let version = self.resolve_version(&instance).await.ok_or_else(|| {
            CoreError::InvalidInput("the Minecraft version of the instance is unknown".into())
        })?;
        let name = name
            .map(|name| name.trim().to_string())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| instance.name.clone());
        let services = self.clone();
        self.tasks.spawn(
            TaskSpec {
                kind: TaskKind::ImportInstance,
                subject: name.clone(),
                instance_id: None,
            },
            move |progress| async move {
                let result = services
                    .run_import(&instance, &version, &name, &progress)
                    .await;
                services.forget_extracted(Path::new(&instance.path)).await;
                result.map(|_| ())
            },
        )
    }

    async fn forget_extracted(&self, path: &Path) {
        let root = self.extracted_root();
        let Some(first) = path
            .strip_prefix(&root)
            .ok()
            .and_then(|rest| rest.components().next())
        else {
            return;
        };
        let dir = root.join(first);
        if let Err(err) = tokio::fs::remove_dir_all(&dir).await {
            tracing::warn!(error = %err, "failed to remove an extracted import");
        }
    }

    async fn run_import(
        &self,
        foreign: &ForeignInstance,
        version: &str,
        name: &str,
        progress: &Arc<Progress>,
    ) -> CoreResult<Instance> {
        progress.set_stage(Stage::Preparing);
        let plan = self.copy_plan(foreign).await?;
        let instance = self
            .instances
            .create(NewInstance {
                name: name.to_string(),
                minecraft_version: version.to_string(),
                loader: loader_spec(&foreign.loader, Some(version)).unwrap_or(LoaderSpec::Vanilla),
                copy_settings_from: None,
            })
            .await?;
        match self.fill_import(&instance, foreign, plan, progress).await {
            Ok(()) => self.instances.get(&instance.id).await,
            Err(err) => {
                if let Err(cleanup) = self.instances.delete(&instance.id).await {
                    tracing::warn!(error = %cleanup, "failed to remove an incomplete import");
                }
                Err(err)
            }
        }
    }

    async fn fill_import(
        &self,
        instance: &Instance,
        foreign: &ForeignInstance,
        plan: CopyPlan,
        progress: &Arc<Progress>,
    ) -> CoreResult<()> {
        if let Some(max) = foreign
            .settings
            .max_memory_mb
            .filter(|max| (MIN_MEMORY_MB..=MAX_MEMORY_MB).contains(max))
        {
            let mut java = instance.java.clone();
            java.max_memory_mb = Some(max);
            java.min_memory_mb = foreign
                .settings
                .min_memory_mb
                .filter(|min| *min >= 256 && *min <= max);
            self.instances
                .update(
                    &instance.id,
                    InstanceUpdate {
                        name: instance.name.clone(),
                        minecraft_version: instance.minecraft_version.clone(),
                        loader: instance.loader.clone(),
                        java,
                        resolution: instance.resolution,
                        launch: instance.launch.clone(),
                    },
                )
                .await?;
        }
        progress.set_stage(Stage::ExtractingFiles);
        progress.add_totals(plan.entries.len() as u64, plan.bytes());
        let source = PathBuf::from(&foreign.game_dir);
        let game_dir = self.instances.game_dir(&instance.id)?;
        let target = game_dir.clone();
        let worker_progress = progress.clone();
        blocking(move || execute_copy(&source, &plan, &target, &worker_progress)).await?;
        progress.check_cancelled()?;
        if foreign.loader.uses_mods() {
            progress.set_stage(Stage::FetchingMetadata);
            let mods = ModsDir::new(self.instances.instance_dir(&instance.id)?, &game_dir);
            if let Err(err) = index_content(&self.modrinth, &mods, &game_dir).await {
                tracing::warn!(error = %err, "could not identify the imported mods");
            }
        }
        progress.set_stage(Stage::Finished);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::importers::ForeignSettings;

    fn foreign(source: ImportSource, loader: ForeignLoader) -> ForeignInstance {
        ForeignInstance {
            source,
            path: String::new(),
            profile: None,
            name: "Test".into(),
            game_dir: String::new(),
            minecraft_version: Some("1.20.1".into()),
            latest: None,
            loader,
            settings: ForeignSettings {
                java_path: Some("C:/Java/bin/javaw.exe".into()),
                jvm_args: None,
                min_memory_mb: None,
                max_memory_mb: None,
                commands: vec!["pre".into()],
            },
        }
    }

    #[test]
    fn warns_about_everything_that_is_not_taken_over() {
        let forge = foreign(
            ImportSource::CurseForge,
            ForeignLoader::Forge {
                version: "36.2.0".into(),
            },
        );
        let found = warnings(&forge, Some("1.16.5"), &CopyPlan::default());
        assert!(matches!(found[0], ImportWarning::LoaderNotSupported { .. }));
        assert!(
            found
                .iter()
                .any(|w| matches!(w, ImportWarning::JavaNotTaken { .. }))
        );
        assert!(
            found
                .iter()
                .any(|w| matches!(w, ImportWarning::CommandsNotTaken { .. }))
        );
        assert_eq!(loader_spec(&forge.loader, Some("1.16.5")), None);
        assert!(
            !warnings(&forge, Some("1.12.2"), &CopyPlan::default())
                .iter()
                .any(|w| matches!(w, ImportWarning::LoaderNotSupported { .. }))
        );
        assert!(
            warnings(&forge, None, &CopyPlan::default()).contains(&ImportWarning::UnknownVersion)
        );
    }

    #[test]
    fn maps_foreign_loaders_to_orrery_loaders() {
        let spec = |loader: ForeignLoader, game: &str| loader_spec(&loader, Some(game));
        let fabric = |version: &str| ForeignLoader::Fabric {
            version: version.into(),
        };
        let forge = |version: &str| ForeignLoader::Forge {
            version: version.into(),
        };
        let neoforge = |version: &str| ForeignLoader::NeoForge {
            version: version.into(),
        };
        assert_eq!(
            spec(fabric("0.16.9"), "1.21.1"),
            Some(LoaderSpec::Fabric {
                loader_version: "0.16.9".into()
            })
        );
        assert_eq!(
            spec(fabric("0.20.1"), "1.8.9"),
            Some(LoaderSpec::LegacyFabric {
                loader_version: "0.20.1".into()
            })
        );
        let forge_spec = |version: &str| {
            Some(LoaderSpec::Forge {
                loader_version: version.into(),
            })
        };
        assert_eq!(spec(forge("47.2.0"), "1.20.1"), forge_spec("47.2.0"));
        assert_eq!(spec(forge("1.20.1-47.2.0"), "1.20.1"), forge_spec("47.2.0"));
        assert_eq!(spec(forge("forge-47.2.0"), "1.20.1"), forge_spec("47.2.0"));
        assert_eq!(
            spec(forge("1.8.9-forge1.8.9-11.15.1.2318-1.8.9"), "1.8.9"),
            forge_spec("11.15.1.2318")
        );
        assert_eq!(spec(forge("36.2.0"), "1.16.5"), None);
        assert_eq!(spec(forge("unknown"), "1.20.1"), None);
        assert_eq!(
            spec(neoforge("21.1.77"), "1.21.1"),
            Some(LoaderSpec::NeoForge {
                loader_version: "21.1.77".into()
            })
        );
        assert!(spec(neoforge("neoforge-21.1.77"), "1.21.1").is_some());
        assert_eq!(spec(neoforge("47.1.106"), "1.20.1"), None);
        assert_eq!(
            spec(
                ForeignLoader::Other {
                    name: "liteloader".into()
                },
                "1.12.2"
            ),
            None
        );
        assert_eq!(
            spec(ForeignLoader::Vanilla, "1.12.2"),
            Some(LoaderSpec::Vanilla)
        );
    }

    #[test]
    fn groups_files_by_top_folder_and_skips_metadata() {
        let dir = tempfile::tempdir().unwrap();
        for (path, bytes) in [
            ("mods/a.jar", 10),
            ("mods/b.jar", 5),
            ("options.txt", 3),
            ("minecraftinstance.json", 2),
        ] {
            let file = dir.path().join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, vec![0u8; bytes]).unwrap();
        }
        let instance = foreign(ImportSource::CurseForge, ForeignLoader::Vanilla);
        let mut plan = plan_copy(dir.path(), selection(&instance)).unwrap();
        skip_metadata(&instance, &mut plan);
        let grouped = folders(&plan);
        assert_eq!(
            grouped[0],
            ImportFolder {
                name: "mods".into(),
                files: 2,
                bytes: 15
            }
        );
        assert_eq!(
            grouped[1],
            ImportFolder {
                name: String::new(),
                files: 1,
                bytes: 3
            }
        );
    }
}

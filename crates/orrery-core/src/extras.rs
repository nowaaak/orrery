use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::curseforge::install::{CfTarget, Clients};
use crate::curseforge::models::parse_project_key;
use crate::curseforge::{self, updates as curseforge_updates};
use crate::error::IoResultExt;
use crate::game_options;
use crate::instances::LoaderSpec;
use crate::instances::mods::{ContentKind, InstalledMod, ModsDir};
use crate::menupack::{self, BUNDLED_MENU_ID};
use crate::modrinth::install::{ModTarget, install};
use crate::modrinth::service::content_loaders;
use crate::services::Services;
use crate::shaderpack::{self, Activation, BUNDLED_SHADER_ID};
use crate::tasks::{TaskId, TaskKind, TaskSpec};
use crate::tuning::{self, SettingsRecommendation, Workload};
use crate::{CoreError, CoreResult};

const SHADER_MODS: [&str; 2] = ["iris", "oculus"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum InstanceFolder {
    Root,
    Mods,
    Saves,
    Screenshots,
    Resourcepacks,
    Shaderpacks,
    Config,
    Logs,
    CrashReports,
}

impl InstanceFolder {
    fn relative(self) -> Option<&'static str> {
        match self {
            Self::Root => None,
            Self::Mods => Some("mods"),
            Self::Saves => Some("saves"),
            Self::Screenshots => Some("screenshots"),
            Self::Resourcepacks => Some("resourcepacks"),
            Self::Shaderpacks => Some("shaderpacks"),
            Self::Config => Some("config"),
            Self::Logs => Some("logs"),
            Self::CrashReports => Some("crash-reports"),
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ModUpdate {
    pub project_id: String,
    pub title: String,
    pub current_version: String,
    pub version_id: String,
    pub version_number: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ModUpdateRequest {
    pub project_id: String,
    pub version_id: String,
}

pub(crate) fn uses_shaders(mods: &[InstalledMod]) -> bool {
    mods.iter().any(|m| {
        m.enabled && {
            let title = m.title.to_lowercase();
            SHADER_MODS.iter().any(|name| title.starts_with(name))
        }
    })
}

impl Services {
    fn instance_mods(&self, id: &str) -> CoreResult<ModsDir> {
        Ok(ModsDir::new(
            self.instances.instance_dir(id)?,
            &self.instances.game_dir(id)?,
        ))
    }

    pub async fn recommend_settings(
        &self,
        instance_id: &str,
    ) -> CoreResult<SettingsRecommendation> {
        let instance = self.instances.get(instance_id).await?;
        let system = tuning::system_memory_mb().ok_or_else(|| {
            CoreError::Unsupported("the system memory size could not be determined".into())
        })?;
        let mods = self.instance_mods(&instance.id)?.load().await?.mods;
        let modded = !matches!(instance.loader, LoaderSpec::Vanilla);
        let workload = Workload {
            minecraft_version: &instance.minecraft_version,
            modded,
            mod_count: mods.iter().filter(|m| m.enabled).count() as u32,
            shaders: uses_shaders(&mods),
        };
        Ok(tuning::recommend(system, &workload))
    }

    pub async fn instance_folder(
        &self,
        instance_id: &str,
        folder: InstanceFolder,
    ) -> CoreResult<PathBuf> {
        self.instances.get(instance_id).await?;
        let game_dir = self.instances.game_dir(instance_id)?;
        let path = match folder.relative() {
            Some(relative) => game_dir.join(relative),
            None => game_dir,
        };
        tokio::fs::create_dir_all(&path).await.at_path(&path)?;
        Ok(path)
    }

    pub async fn export_game_log(&self, instance_id: &str, target: &Path) -> CoreResult<()> {
        let lines = self.games.log_tail(instance_id);
        if lines.is_empty() {
            return Err(CoreError::NotFound(format!(
                "log output for instance {instance_id}"
            )));
        }
        let mut text = lines.join("\n");
        text.push('\n');
        tokio::fs::write(target, text).await.at_path(target)
    }

    async fn backfill_compatibility(&self, instance_id: &str) -> CoreResult<()> {
        let _guard = self.mods_lock.lock().await;
        let mods_dir = self.instance_mods(instance_id)?;
        let mut file = mods_dir.load().await?;
        let hashes: Vec<String> = file
            .mods
            .iter()
            .filter(|m| m.game_versions.is_empty() && !m.version_id.is_empty())
            .map(|m| m.sha512.clone())
            .collect();
        if hashes.is_empty() {
            return Ok(());
        }
        let versions = self.modrinth.versions_for_hashes(&hashes).await?;
        let mut changed = false;
        for installed in &mut file.mods {
            if let Some(version) = versions.get(&installed.sha512)
                && installed.game_versions.is_empty()
            {
                installed.game_versions = version.game_versions.clone();
                installed.loaders = version.loaders.clone();
                changed = true;
            }
        }
        if changed {
            mods_dir.save(&file).await?;
        }
        Ok(())
    }

    pub async fn check_mod_updates(&self, instance_id: &str) -> CoreResult<Vec<ModUpdate>> {
        let instance = self.instances.get(instance_id).await?;
        if let Err(err) = self.backfill_compatibility(&instance.id).await {
            tracing::warn!(instance = %instance.id, error = %err, "failed to look up content compatibility");
        }
        let mods = self.instance_mods(&instance.id)?.load().await?.mods;
        let mut updates: Vec<ModUpdate> = shaderpack::bundled_update(&mods)?.into_iter().collect();
        updates.extend(menupack::bundled_update(&mods));
        for kind in ContentKind::ALL {
            let items: Vec<&InstalledMod> = mods
                .iter()
                .filter(|m| {
                    m.kind == kind
                        && !m.pinned
                        && m.project_id != BUNDLED_SHADER_ID
                        && m.project_id != BUNDLED_MENU_ID
                })
                .collect();
            if items.is_empty() {
                continue;
            }
            let Ok(loaders) = content_loaders(&instance, kind) else {
                continue;
            };
            let (from_curseforge, items): (Vec<&InstalledMod>, Vec<&InstalledMod>) = items
                .into_iter()
                .partition(|m| parse_project_key(&m.project_id).is_some());
            if !from_curseforge.is_empty() && self.curseforge.is_available() {
                let loader = instance.loader.mod_loader().map(|(loader, _)| loader);
                match curseforge_updates::check(
                    &self.curseforge,
                    &instance.minecraft_version,
                    loader,
                    &from_curseforge,
                )
                .await
                {
                    Ok(found) => updates.extend(found),
                    Err(err) => {
                        tracing::warn!(instance = %instance.id, error = %err, "CurseForge update check failed");
                    }
                }
            }
            if items.is_empty() {
                continue;
            }
            let hashes: Vec<String> = items.iter().map(|m| m.sha512.clone()).collect();
            let latest = self
                .modrinth
                .latest_for_hashes(&hashes, &instance.minecraft_version, &loaders)
                .await?;
            updates.extend(items.iter().filter_map(|installed| {
                let version = latest.get(&installed.sha512)?;
                (version.id != installed.version_id && version.project_id == installed.project_id)
                    .then(|| ModUpdate {
                        project_id: installed.project_id.clone(),
                        title: installed.title.clone(),
                        current_version: installed.version_number.clone(),
                        version_id: version.id.clone(),
                        version_number: version.version_number.clone(),
                    })
            }));
        }
        Ok(updates)
    }

    pub async fn spawn_update_mods(
        &self,
        instance_id: &str,
        updates: Vec<ModUpdateRequest>,
    ) -> CoreResult<TaskId> {
        let instance = self.instances.get(instance_id).await?;
        self.ensure_stopped(&instance)?;
        let count = updates.len();
        let services = self.clone();
        self.tasks.spawn(
            TaskSpec {
                kind: TaskKind::UpdateMods,
                subject: count.to_string(),
                instance_id: Some(instance.id.clone()),
            },
            move |progress| async move {
                let _guard = services.mods_lock.lock().await;
                let mods_dir = services.instance_mods(&instance.id)?;
                let installed = mods_dir.load().await?.mods;
                for update in &updates {
                    progress.check_cancelled()?;
                    if update.project_id == BUNDLED_SHADER_ID {
                        shaderpack::install_bundled(
                            &mods_dir,
                            Activation::KeepSelection,
                            game_options::detect_hardware_tier(),
                        )
                        .await?;
                        continue;
                    }
                    if update.project_id == BUNDLED_MENU_ID {
                        menupack::install_bundled(
                            &mods_dir,
                            &services.menu_target(&instance, false),
                            Activation::KeepSelection,
                        )
                        .await?;
                        continue;
                    }
                    let kind = installed
                        .iter()
                        .find(|m| m.project_id == update.project_id)
                        .map(|m| m.kind)
                        .unwrap_or_default();
                    if let Some(mod_id) = parse_project_key(&update.project_id) {
                        let file_id = update.version_id.parse::<u32>().map_err(|_| {
                            CoreError::InvalidInput(format!(
                                "invalid CurseForge file id {:?}",
                                update.version_id
                            ))
                        })?;
                        let clients = Clients {
                            http: &services.http,
                            curseforge: &services.curseforge,
                            modrinth: &services.modrinth,
                        };
                        let target = CfTarget {
                            game_version: &instance.minecraft_version,
                            loader: instance.loader.mod_loader().map(|(loader, _)| loader),
                            kind,
                        };
                        curseforge::install::install(
                            &clients,
                            &mods_dir,
                            &target,
                            mod_id,
                            Some(file_id),
                            &progress,
                        )
                        .await?;
                        continue;
                    }
                    let target = ModTarget {
                        game_version: &instance.minecraft_version,
                        loaders: content_loaders(&instance, kind)?,
                        kind,
                    };
                    install(
                        &services.http,
                        &services.modrinth,
                        &mods_dir,
                        &target,
                        &update.project_id,
                        Some(&update.version_id),
                        &progress,
                    )
                    .await?;
                }
                Ok(())
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn installed(title: &str, enabled: bool) -> InstalledMod {
        InstalledMod {
            project_id: title.into(),
            version_id: "v".into(),
            title: title.into(),
            version_number: "1".into(),
            file_name: "x.jar".into(),
            sha512: "h".into(),
            enabled,
            dependency: false,
            icon_url: None,
            kind: ContentKind::Mod,
            game_versions: Vec::new(),
            loaders: Vec::new(),
            pack: None,
            pinned: false,
        }
    }

    #[test]
    fn shader_mods_are_detected_only_when_enabled() {
        assert!(uses_shaders(&[installed("Iris Shaders", true)]));
        assert!(!uses_shaders(&[installed("Iris Shaders", false)]));
        assert!(!uses_shaders(&[installed("Sodium", true)]));
    }

    #[test]
    fn folders_map_to_game_directories() {
        assert_eq!(
            InstanceFolder::CrashReports.relative(),
            Some("crash-reports")
        );
        assert_eq!(InstanceFolder::Root.relative(), None);
    }
}

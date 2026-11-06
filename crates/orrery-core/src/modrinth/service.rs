use super::install::{ModTarget, install};
use super::models::{ModSearchPage, ModVersion};
use crate::curseforge::install::{CfTarget, Clients};
use crate::curseforge::models::parse_project_key;
use crate::curseforge::{self, class_id};
use crate::instances::Instance;
use crate::instances::mods::{ContentKind, ContentSource, InstalledMod, ModsDir};
use crate::services::Services;
use crate::tasks::{TaskId, TaskKind, TaskSpec};
use crate::{CoreError, CoreResult};

pub fn content_loaders(instance: &Instance, kind: ContentKind) -> CoreResult<Vec<&'static str>> {
    kind.modrinth_loaders(instance.loader.mod_loader().map(|(loader, _)| loader))
        .ok_or_else(|| {
            CoreError::Unsupported(format!(
                "{} require a Fabric or Quilt instance",
                kind.project_type()
            ))
        })
}

fn curseforge_target(instance: &Instance, kind: ContentKind) -> CfTarget<'_> {
    CfTarget {
        game_version: &instance.minecraft_version,
        loader: instance.loader.mod_loader().map(|(loader, _)| loader),
        kind,
    }
}

fn parse_file_id(version_id: Option<&str>) -> CoreResult<Option<u32>> {
    version_id
        .map(|id| {
            id.parse::<u32>()
                .map_err(|_| CoreError::InvalidInput(format!("invalid CurseForge file id {id:?}")))
        })
        .transpose()
}

impl Services {
    pub fn curseforge_available(&self) -> bool {
        self.curseforge.is_available()
    }

    pub(crate) fn mods_dir(&self, instance: &Instance) -> CoreResult<ModsDir> {
        Ok(ModsDir::new(
            self.instances.instance_dir(&instance.id)?,
            &self.instances.game_dir(&instance.id)?,
        ))
    }

    pub async fn search_content(
        &self,
        instance_id: &str,
        source: ContentSource,
        kind: ContentKind,
        query: &str,
        offset: u32,
    ) -> CoreResult<ModSearchPage> {
        let instance = self.instances.get(instance_id).await?;
        let loaders = content_loaders(&instance, kind)?;
        if source == ContentSource::CurseForge {
            let target = curseforge_target(&instance, kind);
            let loader = target.loader.filter(|_| kind == ContentKind::Mod);
            return self
                .curseforge
                .search(
                    class_id(kind),
                    query,
                    Some(&instance.minecraft_version),
                    loader,
                    offset,
                )
                .await;
        }
        let categories: Vec<&str> = match kind {
            ContentKind::ResourcePack => Vec::new(),
            ContentKind::Mod | ContentKind::Shader => loaders,
        };
        self.modrinth
            .search(
                query,
                Some(&instance.minecraft_version),
                kind.project_type(),
                &categories,
                offset,
            )
            .await
    }

    pub async fn content_versions(
        &self,
        instance_id: &str,
        kind: ContentKind,
        project_id: &str,
    ) -> CoreResult<Vec<ModVersion>> {
        let instance = self.instances.get(instance_id).await?;
        let loaders = content_loaders(&instance, kind)?;
        if let Some(mod_id) = parse_project_key(project_id) {
            let target = curseforge_target(&instance, kind);
            let mut files: Vec<_> = self
                .curseforge
                .project_files(mod_id, Some(&instance.minecraft_version))
                .await?
                .into_iter()
                .filter(|file| target.accepts(file))
                .collect();
            files.sort_by(|a, b| b.file_date.cmp(&a.file_date).then(b.id.cmp(&a.id)));
            return Ok(files
                .iter()
                .map(|file| ModVersion {
                    id: file.id.to_string(),
                    project_id: project_id.to_string(),
                    name: file.display_name.clone(),
                    version_number: file.display_name.clone(),
                    version_type: file.release_label().to_string(),
                    date_published: file.file_date.clone(),
                })
                .collect());
        }
        let versions = self
            .modrinth
            .project_versions(project_id, &instance.minecraft_version, &loaders)
            .await?;
        Ok(versions.iter().map(ModVersion::from).collect())
    }

    pub async fn list_mods(&self, instance_id: &str) -> CoreResult<Vec<InstalledMod>> {
        let instance = self.instances.get(instance_id).await?;
        Ok(self.mods_dir(&instance)?.load().await?.mods)
    }

    pub async fn set_mod_enabled(
        &self,
        instance_id: &str,
        project_id: &str,
        enabled: bool,
    ) -> CoreResult<InstalledMod> {
        let instance = self.instances.get(instance_id).await?;
        self.ensure_stopped(&instance)?;
        let _guard = self.mods_lock.lock().await;
        self.mods_dir(&instance)?
            .set_enabled(project_id, enabled)
            .await
    }

    pub async fn set_mod_pinned(
        &self,
        instance_id: &str,
        project_id: &str,
        pinned: bool,
    ) -> CoreResult<InstalledMod> {
        let instance = self.instances.get(instance_id).await?;
        let _guard = self.mods_lock.lock().await;
        self.mods_dir(&instance)?
            .set_pinned(project_id, pinned)
            .await
    }

    pub async fn remove_mod(&self, instance_id: &str, project_id: &str) -> CoreResult<()> {
        let instance = self.instances.get(instance_id).await?;
        self.ensure_stopped(&instance)?;
        let _guard = self.mods_lock.lock().await;
        self.mods_dir(&instance)?.remove(project_id).await
    }

    pub async fn spawn_install_content(
        &self,
        instance_id: &str,
        kind: ContentKind,
        project_id: String,
        version_id: Option<String>,
        title: String,
    ) -> CoreResult<TaskId> {
        let instance = self.instances.get(instance_id).await?;
        content_loaders(&instance, kind)?;
        if parse_project_key(&project_id).is_some() {
            parse_file_id(version_id.as_deref())?;
            if !self.curseforge.is_available() {
                return Err(CoreError::Unsupported(
                    "this build has no CurseForge API key".into(),
                ));
            }
        }
        self.ensure_stopped(&instance)?;
        let services = self.clone();
        self.tasks.spawn(
            TaskSpec {
                kind: TaskKind::InstallMod,
                subject: title.to_string(),
                instance_id: Some(instance.id.clone()),
            },
            move |progress| async move {
                let _guard = services.mods_lock.lock().await;
                if let Some(mod_id) = parse_project_key(&project_id) {
                    let clients = Clients {
                        http: &services.http,
                        curseforge: &services.curseforge,
                        modrinth: &services.modrinth,
                    };
                    return curseforge::install::install(
                        &clients,
                        &services.mods_dir(&instance)?,
                        &curseforge_target(&instance, kind),
                        mod_id,
                        parse_file_id(version_id.as_deref())?,
                        &progress,
                    )
                    .await
                    .map(|_| ());
                }
                let target = ModTarget {
                    game_version: &instance.minecraft_version,
                    loaders: content_loaders(&instance, kind)?,
                    kind,
                };
                install(
                    &services.http,
                    &services.modrinth,
                    &services.mods_dir(&instance)?,
                    &target,
                    &project_id,
                    version_id.as_deref(),
                    &progress,
                )
                .await
                .map(|_| ())
            },
        )
    }

    pub(crate) fn ensure_stopped(&self, instance: &Instance) -> CoreResult<()> {
        if self.games.is_running(&instance.id) {
            return Err(CoreError::InvalidInput(
                "stop the game before changing installed content".into(),
            ));
        }
        Ok(())
    }
}

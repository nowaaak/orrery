use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use ts_rs::TS;

use crate::instances::Instance;
use crate::instances::mods::{ContentKind, InstalledMod};
use crate::modrinth::install::{ModTarget, install};
use crate::modrinth::models::ModSearchPage;
use crate::modrinth::service::content_loaders;
use crate::progress::Progress;
use crate::services::Services;
use crate::storage::{self, Versioned, unix_now};
use crate::tasks::{TaskId, TaskKind, TaskSpec};
use crate::{CoreError, CoreResult};

pub const SYNC_FILE: &str = "sync-packs.json";
const TAG_PREFIX: &str = "sync:";
const MAX_NAME_CHARS: usize = 48;
const MAX_ITEMS: usize = 300;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SyncItem {
    pub project_id: String,
    pub title: String,
    pub kind: ContentKind,
    #[serde(default)]
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SyncPack {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub items: Vec<SyncItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SyncSubscription {
    pub pack_id: String,
    #[serde(default)]
    pub excluded: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct SyncFile {
    schema: u32,
    #[serde(default)]
    packs: Vec<SyncPack>,
}

impl Versioned for SyncFile {
    const SCHEMA: u32 = 1;
    fn schema(&self) -> u32 {
        self.schema
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SyncReport {
    pub instance_id: String,
    pub task_id: Option<TaskId>,
    pub install: Vec<String>,
    pub remove: Vec<String>,
    pub waiting_for_stop: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncPlan {
    pub install: Vec<(String, SyncItem)>,
    pub remove: Vec<String>,
    pub retag: Vec<(String, String)>,
}

impl SyncPlan {
    pub fn is_empty(&self) -> bool {
        self.install.is_empty() && self.remove.is_empty() && self.retag.is_empty()
    }
}

pub fn tag(pack_id: &str) -> String {
    format!("{TAG_PREFIX}{pack_id}")
}

fn tagged_pack(entry: &InstalledMod) -> Option<&str> {
    entry.pack.as_deref()?.strip_prefix(TAG_PREFIX)
}

pub fn plan(
    packs: &[SyncPack],
    subscriptions: &[SyncSubscription],
    installed: &[InstalledMod],
) -> SyncPlan {
    let mut desired: Vec<(String, SyncItem)> = Vec::new();
    let mut seen = HashSet::new();
    for subscription in subscriptions {
        let Some(pack) = packs.iter().find(|pack| pack.id == subscription.pack_id) else {
            continue;
        };
        for item in &pack.items {
            if subscription.excluded.contains(&item.project_id) {
                continue;
            }
            if seen.insert(item.project_id.clone()) {
                desired.push((pack.id.clone(), item.clone()));
            }
        }
    }
    let owner: HashMap<&str, &str> = desired
        .iter()
        .map(|(pack, item)| (item.project_id.as_str(), pack.as_str()))
        .collect();
    let installed_ids: HashSet<&str> = installed.iter().map(|m| m.project_id.as_str()).collect();

    let install = desired
        .iter()
        .filter(|(_, item)| !installed_ids.contains(item.project_id.as_str()))
        .cloned()
        .collect();
    let mut remove = Vec::new();
    let mut retag = Vec::new();
    for entry in installed {
        let Some(current) = tagged_pack(entry) else {
            continue;
        };
        match owner.get(entry.project_id.as_str()) {
            None => remove.push(entry.project_id.clone()),
            Some(pack) if *pack != current => {
                retag.push((entry.project_id.clone(), pack.to_string()));
            }
            Some(_) => {}
        }
    }
    SyncPlan {
        install,
        remove,
        retag,
    }
}

fn validate_name(name: &str) -> CoreResult<String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME_CHARS {
        return Err(CoreError::InvalidInput(format!(
            "a sync pack name needs between 1 and {MAX_NAME_CHARS} characters"
        )));
    }
    Ok(name.to_string())
}

fn validate_project_id(id: &str) -> CoreResult<()> {
    let valid = !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if valid {
        Ok(())
    } else {
        Err(CoreError::InvalidInput(format!(
            "invalid project id {id:?}"
        )))
    }
}

pub struct SyncPackStore {
    path: PathBuf,
    lock: Mutex<()>,
}

impl SyncPackStore {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            lock: Mutex::new(()),
        }
    }

    async fn load(&self) -> CoreResult<SyncFile> {
        Ok(storage::read_versioned::<SyncFile>(&self.path)
            .await?
            .unwrap_or(SyncFile {
                schema: SyncFile::SCHEMA,
                packs: Vec::new(),
            }))
    }

    async fn save(&self, file: &SyncFile) -> CoreResult<()> {
        storage::write_json_atomic(&self.path, file).await
    }

    pub async fn list(&self) -> CoreResult<Vec<SyncPack>> {
        Ok(self.load().await?.packs)
    }

    async fn change<T>(&self, apply: impl FnOnce(&mut SyncFile) -> CoreResult<T>) -> CoreResult<T> {
        let _guard = self.lock.lock().await;
        let mut file = self.load().await?;
        let result = apply(&mut file)?;
        file.schema = SyncFile::SCHEMA;
        self.save(&file).await?;
        Ok(result)
    }

    pub async fn create(&self, name: &str) -> CoreResult<SyncPack> {
        let name = validate_name(name)?;
        self.change(|file| {
            let base = format!("pack-{}", unix_now());
            let id = (0..)
                .map(|n| {
                    if n == 0 {
                        base.clone()
                    } else {
                        format!("{base}-{n}")
                    }
                })
                .find(|id| !file.packs.iter().any(|pack| &pack.id == id))
                .unwrap_or(base);
            let pack = SyncPack {
                id,
                name,
                items: Vec::new(),
            };
            file.packs.push(pack.clone());
            Ok(pack)
        })
        .await
    }

    fn find<'a>(file: &'a mut SyncFile, id: &str) -> CoreResult<&'a mut SyncPack> {
        file.packs
            .iter_mut()
            .find(|pack| pack.id == id)
            .ok_or_else(|| CoreError::NotFound(format!("sync pack {id}")))
    }

    pub async fn rename(&self, id: &str, name: &str) -> CoreResult<SyncPack> {
        let name = validate_name(name)?;
        self.change(|file| {
            let pack = Self::find(file, id)?;
            pack.name = name;
            Ok(pack.clone())
        })
        .await
    }

    pub async fn delete(&self, id: &str) -> CoreResult<()> {
        self.change(|file| {
            let before = file.packs.len();
            file.packs.retain(|pack| pack.id != id);
            if file.packs.len() == before {
                return Err(CoreError::NotFound(format!("sync pack {id}")));
            }
            Ok(())
        })
        .await
    }

    pub async fn add_item(&self, id: &str, item: SyncItem) -> CoreResult<SyncPack> {
        validate_project_id(&item.project_id)?;
        self.change(|file| {
            let pack = Self::find(file, id)?;
            if pack
                .items
                .iter()
                .any(|existing| existing.project_id == item.project_id)
            {
                return Ok(pack.clone());
            }
            if pack.items.len() >= MAX_ITEMS {
                return Err(CoreError::InvalidInput(format!(
                    "a sync pack holds at most {MAX_ITEMS} projects"
                )));
            }
            pack.items.push(item);
            Ok(pack.clone())
        })
        .await
    }

    pub async fn remove_item(&self, id: &str, project_id: &str) -> CoreResult<SyncPack> {
        self.change(|file| {
            let pack = Self::find(file, id)?;
            pack.items.retain(|item| item.project_id != project_id);
            Ok(pack.clone())
        })
        .await
    }
}

impl Services {
    pub async fn search_sync_projects(
        &self,
        kind: ContentKind,
        query: &str,
        offset: u32,
    ) -> CoreResult<ModSearchPage> {
        self.modrinth
            .search(query, None, kind.project_type(), &[], offset)
            .await
    }

    pub async fn list_sync_packs(&self) -> CoreResult<Vec<SyncPack>> {
        self.sync_packs.list().await
    }

    pub async fn create_sync_pack(&self, name: &str) -> CoreResult<SyncPack> {
        self.sync_packs.create(name).await
    }

    pub async fn rename_sync_pack(&self, id: &str, name: &str) -> CoreResult<SyncPack> {
        self.sync_packs.rename(id, name).await
    }

    pub async fn add_sync_item(&self, id: &str, item: SyncItem) -> CoreResult<Vec<SyncReport>> {
        self.sync_packs.add_item(id, item).await?;
        self.sync_subscribers(id).await
    }

    pub async fn remove_sync_item(
        &self,
        id: &str,
        project_id: &str,
    ) -> CoreResult<Vec<SyncReport>> {
        self.sync_packs.remove_item(id, project_id).await?;
        self.sync_subscribers(id).await
    }

    pub async fn delete_sync_pack(&self, id: &str) -> CoreResult<Vec<SyncReport>> {
        let subscribers = self.subscribers(id).await?;
        for instance in &subscribers {
            let subscriptions: Vec<SyncSubscription> = instance
                .sync_packs
                .iter()
                .filter(|s| s.pack_id != id)
                .cloned()
                .collect();
            self.instances
                .set_sync_packs(&instance.id, subscriptions)
                .await?;
        }
        self.sync_packs.delete(id).await?;
        let mut reports = Vec::new();
        for instance in subscribers {
            reports.push(self.sync_instance(&instance.id).await?);
        }
        Ok(reports)
    }

    pub async fn set_instance_sync_packs(
        &self,
        instance_id: &str,
        subscriptions: Vec<SyncSubscription>,
    ) -> CoreResult<SyncReport> {
        let packs = self.sync_packs.list().await?;
        for subscription in &subscriptions {
            if !packs.iter().any(|pack| pack.id == subscription.pack_id) {
                return Err(CoreError::NotFound(format!(
                    "sync pack {}",
                    subscription.pack_id
                )));
            }
        }
        self.instances
            .set_sync_packs(instance_id, subscriptions)
            .await?;
        self.sync_instance(instance_id).await
    }

    async fn subscribers(&self, pack_id: &str) -> CoreResult<Vec<Instance>> {
        Ok(self
            .instances
            .list()
            .await?
            .into_iter()
            .filter(|instance| instance.sync_packs.iter().any(|s| s.pack_id == pack_id))
            .collect())
    }

    async fn sync_subscribers(&self, pack_id: &str) -> CoreResult<Vec<SyncReport>> {
        let mut reports = Vec::new();
        for instance in self.subscribers(pack_id).await? {
            reports.push(self.sync_instance(&instance.id).await?);
        }
        Ok(reports)
    }

    async fn sync_plan(&self, instance: &Instance) -> CoreResult<SyncPlan> {
        let packs = self.sync_packs.list().await?;
        let installed = self.mods_dir(instance)?.load().await?.mods;
        Ok(plan(&packs, &instance.sync_packs, &installed))
    }

    pub async fn sync_instance(&self, instance_id: &str) -> CoreResult<SyncReport> {
        let instance = self.instances.get(instance_id).await?;
        let plan = self.sync_plan(&instance).await?;
        let mut report = SyncReport {
            instance_id: instance.id.clone(),
            task_id: None,
            install: plan
                .install
                .iter()
                .map(|(_, item)| item.title.clone())
                .collect(),
            remove: plan.remove.clone(),
            waiting_for_stop: false,
        };
        if plan.is_empty() {
            return Ok(report);
        }
        if self.ensure_stopped(&instance).is_err() {
            report.waiting_for_stop = true;
            return Ok(report);
        }
        let services = self.clone();
        let task_id = self.tasks.spawn(
            TaskSpec {
                kind: TaskKind::SyncPacks,
                subject: instance.name.clone(),
                instance_id: Some(instance.id.clone()),
            },
            move |progress| async move { services.apply_sync(&instance, &progress).await },
        )?;
        report.task_id = Some(task_id);
        Ok(report)
    }

    pub(crate) async fn apply_sync(
        &self,
        instance: &Instance,
        progress: &Progress,
    ) -> CoreResult<()> {
        if instance.sync_packs.is_empty() {
            let installed = self.mods_dir(instance)?.load().await?.mods;
            if !installed.iter().any(|m| tagged_pack(m).is_some()) {
                return Ok(());
            }
        }
        let _guard = self.mods_lock.lock().await;
        let plan = self.sync_plan(instance).await?;
        if plan.is_empty() {
            return Ok(());
        }
        let mods = self.mods_dir(instance)?;
        let mut file = mods.load().await?;
        let (removed, kept): (Vec<_>, Vec<_>) = file
            .mods
            .into_iter()
            .partition(|m| plan.remove.contains(&m.project_id));
        for entry in &removed {
            mods.delete_files(entry).await?;
        }
        file.mods = kept;
        for entry in &mut file.mods {
            if let Some((_, pack)) = plan.retag.iter().find(|(id, _)| id == &entry.project_id) {
                entry.pack = Some(tag(pack));
            }
        }
        mods.save(&file).await?;

        for (pack_id, item) in &plan.install {
            progress.check_cancelled()?;
            let Ok(loaders) = content_loaders(instance, item.kind) else {
                continue;
            };
            let target = ModTarget {
                game_version: &instance.minecraft_version,
                loaders,
                kind: item.kind,
            };
            match install(
                &self.http,
                &self.modrinth,
                &mods,
                &target,
                &item.project_id,
                None,
                progress,
            )
            .await
            {
                Ok(_) => {}
                Err(CoreError::NotFound(_)) => {
                    tracing::info!(project = %item.project_id, instance = %instance.id, "no matching version for this sync pack project");
                    continue;
                }
                Err(err) => return Err(err),
            }
            let mut file = mods.load().await?;
            for entry in &mut file.mods {
                if entry.project_id == item.project_id && !entry.dependency && entry.pack.is_none()
                {
                    entry.pack = Some(tag(pack_id));
                }
            }
            mods.save(&file).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str) -> SyncItem {
        SyncItem {
            project_id: id.into(),
            title: id.into(),
            kind: ContentKind::Mod,
            icon_url: None,
        }
    }

    fn pack(id: &str, items: &[&str]) -> SyncPack {
        SyncPack {
            id: id.into(),
            name: id.into(),
            items: items.iter().map(|i| item(i)).collect(),
        }
    }

    fn installed(id: &str, pack: Option<&str>) -> InstalledMod {
        serde_json::from_value(serde_json::json!({
            "projectId": id, "versionId": "v", "title": id, "versionNumber": "1",
            "fileName": format!("{id}.jar"), "sha512": "x", "enabled": true, "dependency": false,
            "pack": pack
        }))
        .unwrap()
    }

    fn subscribe(id: &str, excluded: &[&str]) -> SyncSubscription {
        SyncSubscription {
            pack_id: id.into(),
            excluded: excluded.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn plans_installs_removals_and_exclusions() {
        let packs = [
            pack("a", &["sodium", "lithium", "iris"]),
            pack("b", &["iris", "modmenu"]),
        ];
        let current = [
            installed("sodium", Some("sync:a")),
            installed("old", Some("sync:a")),
            installed("manual", None),
            installed("modmenu", None),
        ];
        let result = plan(
            &packs,
            &[subscribe("a", &["lithium"]), subscribe("b", &[])],
            &current,
        );
        let install: Vec<(&str, &str)> = result
            .install
            .iter()
            .map(|(pack, item)| (pack.as_str(), item.project_id.as_str()))
            .collect();
        assert_eq!(install, [("a", "iris")]);
        assert_eq!(result.remove, ["old"]);
        assert!(result.retag.is_empty());
    }

    #[test]
    fn moves_ownership_when_a_pack_goes_away() {
        let packs = [pack("b", &["iris"])];
        let current = [
            installed("iris", Some("sync:a")),
            installed("x", Some("other")),
        ];
        let result = plan(
            &packs,
            &[subscribe("a", &[]), subscribe("b", &[])],
            &current,
        );
        assert!(result.install.is_empty());
        assert!(result.remove.is_empty());
        assert_eq!(result.retag, [("iris".to_string(), "b".to_string())]);

        let unsubscribed = plan(&packs, &[], &current);
        assert_eq!(unsubscribed.remove, ["iris"]);
        assert!(plan(&[], &[], &[installed("x", None)]).is_empty());
    }

    #[tokio::test]
    async fn stores_packs_and_items() {
        let dir = tempfile::tempdir().unwrap();
        let store = SyncPackStore::new(dir.path().join(SYNC_FILE));
        assert!(store.create("  ").await.is_err());
        let created = store.create("Performance").await.unwrap();
        store.add_item(&created.id, item("sodium")).await.unwrap();
        store.add_item(&created.id, item("sodium")).await.unwrap();
        assert!(store.add_item(&created.id, item("../bad")).await.is_err());
        let renamed = store.rename(&created.id, "Leistung").await.unwrap();
        assert_eq!(renamed.items.len(), 1);
        assert_eq!(store.list().await.unwrap()[0].name, "Leistung");
        store.remove_item(&created.id, "sodium").await.unwrap();
        assert!(store.list().await.unwrap()[0].items.is_empty());
        store.delete(&created.id).await.unwrap();
        assert!(store.list().await.unwrap().is_empty());
        assert!(store.delete(&created.id).await.is_err());
    }
}

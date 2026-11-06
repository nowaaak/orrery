use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::cleanup::path_size;
use crate::error::IoResultExt;
use crate::services::Services;
use crate::storage::{self, unix_now};
use crate::{CoreError, CoreResult};

const ENTRY_FILE: &str = "entry.json";
const CONTENT_DIR: &str = "content";
const DAY_SECS: i64 = 24 * 60 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum TrashKind {
    Instance,
    World,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TrashEntry {
    pub id: String,
    pub kind: TrashKind,
    pub name: String,
    pub instance_id: String,
    pub folder: Option<String>,
    pub deleted_at: i64,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrashItem {
    pub kind: TrashKind,
    pub name: String,
    pub instance_id: String,
    pub folder: Option<String>,
}

fn validate_entry_id(id: &str) -> CoreResult<()> {
    let valid = !id.is_empty()
        && id.len() <= 64
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    if valid {
        Ok(())
    } else {
        Err(CoreError::InvalidInput(format!(
            "invalid trash entry {id:?}"
        )))
    }
}

async fn blocking<T: Send + 'static>(task: impl FnOnce() -> T + Send + 'static) -> CoreResult<T> {
    tokio::task::spawn_blocking(task)
        .await
        .map_err(|err| CoreError::Unsupported(format!("trash task panicked: {err}")))
}

pub struct Trash {
    root: PathBuf,
}

impl Trash {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn entry_dir(&self, id: &str) -> CoreResult<PathBuf> {
        validate_entry_id(id)?;
        Ok(self.root.join(id))
    }

    pub fn content_dir(&self, id: &str) -> CoreResult<PathBuf> {
        Ok(self.entry_dir(id)?.join(CONTENT_DIR))
    }

    async fn new_id(&self, kind: TrashKind) -> CoreResult<String> {
        let prefix = match kind {
            TrashKind::Instance => "instance",
            TrashKind::World => "world",
        };
        let now = unix_now();
        for attempt in 0..1000 {
            let id = format!("{now}-{prefix}-{attempt}");
            if tokio::fs::metadata(self.root.join(&id)).await.is_err() {
                return Ok(id);
            }
        }
        Err(CoreError::InvalidInput(
            "the trash has no free entry name".into(),
        ))
    }

    pub async fn put(&self, item: TrashItem, source: &Path) -> CoreResult<TrashEntry> {
        let id = self.new_id(item.kind).await?;
        let dir = self.entry_dir(&id)?;
        tokio::fs::create_dir_all(&dir).await.at_path(&dir)?;
        let content = dir.join(CONTENT_DIR);
        if let Err(err) = tokio::fs::rename(source, &content).await {
            let _ = tokio::fs::remove_dir_all(&dir).await;
            return Err(err).at_path(source);
        }
        let measured = content.clone();
        let size_bytes = blocking(move || path_size(&measured)).await?;
        let entry = TrashEntry {
            id,
            kind: item.kind,
            name: item.name,
            instance_id: item.instance_id,
            folder: item.folder,
            deleted_at: unix_now(),
            size_bytes,
        };
        storage::write_json_atomic(&dir.join(ENTRY_FILE), &entry).await?;
        Ok(entry)
    }

    pub async fn get(&self, id: &str) -> CoreResult<TrashEntry> {
        let path = self.entry_dir(id)?.join(ENTRY_FILE);
        storage::read_json::<TrashEntry>(&path)
            .await?
            .ok_or_else(|| CoreError::NotFound(format!("trash entry {id}")))
    }

    pub async fn list(&self) -> CoreResult<Vec<TrashEntry>> {
        let mut entries = Vec::new();
        let mut dir = match tokio::fs::read_dir(&self.root).await {
            Ok(dir) => dir,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(entries),
            Err(err) => return Err(err).at_path(&self.root),
        };
        while let Some(item) = dir.next_entry().await.at_path(&self.root)? {
            let Some(name) = item.file_name().to_str().map(str::to_string) else {
                continue;
            };
            if validate_entry_id(&name).is_err() {
                continue;
            }
            match self.get(&name).await {
                Ok(entry) if entry.id == name => entries.push(entry),
                Ok(_) => tracing::warn!(entry = %name, "trash entry names another folder"),
                Err(err) => {
                    tracing::warn!(entry = %name, error = %err, "skipping unreadable trash entry")
                }
            }
        }
        entries.sort_by_key(|entry| std::cmp::Reverse(entry.deleted_at));
        Ok(entries)
    }

    pub async fn take(&self, id: &str, target: &Path) -> CoreResult<TrashEntry> {
        let entry = self.get(id).await?;
        let content = self.content_dir(id)?;
        if let Some(parent) = target.parent() {
            tokio::fs::create_dir_all(parent).await.at_path(parent)?;
        }
        tokio::fs::rename(&content, target)
            .await
            .at_path(&content)?;
        let dir = self.entry_dir(id)?;
        if let Err(err) = tokio::fs::remove_dir_all(&dir).await {
            tracing::warn!(entry = %id, error = %err, "could not remove the emptied trash entry");
        }
        Ok(entry)
    }

    pub async fn remove(&self, id: &str) -> CoreResult<()> {
        let dir = self.entry_dir(id)?;
        if tokio::fs::metadata(dir.join(ENTRY_FILE)).await.is_err() {
            return Err(CoreError::NotFound(format!("trash entry {id}")));
        }
        tokio::fs::remove_dir_all(&dir).await.at_path(&dir)
    }

    pub async fn empty(&self) -> CoreResult<u32> {
        let mut removed = 0;
        for entry in self.list().await? {
            self.remove(&entry.id).await?;
            removed += 1;
        }
        Ok(removed)
    }

    pub async fn purge_older_than(&self, days: u32) -> CoreResult<u32> {
        if days == 0 {
            return Ok(0);
        }
        let cutoff = unix_now() - i64::from(days) * DAY_SECS;
        let mut removed = 0;
        for entry in self.list().await? {
            if entry.deleted_at < cutoff {
                self.remove(&entry.id).await?;
                removed += 1;
            }
        }
        Ok(removed)
    }
}

impl Services {
    pub async fn trash_instance(&self, instance_id: &str) -> CoreResult<TrashEntry> {
        let instance = self.instances.get(instance_id).await?;
        self.ensure_stopped(&instance)?;
        self.instances
            .move_to_trash(&instance.id, &self.trash)
            .await
    }

    pub async fn list_trash(&self) -> CoreResult<Vec<TrashEntry>> {
        self.trash.list().await
    }

    pub async fn restore_trash(&self, id: &str) -> CoreResult<TrashEntry> {
        let entry = self.trash.get(id).await?;
        match entry.kind {
            TrashKind::Instance => {
                self.instances
                    .restore_from_trash(&entry, &self.trash)
                    .await?;
                Ok(entry)
            }
            TrashKind::World => {
                let folder = self.restore_world(&entry).await?;
                Ok(TrashEntry {
                    folder: Some(folder),
                    ..entry
                })
            }
        }
    }

    pub async fn delete_trash_entry(&self, id: &str) -> CoreResult<()> {
        self.trash.remove(id).await
    }

    pub async fn empty_trash(&self) -> CoreResult<u32> {
        self.trash.empty().await
    }

    pub async fn purge_trash(&self) -> CoreResult<u32> {
        let days = self.settings.get().await.trash_days;
        self.trash.purge_older_than(days).await
    }
}

pub fn free_folder_name(parent: &Path, wanted: &str) -> String {
    if !parent.join(wanted).exists() {
        return wanted.to_string();
    }
    (2..)
        .map(|n| format!("{wanted} ({n})"))
        .find(|candidate| !parent.join(candidate).exists())
        .unwrap_or_else(|| wanted.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(kind: TrashKind, name: &str) -> TrashItem {
        TrashItem {
            kind,
            name: name.into(),
            instance_id: "survival".into(),
            folder: (kind == TrashKind::World).then(|| name.into()),
        }
    }

    #[tokio::test]
    async fn moves_lists_restores_and_removes_entries() {
        let dir = tempfile::tempdir().unwrap();
        let trash = Trash::new(dir.path().join("trash"));
        let source = dir.path().join("instances/survival");
        std::fs::create_dir_all(source.join("minecraft/saves/Welt")).unwrap();
        std::fs::write(source.join("instance.json"), "{}").unwrap();

        let entry = trash
            .put(item(TrashKind::Instance, "Survival"), &source)
            .await
            .unwrap();
        assert!(!source.exists());
        assert_eq!(entry.size_bytes, 2);
        assert_eq!(trash.list().await.unwrap(), std::slice::from_ref(&entry));

        let restored = dir.path().join("instances/survival");
        trash.take(&entry.id, &restored).await.unwrap();
        assert!(restored.join("instance.json").is_file());
        assert!(trash.list().await.unwrap().is_empty());

        let world = restored.join("minecraft/saves/Welt");
        let second = trash
            .put(item(TrashKind::World, "Welt"), &world)
            .await
            .unwrap();
        assert!(!world.exists());
        trash.remove(&second.id).await.unwrap();
        assert!(trash.list().await.unwrap().is_empty());
        assert!(trash.remove(&second.id).await.is_err());
    }

    #[tokio::test]
    async fn purges_only_old_entries_and_empties_on_request() {
        let dir = tempfile::tempdir().unwrap();
        let trash = Trash::new(dir.path().join("trash"));
        for name in ["a", "b"] {
            let source = dir.path().join(name);
            std::fs::create_dir_all(&source).unwrap();
            trash
                .put(item(TrashKind::Instance, name), &source)
                .await
                .unwrap();
        }
        let mut entries = trash.list().await.unwrap();
        let old = entries.remove(0);
        let path = dir.path().join("trash").join(&old.id).join(ENTRY_FILE);
        let aged = TrashEntry {
            deleted_at: unix_now() - 20 * DAY_SECS,
            ..old
        };
        storage::write_json_atomic(&path, &aged).await.unwrap();

        assert_eq!(trash.purge_older_than(0).await.unwrap(), 0);
        assert_eq!(trash.purge_older_than(14).await.unwrap(), 1);
        assert_eq!(trash.list().await.unwrap().len(), 1);
        assert_eq!(trash.empty().await.unwrap(), 1);
        assert!(trash.list().await.unwrap().is_empty());
    }

    #[test]
    fn rejects_unsafe_entry_ids_and_finds_free_folder_names() {
        assert!(validate_entry_id("1730000000-world-0").is_ok());
        assert!(validate_entry_id("../x").is_err());
        assert!(validate_entry_id("").is_err());
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Welt")).unwrap();
        std::fs::create_dir_all(dir.path().join("Welt (2)")).unwrap();
        assert_eq!(free_folder_name(dir.path(), "Neu"), "Neu");
        assert_eq!(free_folder_name(dir.path(), "Welt"), "Welt (3)");
    }
}

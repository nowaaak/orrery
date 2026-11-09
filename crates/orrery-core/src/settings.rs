use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use ts_rs::TS;

use crate::discord::PresenceLevel;
use crate::storage::{self, Versioned};
use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Theme {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Language {
    System,
    De,
    En,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum LauncherWindow {
    #[default]
    KeepOpen,
    Minimize,
    Hide,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum InstanceSort {
    #[default]
    LastPlayed,
    Name,
    Created,
    Playtime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct Settings {
    pub schema: u32,
    pub max_concurrent_downloads: u32,
    pub default_min_memory_mb: u32,
    pub default_max_memory_mb: u32,
    pub show_snapshots: bool,
    pub theme: Theme,
    pub language: Language,
    pub optimize_game_options: bool,
    pub prefer_dedicated_gpu: bool,
    pub launcher_window: LauncherWindow,
    pub instance_sort: InstanceSort,
    pub discord_presence: PresenceLevel,
    pub trash_days: u32,
    #[ts(type = "string | null")]
    pub data_dir: Option<PathBuf>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema: Self::SCHEMA,
            max_concurrent_downloads: 16,
            default_min_memory_mb: 512,
            default_max_memory_mb: 4096,
            show_snapshots: false,
            theme: Theme::Dark,
            language: Language::System,
            optimize_game_options: true,
            prefer_dedicated_gpu: true,
            launcher_window: LauncherWindow::KeepOpen,
            instance_sort: InstanceSort::LastPlayed,
            discord_presence: PresenceLevel::Basic,
            trash_days: 14,
            data_dir: None,
        }
    }
}

impl Versioned for Settings {
    const SCHEMA: u32 = 2;
    fn schema(&self) -> u32 {
        self.schema
    }
}

impl Settings {
    pub fn migrate(mut self) -> Self {
        if self.schema < 2 {
            self.data_dir = None;
        }
        self.schema = Self::SCHEMA;
        self
    }

    pub fn validate(&self) -> CoreResult<()> {
        if !(1..=64).contains(&self.max_concurrent_downloads) {
            return Err(CoreError::InvalidInput(
                "parallel downloads must be between 1 and 64".into(),
            ));
        }
        if self.trash_days > 365 {
            return Err(CoreError::InvalidInput(
                "the trash keeps items for at most 365 days".into(),
            ));
        }
        validate_memory(self.default_min_memory_mb, self.default_max_memory_mb)
    }
}

pub fn validate_memory(min_mb: u32, max_mb: u32) -> CoreResult<()> {
    if min_mb < 256 || !(512..=65536).contains(&max_mb) {
        return Err(CoreError::InvalidInput(
            "memory must be at least 256 MB minimum and between 512 MB and 64 GB maximum".into(),
        ));
    }
    if min_mb > max_mb {
        return Err(CoreError::InvalidInput(
            "minimum memory must not exceed maximum memory".into(),
        ));
    }
    Ok(())
}

pub struct SettingsStore {
    path: PathBuf,
    current: RwLock<Settings>,
}

impl SettingsStore {
    pub async fn load(path: PathBuf) -> CoreResult<Self> {
        let settings = match storage::read_versioned::<Settings>(&path).await? {
            Some(stored) if stored.schema < Settings::SCHEMA => {
                let migrated = stored.migrate();
                storage::write_json_atomic(&path, &migrated).await?;
                migrated
            }
            Some(stored) => stored,
            None => Settings::default(),
        };
        Ok(Self {
            path,
            current: RwLock::new(settings),
        })
    }

    pub async fn set_data_dir(&self, data_dir: Option<PathBuf>) -> CoreResult<Settings> {
        let mut current = self.current.write().await;
        let mut next = current.clone();
        next.data_dir = data_dir;
        storage::write_json_atomic(&self.path, &next).await?;
        *current = next.clone();
        Ok(next)
    }

    pub async fn get(&self) -> Settings {
        self.current.read().await.clone()
    }

    pub async fn update(&self, mut next: Settings) -> CoreResult<Settings> {
        next.schema = Settings::SCHEMA;
        next.validate()?;
        let mut current = self.current.write().await;
        next.data_dir = current.data_dir.clone();
        storage::write_json_atomic(&self.path, &next).await?;
        *current = next.clone();
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn schema_one_files_are_migrated_and_written_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(
            &path,
            r#"{ "schema": 1, "showSnapshots": true, "theme": "light" }"#,
        )
        .unwrap();
        let store = SettingsStore::load(path.clone()).await.unwrap();
        let settings = store.get().await;
        assert_eq!(settings.schema, 2);
        assert!(settings.show_snapshots);
        assert_eq!(settings.data_dir, None);
        let written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(written["schema"], 2);
        assert_eq!(written["theme"], "light");
    }

    #[tokio::test]
    async fn update_persists_and_rejects_invalid_values() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let store = SettingsStore::load(path.clone()).await.unwrap();
        assert_eq!(store.get().await, Settings::default());

        let mut next = store.get().await;
        next.show_snapshots = true;
        store.update(next).await.unwrap();
        let reloaded = SettingsStore::load(path).await.unwrap();
        assert!(reloaded.get().await.show_snapshots);

        let mut sneaky = reloaded.get().await;
        sneaky.data_dir = Some(PathBuf::from("D:/elsewhere"));
        assert_eq!(reloaded.update(sneaky).await.unwrap().data_dir, None);
        reloaded
            .set_data_dir(Some(PathBuf::from("D:/Games/Orrery")))
            .await
            .unwrap();
        let mut keeps = reloaded.get().await;
        keeps.data_dir = None;
        assert_eq!(
            reloaded.update(keeps).await.unwrap().data_dir,
            Some(PathBuf::from("D:/Games/Orrery"))
        );

        let mut invalid = reloaded.get().await;
        invalid.default_min_memory_mb = 8192;
        assert!(reloaded.update(invalid).await.is_err());
        assert!(reloaded.get().await.default_min_memory_mb == 512);
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let parsed: Settings =
            serde_json::from_str(r#"{"schema":1,"showSnapshots":true}"#).unwrap();
        assert!(parsed.show_snapshots);
        assert_eq!(parsed.max_concurrent_downloads, 16);
    }

    #[test]
    fn removed_fields_from_older_versions_are_ignored() {
        let parsed: Settings = serde_json::from_str(
            r#"{"schema":2,"welcomeDone":true,"trashDays":30,"showSnapshots":true}"#,
        )
        .unwrap();
        assert!(parsed.show_snapshots);
        assert_eq!(parsed.trash_days, 30);
    }
}

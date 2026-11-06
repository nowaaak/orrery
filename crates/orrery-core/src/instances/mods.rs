use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::IoResultExt;
use crate::loaders::ModLoader;
use crate::storage::{self, Versioned};
use crate::{CoreError, CoreResult};

const MODS_FILE: &str = "mods.json";
const DISABLED_SUFFIX: &str = ".disabled";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ContentKind {
    #[default]
    Mod,
    ResourcePack,
    Shader,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ContentSource {
    #[default]
    Modrinth,
    CurseForge,
}

impl ContentKind {
    pub const ALL: [ContentKind; 3] = [Self::Mod, Self::ResourcePack, Self::Shader];

    pub fn project_type(self) -> &'static str {
        match self {
            Self::Mod => "mod",
            Self::ResourcePack => "resourcepack",
            Self::Shader => "shader",
        }
    }

    pub fn folder(self) -> &'static str {
        match self {
            Self::Mod => "mods",
            Self::ResourcePack => "resourcepacks",
            Self::Shader => "shaderpacks",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Mod => ".jar",
            Self::ResourcePack | Self::Shader => ".zip",
        }
    }

    pub fn modrinth_loaders(self, loader: Option<ModLoader>) -> Option<Vec<&'static str>> {
        match (self, loader) {
            (Self::ResourcePack, _) => Some(vec!["minecraft"]),
            (Self::Mod, Some(loader)) => Some(loader.modrinth_loaders().to_vec()),
            (Self::Shader, Some(_)) => Some(vec!["iris"]),
            (_, None) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct InstalledMod {
    pub project_id: String,
    pub version_id: String,
    pub title: String,
    pub version_number: String,
    pub file_name: String,
    pub sha512: String,
    pub enabled: bool,
    pub dependency: bool,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub kind: ContentKind,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub loaders: Vec<String>,
    #[serde(default)]
    pub pack: Option<String>,
    #[serde(default)]
    pub pinned: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModsFile {
    pub schema: u32,
    #[serde(default)]
    pub mods: Vec<InstalledMod>,
}

impl Versioned for ModsFile {
    const SCHEMA: u32 = 1;
    fn schema(&self) -> u32 {
        self.schema
    }
}

pub fn validate_content_file_name(name: &str, kind: ContentKind) -> CoreResult<()> {
    let valid = name.len() <= 200
        && name.ends_with(kind.extension())
        && Path::new(name).file_name().and_then(|n| n.to_str()) == Some(name)
        && !name.starts_with('.')
        && name.chars().all(|c| {
            !c.is_control() && !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
        });
    if valid {
        Ok(())
    } else {
        Err(CoreError::InvalidInput(format!(
            "unsafe file name {name:?}"
        )))
    }
}

pub struct ModsDir {
    instance_dir: PathBuf,
    game_dir: PathBuf,
}

impl ModsDir {
    pub fn new(instance_dir: PathBuf, game_dir: &Path) -> Self {
        Self {
            instance_dir,
            game_dir: game_dir.to_path_buf(),
        }
    }

    pub fn game_dir(&self) -> &Path {
        &self.game_dir
    }

    pub fn dir(&self, kind: ContentKind) -> PathBuf {
        self.game_dir.join(kind.folder())
    }

    pub fn mods_dir(&self) -> PathBuf {
        self.dir(ContentKind::Mod)
    }

    pub fn file_path(&self, installed: &InstalledMod) -> PathBuf {
        let dir = self.dir(installed.kind);
        if installed.enabled {
            dir.join(&installed.file_name)
        } else {
            dir.join(format!("{}{DISABLED_SUFFIX}", installed.file_name))
        }
    }

    pub async fn load(&self) -> CoreResult<ModsFile> {
        Ok(
            storage::read_versioned::<ModsFile>(&self.instance_dir.join(MODS_FILE))
                .await?
                .unwrap_or(ModsFile {
                    schema: ModsFile::SCHEMA,
                    mods: Vec::new(),
                }),
        )
    }

    pub async fn save(&self, file: &ModsFile) -> CoreResult<()> {
        storage::write_json_atomic(&self.instance_dir.join(MODS_FILE), file).await
    }

    pub async fn set_enabled(&self, project_id: &str, enabled: bool) -> CoreResult<InstalledMod> {
        let mut file = self.load().await?;
        let entry = file
            .mods
            .iter_mut()
            .find(|m| m.project_id == project_id)
            .ok_or_else(|| CoreError::NotFound(format!("installed content {project_id}")))?;
        if entry.enabled != enabled {
            let from = self.file_path(entry);
            entry.enabled = enabled;
            let to = self.file_path(entry);
            tokio::fs::rename(&from, &to).await.at_path(&from)?;
        }
        let updated = entry.clone();
        self.save(&file).await?;
        Ok(updated)
    }

    pub async fn set_pinned(&self, project_id: &str, pinned: bool) -> CoreResult<InstalledMod> {
        let mut file = self.load().await?;
        let entry = file
            .mods
            .iter_mut()
            .find(|m| m.project_id == project_id)
            .ok_or_else(|| CoreError::NotFound(format!("installed content {project_id}")))?;
        entry.pinned = pinned;
        let updated = entry.clone();
        self.save(&file).await?;
        Ok(updated)
    }

    pub async fn remove(&self, project_id: &str) -> CoreResult<()> {
        let mut file = self.load().await?;
        let index = file
            .mods
            .iter()
            .position(|m| m.project_id == project_id)
            .ok_or_else(|| CoreError::NotFound(format!("installed content {project_id}")))?;
        let removed = file.mods.remove(index);
        self.delete_files(&removed).await?;
        self.save(&file).await
    }

    pub async fn commit(&self, mut index: ModsFile, entries: &[InstalledMod]) -> CoreResult<()> {
        for entry in entries {
            if let Some(position) = index
                .mods
                .iter()
                .position(|m| m.project_id == entry.project_id)
            {
                let old = index.mods.remove(position);
                if old.file_name != entry.file_name {
                    self.delete_files(&old).await?;
                } else if !old.enabled {
                    self.delete_disabled_copy(&old).await?;
                }
            }
        }
        index.mods.extend(entries.iter().cloned());
        index.mods.sort_by_key(|m| m.title.to_lowercase());
        self.save(&index).await
    }

    pub async fn delete_files(&self, installed: &InstalledMod) -> CoreResult<()> {
        validate_content_file_name(&installed.file_name, installed.kind)?;
        remove_if_exists(&self.dir(installed.kind).join(&installed.file_name)).await?;
        self.delete_disabled_copy(installed).await
    }

    pub async fn delete_disabled_copy(&self, installed: &InstalledMod) -> CoreResult<()> {
        validate_content_file_name(&installed.file_name, installed.kind)?;
        remove_if_exists(
            &self
                .dir(installed.kind)
                .join(format!("{}{DISABLED_SUFFIX}", installed.file_name)),
        )
        .await
    }
}

async fn remove_if_exists(path: &Path) -> CoreResult<()> {
    match tokio::fs::remove_file(path).await {
        Err(err) if err.kind() != std::io::ErrorKind::NotFound => Err(err).at_path(path),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn installed(file_name: &str, kind: ContentKind) -> InstalledMod {
        InstalledMod {
            project_id: "P1".into(),
            version_id: "V1".into(),
            title: "Sodium".into(),
            version_number: "1.0".into(),
            file_name: file_name.into(),
            sha512: "x".into(),
            enabled: true,
            dependency: false,
            icon_url: None,
            kind,
            game_versions: Vec::new(),
            loaders: Vec::new(),
            pack: None,
            pinned: false,
        }
    }

    #[test]
    fn file_names_are_validated_per_kind() {
        assert!(validate_content_file_name("sodium-fabric-0.5.jar", ContentKind::Mod).is_ok());
        assert!(validate_content_file_name("pack.zip", ContentKind::ResourcePack).is_ok());
        assert!(validate_content_file_name("pack.zip", ContentKind::Mod).is_err());
        assert!(validate_content_file_name("../evil.jar", ContentKind::Mod).is_err());
        assert!(validate_content_file_name("evil.exe", ContentKind::Shader).is_err());
        assert!(validate_content_file_name("a\\b.jar", ContentKind::Mod).is_err());
        assert!(validate_content_file_name(".hidden.jar", ContentKind::Mod).is_err());
    }

    #[test]
    fn content_kinds_map_to_loaders_and_folders() {
        assert_eq!(ContentKind::Shader.folder(), "shaderpacks");
        assert_eq!(
            ContentKind::ResourcePack.modrinth_loaders(None),
            Some(vec!["minecraft"])
        );
        assert_eq!(ContentKind::Mod.modrinth_loaders(None), None);
        assert_eq!(
            ContentKind::Mod.modrinth_loaders(Some(ModLoader::Quilt)),
            Some(vec!["quilt", "fabric"])
        );
        assert!(serde_json::from_str::<InstalledMod>(
            r#"{"projectId":"a","versionId":"b","title":"t","versionNumber":"1","fileName":"f.jar","sha512":"x","enabled":true,"dependency":false}"#
        )
        .is_ok_and(|m| m.kind == ContentKind::Mod));
    }

    #[tokio::test]
    async fn pinning_is_stored_and_old_files_default_to_unpinned() {
        let dir = tempfile::tempdir().unwrap();
        let mods = ModsDir::new(dir.path().to_path_buf(), &dir.path().join("minecraft"));
        mods.save(&ModsFile {
            schema: 1,
            mods: vec![installed("sodium.jar", ContentKind::Mod)],
        })
        .await
        .unwrap();
        assert!(!mods.load().await.unwrap().mods[0].pinned);
        assert!(mods.set_pinned("P1", true).await.unwrap().pinned);
        assert!(mods.load().await.unwrap().mods[0].pinned);
        assert!(mods.set_pinned("missing", true).await.is_err());

        let old: InstalledMod = serde_json::from_str(
            r#"{"projectId":"a","versionId":"b","title":"t","versionNumber":"1","fileName":"f.jar","sha512":"x","enabled":true,"dependency":false}"#,
        )
        .unwrap();
        assert!(!old.pinned);
    }

    #[tokio::test]
    async fn disable_enable_and_remove_move_files() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("minecraft");
        let mods = ModsDir::new(dir.path().to_path_buf(), &game);
        let packs = mods.dir(ContentKind::ResourcePack);
        std::fs::create_dir_all(&packs).unwrap();
        std::fs::write(packs.join("faithful.zip"), b"zip").unwrap();
        mods.save(&ModsFile {
            schema: 1,
            mods: vec![installed("faithful.zip", ContentKind::ResourcePack)],
        })
        .await
        .unwrap();

        mods.set_enabled("P1", false).await.unwrap();
        assert!(packs.join("faithful.zip.disabled").exists());
        assert!(!packs.join("faithful.zip").exists());

        mods.set_enabled("P1", true).await.unwrap();
        assert!(packs.join("faithful.zip").exists());

        mods.remove("P1").await.unwrap();
        assert!(!packs.join("faithful.zip").exists());
        assert!(mods.load().await.unwrap().mods.is_empty());
        assert!(mods.remove("P1").await.is_err());
    }
}

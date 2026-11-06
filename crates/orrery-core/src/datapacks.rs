use std::io::Read;
use std::path::{Path, PathBuf};

use serde::Serialize;
use ts_rs::TS;

use crate::chat;
use crate::error::IoResultExt;
use crate::nbt::Tag;
use crate::services::Services;
use crate::worlds::{ensure_world, read_level_nbt, validate_world_folder};
use crate::{CoreError, CoreResult};

const DATAPACKS_DIR: &str = "datapacks";
const PACK_META: &str = "pack.mcmeta";
const MAX_META_BYTES: u64 = 64 * 1024;
const MAX_PACK_BYTES: u64 = 512 * 1024 * 1024;
const MAX_DESCRIPTION_CHARS: usize = 300;
const PARTIAL_SUFFIX: &str = ".orrery-part";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum DataPackState {
    Enabled,
    Disabled,
    Pending,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DataPack {
    pub name: String,
    pub folder: bool,
    pub description: String,
    pub state: DataPackState,
}

fn description(meta: &str) -> String {
    serde_json::from_str::<serde_json::Value>(meta)
        .map(|value| chat::plain_text(&value["pack"]["description"]))
        .unwrap_or_default()
        .trim()
        .chars()
        .take(MAX_DESCRIPTION_CHARS)
        .collect()
}

fn zip_meta(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;
    let entry = archive.by_name(PACK_META).ok()?;
    let mut text = String::new();
    entry.take(MAX_META_BYTES).read_to_string(&mut text).ok()?;
    Some(text)
}

fn folder_meta(path: &Path) -> Option<String> {
    let mut text = String::new();
    std::fs::File::open(path.join(PACK_META))
        .ok()?
        .take(MAX_META_BYTES)
        .read_to_string(&mut text)
        .ok()?;
    Some(text)
}

fn pack_ids(level: Option<&Tag>, key: &str) -> Vec<String> {
    level
        .and_then(|level| level.get("Data"))
        .and_then(|data| data.get("DataPacks"))
        .and_then(|packs| packs.get(key))
        .and_then(Tag::as_list)
        .unwrap_or_default()
        .iter()
        .filter_map(Tag::as_str)
        .map(str::to_string)
        .collect()
}

fn list_packs(world_dir: &Path) -> CoreResult<Vec<DataPack>> {
    let dir = world_dir.join(DATAPACKS_DIR);
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err).at_path(&dir),
    };
    let level = read_level_nbt(world_dir);
    let enabled = pack_ids(level.as_ref(), "Enabled");
    let disabled = pack_ids(level.as_ref(), "Disabled");
    let mut packs: Vec<DataPack> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_str()?.to_string();
            validate_pack_name(&name).ok()?;
            let path = entry.path();
            let folder = entry.file_type().ok()?.is_dir();
            let meta = if folder {
                folder_meta(&path)?
            } else if name.to_ascii_lowercase().ends_with(".zip") {
                zip_meta(&path)?
            } else {
                return None;
            };
            let id = format!("file/{name}");
            let state = if enabled.contains(&id) {
                DataPackState::Enabled
            } else if disabled.contains(&id) {
                DataPackState::Disabled
            } else {
                DataPackState::Pending
            };
            Some(DataPack {
                description: description(&meta),
                name,
                folder,
                state,
            })
        })
        .collect();
    packs.sort_by_key(|pack| pack.name.to_lowercase());
    Ok(packs)
}

fn validate_pack_name(name: &str) -> CoreResult<()> {
    validate_world_folder(name)
        .map_err(|_| CoreError::InvalidInput(format!("invalid data pack name {name:?}")))?;
    if name.ends_with(PARTIAL_SUFFIX) {
        return Err(CoreError::InvalidInput(format!(
            "invalid data pack name {name:?}"
        )));
    }
    Ok(())
}

fn check_archive(path: &Path) -> CoreResult<()> {
    let metadata = std::fs::metadata(path).at_path(path)?;
    if !metadata.is_file() || metadata.len() > MAX_PACK_BYTES {
        return Err(CoreError::InvalidInput(
            "data packs must be zip files smaller than 512 MB".into(),
        ));
    }
    if zip_meta(path).is_none() {
        return Err(CoreError::Archive {
            path: path.to_path_buf(),
            message: "the zip has no pack.mcmeta at its top level, so it is not a data pack".into(),
        });
    }
    Ok(())
}

fn install_pack(world_dir: &Path, source: &Path) -> CoreResult<DataPack> {
    let name = source
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| name.to_ascii_lowercase().ends_with(".zip"))
        .ok_or_else(|| CoreError::InvalidInput("choose a .zip data pack".into()))?
        .to_string();
    validate_pack_name(&name)?;
    check_archive(source)?;
    let dir = world_dir.join(DATAPACKS_DIR);
    std::fs::create_dir_all(&dir).at_path(&dir)?;
    let target = dir.join(&name);
    if target.exists() {
        return Err(CoreError::InvalidInput(format!(
            "{name} is already installed in this world"
        )));
    }
    let partial: PathBuf = dir.join(format!("{name}{PARTIAL_SUFFIX}"));
    std::fs::copy(source, &partial).at_path(&partial)?;
    if let Err(err) = std::fs::rename(&partial, &target) {
        let _ = std::fs::remove_file(&partial);
        return Err(err).at_path(&target);
    }
    list_packs(world_dir)?
        .into_iter()
        .find(|pack| pack.name == name)
        .ok_or_else(|| CoreError::NotFound(format!("data pack {name}")))
}

fn remove_pack(world_dir: &Path, name: &str) -> CoreResult<()> {
    validate_pack_name(name)?;
    let path = world_dir.join(DATAPACKS_DIR).join(name);
    let metadata = std::fs::symlink_metadata(&path).at_path(&path)?;
    if metadata.is_dir() {
        if path.join(PACK_META).is_file() {
            std::fs::remove_dir_all(&path).at_path(&path)
        } else {
            Err(CoreError::InvalidInput(format!(
                "{name} is not a data pack folder"
            )))
        }
    } else {
        std::fs::remove_file(&path).at_path(&path)
    }
}

async fn blocking<T: Send + 'static>(
    job: impl FnOnce() -> CoreResult<T> + Send + 'static,
) -> CoreResult<T> {
    tokio::task::spawn_blocking(job)
        .await
        .map_err(|err| CoreError::Unsupported(format!("data pack operation panicked: {err}")))?
}

impl Services {
    async fn world_dir(&self, instance_id: &str, world: &str) -> CoreResult<PathBuf> {
        let instance = self.instances.get(instance_id).await?;
        ensure_world(&self.instances.game_dir(&instance.id)?, world).await
    }

    fn ensure_world_unlocked(&self, instance_id: &str) -> CoreResult<()> {
        if self.games.is_running(instance_id) {
            return Err(CoreError::InvalidInput(
                "stop the game before changing data packs".into(),
            ));
        }
        Ok(())
    }

    pub async fn list_datapacks(
        &self,
        instance_id: &str,
        world: &str,
    ) -> CoreResult<Vec<DataPack>> {
        let dir = self.world_dir(instance_id, world).await?;
        blocking(move || list_packs(&dir)).await
    }

    pub async fn add_datapack(
        &self,
        instance_id: &str,
        world: &str,
        source: PathBuf,
    ) -> CoreResult<DataPack> {
        self.ensure_world_unlocked(instance_id)?;
        let dir = self.world_dir(instance_id, world).await?;
        blocking(move || install_pack(&dir, &source)).await
    }

    pub async fn remove_datapack(
        &self,
        instance_id: &str,
        world: &str,
        name: &str,
    ) -> CoreResult<()> {
        self.ensure_world_unlocked(instance_id)?;
        let dir = self.world_dir(instance_id, world).await?;
        let name = name.to_string();
        blocking(move || remove_pack(&dir, &name)).await
    }

    pub async fn datapacks_dir(&self, instance_id: &str, world: &str) -> CoreResult<PathBuf> {
        let dir = self
            .world_dir(instance_id, world)
            .await?
            .join(DATAPACKS_DIR);
        tokio::fs::create_dir_all(&dir).await.at_path(&dir)?;
        Ok(dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nbt;
    use std::io::Write;

    fn write_zip(path: &Path, files: &[(&str, &str)]) {
        let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        for (name, content) in files {
            zip.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(content.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }

    fn write_level(world: &Path, enabled: &[&str], disabled: &[&str]) {
        let list =
            |ids: &[&str]| Tag::List(8, ids.iter().map(|id| Tag::String((*id).into())).collect());
        let root = Tag::Compound(vec![(
            "Data".into(),
            Tag::Compound(vec![(
                "DataPacks".into(),
                Tag::Compound(vec![
                    ("Enabled".into(), list(enabled)),
                    ("Disabled".into(), list(disabled)),
                ]),
            )]),
        )]);
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&nbt::write("", &root)).unwrap();
        std::fs::write(world.join("level.dat"), encoder.finish().unwrap()).unwrap();
    }

    #[test]
    fn lists_installs_and_removes_packs() {
        let dir = tempfile::tempdir().unwrap();
        let world = dir.path().join("world");
        std::fs::create_dir_all(world.join(DATAPACKS_DIR).join("folder-pack")).unwrap();
        std::fs::write(
            world
                .join(DATAPACKS_DIR)
                .join("folder-pack")
                .join(PACK_META),
            r#"{"pack":{"pack_format":48,"description":{"text":"§6Folder ","extra":["pack"]}}}"#,
        )
        .unwrap();
        write_level(&world, &["vanilla", "file/folder-pack"], &["file/old.zip"]);
        assert!(list_packs(&dir.path().join("missing")).unwrap().is_empty());

        let source = dir.path().join("Cool Pack.zip");
        write_zip(
            &source,
            &[
                (
                    PACK_META,
                    r#"{"pack":{"pack_format":48,"description":"Cool"}}"#,
                ),
                ("data/x/function/a.mcfunction", "say hi"),
            ],
        );
        let installed = install_pack(&world, &source).unwrap();
        assert_eq!(installed.description, "Cool");
        assert_eq!(installed.state, DataPackState::Pending);
        assert!(install_pack(&world, &source).is_err());

        let packs = list_packs(&world).unwrap();
        assert_eq!(packs.len(), 2);
        assert_eq!(packs[1].name, "folder-pack");
        assert!(packs[1].folder);
        assert_eq!(packs[1].description, "Folder pack");
        assert_eq!(packs[1].state, DataPackState::Enabled);

        remove_pack(&world, "Cool Pack.zip").unwrap();
        remove_pack(&world, "folder-pack").unwrap();
        assert!(list_packs(&world).unwrap().is_empty());
    }

    #[test]
    fn rejects_bad_archives_and_names() {
        let dir = tempfile::tempdir().unwrap();
        let world = dir.path().join("world");
        std::fs::create_dir_all(&world).unwrap();
        let nested = dir.path().join("nested.zip");
        write_zip(&nested, &[("inner/pack.mcmeta", "{}")]);
        assert!(matches!(
            install_pack(&world, &nested),
            Err(CoreError::Archive { .. })
        ));
        let text = dir.path().join("pack.txt");
        std::fs::write(&text, "x").unwrap();
        assert!(install_pack(&world, &text).is_err());
        assert!(remove_pack(&world, "../level.dat").is_err());
        std::fs::create_dir_all(world.join(DATAPACKS_DIR).join("plain")).unwrap();
        assert!(remove_pack(&world, "plain").is_err());
    }
}

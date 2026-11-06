use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use ts_rs::TS;

use crate::error::IoResultExt;
use crate::modpacks::validate_pack_path;
use crate::nbt::{self, Tag};
use crate::services::Services;
use crate::trash::{TrashEntry, TrashItem, TrashKind, free_folder_name};
use crate::{CoreError, CoreResult};

const SAVES_DIR: &str = "saves";
const BACKUPS_DIR: &str = "backups";
const LEVEL_FILE: &str = "level.dat";
const ICON_FILE: &str = "icon.png";
const SESSION_LOCK: &str = "session.lock";
const RESTORE_PREFIX: &str = ".orrery-restore-";
const REPLACED_PREFIX: &str = ".orrery-replaced-";
const MAX_LEVEL_BYTES: u64 = 64 * 1024 * 1024;
const MAX_ICON_BYTES: u64 = 256 * 1024;
const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum GameMode {
    Survival,
    Creative,
    Adventure,
    Spectator,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorldBackup {
    pub file_name: String,
    pub created_at_ms: i64,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct World {
    pub folder: String,
    pub name: String,
    pub exists: bool,
    pub last_played_ms: Option<i64>,
    pub game_mode: Option<GameMode>,
    pub hardcore: bool,
    pub version: Option<String>,
    pub size_bytes: u64,
    pub icon: Option<String>,
    pub backups: Vec<WorldBackup>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorldList {
    pub worlds: Vec<World>,
    pub quick_play: bool,
}

#[derive(Debug, Default, PartialEq)]
struct LevelInfo {
    name: Option<String>,
    last_played_ms: Option<i64>,
    game_mode: Option<GameMode>,
    hardcore: bool,
    version: Option<String>,
}

fn parse_level(data: &[u8]) -> Option<LevelInfo> {
    let (_, root) = nbt::read(data)?;
    let level = root.get("Data")?;
    if !matches!(level, Tag::Compound(_)) {
        return None;
    }
    let int = |key: &str| level.get(key).and_then(Tag::as_i64);
    Some(LevelInfo {
        name: level
            .get("LevelName")
            .and_then(Tag::as_str)
            .filter(|name| !name.trim().is_empty())
            .map(str::to_string),
        last_played_ms: int("LastPlayed"),
        game_mode: match int("GameType") {
            Some(0) => Some(GameMode::Survival),
            Some(1) => Some(GameMode::Creative),
            Some(2) => Some(GameMode::Adventure),
            Some(3) => Some(GameMode::Spectator),
            _ => None,
        },
        hardcore: int("hardcore").is_some_and(|value| value != 0),
        version: level
            .get("Version")
            .and_then(|version| version.get("Name"))
            .and_then(Tag::as_str)
            .map(str::to_string),
    })
}

fn level_bytes(world_dir: &Path) -> Option<Vec<u8>> {
    let file = File::open(world_dir.join(LEVEL_FILE)).ok()?;
    let mut data = Vec::new();
    flate2::read::GzDecoder::new(file)
        .take(MAX_LEVEL_BYTES)
        .read_to_end(&mut data)
        .ok()?;
    Some(data)
}

fn read_level(world_dir: &Path) -> Option<LevelInfo> {
    parse_level(&level_bytes(world_dir)?)
}

pub(crate) fn read_level_nbt(world_dir: &Path) -> Option<Tag> {
    nbt::read(&level_bytes(world_dir)?).map(|(_, root)| root)
}

fn base64(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for (i, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            if i <= chunk.len() {
                out.push(char::from(BASE64[((n >> shift) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn read_icon(world_dir: &Path) -> Option<String> {
    let path = world_dir.join(ICON_FILE);
    let metadata = std::fs::metadata(&path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_ICON_BYTES {
        return None;
    }
    let data = std::fs::read(&path).ok()?;
    data.starts_with(b"\x89PNG")
        .then(|| format!("data:image/png;base64,{}", base64(&data)))
}

fn dir_size(dir: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| match entry.file_type() {
            Ok(kind) if kind.is_dir() => dir_size(&entry.path()),
            Ok(kind) if kind.is_file() => entry.metadata().map(|m| m.len()).unwrap_or(0),
            _ => 0,
        })
        .sum()
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

pub fn validate_world_folder(folder: &str) -> CoreResult<()> {
    let valid = !folder.is_empty()
        && folder.chars().count() <= 255
        && !folder.starts_with('.')
        && folder != ".."
        && !folder.chars().any(|c| {
            c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
        });
    if valid {
        Ok(())
    } else {
        Err(CoreError::InvalidInput(format!(
            "invalid world folder {folder:?}"
        )))
    }
}

fn validate_backup_name(name: &str) -> CoreResult<i64> {
    name.strip_suffix(".zip")
        .filter(|stem| !stem.is_empty() && stem.chars().all(|c| c.is_ascii_digit()))
        .and_then(|stem| stem.parse().ok())
        .ok_or_else(|| CoreError::InvalidInput(format!("invalid backup name {name:?}")))
}

pub fn supports_quick_play(version: &str) -> bool {
    if let Some((year, _)) = version.split_once('w')
        && year.len() == 2
        && let Ok(year) = year.parse::<u32>()
    {
        return year >= 23;
    }
    let mut parts = version.split(['.', '-', ' ']);
    match (
        parts.next().and_then(|p| p.parse::<u32>().ok()),
        parts.next().and_then(|p| p.parse::<u32>().ok()),
    ) {
        (Some(1), Some(minor)) => minor >= 20,
        (Some(major), _) => major >= 2,
        _ => false,
    }
}

pub async fn ensure_world(game_dir: &Path, folder: &str) -> CoreResult<PathBuf> {
    validate_world_folder(folder)?;
    let path = game_dir.join(SAVES_DIR).join(folder);
    if tokio::fs::metadata(&path).await.is_ok_and(|m| m.is_dir()) {
        Ok(path)
    } else {
        Err(CoreError::NotFound(format!("world {folder}")))
    }
}

fn list_backups(dir: &Path) -> Vec<WorldBackup> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut backups: Vec<WorldBackup> = entries
        .flatten()
        .filter_map(|entry| {
            let file_name = entry.file_name().to_str()?.to_string();
            let created_at_ms = validate_backup_name(&file_name).ok()?;
            let metadata = entry.metadata().ok().filter(std::fs::Metadata::is_file)?;
            Some(WorldBackup {
                file_name,
                created_at_ms,
                size_bytes: metadata.len(),
            })
        })
        .collect();
    backups.sort_by_key(|b| std::cmp::Reverse(b.created_at_ms));
    backups
}

pub fn list_worlds(game_dir: &Path, backups_dir: &Path) -> Vec<World> {
    let mut backups: BTreeMap<String, Vec<WorldBackup>> = std::fs::read_dir(backups_dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let folder = entry.file_name().to_str()?.to_string();
            validate_world_folder(&folder).ok()?;
            let list = list_backups(&entry.path());
            (!list.is_empty()).then_some((folder, list))
        })
        .collect();

    let mut worlds: Vec<World> = std::fs::read_dir(game_dir.join(SAVES_DIR))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|entry| {
            let folder = entry.file_name().to_str()?.to_string();
            validate_world_folder(&folder).ok()?;
            let path = entry.path();
            if !path.join(LEVEL_FILE).is_file() {
                return None;
            }
            let level = read_level(&path).unwrap_or_default();
            Some(World {
                name: level.name.unwrap_or_else(|| folder.clone()),
                exists: true,
                last_played_ms: level.last_played_ms,
                game_mode: level.game_mode,
                hardcore: level.hardcore,
                version: level.version,
                size_bytes: dir_size(&path),
                icon: read_icon(&path),
                backups: backups.remove(&folder).unwrap_or_default(),
                folder,
            })
        })
        .collect();
    worlds.extend(backups.into_iter().map(|(folder, backups)| World {
        name: folder.clone(),
        folder,
        exists: false,
        last_played_ms: None,
        game_mode: None,
        hardcore: false,
        version: None,
        size_bytes: 0,
        icon: None,
        backups,
    }));
    worlds.sort_by_key(|w| (!w.exists, std::cmp::Reverse(w.last_played_ms.unwrap_or(0))));
    worlds
}

fn archive_error(path: &Path) -> impl Fn(zip::result::ZipError) -> CoreError + '_ {
    move |err| CoreError::Archive {
        path: path.to_path_buf(),
        message: err.to_string(),
    }
}

fn add_world_dir(
    zip: &mut zip::ZipWriter<File>,
    options: zip::write::SimpleFileOptions,
    dir: &Path,
    prefix: &str,
    archive: &Path,
) -> CoreResult<()> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .at_path(dir)?
        .collect::<Result<_, _>>()
        .at_path(dir)?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        let entry_name = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        let kind = entry.file_type().at_path(&path)?;
        if kind.is_dir() {
            add_world_dir(zip, options, &path, &entry_name, archive)?;
        } else if kind.is_file() && !(prefix.is_empty() && name == SESSION_LOCK) {
            zip.start_file(&entry_name, options)
                .map_err(archive_error(archive))?;
            let mut source = File::open(&path).at_path(&path)?;
            std::io::copy(&mut source, zip).at_path(&path)?;
        }
    }
    Ok(())
}

pub fn backup_world(world_dir: &Path, backup_dir: &Path) -> CoreResult<WorldBackup> {
    std::fs::create_dir_all(backup_dir).at_path(backup_dir)?;
    let created_at_ms = now_ms();
    let file_name = format!("{created_at_ms}.zip");
    let target = backup_dir.join(&file_name);
    let temp = backup_dir.join(format!("{created_at_ms}.zip.part"));
    let result = (|| {
        let file = File::create(&temp).at_path(&temp)?;
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .large_file(true);
        add_world_dir(&mut zip, options, world_dir, "", &temp)?;
        zip.finish()
            .map_err(archive_error(&temp))?
            .flush()
            .at_path(&temp)?;
        std::fs::rename(&temp, &target).at_path(&target)
    })();
    if let Err(err) = result {
        let _ = std::fs::remove_file(&temp);
        return Err(err);
    }
    let size_bytes = std::fs::metadata(&target).at_path(&target)?.len();
    Ok(WorldBackup {
        file_name,
        created_at_ms,
        size_bytes,
    })
}

fn extract_backup(archive: &Path, target: &Path) -> CoreResult<()> {
    let file = File::open(archive).at_path(archive)?;
    let mut zip = zip::ZipArchive::new(file).map_err(archive_error(archive))?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(archive_error(archive))?;
        let name = entry.name().map_err(archive_error(archive))?.to_string();
        let relative = name.trim_end_matches('/');
        if relative.is_empty() || entry.is_symlink() {
            continue;
        }
        let path = target.join(validate_pack_path(relative)?);
        if entry.is_dir() {
            std::fs::create_dir_all(&path).at_path(&path)?;
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).at_path(parent)?;
        }
        let mut out = File::create(&path).at_path(&path)?;
        std::io::copy(&mut entry, &mut out).at_path(&path)?;
    }
    Ok(())
}

pub fn restore_backup(saves_dir: &Path, folder: &str, archive: &Path) -> CoreResult<()> {
    validate_world_folder(folder)?;
    std::fs::create_dir_all(saves_dir).at_path(saves_dir)?;
    let staging = saves_dir.join(format!("{RESTORE_PREFIX}{folder}"));
    let replaced = saves_dir.join(format!("{REPLACED_PREFIX}{folder}"));
    let target = saves_dir.join(folder);
    for leftover in [&staging, &replaced] {
        if leftover.exists() {
            std::fs::remove_dir_all(leftover).at_path(leftover)?;
        }
    }
    std::fs::create_dir_all(&staging).at_path(&staging)?;
    if let Err(err) = extract_backup(archive, &staging) {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(err);
    }
    if !staging.join(LEVEL_FILE).is_file() {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(CoreError::InvalidInput(
            "backup contains no level.dat".into(),
        ));
    }
    let had_world = target.exists();
    if had_world {
        std::fs::rename(&target, &replaced).at_path(&target)?;
    }
    if let Err(err) = std::fs::rename(&staging, &target).at_path(&target) {
        if had_world {
            let _ = std::fs::rename(&replaced, &target);
        }
        let _ = std::fs::remove_dir_all(&staging);
        return Err(err);
    }
    if had_world && let Err(err) = std::fs::remove_dir_all(&replaced) {
        tracing::warn!(error = %err, "failed to remove replaced world copy");
    }
    Ok(())
}

async fn blocking<T: Send + 'static>(
    job: impl FnOnce() -> CoreResult<T> + Send + 'static,
) -> CoreResult<T> {
    tokio::task::spawn_blocking(job)
        .await
        .map_err(|err| CoreError::Unsupported(format!("world operation panicked: {err}")))?
}

impl Services {
    fn world_paths(&self, instance_id: &str) -> CoreResult<(PathBuf, PathBuf)> {
        Ok((
            self.instances.game_dir(instance_id)?,
            self.instances.instance_dir(instance_id)?.join(BACKUPS_DIR),
        ))
    }

    pub async fn list_worlds(&self, instance_id: &str) -> CoreResult<WorldList> {
        let instance = self.instances.get(instance_id).await?;
        let (game_dir, backups_dir) = self.world_paths(&instance.id)?;
        let worlds = blocking(move || Ok(list_worlds(&game_dir, &backups_dir))).await?;
        Ok(WorldList {
            worlds,
            quick_play: supports_quick_play(&instance.minecraft_version),
        })
    }

    pub async fn backup_world(&self, instance_id: &str, folder: &str) -> CoreResult<WorldBackup> {
        let instance = self.instances.get(instance_id).await?;
        self.ensure_stopped(&instance)?;
        let (game_dir, backups_dir) = self.world_paths(&instance.id)?;
        let world_dir = ensure_world(&game_dir, folder).await?;
        let backup_dir = backups_dir.join(folder);
        blocking(move || backup_world(&world_dir, &backup_dir)).await
    }

    pub async fn restore_world_backup(
        &self,
        instance_id: &str,
        folder: &str,
        file_name: &str,
    ) -> CoreResult<()> {
        let instance = self.instances.get(instance_id).await?;
        self.ensure_stopped(&instance)?;
        validate_world_folder(folder)?;
        validate_backup_name(file_name)?;
        let (game_dir, backups_dir) = self.world_paths(&instance.id)?;
        let archive = backups_dir.join(folder).join(file_name);
        if !archive.is_file() {
            return Err(CoreError::NotFound(format!("backup {file_name}")));
        }
        let folder = folder.to_string();
        blocking(move || restore_backup(&game_dir.join(SAVES_DIR), &folder, &archive)).await
    }

    pub async fn delete_world(&self, instance_id: &str, folder: &str) -> CoreResult<TrashEntry> {
        let instance = self.instances.get(instance_id).await?;
        self.ensure_stopped(&instance)?;
        let (game_dir, _) = self.world_paths(&instance.id)?;
        let world_dir = ensure_world(&game_dir, folder).await?;
        let item = TrashItem {
            kind: TrashKind::World,
            name: folder.to_string(),
            instance_id: instance.id.clone(),
            folder: Some(folder.to_string()),
        };
        self.trash.put(item, &world_dir).await
    }

    pub(crate) async fn restore_world(&self, entry: &TrashEntry) -> CoreResult<String> {
        let instance = self.instances.get(&entry.instance_id).await?;
        self.ensure_stopped(&instance)?;
        let wanted = entry.folder.as_deref().unwrap_or(&entry.name);
        validate_world_folder(wanted)?;
        let saves = self.instances.game_dir(&instance.id)?.join(SAVES_DIR);
        let lookup = saves.clone();
        let wanted_name = wanted.to_string();
        let folder = blocking(move || Ok(free_folder_name(&lookup, &wanted_name))).await?;
        self.trash.take(&entry.id, &saves.join(&folder)).await?;
        Ok(folder)
    }

    pub async fn delete_world_backup(
        &self,
        instance_id: &str,
        folder: &str,
        file_name: &str,
    ) -> CoreResult<()> {
        validate_world_folder(folder)?;
        validate_backup_name(file_name)?;
        let (_, backups_dir) = self.world_paths(instance_id)?;
        let dir = backups_dir.join(folder);
        let path = dir.join(file_name);
        tokio::fs::remove_file(&path).await.at_path(&path)?;
        if tokio::fs::read_dir(&dir)
            .await
            .at_path(&dir)?
            .next_entry()
            .await
            .at_path(&dir)?
            .is_none()
        {
            tokio::fs::remove_dir(&dir).await.at_path(&dir)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag_string(out: &mut Vec<u8>, value: &str) {
        out.extend((value.len() as u16).to_be_bytes());
        out.extend(value.as_bytes());
    }

    fn level_dat(name: &str, last_played: i64, game_type: i32, hardcore: i8) -> Vec<u8> {
        let mut data = vec![10];
        tag_string(&mut data, "");
        data.push(10);
        tag_string(&mut data, "Data");
        data.push(8);
        tag_string(&mut data, "LevelName");
        tag_string(&mut data, name);
        data.push(4);
        tag_string(&mut data, "LastPlayed");
        data.extend(last_played.to_be_bytes());
        data.push(3);
        tag_string(&mut data, "GameType");
        data.extend(game_type.to_be_bytes());
        data.push(1);
        tag_string(&mut data, "hardcore");
        data.push(hardcore as u8);
        data.push(9);
        tag_string(&mut data, "ServerBrands");
        data.push(8);
        data.extend(1i32.to_be_bytes());
        tag_string(&mut data, "vanilla");
        data.push(11);
        tag_string(&mut data, "Ids");
        data.extend(2i32.to_be_bytes());
        data.extend([0u8; 8]);
        data.push(10);
        tag_string(&mut data, "Version");
        data.push(8);
        tag_string(&mut data, "Name");
        tag_string(&mut data, "1.20.1");
        data.push(0);
        data.push(0);
        data.push(0);
        data
    }

    fn write_world(saves: &Path, folder: &str, name: &str) -> PathBuf {
        let dir = saves.join(folder);
        std::fs::create_dir_all(dir.join("region")).unwrap();
        let mut gz = flate2::write::GzEncoder::new(
            File::create(dir.join(LEVEL_FILE)).unwrap(),
            flate2::Compression::default(),
        );
        gz.write_all(&level_dat(name, 1_700_000_000_000, 1, 1))
            .unwrap();
        gz.finish().unwrap();
        std::fs::write(dir.join("region").join("r.0.0.mca"), vec![7u8; 4096]).unwrap();
        std::fs::write(dir.join(SESSION_LOCK), b"lock").unwrap();
        dir
    }

    #[test]
    fn level_dat_fields_are_read() {
        let info = parse_level(&level_dat("Meine Welt", 42, 1, 1)).unwrap();
        assert_eq!(info.name.as_deref(), Some("Meine Welt"));
        assert_eq!(info.last_played_ms, Some(42));
        assert_eq!(info.game_mode, Some(GameMode::Creative));
        assert!(info.hardcore);
        assert_eq!(info.version.as_deref(), Some("1.20.1"));
    }

    #[test]
    fn truncated_or_hostile_nbt_is_rejected() {
        let data = level_dat("x", 1, 0, 0);
        for len in [0, 1, 5, data.len() / 2, data.len() - 1] {
            assert!(parse_level(&data[..len]).is_none());
        }
        let mut deep = vec![10, 0, 0];
        for _ in 0..100 {
            deep.extend([10, 0, 1, b'a']);
        }
        assert!(parse_level(&deep).is_none());
        let huge_list = [10, 0, 0, 9, 0, 1, b'l', 3, 0x7f, 0xff, 0xff, 0xff];
        assert!(parse_level(&huge_list).is_none());
    }

    #[test]
    fn base64_matches_reference() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn folder_names_are_validated() {
        for bad in ["", ".", "..", "../x", "a/b", "a\\b", ".hidden", "C:"] {
            assert!(validate_world_folder(bad).is_err(), "{bad}");
        }
        assert!(validate_world_folder("Neue Welt (2)").is_ok());
        assert!(validate_backup_name("1700000000000.zip").is_ok());
        assert!(validate_backup_name("../1.zip").is_err());
    }

    #[test]
    fn quick_play_versions() {
        for yes in ["1.20", "1.20.1", "1.21.4", "26.3", "23w14a", "1.20-pre1"] {
            assert!(supports_quick_play(yes), "{yes}");
        }
        for no in ["1.8.9", "1.12.2", "1.19.4", "22w45a", "b1.7.3"] {
            assert!(!supports_quick_play(no), "{no}");
        }
    }

    #[test]
    fn backup_and_restore_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        let saves = game.join(SAVES_DIR);
        let backups = dir.path().join(BACKUPS_DIR);
        let world = write_world(&saves, "Welt", "Meine Welt");

        let backup = backup_world(&world, &backups.join("Welt")).unwrap();
        let archive = backups.join("Welt").join(&backup.file_name);
        let mut zip = zip::ZipArchive::new(File::open(&archive).unwrap()).unwrap();
        assert!(zip.by_name(LEVEL_FILE).is_ok());
        assert!(zip.by_name(SESSION_LOCK).is_err());

        std::fs::write(world.join("region").join("r.0.0.mca"), b"changed").unwrap();
        restore_backup(&saves, "Welt", &archive).unwrap();
        assert_eq!(
            std::fs::read(world.join("region").join("r.0.0.mca")).unwrap(),
            vec![7u8; 4096]
        );
        assert!(!saves.join(format!("{REPLACED_PREFIX}Welt")).exists());
        assert!(!saves.join(format!("{RESTORE_PREFIX}Welt")).exists());

        let worlds = list_worlds(&game, &backups);
        assert_eq!(worlds.len(), 1);
        assert_eq!(worlds[0].name, "Meine Welt");
        assert_eq!(worlds[0].backups.len(), 1);
        assert!(worlds[0].size_bytes >= 4096);

        std::fs::remove_dir_all(&world).unwrap();
        let orphan = list_worlds(&game, &backups);
        assert_eq!(orphan.len(), 1);
        assert!(!orphan[0].exists);
        restore_backup(&saves, "Welt", &archive).unwrap();
        assert!(world.join(LEVEL_FILE).is_file());
    }

    #[test]
    fn restore_rejects_archives_that_escape() {
        let dir = tempfile::tempdir().unwrap();
        let saves = dir.path().join(SAVES_DIR);
        let world = write_world(&saves, "Welt", "Welt");
        let evil = dir.path().join("1.zip");
        {
            let mut zip = zip::ZipWriter::new(File::create(&evil).unwrap());
            let options = zip::write::SimpleFileOptions::default();
            zip.start_file("../../escape.txt", options).unwrap();
            zip.write_all(b"x").unwrap();
            zip.finish().unwrap();
        }
        assert!(restore_backup(&saves, "Welt", &evil).is_err());
        assert!(world.join(LEVEL_FILE).is_file());
        assert!(!dir.path().join("escape.txt").exists());
    }
}

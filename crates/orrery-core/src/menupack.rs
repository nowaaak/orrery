use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde_json::{Value, json};
use sha2::{Digest, Sha512};

use crate::error::IoResultExt;
use crate::extras::ModUpdate;
use crate::game_options::{self, HardwareTier};
use crate::instances::Instance;
use crate::instances::mods::{ContentKind, InstalledMod, ModsDir};
use crate::progress::Stage;
use crate::services::Services;
use crate::storage;
use crate::tasks::{TaskId, TaskKind, TaskSpec};
use crate::tuning::{self, VersionTier};
use crate::{CoreError, CoreResult};

pub const BUNDLED_MENU_ID: &str = "orrery-menu";
pub const MENUPACK_VERSION: &str = "1.0.0";
pub const MENUPACK_TITLE: &str = "Orrery Menu";

const PACK_META: &str = "pack.mcmeta";
const OPTIONS_FILE: &str = "options.txt";
const RESOURCE_PACKS_KEY: &str = "resourcePacks:";
const FALLBACK_FORMAT: u32 = 15;
const MAX_META_BYTES: u64 = 4096;
const FILES: &[(&str, &[u8])] = include!(concat!(env!("OUT_DIR"), "/menupack_files.rs"));

pub struct BundledPack {
    pub bytes: Vec<u8>,
    pub sha512: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activation {
    Select,
    KeepSelection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackFormat {
    Legacy(u32),
    Major(u32),
}

pub struct GameTarget<'a> {
    pub minecraft_version: &'a str,
    pub client_jar: PathBuf,
    pub optimize_options: bool,
}

pub fn file_name() -> String {
    format!("Orrery-Menu-{MENUPACK_VERSION}.zip")
}

pub fn content_version() -> &'static str {
    static VERSION: OnceLock<String> = OnceLock::new();
    VERSION.get_or_init(|| {
        let mut hasher = Sha512::new();
        for (name, bytes) in FILES {
            hasher.update(name.as_bytes());
            hasher.update((bytes.len() as u64).to_le_bytes());
            hasher.update(bytes);
        }
        let digest = hex::encode(hasher.finalize());
        format!("{MENUPACK_VERSION}+{}", &digest[..12])
    })
}

fn legacy_format(minecraft_version: &str) -> Option<u32> {
    let mut parts = minecraft_version.split(['.', '-', ' ']);
    if parts.next()? != "1" {
        return None;
    }
    match parts.next()?.parse::<u32>().ok()? {
        8 => Some(1),
        9 | 10 => Some(2),
        11 | 12 => Some(3),
        13 => Some(4),
        _ => None,
    }
}

fn format_from_version_json(version_json: &Value) -> Option<PackFormat> {
    let pack = version_json.get("pack_version")?;
    let number = |value: Option<&Value>| value?.as_u64().and_then(|v| u32::try_from(v).ok());
    if let Some(format) = number(Some(pack)) {
        return Some(PackFormat::Legacy(format));
    }
    if let Some(major) = number(pack.get("resource_major")) {
        return Some(PackFormat::Major(major));
    }
    number(pack.get("resource")).map(PackFormat::Legacy)
}

pub fn pack_format(minecraft_version: &str, version_json: Option<&Value>) -> PackFormat {
    version_json
        .and_then(format_from_version_json)
        .or_else(|| legacy_format(minecraft_version).map(PackFormat::Legacy))
        .unwrap_or(PackFormat::Legacy(FALLBACK_FORMAT))
}

fn pack_meta(format: PackFormat) -> String {
    let description = format!("{MENUPACK_TITLE} {MENUPACK_VERSION}");
    let pack = match format {
        PackFormat::Legacy(format) => json!({ "description": description, "pack_format": format }),
        PackFormat::Major(major) => {
            json!({ "description": description, "min_format": major, "max_format": major })
        }
    };
    let mut text =
        serde_json::to_string_pretty(&json!({ "pack": pack })).expect("serialize pack metadata");
    text.push('\n');
    text
}

fn build_pack(meta: &str) -> CoreResult<BundledPack> {
    let archive = PathBuf::from(file_name());
    let zip_error = |err: zip::result::ZipError| CoreError::Archive {
        path: archive.clone(),
        message: err.to_string(),
    };
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .last_modified_time(zip::DateTime::DEFAULT)
        .unix_permissions(0o644);
    let entries = std::iter::once((PACK_META, meta.as_bytes())).chain(FILES.iter().copied());
    for (name, bytes) in entries {
        zip.start_file(name, options).map_err(zip_error)?;
        zip.write_all(bytes).at_path(&archive)?;
    }
    let bytes = zip.finish().map_err(zip_error)?.into_inner();
    let sha512 = hex::encode(Sha512::digest(&bytes));
    Ok(BundledPack { bytes, sha512 })
}

fn read_pack_meta(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;
    let entry = archive.by_name(PACK_META).ok()?;
    let mut text = String::new();
    entry.take(MAX_META_BYTES).read_to_string(&mut text).ok()?;
    Some(text)
}

fn is_legacy(minecraft_version: &str) -> bool {
    tuning::version_tier(minecraft_version) == VersionTier::Legacy
}

fn pack_option(minecraft_version: &str, file_name: &str) -> String {
    if is_legacy(minecraft_version) {
        file_name.to_string()
    } else {
        format!("file/{file_name}")
    }
}

fn update_pack_list(
    packs: &[String],
    entry: &str,
    previous: Option<&str>,
    activation: Activation,
) -> Vec<String> {
    let mut packs = packs.to_vec();
    let previous_index = previous
        .filter(|previous| *previous != entry)
        .and_then(|previous| packs.iter().position(|pack| pack == previous));
    if let Some(index) = previous_index {
        if packs.iter().any(|pack| pack == entry) {
            packs.remove(index);
        } else {
            packs[index] = entry.to_string();
        }
    }
    if activation == Activation::Select && !packs.iter().any(|pack| pack == entry) {
        let index = packs
            .iter()
            .rposition(|pack| pack == "vanilla" || pack == "fabric")
            .map_or(0, |index| index + 1);
        packs.insert(index, entry.to_string());
    }
    packs
}

fn resource_packs_line(packs: &[String]) -> String {
    format!(
        "{RESOURCE_PACKS_KEY}{}",
        serde_json::to_string(packs).expect("serialize pack list")
    )
}

fn set_resource_packs(
    text: &str,
    entry: &str,
    previous: Option<&str>,
    activation: Activation,
    legacy: bool,
) -> Option<String> {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let index = lines
        .iter()
        .position(|line| line.starts_with(RESOURCE_PACKS_KEY));
    let current: Vec<String> = match index {
        Some(index) => match serde_json::from_str(&lines[index][RESOURCE_PACKS_KEY.len()..]) {
            Ok(packs) => packs,
            Err(err) => {
                tracing::warn!(error = %err, "options.txt has an unreadable resource pack list");
                return None;
            }
        },
        None if legacy => Vec::new(),
        None => vec!["vanilla".to_string()],
    };
    let updated = update_pack_list(&current, entry, previous, activation);
    if index.is_some() && updated == current {
        return None;
    }
    if index.is_none() && !updated.iter().any(|pack| pack == entry) {
        return None;
    }
    let line = resource_packs_line(&updated);
    match index {
        Some(index) => lines[index] = line,
        None => lines.push(line),
    }
    let mut out = lines.join("\n");
    out.push('\n');
    Some(out)
}

fn initial_options(
    target: &GameTarget<'_>,
    data_version: Option<u32>,
    tier: HardwareTier,
) -> String {
    target
        .optimize_options
        .then(|| game_options::default_options(target.minecraft_version, data_version, tier))
        .flatten()
        .unwrap_or_else(|| {
            data_version
                .map(|version| format!("version:{version}\n"))
                .unwrap_or_default()
        })
}

async fn read_text(path: &Path) -> CoreResult<Option<String>> {
    match tokio::fs::read_to_string(path).await {
        Ok(text) => Ok(Some(text)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err).at_path(path),
    }
}

struct GameInfo {
    version_json: Option<Value>,
    tier: HardwareTier,
}

async fn inspect_game(client_jar: PathBuf) -> CoreResult<GameInfo> {
    tokio::task::spawn_blocking(move || GameInfo {
        version_json: game_options::read_version_json(&client_jar),
        tier: game_options::detect_hardware_tier(),
    })
    .await
    .map_err(|err| CoreError::Unsupported(format!("game inspection panicked: {err}")))
}

async fn update_options(
    game_dir: &Path,
    target: &GameTarget<'_>,
    previous: Option<&str>,
    activation: Activation,
    info: &GameInfo,
) -> CoreResult<()> {
    let path = game_dir.join(OPTIONS_FILE);
    let entry = pack_option(target.minecraft_version, &file_name());
    let previous = previous.map(|previous| pack_option(target.minecraft_version, previous));
    let legacy = is_legacy(target.minecraft_version);
    let updated = match read_text(&path).await? {
        Some(text) => set_resource_packs(&text, &entry, previous.as_deref(), activation, legacy),
        None if activation == Activation::Select => {
            let data_version = info
                .version_json
                .as_ref()
                .and_then(game_options::data_version);
            let text = initial_options(target, data_version, info.tier);
            set_resource_packs(&text, &entry, None, activation, legacy)
        }
        None => None,
    };
    match updated {
        Some(text) => storage::write_bytes_atomic(&path, text.as_bytes()).await,
        None => Ok(()),
    }
}

pub async fn install_bundled(
    mods: &ModsDir,
    target: &GameTarget<'_>,
    activation: Activation,
) -> CoreResult<InstalledMod> {
    let info = inspect_game(target.client_jar.clone()).await?;
    let format = pack_format(target.minecraft_version, info.version_json.as_ref());
    let pack = build_pack(&pack_meta(format))?;
    let file_name = file_name();
    storage::write_bytes_atomic(
        &mods.dir(ContentKind::ResourcePack).join(&file_name),
        &pack.bytes,
    )
    .await?;

    let mut index = mods.load().await?;
    let previous = index
        .mods
        .iter()
        .position(|m| m.project_id == BUNDLED_MENU_ID)
        .map(|position| index.mods.remove(position));
    if let Some(old) = &previous {
        if old.file_name != file_name {
            mods.delete_files(old).await?;
        } else if !old.enabled {
            mods.delete_disabled_copy(old).await?;
        }
    }

    let entry = InstalledMod {
        project_id: BUNDLED_MENU_ID.to_string(),
        version_id: content_version().to_string(),
        title: MENUPACK_TITLE.to_string(),
        version_number: MENUPACK_VERSION.to_string(),
        file_name,
        sha512: pack.sha512.clone(),
        enabled: true,
        dependency: false,
        icon_url: None,
        kind: ContentKind::ResourcePack,
        game_versions: Vec::new(),
        loaders: Vec::new(),
        pack: None,
        pinned: false,
    };
    index.mods.push(entry.clone());
    index.mods.sort_by_key(|m| m.title.to_lowercase());
    mods.save(&index).await?;

    let previous_name = previous.as_ref().map(|old| old.file_name.as_str());
    update_options(mods.game_dir(), target, previous_name, activation, &info).await?;
    Ok(entry)
}

pub async fn refresh(mods: &ModsDir, target: &GameTarget<'_>) -> CoreResult<bool> {
    let index = mods.load().await?;
    let Some(entry) = index
        .mods
        .iter()
        .find(|m| m.project_id == BUNDLED_MENU_ID && m.enabled)
    else {
        return Ok(false);
    };
    let path = mods.file_path(entry);
    let client_jar = target.client_jar.clone();
    let minecraft_version = target.minecraft_version.to_string();
    let metas = tokio::task::spawn_blocking(move || {
        path.is_file().then(|| {
            let version_json = game_options::read_version_json(&client_jar);
            let expected = pack_meta(pack_format(&minecraft_version, version_json.as_ref()));
            (read_pack_meta(&path), expected)
        })
    })
    .await
    .map_err(|err| CoreError::Unsupported(format!("menu pack check panicked: {err}")))?;
    match metas {
        Some((current, expected)) if current.as_deref() != Some(expected.as_str()) => {
            install_bundled(mods, target, Activation::KeepSelection).await?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

pub fn bundled_update(installed: &[InstalledMod]) -> Option<ModUpdate> {
    let entry = installed.iter().find(|m| m.project_id == BUNDLED_MENU_ID)?;
    (entry.version_id != content_version()).then(|| ModUpdate {
        project_id: BUNDLED_MENU_ID.to_string(),
        title: MENUPACK_TITLE.to_string(),
        current_version: entry.version_number.clone(),
        version_id: content_version().to_string(),
        version_number: MENUPACK_VERSION.to_string(),
    })
}

impl Services {
    pub(crate) fn menu_target<'a>(
        &self,
        instance: &'a Instance,
        optimize_options: bool,
    ) -> GameTarget<'a> {
        GameTarget {
            minecraft_version: &instance.minecraft_version,
            client_jar: self.paths.client_jar(&instance.minecraft_version),
            optimize_options,
        }
    }

    pub async fn install_orrery_menu(&self, instance_id: &str) -> CoreResult<TaskId> {
        let instance = self.instances.get(instance_id).await?;
        self.ensure_stopped(&instance)?;
        let services = self.clone();
        self.tasks.spawn(
            TaskSpec {
                kind: TaskKind::InstallOrreryMenu,
                subject: MENUPACK_TITLE.to_string(),
                instance_id: Some(instance.id.clone()),
            },
            move |progress| async move {
                let _guard = services.mods_lock.lock().await;
                let mods = services.mods_dir(&instance)?;
                progress.set_stage(Stage::ExtractingFiles);
                let optimize = services.settings.get().await.optimize_game_options;
                install_bundled(
                    &mods,
                    &services.menu_target(&instance, optimize),
                    Activation::Select,
                )
                .await?;
                progress.set_stage(Stage::Finished);
                Ok(())
            },
        )
    }

    pub(crate) async fn refresh_orrery_menu(&self, instance: &Instance) -> CoreResult<bool> {
        let _guard = self.mods_lock.lock().await;
        let mods = self.mods_dir(instance)?;
        refresh(&mods, &self.menu_target(instance, false)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_size(bytes: &[u8]) -> (u32, u32) {
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(&bytes[12..16], b"IHDR");
        let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
        let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
        (width, height)
    }

    fn file(name: &str) -> Option<&'static [u8]> {
        FILES
            .iter()
            .find(|(file, _)| *file == name)
            .map(|(_, bytes)| *bytes)
    }

    fn mods_dir(root: &Path) -> ModsDir {
        ModsDir::new(root.to_path_buf(), &root.join("minecraft"))
    }

    fn fake_jar(root: &Path, version_json: &str) -> PathBuf {
        let path = root.join("client.jar");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        zip.start_file("version.json", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(version_json.as_bytes()).unwrap();
        zip.finish().unwrap();
        path
    }

    fn target(version: &str, client_jar: PathBuf) -> GameTarget<'_> {
        GameTarget {
            minecraft_version: version,
            client_jar,
            optimize_options: false,
        }
    }

    fn installed_meta(mods: &ModsDir) -> Value {
        let path = mods.dir(ContentKind::ResourcePack).join(file_name());
        serde_json::from_str(&read_pack_meta(&path).unwrap()).unwrap()
    }

    #[test]
    fn pack_contains_valid_menu_assets() {
        let mut panorama_size = None;
        for face in 0..6 {
            let name =
                format!("assets/minecraft/textures/gui/title/background/panorama_{face}.png");
            let (width, height) = png_size(file(&name).unwrap_or_else(|| panic!("{name}")));
            assert_eq!(width, height, "{name} must be square");
            assert_eq!(*panorama_size.get_or_insert(width), width, "{name} size");
        }
        let widgets = "assets/minecraft/textures/gui/sprites/widget/";
        let sprites: Vec<&str> = FILES
            .iter()
            .map(|(name, _)| *name)
            .filter(|name| name.starts_with(widgets) && name.ends_with(".png"))
            .collect();
        assert!(sprites.contains(&"assets/minecraft/textures/gui/sprites/widget/button.png"));
        for name in sprites {
            let (width, height) = png_size(file(name).unwrap());
            let meta: Value =
                serde_json::from_slice(file(&format!("{name}.mcmeta")).unwrap()).unwrap();
            let scaling = &meta["gui"]["scaling"];
            assert_eq!(scaling["type"], "nine_slice", "{name}");
            assert_eq!(scaling["width"], width, "{name}");
            assert_eq!(scaling["height"], height, "{name}");
        }
        for (name, bytes) in FILES {
            if name.ends_with(".png") {
                png_size(bytes);
            }
        }
        let splashes = std::str::from_utf8(file("assets/minecraft/texts/splashes.txt").unwrap())
            .expect("splashes are UTF-8");
        assert!(splashes.lines().count() >= 20);
        assert!(
            splashes
                .lines()
                .all(|line| !line.trim().is_empty() && line.len() <= 40)
        );
        assert!(
            file(PACK_META).is_none(),
            "pack.mcmeta is generated per instance"
        );
    }

    #[test]
    fn pack_is_deterministic_and_includes_metadata() {
        let meta = pack_meta(PackFormat::Major(97));
        let first = build_pack(&meta).unwrap();
        assert_eq!(first.sha512, build_pack(&meta).unwrap().sha512);
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(first.bytes)).unwrap();
        assert_eq!(archive.len(), FILES.len() + 1);
        let mut text = String::new();
        archive
            .by_name(PACK_META)
            .unwrap()
            .read_to_string(&mut text)
            .unwrap();
        assert_eq!(text, meta);
        assert!(content_version().starts_with(&format!("{MENUPACK_VERSION}+")));
    }

    #[test]
    fn pack_format_follows_the_game_version() {
        let parse = |text: &str| serde_json::from_str::<Value>(text).unwrap();
        assert_eq!(
            pack_format("1.16.5", Some(&parse(r#"{"pack_version": 6}"#))),
            PackFormat::Legacy(6)
        );
        assert_eq!(
            pack_format(
                "1.21.1",
                Some(&parse(r#"{"pack_version": {"resource": 34, "data": 48}}"#))
            ),
            PackFormat::Legacy(34)
        );
        assert_eq!(
            pack_format(
                "26.3",
                Some(&parse(
                    r#"{"pack_version": {"resource_major": 97, "resource_minor": 1, "data_major": 121, "data_minor": 0}}"#
                ))
            ),
            PackFormat::Major(97)
        );
        assert_eq!(pack_format("1.8.9", None), PackFormat::Legacy(1));
        assert_eq!(pack_format("1.10.2", None), PackFormat::Legacy(2));
        assert_eq!(pack_format("1.12.2", None), PackFormat::Legacy(3));
        assert_eq!(pack_format("1.13.2", None), PackFormat::Legacy(4));
        assert_eq!(
            pack_format("26.3", None),
            PackFormat::Legacy(FALLBACK_FORMAT)
        );

        let legacy = parse(&pack_meta(PackFormat::Legacy(15)));
        assert_eq!(legacy["pack"]["pack_format"], 15);
        assert!(legacy["pack"].get("min_format").is_none());
        let major = parse(&pack_meta(PackFormat::Major(97)));
        assert_eq!(major["pack"]["min_format"], 97);
        assert_eq!(major["pack"]["max_format"], 97);
        assert!(major["pack"].get("pack_format").is_none());
    }

    #[test]
    fn pack_list_keeps_player_packs_on_top() {
        let list = |items: &[&str]| items.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let entry = "file/Orrery-Menu-1.0.0.zip";
        assert_eq!(
            update_pack_list(
                &list(&["vanilla", "fabric", "file/Faithful.zip"]),
                entry,
                None,
                Activation::Select
            ),
            list(&["vanilla", "fabric", entry, "file/Faithful.zip"])
        );
        assert_eq!(
            update_pack_list(
                &list(&["Faithful.zip"]),
                "Orrery-Menu-1.0.0.zip",
                None,
                Activation::Select
            ),
            list(&["Orrery-Menu-1.0.0.zip", "Faithful.zip"])
        );
        assert_eq!(
            update_pack_list(
                &list(&["vanilla", "file/Orrery-Menu-0.9.0.zip", "file/x.zip"]),
                entry,
                Some("file/Orrery-Menu-0.9.0.zip"),
                Activation::KeepSelection
            ),
            list(&["vanilla", entry, "file/x.zip"])
        );
        assert_eq!(
            update_pack_list(
                &list(&["vanilla", "file/x.zip"]),
                entry,
                Some("file/Orrery-Menu-0.9.0.zip"),
                Activation::KeepSelection
            ),
            list(&["vanilla", "file/x.zip"])
        );
        assert_eq!(
            update_pack_list(&list(&["vanilla", entry]), entry, None, Activation::Select),
            list(&["vanilla", entry])
        );
    }

    #[test]
    fn options_lines_are_replaced_in_place() {
        let text = "version:3465\nresourcePacks:[\"vanilla\",\"file/x.zip\"]\nfov:0.0\n";
        let updated =
            set_resource_packs(text, "file/o.zip", None, Activation::Select, false).unwrap();
        assert_eq!(
            updated,
            "version:3465\nresourcePacks:[\"vanilla\",\"file/o.zip\",\"file/x.zip\"]\nfov:0.0\n"
        );
        assert!(
            set_resource_packs(&updated, "file/o.zip", None, Activation::Select, false).is_none()
        );
        assert_eq!(
            set_resource_packs("fov:0.0\n", "file/o.zip", None, Activation::Select, false).unwrap(),
            "fov:0.0\nresourcePacks:[\"vanilla\",\"file/o.zip\"]\n"
        );
        assert_eq!(
            set_resource_packs("fov:70.0\n", "o.zip", None, Activation::Select, true).unwrap(),
            "fov:70.0\nresourcePacks:[\"o.zip\"]\n"
        );
        assert!(
            set_resource_packs(
                "resourcePacks:[broken\n",
                "file/o.zip",
                None,
                Activation::Select,
                false
            )
            .is_none()
        );
        assert!(
            set_resource_packs(
                "fov:0.0\n",
                "file/o.zip",
                None,
                Activation::KeepSelection,
                false
            )
            .is_none()
        );
    }

    #[tokio::test]
    async fn install_writes_metadata_and_selects_pack() {
        let dir = tempfile::tempdir().unwrap();
        let mods = mods_dir(dir.path());
        let jar = fake_jar(
            dir.path(),
            r#"{"world_version": 3955, "pack_version": {"resource": 34, "data": 48}}"#,
        );
        let entry = install_bundled(&mods, &target("1.21.1", jar), Activation::Select)
            .await
            .unwrap();
        assert_eq!(entry.version_id, content_version());
        assert_eq!(mods.load().await.unwrap().mods, vec![entry.clone()]);
        assert_eq!(installed_meta(&mods)["pack"]["pack_format"], 34);
        let options =
            std::fs::read_to_string(dir.path().join("minecraft").join(OPTIONS_FILE)).unwrap();
        assert_eq!(
            options,
            format!(
                "version:3955\nresourcePacks:[\"vanilla\",\"file/{}\"]\n",
                entry.file_name
            )
        );
        assert!(bundled_update(&[entry]).is_none());
    }

    #[tokio::test]
    async fn legacy_install_uses_plain_file_names() {
        let dir = tempfile::tempdir().unwrap();
        let mods = mods_dir(dir.path());
        let game_dir = dir.path().join("minecraft");
        std::fs::create_dir_all(&game_dir).unwrap();
        std::fs::write(
            game_dir.join(OPTIONS_FILE),
            "resourcePacks:[\"Faithful.zip\"]\n",
        )
        .unwrap();
        install_bundled(
            &mods,
            &target("1.8.9", dir.path().join("missing.jar")),
            Activation::Select,
        )
        .await
        .unwrap();
        assert_eq!(installed_meta(&mods)["pack"]["pack_format"], 1);
        assert_eq!(
            std::fs::read_to_string(game_dir.join(OPTIONS_FILE)).unwrap(),
            format!("resourcePacks:[\"{}\",\"Faithful.zip\"]\n", file_name())
        );
    }

    #[tokio::test]
    async fn refresh_rebuilds_metadata_after_a_version_change() {
        let dir = tempfile::tempdir().unwrap();
        let mods = mods_dir(dir.path());
        let jar = fake_jar(dir.path(), r#"{"pack_version": {"resource": 34}}"#);
        install_bundled(&mods, &target("1.21.1", jar.clone()), Activation::Select)
            .await
            .unwrap();
        assert!(!refresh(&mods, &target("1.21.1", jar)).await.unwrap());

        let jar = fake_jar(
            dir.path(),
            r#"{"pack_version": {"resource_major": 97, "resource_minor": 1}}"#,
        );
        assert!(refresh(&mods, &target("26.3", jar.clone())).await.unwrap());
        assert_eq!(installed_meta(&mods)["pack"]["min_format"], 97);
        let index = mods.load().await.unwrap().mods;
        assert_eq!(index.len(), 1);
        let bytes = std::fs::read(mods.file_path(&index[0])).unwrap();
        assert_eq!(index[0].sha512, hex::encode(Sha512::digest(&bytes)));
        assert!(!refresh(&mods, &target("26.3", jar.clone())).await.unwrap());

        mods.set_enabled(BUNDLED_MENU_ID, false).await.unwrap();
        let old = fake_jar(dir.path(), r#"{"pack_version": 6}"#);
        assert!(!refresh(&mods, &target("1.16.5", old)).await.unwrap());
    }

    #[tokio::test]
    async fn update_keeps_a_deselected_pack_deselected() {
        let dir = tempfile::tempdir().unwrap();
        let mods = mods_dir(dir.path());
        let game_dir = dir.path().join("minecraft");
        let pack_dir = mods.dir(ContentKind::ResourcePack);
        std::fs::create_dir_all(&pack_dir).unwrap();
        let old_name = "Orrery-Menu-0.9.0.zip";
        std::fs::write(pack_dir.join(old_name), b"old").unwrap();
        let old = InstalledMod {
            project_id: BUNDLED_MENU_ID.into(),
            version_id: "0.9.0+abc".into(),
            title: MENUPACK_TITLE.into(),
            version_number: "0.9.0".into(),
            file_name: old_name.into(),
            sha512: "old".into(),
            enabled: true,
            dependency: false,
            icon_url: None,
            kind: ContentKind::ResourcePack,
            game_versions: Vec::new(),
            loaders: Vec::new(),
            pack: None,
            pinned: false,
        };
        assert!(bundled_update(std::slice::from_ref(&old)).is_some());
        mods.save(&crate::instances::mods::ModsFile {
            schema: 1,
            mods: vec![old],
        })
        .await
        .unwrap();
        std::fs::write(game_dir.join(OPTIONS_FILE), "resourcePacks:[\"vanilla\"]\n").unwrap();

        install_bundled(
            &mods,
            &target("1.20.1", dir.path().join("missing.jar")),
            Activation::KeepSelection,
        )
        .await
        .unwrap();
        assert!(!pack_dir.join(old_name).exists());
        assert!(pack_dir.join(file_name()).exists());
        assert_eq!(
            std::fs::read_to_string(game_dir.join(OPTIONS_FILE)).unwrap(),
            "resourcePacks:[\"vanilla\"]\n"
        );
    }
}

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::Digest;
use ts_rs::TS;

use crate::curseforge::models::parse_project_key;
use crate::curseforge::modpack::{is_curseforge_pack, plan as plan_curseforge, read_manifest};
use crate::error::IoResultExt;
use crate::instances::mods::{ContentKind, ContentSource, InstalledMod, ModsDir};
use crate::instances::{Instance, LoaderSpec, NewInstance};
use crate::loaders;
use crate::modrinth::ModrinthClient;
use crate::modrinth::install::pick_version;
use crate::modrinth::models::{ModSearchPage, Version};
use crate::net::{Checksum, Download, Http, VerifyMode, download_all};
use crate::progress::{Progress, Stage};
use crate::services::Services;
use crate::tasks::{TaskId, TaskKind, TaskSpec};
use crate::{CoreError, CoreResult};

pub const INDEX_FILE: &str = "modrinth.index.json";
pub const MODPACK_LOADERS: [&str; 2] = ["fabric", "quilt"];
const OVERRIDE_PREFIXES: [&str; 2] = ["overrides/", "client-overrides/"];
const ALLOWED_DOWNLOAD_HOSTS: [&str; 4] = [
    "cdn.modrinth.com",
    "github.com",
    "raw.githubusercontent.com",
    "gitlab.com",
];
const EXPORTED_FOLDERS: [&str; 6] = [
    "config",
    "defaultconfigs",
    "kubejs",
    "scripts",
    "resources",
    "structures",
];
const RESERVED_NAMES: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];
const MAX_INDEX_BYTES: u64 = 16 * 1024 * 1024;
const MAX_OVERRIDE_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_PATH_CHARS: usize = 512;
const MAX_INSTANCE_NAME_CHARS: usize = 64;
const DOWNLOAD_CONCURRENCY: usize = 8;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackIndex {
    pub format_version: u32,
    pub game: String,
    pub version_id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default)]
    pub files: Vec<PackFile>,
    #[serde(default)]
    pub dependencies: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackFile {
    pub path: String,
    pub hashes: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<PackEnv>,
    pub downloads: Vec<String>,
    pub file_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackEnv {
    pub client: EnvSupport,
    pub server: EnvSupport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnvSupport {
    Required,
    Optional,
    Unsupported,
}

#[derive(Debug, Clone)]
pub struct PackPlan {
    pub name: String,
    pub minecraft_version: String,
    pub loader: LoaderSpec,
    pub files: Vec<PlannedFile>,
}

#[derive(Debug, Clone)]
pub struct PlannedFile {
    pub path: PathBuf,
    pub url: String,
    pub sha512: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ModpackExport {
    pub linked: u32,
    pub override_files: u32,
    pub skipped: Vec<String>,
}

pub(crate) struct LocalContent {
    pub(crate) kind: ContentKind,
    pub(crate) file_name: String,
    pub(crate) sha512: String,
    pub(crate) size: u64,
}

pub fn validate_pack_path(path: &str) -> CoreResult<PathBuf> {
    let invalid = || CoreError::InvalidInput(format!("unsafe path in modpack: {path:?}"));
    if path.is_empty()
        || path.chars().count() > MAX_PATH_CHARS
        || path.starts_with('/')
        || path.contains(['\\', ':'])
    {
        return Err(invalid());
    }
    let mut relative = PathBuf::new();
    for part in path.split('/') {
        let stem = part
            .split('.')
            .next()
            .unwrap_or_default()
            .to_ascii_uppercase();
        if part.is_empty()
            || part == "."
            || part == ".."
            || part.chars().any(char::is_control)
            || RESERVED_NAMES.contains(&stem.as_str())
        {
            return Err(invalid());
        }
        relative.push(part);
    }
    Ok(relative)
}

pub fn is_allowed_download(url: &str) -> bool {
    url.strip_prefix("https://")
        .and_then(|rest| rest.split(['/', '?', '#']).next())
        .is_some_and(|host| ALLOWED_DOWNLOAD_HOSTS.contains(&host))
}

pub(crate) fn archive_error(path: &Path) -> impl Fn(zip::result::ZipError) -> CoreError + '_ {
    move |err| CoreError::Archive {
        path: path.to_path_buf(),
        message: err.to_string(),
    }
}

pub fn read_index(archive: &Path) -> CoreResult<PackIndex> {
    let file = File::open(archive).at_path(archive)?;
    let mut zip = zip::ZipArchive::new(file).map_err(archive_error(archive))?;
    let entry = zip.by_name(INDEX_FILE).map_err(|_| {
        CoreError::InvalidInput("not a Modrinth modpack: modrinth.index.json is missing".into())
    })?;
    let mut text = String::new();
    entry
        .take(MAX_INDEX_BYTES)
        .read_to_string(&mut text)
        .at_path(archive)?;
    serde_json::from_str(&text)
        .map_err(|err| CoreError::InvalidInput(format!("invalid modrinth.index.json: {err}")))
}

pub fn plan(index: PackIndex) -> CoreResult<PackPlan> {
    if index.format_version != 1 {
        return Err(CoreError::Unsupported(format!(
            "modpack format version {}",
            index.format_version
        )));
    }
    if index.game != "minecraft" {
        return Err(CoreError::Unsupported(format!(
            "modpacks for {}",
            index.game
        )));
    }
    let mut minecraft_version = None;
    let mut loader = LoaderSpec::Vanilla;
    for (key, value) in &index.dependencies {
        match key.as_str() {
            "minecraft" => minecraft_version = Some(value.clone()),
            "fabric-loader" => {
                loader = LoaderSpec::Fabric {
                    loader_version: value.clone(),
                }
            }
            "quilt-loader" => {
                loader = LoaderSpec::Quilt {
                    loader_version: value.clone(),
                }
            }
            "neoforge" => {
                loader = LoaderSpec::NeoForge {
                    loader_version: value.clone(),
                }
            }
            "forge" => {
                loader = LoaderSpec::Forge {
                    loader_version: value.clone(),
                }
            }
            other => {
                return Err(CoreError::Unsupported(format!(
                    "modpack dependency {other}"
                )));
            }
        }
    }
    let minecraft_version = minecraft_version.ok_or_else(|| {
        CoreError::InvalidInput("modpack does not name a Minecraft version".into())
    })?;
    if let LoaderSpec::Fabric { loader_version } = &loader
        && loaders::fabric_needs_legacy(&minecraft_version)
    {
        loader = LoaderSpec::LegacyFabric {
            loader_version: loader_version.clone(),
        };
    }

    let mut seen = HashSet::new();
    let mut files = Vec::new();
    for file in index.files {
        if file
            .env
            .as_ref()
            .is_some_and(|env| env.client == EnvSupport::Unsupported)
        {
            continue;
        }
        let path = validate_pack_path(&file.path)?;
        if !seen.insert(path.clone()) {
            return Err(CoreError::InvalidInput(format!(
                "modpack lists {} twice",
                file.path
            )));
        }
        let url = file
            .downloads
            .iter()
            .find(|url| is_allowed_download(url))
            .ok_or_else(|| {
                CoreError::InvalidInput(format!(
                    "{} has no download from an allowed host",
                    file.path
                ))
            })?;
        let sha512 = file
            .hashes
            .get("sha512")
            .filter(|hash| hash.len() == 128 && hash.chars().all(|c| c.is_ascii_hexdigit()))
            .ok_or_else(|| CoreError::InvalidInput(format!("{} has no sha512 hash", file.path)))?;
        files.push(PlannedFile {
            path,
            url: url.clone(),
            sha512: sha512.to_ascii_lowercase(),
            size: file.file_size,
        });
    }

    let name: String = index
        .name
        .trim()
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_INSTANCE_NAME_CHARS)
        .collect();
    Ok(PackPlan {
        name: if name.trim().is_empty() {
            "Modpack".into()
        } else {
            name.trim().to_string()
        },
        minecraft_version,
        loader,
        files,
    })
}

pub fn extract_overrides(archive: &Path, game_dir: &Path) -> CoreResult<u32> {
    extract_prefixed(archive, game_dir, &OVERRIDE_PREFIXES)
}

pub fn extract_prefixed(archive: &Path, game_dir: &Path, prefixes: &[&str]) -> CoreResult<u32> {
    let file = File::open(archive).at_path(archive)?;
    let mut zip = zip::ZipArchive::new(file).map_err(archive_error(archive))?;
    let mut budget = MAX_OVERRIDE_BYTES;
    let mut count = 0;
    for prefix in prefixes {
        for index in 0..zip.len() {
            let mut entry = zip.by_index(index).map_err(archive_error(archive))?;
            let name = entry.name().map_err(archive_error(archive))?.to_string();
            let Some(relative) = name.strip_prefix(prefix) else {
                continue;
            };
            let relative = relative.trim_end_matches('/');
            if relative.is_empty() || entry.is_symlink() {
                continue;
            }
            let target = game_dir.join(validate_pack_path(relative)?);
            if entry.is_dir() {
                std::fs::create_dir_all(&target).at_path(&target)?;
                continue;
            }
            if entry.size() > budget {
                return Err(CoreError::InvalidInput(
                    "modpack overrides are too large".into(),
                ));
            }
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).at_path(parent)?;
            }
            let mut out = File::create(&target).at_path(&target)?;
            let written =
                std::io::copy(&mut (&mut entry).take(budget), &mut out).at_path(&target)?;
            budget -= written;
            count += 1;
        }
    }
    Ok(count)
}

pub(crate) fn sha512_file(path: &Path) -> CoreResult<String> {
    let mut file = File::open(path).at_path(path)?;
    let mut hasher = sha2::Sha512::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).at_path(path)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn scan_content(game_dir: &Path) -> CoreResult<Vec<LocalContent>> {
    let mut found = Vec::new();
    for kind in ContentKind::ALL {
        let dir = game_dir.join(kind.folder());
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries {
            let entry = entry.at_path(&dir)?;
            let path = entry.path();
            let file_type = entry.file_type().at_path(&path)?;
            let Some(file_name) = entry.file_name().to_str().map(str::to_string) else {
                continue;
            };
            if !file_type.is_file() || !file_name.ends_with(kind.extension()) {
                continue;
            }
            let size = entry.metadata().at_path(&path)?.len();
            found.push(LocalContent {
                kind,
                file_name,
                sha512: sha512_file(&path)?,
                size,
            });
        }
    }
    Ok(found)
}

pub(crate) async fn scan_content_async(game_dir: &Path) -> CoreResult<Vec<LocalContent>> {
    let game_dir = game_dir.to_path_buf();
    tokio::task::spawn_blocking(move || scan_content(&game_dir))
        .await
        .map_err(|err| CoreError::Unsupported(format!("content scan panicked: {err}")))?
}

pub async fn install_files(
    http: &Http,
    plan: &PackPlan,
    game_dir: &Path,
    progress: &Progress,
) -> CoreResult<()> {
    let downloads = plan
        .files
        .iter()
        .map(|file| Download {
            url: file.url.clone(),
            path: game_dir.join(&file.path),
            checksum: Some(Checksum::Sha512(file.sha512.clone())),
            size: Some(file.size),
        })
        .collect();
    download_all(
        http,
        downloads,
        DOWNLOAD_CONCURRENCY,
        VerifyMode::Full,
        progress,
    )
    .await
}

pub async fn index_content(
    client: &ModrinthClient,
    mods: &ModsDir,
    game_dir: &Path,
) -> CoreResult<usize> {
    let local = scan_content_async(game_dir).await?;
    let hashes: Vec<String> = local.iter().map(|c| c.sha512.clone()).collect();
    let versions = client.versions_for_hashes(&hashes).await?;
    let mut project_ids: Vec<String> = versions.values().map(|v| v.project_id.clone()).collect();
    project_ids.sort();
    project_ids.dedup();
    let projects: HashMap<String, (String, Option<String>)> = client
        .projects(&project_ids)
        .await?
        .into_iter()
        .map(|p| (p.id, (p.title, p.icon_url)))
        .collect();

    let mut index = mods.load().await?;
    for content in local {
        let Some(version) = versions.get(&content.sha512) else {
            continue;
        };
        if index.mods.iter().any(|m| {
            m.sha512 == content.sha512
                || (m.kind == content.kind && m.file_name == content.file_name)
        }) {
            continue;
        }
        if index
            .mods
            .iter()
            .any(|m| m.project_id == version.project_id)
        {
            continue;
        }
        let (title, icon_url) = projects
            .get(&version.project_id)
            .cloned()
            .unwrap_or_else(|| (version.name.clone(), None));
        index.mods.push(InstalledMod {
            project_id: version.project_id.clone(),
            version_id: version.id.clone(),
            title,
            version_number: version.version_number.clone(),
            file_name: content.file_name,
            sha512: content.sha512,
            enabled: true,
            dependency: false,
            icon_url,
            kind: content.kind,
            game_versions: version.game_versions.clone(),
            loaders: version.loaders.clone(),
            pack: None,
            pinned: false,
        });
    }
    index.mods.sort_by_key(|m| m.title.to_lowercase());
    let count = index.mods.len();
    mods.save(&index).await?;
    Ok(count)
}

pub fn pick_modpack_version(versions: Vec<Version>, game_version: Option<&str>) -> Option<Version> {
    let versions = versions
        .into_iter()
        .filter(|v| game_version.is_none_or(|gv| v.game_versions.iter().any(|g| g == gv)))
        .collect();
    pick_version(versions)
}

pub async fn download_pack(
    http: &Http,
    version: &Version,
    cache_dir: &Path,
    progress: &Progress,
) -> CoreResult<PathBuf> {
    let file = version
        .files
        .iter()
        .filter(|f| f.filename.ends_with(".mrpack"))
        .max_by_key(|f| f.primary)
        .ok_or_else(|| CoreError::NotFound(format!("a .mrpack file in {}", version.name)))?;
    if !file.url.starts_with("https://cdn.modrinth.com/") {
        return Err(CoreError::InvalidInput(format!(
            "unexpected download host for {}",
            file.filename
        )));
    }
    let sha512 = file
        .hashes
        .get("sha512")
        .filter(|hash| hash.len() == 128 && hash.chars().all(|c| c.is_ascii_hexdigit()))
        .ok_or_else(|| CoreError::NotFound(format!("sha512 for {}", file.filename)))?;
    let path = cache_dir
        .join("modpacks")
        .join(format!("{}.mrpack", &sha512[..32]));
    download_all(
        http,
        vec![Download {
            url: file.url.clone(),
            path: path.clone(),
            checksum: Some(Checksum::Sha512(sha512.clone())),
            size: Some(file.size),
        }],
        1,
        VerifyMode::Full,
        progress,
    )
    .await?;
    Ok(path)
}

pub async fn export(
    client: &ModrinthClient,
    instance: &Instance,
    game_dir: &Path,
    target: &Path,
) -> CoreResult<ModpackExport> {
    if !target.is_absolute() || target.extension().and_then(|e| e.to_str()) != Some("mrpack") {
        return Err(CoreError::InvalidInput(
            "export target must be an absolute .mrpack path".into(),
        ));
    }
    let local = scan_content_async(game_dir).await?;
    let hashes: Vec<String> = local.iter().map(|c| c.sha512.clone()).collect();
    let versions = client.versions_for_hashes(&hashes).await?;

    let mut files = Vec::new();
    let mut skipped = Vec::new();
    for content in &local {
        let remote = versions.get(&content.sha512).and_then(|version| {
            version
                .files
                .iter()
                .find(|f| f.hashes.get("sha512") == Some(&content.sha512))
        });
        match remote {
            Some(remote)
                if is_allowed_download(&remote.url) && remote.hashes.contains_key("sha1") =>
            {
                files.push(PackFile {
                    path: format!("{}/{}", content.kind.folder(), content.file_name),
                    hashes: BTreeMap::from([
                        ("sha1".to_string(), remote.hashes["sha1"].clone()),
                        ("sha512".to_string(), content.sha512.clone()),
                    ]),
                    env: None,
                    downloads: vec![remote.url.clone()],
                    file_size: content.size,
                });
            }
            _ => skipped.push(content.file_name.clone()),
        }
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    skipped.sort();

    let mut dependencies =
        BTreeMap::from([("minecraft".to_string(), instance.minecraft_version.clone())]);
    match &instance.loader {
        LoaderSpec::Vanilla => {}
        LoaderSpec::Fabric { loader_version } | LoaderSpec::LegacyFabric { loader_version } => {
            dependencies.insert("fabric-loader".into(), loader_version.clone());
        }
        LoaderSpec::Quilt { loader_version } => {
            dependencies.insert("quilt-loader".into(), loader_version.clone());
        }
        LoaderSpec::Forge { loader_version } => {
            dependencies.insert("forge".into(), loader_version.clone());
        }
        LoaderSpec::NeoForge { loader_version } => {
            dependencies.insert("neoforge".into(), loader_version.clone());
        }
    }
    let index = PackIndex {
        format_version: 1,
        game: "minecraft".into(),
        version_id: "1.0.0".into(),
        name: instance.name.clone(),
        summary: None,
        files,
        dependencies,
    };
    let linked = u32::try_from(index.files.len()).unwrap_or(u32::MAX);
    let game_dir = game_dir.to_path_buf();
    let target = target.to_path_buf();
    let index_json = serde_json::to_vec_pretty(&index)
        .map_err(|err| CoreError::InvalidInput(format!("modpack index: {err}")))?;
    let override_files = tokio::task::spawn_blocking(move || {
        write_pack(&target, INDEX_FILE, &index_json, &game_dir)
    })
    .await
    .map_err(|err| CoreError::Unsupported(format!("modpack export panicked: {err}")))??;
    Ok(ModpackExport {
        linked,
        override_files,
        skipped,
    })
}

pub(crate) fn write_pack(
    target: &Path,
    index_name: &str,
    index: &[u8],
    game_dir: &Path,
) -> CoreResult<u32> {
    let extension = target
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("pack");
    let temp = target.with_extension(format!("{extension}.part"));
    let result = write_pack_to(&temp, index_name, index, game_dir).and_then(|count| {
        std::fs::rename(&temp, target)
            .at_path(target)
            .map(|()| count)
    });
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

fn write_pack_to(path: &Path, index_name: &str, index: &[u8], game_dir: &Path) -> CoreResult<u32> {
    let file = File::create(path).at_path(path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file(index_name, options)
        .map_err(archive_error(path))?;
    zip.write_all(index).at_path(path)?;

    let mut count = 0;
    for folder in EXPORTED_FOLDERS {
        let dir = game_dir.join(folder);
        if dir.is_dir() {
            add_dir(
                &mut zip,
                options,
                &dir,
                &format!("overrides/{folder}"),
                path,
                &mut count,
            )?;
        }
    }
    zip.finish().map_err(archive_error(path))?;
    Ok(count)
}

fn add_dir(
    zip: &mut zip::ZipWriter<File>,
    options: zip::write::SimpleFileOptions,
    dir: &Path,
    prefix: &str,
    archive: &Path,
    count: &mut u32,
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
        let file_type = entry.file_type().at_path(&path)?;
        let entry_name = format!("{prefix}/{name}");
        if file_type.is_dir() {
            add_dir(zip, options, &path, &entry_name, archive, count)?;
        } else if file_type.is_file() {
            zip.start_file(&entry_name, options)
                .map_err(archive_error(archive))?;
            let mut source = File::open(&path).at_path(&path)?;
            std::io::copy(&mut source, zip).at_path(&path)?;
            *count += 1;
        }
    }
    Ok(())
}

impl Services {
    pub async fn search_modpacks(
        &self,
        source: ContentSource,
        query: &str,
        offset: u32,
    ) -> CoreResult<ModSearchPage> {
        match source {
            ContentSource::CurseForge => self.search_curseforge_modpacks(query, offset).await,
            ContentSource::Modrinth => {
                self.modrinth
                    .search(query, None, "modpack", &MODPACK_LOADERS, offset)
                    .await
            }
        }
    }

    pub async fn is_modpack_file(&self, path: PathBuf) -> bool {
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase);
        match extension.as_deref() {
            Some("mrpack") => path.is_file(),
            Some("zip") => tokio::task::spawn_blocking(move || is_curseforge_pack(&path))
                .await
                .unwrap_or(false),
            _ => false,
        }
    }

    pub async fn install_modpack(&self, project_id: &str, title: &str) -> CoreResult<TaskId> {
        if let Some(mod_id) = parse_project_key(project_id) {
            return self.install_curseforge_modpack(mod_id, title).await;
        }
        let versions = self
            .modrinth
            .modpack_versions(project_id, &MODPACK_LOADERS)
            .await?;
        let version = pick_modpack_version(versions, None)
            .ok_or_else(|| CoreError::NotFound(format!("a Fabric or Quilt version of {title}")))?;
        let services = self.clone();
        self.tasks.spawn(
            TaskSpec {
                kind: TaskKind::InstallModpack,
                subject: title.to_string(),
                instance_id: None,
            },
            move |progress| async move {
                progress.set_stage(Stage::DownloadingMods);
                let archive =
                    download_pack(&services.http, &version, &services.paths.cache, &progress)
                        .await?;
                let result = services.import_archive(&archive, &progress).await;
                if let Err(err) = tokio::fs::remove_file(&archive).await {
                    tracing::warn!(error = %err, "failed to remove cached modpack");
                }
                result.map(|_| ())
            },
        )
    }

    pub async fn import_modpack_file(&self, path: PathBuf) -> CoreResult<TaskId> {
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase);
        if extension.as_deref() == Some("zip") && path.is_file() {
            return self.import_curseforge_file(path).await;
        }
        if extension.as_deref() != Some("mrpack") || !path.is_file() {
            return Err(CoreError::InvalidInput(
                "choose a .mrpack or CurseForge .zip file".into(),
            ));
        }
        let archive = path.clone();
        let index = tokio::task::spawn_blocking(move || read_index(&archive))
            .await
            .map_err(|err| CoreError::Unsupported(format!("modpack read panicked: {err}")))??;
        let title = plan(index)?.name;
        let services = self.clone();
        self.tasks.spawn(
            TaskSpec {
                kind: TaskKind::ImportModpack,
                subject: title,
                instance_id: None,
            },
            move |progress| async move {
                services.import_archive(&path, &progress).await.map(|_| ())
            },
        )
    }

    async fn import_curseforge_file(&self, path: PathBuf) -> CoreResult<TaskId> {
        if !self.curseforge.is_available() {
            return Err(CoreError::Unsupported(
                "this build has no CurseForge API key".into(),
            ));
        }
        let archive = path.clone();
        let manifest = tokio::task::spawn_blocking(move || read_manifest(&archive))
            .await
            .map_err(|err| CoreError::Unsupported(format!("modpack read panicked: {err}")))??;
        let title = plan_curseforge(manifest)?.name;
        let services = self.clone();
        self.tasks.spawn(
            TaskSpec {
                kind: TaskKind::ImportModpack,
                subject: title,
                instance_id: None,
            },
            move |progress| async move {
                services
                    .import_curseforge_archive(&path, &progress)
                    .await
                    .map(|_| ())
            },
        )
    }

    pub async fn export_modpack(
        &self,
        instance_id: &str,
        target: PathBuf,
    ) -> CoreResult<ModpackExport> {
        let instance = self.instances.get(instance_id).await?;
        let game_dir = self.instances.game_dir(&instance.id)?;
        if target.extension().and_then(|e| e.to_str()) == Some("zip") {
            return crate::curseforge::modpack::export(
                &self.curseforge,
                &instance,
                &game_dir,
                &target,
            )
            .await;
        }
        export(&self.modrinth, &instance, &game_dir, &target).await
    }

    async fn import_archive(&self, archive: &Path, progress: &Progress) -> CoreResult<Instance> {
        progress.set_stage(Stage::FetchingMetadata);
        let source = archive.to_path_buf();
        let index = tokio::task::spawn_blocking(move || read_index(&source))
            .await
            .map_err(|err| CoreError::Unsupported(format!("modpack read panicked: {err}")))??;
        let plan = plan(index)?;
        let instance = self
            .instances
            .create(NewInstance {
                name: plan.name.clone(),
                minecraft_version: plan.minecraft_version.clone(),
                loader: plan.loader.clone(),
                copy_settings_from: None,
            })
            .await?;
        match self
            .fill_instance(&instance, &plan, archive, progress)
            .await
        {
            Ok(()) => Ok(instance),
            Err(err) => {
                if let Err(cleanup) = self.instances.delete(&instance.id).await {
                    tracing::warn!(error = %cleanup, "failed to remove incomplete modpack instance");
                }
                Err(err)
            }
        }
    }

    async fn fill_instance(
        &self,
        instance: &Instance,
        plan: &PackPlan,
        archive: &Path,
        progress: &Progress,
    ) -> CoreResult<()> {
        let game_dir = self.instances.game_dir(&instance.id)?;
        progress.set_stage(Stage::DownloadingMods);
        install_files(&self.http, plan, &game_dir, progress).await?;
        progress.check_cancelled()?;

        progress.set_stage(Stage::ExtractingFiles);
        let source = archive.to_path_buf();
        let target = game_dir.clone();
        tokio::task::spawn_blocking(move || extract_overrides(&source, &target))
            .await
            .map_err(|err| {
                CoreError::Unsupported(format!("override extraction panicked: {err}"))
            })??;
        progress.check_cancelled()?;

        progress.set_stage(Stage::FetchingMetadata);
        let mods = ModsDir::new(self.instances.instance_dir(&instance.id)?, &game_dir);
        index_content(&self.modrinth, &mods, &game_dir).await?;
        progress.set_stage(Stage::Finished);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index(files: serde_json::Value, dependencies: serde_json::Value) -> PackIndex {
        serde_json::from_value(serde_json::json!({
            "formatVersion": 1,
            "game": "minecraft",
            "versionId": "1.0",
            "name": "Test Pack",
            "files": files,
            "dependencies": dependencies,
        }))
        .unwrap()
    }

    fn file(path: &str, url: &str, client: &str) -> serde_json::Value {
        serde_json::json!({
            "path": path,
            "hashes": { "sha1": "a".repeat(40), "sha512": "b".repeat(128) },
            "env": { "client": client, "server": "required" },
            "downloads": [url],
            "fileSize": 3,
        })
    }

    #[test]
    fn unsafe_paths_are_rejected() {
        for path in [
            "../evil.jar",
            "mods/../../evil.jar",
            "/etc/passwd",
            "C:/Windows/x.dll",
            "mods\\x.jar",
            "mods//x.jar",
            "mods/./x.jar",
            "mods/NUL.jar",
            "",
        ] {
            assert!(
                validate_pack_path(path).is_err(),
                "{path} should be rejected"
            );
        }
        assert_eq!(
            validate_pack_path("config/sodium-options.json").unwrap(),
            PathBuf::from("config").join("sodium-options.json")
        );
    }

    #[test]
    fn only_allowed_hosts_are_used() {
        assert!(is_allowed_download("https://cdn.modrinth.com/data/x/y.jar"));
        assert!(is_allowed_download("https://github.com/a/b/releases/x.jar"));
        assert!(!is_allowed_download("http://cdn.modrinth.com/x.jar"));
        assert!(!is_allowed_download(
            "https://cdn.modrinth.com.evil.io/x.jar"
        ));
        assert!(!is_allowed_download("https://user@cdn.modrinth.com/x.jar"));
        assert!(!is_allowed_download("https://example.com/x.jar"));
    }

    #[test]
    fn plan_reads_loader_and_skips_server_only_files() {
        let plan = plan(index(
            serde_json::json!([
                file("mods/a.jar", "https://cdn.modrinth.com/a.jar", "required"),
                file(
                    "mods/server.jar",
                    "https://cdn.modrinth.com/s.jar",
                    "unsupported"
                ),
            ]),
            serde_json::json!({ "minecraft": "1.20.1", "fabric-loader": "0.16.0" }),
        ))
        .unwrap();
        assert_eq!(plan.minecraft_version, "1.20.1");
        assert!(
            matches!(plan.loader, LoaderSpec::Fabric { ref loader_version } if loader_version == "0.16.0")
        );
        assert_eq!(plan.files.len(), 1);
        assert_eq!(plan.name, "Test Pack");
    }

    #[test]
    fn plan_reads_forge_loaders_and_rejects_foreign_hosts() {
        let forge = plan(index(
            serde_json::json!([]),
            serde_json::json!({ "minecraft": "1.20.1", "forge": "47.0.0" }),
        ))
        .unwrap();
        assert_eq!(
            forge.loader,
            LoaderSpec::Forge {
                loader_version: "47.0.0".into()
            }
        );
        let legacy = plan(index(
            serde_json::json!([]),
            serde_json::json!({ "minecraft": "1.8.9", "fabric-loader": "0.20.1" }),
        ))
        .unwrap();
        assert_eq!(
            legacy.loader,
            LoaderSpec::LegacyFabric {
                loader_version: "0.20.1".into()
            }
        );
        let neoforge = plan(index(
            serde_json::json!([]),
            serde_json::json!({ "minecraft": "1.21.1", "neoforge": "21.1.265" }),
        ))
        .unwrap();
        assert_eq!(
            neoforge.loader,
            LoaderSpec::NeoForge {
                loader_version: "21.1.265".into()
            }
        );
        let foreign = plan(index(
            serde_json::json!([file("mods/a.jar", "https://example.com/a.jar", "required")]),
            serde_json::json!({ "minecraft": "1.20.1" }),
        ));
        assert!(matches!(foreign, Err(CoreError::InvalidInput(_))));
    }

    #[test]
    fn overrides_are_extracted_without_escaping_the_game_dir() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("pack.mrpack");
        {
            let mut zip = zip::ZipWriter::new(File::create(&archive).unwrap());
            let options = zip::write::SimpleFileOptions::default();
            zip.start_file(INDEX_FILE, options).unwrap();
            zip.write_all(b"{}").unwrap();
            zip.start_file("overrides/config/a.txt", options).unwrap();
            zip.write_all(b"base").unwrap();
            zip.start_file("client-overrides/config/a.txt", options)
                .unwrap();
            zip.write_all(b"client").unwrap();
            zip.start_file("server-overrides/config/b.txt", options)
                .unwrap();
            zip.write_all(b"server").unwrap();
            zip.finish().unwrap();
        }
        let game_dir = dir.path().join("game");
        std::fs::create_dir_all(&game_dir).unwrap();
        let count = extract_overrides(&archive, &game_dir).unwrap();
        assert_eq!(count, 2);
        assert_eq!(
            std::fs::read_to_string(game_dir.join("config").join("a.txt")).unwrap(),
            "client"
        );
        assert!(!game_dir.join("config").join("b.txt").exists());

        let evil = dir.path().join("evil.mrpack");
        {
            let mut zip = zip::ZipWriter::new(File::create(&evil).unwrap());
            let options = zip::write::SimpleFileOptions::default();
            zip.start_file("overrides/../escape.txt", options).unwrap();
            zip.write_all(b"x").unwrap();
            zip.finish().unwrap();
        }
        assert!(extract_overrides(&evil, &game_dir).is_err());
        assert!(!dir.path().join("escape.txt").exists());
    }

    #[test]
    fn exported_pack_contains_index_and_config_overrides() {
        let dir = tempfile::tempdir().unwrap();
        let game_dir = dir.path().join("game");
        std::fs::create_dir_all(game_dir.join("config").join("sub")).unwrap();
        std::fs::write(game_dir.join("config").join("sub").join("x.json"), "{}").unwrap();
        std::fs::create_dir_all(game_dir.join("scripts")).unwrap();
        std::fs::write(game_dir.join("scripts").join("recipes.zs"), "x").unwrap();
        std::fs::create_dir_all(game_dir.join("saves")).unwrap();
        std::fs::write(game_dir.join("saves").join("level.dat"), "w").unwrap();
        let pack = index(
            serde_json::json!([]),
            serde_json::json!({ "minecraft": "1.20.1" }),
        );
        let target = dir.path().join("out.mrpack");
        let json = serde_json::to_vec(&pack).unwrap();
        let count = write_pack(&target, INDEX_FILE, &json, &game_dir).unwrap();
        assert_eq!(count, 2);
        let read = read_index(&target).unwrap();
        assert_eq!(read.name, "Test Pack");
        let mut zip = zip::ZipArchive::new(File::open(&target).unwrap()).unwrap();
        assert!(zip.by_name("overrides/config/sub/x.json").is_ok());
        assert!(zip.by_name("overrides/scripts/recipes.zs").is_ok());
        assert!(zip.by_name("overrides/saves/level.dat").is_err());
        assert!(!dir.path().join("out.mrpack.part").exists());
    }
}

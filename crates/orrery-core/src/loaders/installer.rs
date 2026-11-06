use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use super::processors::{self, DataValue, InstallProfile, ProcessorRun};
use super::{ModLoader, forge, neoforge, profile_id, remote_sha1};
use crate::error::IoResultExt;
use crate::minecraft::libraries::join_url;
use crate::minecraft::manifest::{MetaStore, validate_version_id};
use crate::minecraft::maven::{MavenCoordinate, safe_join};
use crate::minecraft::version::{Library, VersionJson};
use crate::net::{Checksum, Download, Http, VerifyMode, download_all, sha1_hex};
use crate::paths::LauncherPaths;
use crate::platform::Platform;
use crate::progress::{Progress, Stage};
use crate::storage;
use crate::{CoreError, CoreResult};

const MAX_ENTRY_BYTES: u64 = 256 * 1024 * 1024;
const BUNDLED_PREFIX: &str = "maven/";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LoaderManifest {
    pub generated: Vec<String>,
}

fn install_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn allowed_hosts(loader: ModLoader) -> &'static [&'static str] {
    match loader {
        ModLoader::NeoForge => &["maven.neoforged.net", "libraries.minecraft.net"],
        ModLoader::Forge => &["maven.minecraftforge.net", "libraries.minecraft.net"],
        ModLoader::Fabric | ModLoader::LegacyFabric | ModLoader::Quilt => &[],
    }
}

const MINECRAFT_LIBRARIES: &str = "https://libraries.minecraft.net/";
const OLD_FORGE_MAVEN: &str = "http://files.minecraftforge.net/maven/";
const FORGE_MAVEN: &str = "https://maven.minecraftforge.net/";

fn check_pairing(loader: ModLoader, game_version: &str, loader_version: &str) -> CoreResult<()> {
    match loader {
        ModLoader::NeoForge
            if neoforge::minecraft_version(loader_version).as_deref() != Some(game_version) =>
        {
            Err(CoreError::InvalidInput(format!(
                "NeoForge {loader_version} is not made for Minecraft {game_version}"
            )))
        }
        ModLoader::Forge if !forge::supported(game_version) => Err(CoreError::Unsupported(
            format!("Forge for Minecraft {game_version}"),
        )),
        ModLoader::Fabric | ModLoader::LegacyFabric | ModLoader::Quilt => Err(
            CoreError::Unsupported(format!("{} has no installer", loader.display_name())),
        ),
        _ => Ok(()),
    }
}

async fn installer_url(
    http: &Http,
    loader: ModLoader,
    game_version: &str,
    loader_version: &str,
) -> CoreResult<String> {
    match loader {
        ModLoader::Forge => Ok(forge::installer_url(
            &forge::maven_version(http, game_version, loader_version).await?,
        )),
        _ => Ok(neoforge::installer_url(loader_version)),
    }
}

fn legacy_base(url: Option<&str>) -> String {
    let base = match url.map(str::trim).filter(|u| !u.is_empty()) {
        None => MINECRAFT_LIBRARIES,
        Some(OLD_FORGE_MAVEN) => FORGE_MAVEN,
        Some(base) => base,
    };
    if base.ends_with('/') {
        base.to_string()
    } else {
        format!("{base}/")
    }
}

fn legacy_library(name: &str, url: Option<&str>, sha1: &str) -> CoreResult<Library> {
    let mut value = serde_json::json!({ "name": name, "sha1": sha1 });
    if let Some(url) = url {
        value["url"] = serde_json::Value::String(url.to_string());
    }
    serde_json::from_value(value)
        .map_err(|e| CoreError::InvalidInput(format!("invalid library {name}: {e}")))
}

pub fn host_allowed(loader: ModLoader, url: &str) -> bool {
    reqwest::Url::parse(url).is_ok_and(|url| {
        url.scheme() == "https"
            && url
                .host_str()
                .is_some_and(|host| allowed_hosts(loader).contains(&host))
    })
}

pub fn check_hosts<'a>(
    loader: ModLoader,
    mut urls: impl Iterator<Item = &'a str>,
) -> CoreResult<()> {
    match urls.find(|url| !host_allowed(loader, url)) {
        Some(url) => Err(CoreError::InvalidInput(format!(
            "{} wants to download from an unexpected host: {url}",
            loader.display_name()
        ))),
        None => Ok(()),
    }
}

pub fn validate_profile(
    profile: &InstallProfile,
    version: &VersionJson,
    game_version: &str,
    id: &str,
) -> CoreResult<()> {
    if profile.minecraft != game_version || profile.version != id {
        return Err(CoreError::InvalidInput(format!(
            "the installer is for {} on Minecraft {}, expected {id} on {game_version}",
            profile.version, profile.minecraft
        )));
    }
    if version.id != id || version.inherits_from.as_deref() != Some(game_version) {
        return Err(CoreError::InvalidInput(format!(
            "the loader profile {} does not inherit from {game_version}",
            version.id
        )));
    }
    Ok(())
}

pub fn generated_artifacts(profile: &InstallProfile) -> Vec<String> {
    let referenced = processors::referenced_keys(&profile.processors);
    let mut generated: Vec<String> = profile
        .data
        .iter()
        .filter(|(key, _)| referenced.contains(key.as_str()))
        .filter_map(|(_, value)| match processors::classify(&value.client) {
            DataValue::Artifact(coordinate) => MavenCoordinate::parse(coordinate)
                .ok()
                .map(|c| c.relative_path()),
            _ => None,
        })
        .collect();
    generated.sort();
    generated.dedup();
    generated
}

pub fn generated_files(paths: &LauncherPaths, id: &str) -> Option<Vec<PathBuf>> {
    let text = std::fs::read_to_string(paths.loader_manifest_file(id)).ok()?;
    let manifest: LoaderManifest = serde_json::from_str(&text).ok()?;
    let libraries = paths.libraries_dir();
    manifest
        .generated
        .iter()
        .map(|relative| safe_join(&libraries, relative).ok())
        .collect()
}

fn library_relative_path(library: &Library) -> CoreResult<String> {
    match library
        .downloads
        .as_ref()
        .and_then(|d| d.artifact.as_ref())
        .and_then(|a| a.path.clone())
    {
        Some(path) => Ok(path),
        None => Ok(MavenCoordinate::parse(&library.name)?.relative_path()),
    }
}

fn library_sha1(library: &Library) -> Option<&str> {
    library
        .downloads
        .as_ref()
        .and_then(|d| d.artifact.as_ref())
        .map(|a| a.sha1.as_str())
        .filter(|sha1| !sha1.is_empty())
}

fn read_entry(
    zip: &mut zip::ZipArchive<std::fs::File>,
    name: &str,
    jar: &Path,
) -> CoreResult<Vec<u8>> {
    let entry = zip.by_name(name).map_err(|e| CoreError::Archive {
        path: jar.to_path_buf(),
        message: format!("{name}: {e}"),
    })?;
    if entry.size() > MAX_ENTRY_BYTES {
        return Err(CoreError::Archive {
            path: jar.to_path_buf(),
            message: format!("{name} is too large"),
        });
    }
    let mut bytes = Vec::with_capacity(entry.size() as usize);
    entry
        .take(MAX_ENTRY_BYTES)
        .read_to_end(&mut bytes)
        .at_path(jar)?;
    Ok(bytes)
}

fn open_installer(jar: &Path) -> CoreResult<zip::ZipArchive<std::fs::File>> {
    let file = std::fs::File::open(jar).at_path(jar)?;
    zip::ZipArchive::new(file).map_err(|e| CoreError::Archive {
        path: jar.to_path_buf(),
        message: e.to_string(),
    })
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyInstall {
    pub path: String,
    pub file_path: String,
    pub minecraft: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LegacyLibrary {
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub checksums: Vec<String>,
    #[serde(default)]
    pub clientreq: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyProfile {
    install: LegacyInstall,
    version_info: serde_json::Value,
}

pub enum InstallerContents {
    Modern(InstallProfile, VersionJson),
    Legacy(LegacyInstall, VersionJson, Vec<LegacyLibrary>),
}

pub fn parse_installer_profile(
    profile: &[u8],
    version: impl FnOnce() -> CoreResult<Vec<u8>>,
    jar: &Path,
) -> CoreResult<InstallerContents> {
    let parse_error = |name: &str, source| CoreError::Json {
        path: jar.join(name),
        source,
    };
    let raw: serde_json::Value =
        serde_json::from_slice(profile).map_err(|e| parse_error("install_profile.json", e))?;
    if raw.get("versionInfo").is_some() {
        let legacy: LegacyProfile =
            serde_json::from_value(raw).map_err(|e| parse_error("install_profile.json", e))?;
        let libraries: Vec<LegacyLibrary> = serde_json::from_value(
            legacy
                .version_info
                .get("libraries")
                .cloned()
                .unwrap_or_default(),
        )
        .map_err(|e| parse_error("install_profile.json", e))?;
        let version: VersionJson = serde_json::from_value(legacy.version_info)
            .map_err(|e| parse_error("install_profile.json", e))?;
        return Ok(InstallerContents::Legacy(
            legacy.install,
            version,
            libraries,
        ));
    }
    let profile: InstallProfile =
        serde_json::from_value(raw).map_err(|e| parse_error("install_profile.json", e))?;
    let version: VersionJson =
        serde_json::from_slice(&version()?).map_err(|e| parse_error("version.json", e))?;
    Ok(InstallerContents::Modern(profile, version))
}

fn read_installer(jar: &Path) -> CoreResult<InstallerContents> {
    let mut zip = open_installer(jar)?;
    let profile = read_entry(&mut zip, "install_profile.json", jar)?;
    parse_installer_profile(&profile, || read_entry(&mut zip, "version.json", jar), jar)
}

fn extract_entry(jar: &Path, entry: &str, target: &Path) -> CoreResult<String> {
    let mut zip = open_installer(jar)?;
    let bytes = read_entry(&mut zip, entry, jar)?;
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).at_path(parent)?;
    }
    std::fs::write(target, &bytes).at_path(target)?;
    Ok(sha1_hex(&bytes))
}

fn extract_installer_files(
    jar: &Path,
    profile: &InstallProfile,
    work_dir: &Path,
) -> CoreResult<HashMap<String, PathBuf>> {
    let mut zip = open_installer(jar)?;
    let mut extracted = HashMap::new();
    for value in profile.data.values() {
        let DataValue::InstallerFile(name) = processors::classify(&value.client) else {
            continue;
        };
        let entry = name.trim_start_matches('/');
        let target = safe_join(work_dir, entry)?;
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).at_path(parent)?;
        }
        let bytes = read_entry(&mut zip, entry, jar)?;
        std::fs::write(&target, bytes).at_path(&target)?;
        extracted.insert(name.to_string(), target);
    }
    Ok(extracted)
}

fn extract_bundled_libraries(
    jar: &Path,
    libraries: &[Library],
    libraries_dir: &Path,
) -> CoreResult<HashSet<PathBuf>> {
    let mut zip = open_installer(jar)?;
    let bundled: HashSet<String> = zip
        .file_names()
        .filter_map(|name| name.ok()?.strip_prefix(BUNDLED_PREFIX).map(str::to_string))
        .collect();
    let mut extracted = HashSet::new();
    for library in libraries {
        let relative = library_relative_path(library)?;
        if !bundled.contains(&relative) {
            continue;
        }
        let target = safe_join(libraries_dir, &relative)?;
        let bytes = read_entry(&mut zip, &format!("{BUNDLED_PREFIX}{relative}"), jar)?;
        if let Some(expected) = library_sha1(library) {
            let actual = sha1_hex(&bytes);
            if !actual.eq_ignore_ascii_case(expected) {
                return Err(CoreError::ChecksumMismatch {
                    url: format!("{}!{BUNDLED_PREFIX}{relative}", jar.display()),
                    expected: expected.to_string(),
                    actual,
                });
            }
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).at_path(parent)?;
        }
        std::fs::write(&target, bytes).at_path(&target)?;
        extracted.insert(target);
    }
    Ok(extracted)
}

async fn blocking<T: Send + 'static>(
    task: impl FnOnce() -> CoreResult<T> + Send + 'static,
) -> CoreResult<T> {
    tokio::task::spawn_blocking(task)
        .await
        .map_err(|e| CoreError::InvalidInput(format!("loader install task failed: {e}")))?
}

fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

async fn is_file(path: &Path) -> bool {
    tokio::fs::metadata(path).await.is_ok_and(|m| m.is_file())
}

pub struct LoaderInstall<'a> {
    pub http: &'a Http,
    pub paths: &'a LauncherPaths,
    pub meta: &'a MetaStore,
    pub java: &'a Path,
    pub platform: Platform,
    pub concurrency: usize,
    pub mode: VerifyMode,
    pub progress: &'a Progress,
}

impl LoaderInstall<'_> {
    pub async fn ensure(
        &self,
        loader: ModLoader,
        game_version: &str,
        loader_version: &str,
    ) -> CoreResult<String> {
        validate_version_id(game_version)?;
        validate_version_id(loader_version)?;
        check_pairing(loader, game_version, loader_version)?;
        let id = profile_id(loader, game_version, loader_version);
        let _guard = install_lock().lock().await;
        if self.mode == VerifyMode::Quick && self.is_installed(&id).await? {
            return Ok(id);
        }

        self.progress.set_stage(Stage::InstallingLoader);
        let url = installer_url(self.http, loader, game_version, loader_version).await?;
        let work_dir = self.paths.cache.join("loaders").join(&id);
        if tokio::fs::metadata(&work_dir).await.is_ok() {
            tokio::fs::remove_dir_all(&work_dir)
                .await
                .at_path(&work_dir)?;
        }
        tokio::fs::create_dir_all(&work_dir)
            .await
            .at_path(&work_dir)?;
        let result = self
            .install(loader, url, game_version, &id, &work_dir)
            .await;
        if let Err(err) = tokio::fs::remove_dir_all(&work_dir).await {
            tracing::warn!(error = %err, "could not remove the loader work folder");
        }
        result.map(|()| id)
    }

    async fn is_installed(&self, id: &str) -> CoreResult<bool> {
        if self.meta.stored_version(id).await?.is_none() {
            return Ok(false);
        }
        let Some(files) = generated_files(self.paths, id) else {
            return Ok(false);
        };
        for file in files {
            if !is_file(&file).await {
                return Ok(false);
            }
        }
        Ok(true)
    }

    async fn install(
        &self,
        loader: ModLoader,
        url: String,
        game_version: &str,
        id: &str,
        work_dir: &Path,
    ) -> CoreResult<()> {
        let sha1 = remote_sha1(self.http, &format!("{url}.sha1")).await?;
        let jar = work_dir.join("installer.jar");
        let installer = Download {
            url,
            path: jar.clone(),
            checksum: Some(Checksum::Sha1(sha1)),
            size: None,
        };
        download_all(
            self.http,
            vec![installer],
            1,
            VerifyMode::Full,
            self.progress,
        )
        .await?;

        let contents = {
            let jar = jar.clone();
            blocking(move || read_installer(&jar)).await?
        };
        match contents {
            InstallerContents::Modern(profile, version) => {
                self.install_modern(loader, &jar, profile, version, game_version, id, work_dir)
                    .await
            }
            InstallerContents::Legacy(install, version, libraries) => {
                self.install_legacy(loader, &jar, install, version, libraries, game_version, id)
                    .await
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn install_legacy(
        &self,
        loader: ModLoader,
        jar: &Path,
        install: LegacyInstall,
        mut version: VersionJson,
        libraries: Vec<LegacyLibrary>,
        game_version: &str,
        id: &str,
    ) -> CoreResult<()> {
        if install.minecraft != game_version
            || version.inherits_from.as_deref() != Some(game_version)
        {
            return Err(CoreError::InvalidInput(format!(
                "the installer is for Minecraft {}, expected {game_version}",
                install.minecraft
            )));
        }
        let libraries_dir = self.paths.libraries_dir();
        let relative = MavenCoordinate::parse(&install.path)?.relative_path();
        let target = safe_join(&libraries_dir, &relative)?;
        let forge_sha1 = {
            let jar = jar.to_path_buf();
            let entry = install.file_path.clone();
            let target = target.clone();
            blocking(move || extract_entry(&jar, &entry, &target)).await?
        };

        let mut resolved = Vec::new();
        for library in libraries.iter().filter(|l| l.clientreq != Some(false)) {
            self.progress.check_cancelled()?;
            if library.name == install.path {
                resolved.push(legacy_library(&library.name, None, &forge_sha1)?);
                continue;
            }
            let base = legacy_base(library.url.as_deref());
            let relative = MavenCoordinate::parse(&library.name)?.relative_path();
            let url = join_url(&base, &relative);
            check_hosts(loader, std::iter::once(url.as_str()))?;
            let sha1 = if library.checksums.is_empty() {
                remote_sha1(self.http, &format!("{url}.sha1")).await?
            } else {
                let path = safe_join(&libraries_dir, &relative)?;
                self.fetch_listed(&url, &path, &library.checksums).await?
            };
            resolved.push(legacy_library(&library.name, Some(&base), &sha1)?);
        }
        version.libraries = resolved;
        version.id = id.to_string();
        self.meta.store_version(&version).await?;
        storage::write_json_atomic(
            &self.paths.loader_manifest_file(id),
            &LoaderManifest {
                generated: vec![relative],
            },
        )
        .await
    }

    async fn fetch_listed(
        &self,
        url: &str,
        path: &Path,
        checksums: &[String],
    ) -> CoreResult<String> {
        let listed = |sha1: &str| {
            checksums
                .iter()
                .any(|known| known.eq_ignore_ascii_case(sha1))
        };
        if let Ok(bytes) = tokio::fs::read(path).await {
            let sha1 = sha1_hex(&bytes);
            if listed(&sha1) {
                return Ok(sha1);
            }
        }
        let bytes = self.http.get_bytes(url).await?;
        let sha1 = sha1_hex(&bytes);
        if !listed(&sha1) {
            return Err(CoreError::ChecksumMismatch {
                url: url.to_string(),
                expected: checksums.join(" or "),
                actual: sha1,
            });
        }
        storage::write_bytes_atomic(path, &bytes).await?;
        Ok(sha1)
    }

    #[allow(clippy::too_many_arguments)]
    async fn install_modern(
        &self,
        loader: ModLoader,
        jar: &Path,
        profile: InstallProfile,
        version: VersionJson,
        game_version: &str,
        id: &str,
        work_dir: &Path,
    ) -> CoreResult<()> {
        let jar = jar.to_path_buf();
        validate_profile(&profile, &version, game_version, id)?;

        let libraries_dir = self.paths.libraries_dir();
        let bundled = {
            let jar = jar.clone();
            let libraries: Vec<Library> = profile
                .libraries
                .iter()
                .chain(&version.libraries)
                .cloned()
                .collect();
            let libraries_dir = libraries_dir.clone();
            blocking(move || extract_bundled_libraries(&jar, &libraries, &libraries_dir)).await?
        };
        let pending: Vec<Library> = profile
            .libraries
            .iter()
            .filter(|library| {
                library_relative_path(library)
                    .and_then(|relative| safe_join(&libraries_dir, &relative))
                    .map_or(true, |path| !bundled.contains(&path))
            })
            .cloned()
            .collect();
        let downloads = processors::library_downloads(&pending, &libraries_dir)?;
        check_hosts(loader, downloads.iter().map(|d| d.url.as_str()))?;
        check_hosts(
            loader,
            version
                .libraries
                .iter()
                .filter_map(|library| library.downloads.as_ref()?.artifact.as_ref())
                .map(|artifact| artifact.url.as_str())
                .filter(|url| !url.is_empty()),
        )?;
        self.progress.set_stage(Stage::DownloadingLibraries);
        download_all(
            self.http,
            downloads,
            self.concurrency,
            self.mode,
            self.progress,
        )
        .await?;

        self.progress.set_stage(Stage::InstallingLoader);
        let extracted = {
            let jar = jar.clone();
            let profile = profile.clone();
            let work_dir = work_dir.to_path_buf();
            blocking(move || extract_installer_files(&jar, &profile, &work_dir)).await?
        };
        let mut values = HashMap::from([
            ("SIDE".to_string(), "client".to_string()),
            (
                "MINECRAFT_JAR".to_string(),
                path_text(&self.paths.client_jar(game_version)),
            ),
            ("MINECRAFT_VERSION".to_string(), game_version.to_string()),
            ("ROOT".to_string(), path_text(self.paths.root())),
            ("INSTALLER".to_string(), path_text(&jar)),
            ("LIBRARY_DIR".to_string(), path_text(&libraries_dir)),
        ]);
        for (key, value) in &profile.data {
            let resolved = match processors::classify(&value.client) {
                DataValue::Artifact(coordinate) => {
                    path_text(&processors::artifact_path(&libraries_dir, coordinate)?)
                }
                DataValue::Literal(literal) => literal.to_string(),
                DataValue::InstallerFile(name) => path_text(&extracted[name]),
            };
            values.insert(key.clone(), resolved);
        }

        let vanilla = self.meta.vanilla_version(game_version).await?;
        let mappings = vanilla
            .downloads
            .as_ref()
            .and_then(|d| d.client_mappings.as_ref());
        let log = self.paths.logs.join("loaders").join(format!("{id}.log"));
        let run = ProcessorRun {
            http: self.http,
            java: self.java,
            libraries_dir: &libraries_dir,
            work_dir,
            log: &log,
            platform: self.platform,
            mappings,
            progress: self.progress,
        };
        run.run_all(&profile.processors, &values).await?;

        let generated = generated_artifacts(&profile);
        let mut required: Vec<PathBuf> = generated
            .iter()
            .map(|relative| safe_join(&libraries_dir, relative))
            .collect::<CoreResult<_>>()?;
        for library in &version.libraries {
            let without_url = library
                .downloads
                .as_ref()
                .and_then(|d| d.artifact.as_ref())
                .is_some_and(|a| a.url.is_empty());
            if without_url {
                required.push(safe_join(&libraries_dir, &library_relative_path(library)?)?);
            }
        }
        for path in &required {
            if !is_file(path).await {
                return Err(CoreError::NotFound(format!(
                    "{} after the {} install, see {}",
                    path.display(),
                    loader.display_name(),
                    log.display()
                )));
            }
        }
        self.meta.store_version(&version).await?;
        storage::write_json_atomic(
            &self.paths.loader_manifest_file(id),
            &LoaderManifest { generated },
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn only_official_hosts_are_allowed() {
        let neo = ModLoader::NeoForge;
        assert!(host_allowed(
            neo,
            "https://maven.neoforged.net/releases/a.jar"
        ));
        assert!(host_allowed(neo, "https://libraries.minecraft.net/a.jar"));
        assert!(!host_allowed(
            neo,
            "http://maven.neoforged.net/releases/a.jar"
        ));
        assert!(!host_allowed(
            neo,
            "https://maven.neoforged.net.evil.example/a.jar"
        ));
        assert!(!host_allowed(neo, "https://maven.minecraftforge.net/a.jar"));
        assert!(host_allowed(
            ModLoader::Forge,
            "https://maven.minecraftforge.net/a.jar"
        ));
        assert!(!host_allowed(
            ModLoader::Forge,
            "https://maven.neoforged.net/a.jar"
        ));
        assert!(!host_allowed(
            ModLoader::Fabric,
            "https://libraries.minecraft.net/a.jar"
        ));
    }

    #[test]
    fn loader_and_game_versions_must_fit() {
        assert!(check_pairing(ModLoader::NeoForge, "1.21.1", "21.1.265").is_ok());
        assert!(check_pairing(ModLoader::NeoForge, "1.20.1", "21.1.265").is_err());
        assert!(check_pairing(ModLoader::Forge, "1.20.1", "47.4.28").is_ok());
        assert!(check_pairing(ModLoader::Forge, "1.8.9", "11.15.1.2318").is_ok());
        assert!(check_pairing(ModLoader::Forge, "1.16.5", "36.2.39").is_err());
        assert!(check_pairing(ModLoader::Fabric, "1.20.1", "0.16.0").is_err());
    }

    #[test]
    fn legacy_libraries_come_from_https_mirrors() {
        assert_eq!(legacy_base(None), "https://libraries.minecraft.net/");
        assert_eq!(
            legacy_base(Some("http://files.minecraftforge.net/maven/")),
            "https://maven.minecraftforge.net/"
        );
        assert_eq!(
            legacy_base(Some("https://maven.minecraftforge.net")),
            "https://maven.minecraftforge.net/"
        );
    }

    #[test]
    fn reads_the_legacy_installer_format() {
        let profile = br#"{
            "install": {
                "path": "net.minecraftforge:forge:1.8.9-11.15.1.2318-1.8.9",
                "filePath": "forge-1.8.9-11.15.1.2318-1.8.9-universal.jar",
                "minecraft": "1.8.9"
            },
            "versionInfo": {
                "id": "1.8.9-forge1.8.9-11.15.1.2318-1.8.9",
                "inheritsFrom": "1.8.9",
                "mainClass": "net.minecraft.launchwrapper.Launch",
                "libraries": [
                    { "name": "net.minecraftforge:forge:1.8.9-11.15.1.2318-1.8.9", "url": "https://maven.minecraftforge.net/" },
                    { "name": "jline:jline:2.13", "checksums": ["aa"], "serverreq": true, "clientreq": false },
                    { "name": "net.minecraft:launchwrapper:1.12", "serverreq": true }
                ]
            }
        }"#;
        let contents = parse_installer_profile(
            profile,
            || panic!("legacy installers have no version.json"),
            Path::new("installer.jar"),
        )
        .unwrap();
        let InstallerContents::Legacy(install, version, libraries) = contents else {
            panic!("expected the legacy format");
        };
        assert_eq!(install.minecraft, "1.8.9");
        assert_eq!(
            install.file_path,
            "forge-1.8.9-11.15.1.2318-1.8.9-universal.jar"
        );
        assert_eq!(version.inherits_from.as_deref(), Some("1.8.9"));
        assert_eq!(libraries.len(), 3);
        assert_eq!(libraries[1].clientreq, Some(false));
        assert_eq!(libraries[1].checksums, ["aa"]);
        assert_eq!(libraries[2].clientreq, None);
    }

    #[test]
    fn modern_installers_read_their_version_file() {
        let profile = br#"{"version":"1.12.2-forge-14.23.5.2864","minecraft":"1.12.2","data":{},"processors":[],"libraries":[]}"#;
        let contents = parse_installer_profile(
            profile,
            || {
                Ok(
                    br#"{"id":"1.12.2-forge-14.23.5.2864","inheritsFrom":"1.12.2","libraries":[]}"#
                        .to_vec(),
                )
            },
            Path::new("installer.jar"),
        )
        .unwrap();
        assert!(matches!(contents, InstallerContents::Modern(..)));
    }

    #[test]
    fn validates_the_installer_profile() {
        let profile: InstallProfile = serde_json::from_str(
            r#"{"version":"neoforge-21.1.265","minecraft":"1.21.1","data":{
                "PATCHED":{"client":"[net.neoforged:neoforge:21.1.265:client]","server":"x"},
                "MCP_VERSION":{"client":"'1.21.1-20240808.144430'","server":"x"},
                "MC_UNPACKED":{"client":"[net.minecraft:client:1.21.1:unpacked]","server":"x"},
                "BINPATCH":{"client":"/data/client.lzma","server":"x"}},
                "processors":[
                    {"sides":["server"],"jar":"a:b:1","args":["--output","{MC_UNPACKED}"]},
                    {"jar":"a:b:1","args":["--apply","{BINPATCH}","--output","{PATCHED}"]}
                ],"libraries":[]}"#,
        )
        .unwrap();
        let version: VersionJson = serde_json::from_str(
            r#"{"id":"neoforge-21.1.265","inheritsFrom":"1.21.1","libraries":[]}"#,
        )
        .unwrap();
        assert!(validate_profile(&profile, &version, "1.21.1", "neoforge-21.1.265").is_ok());
        assert!(validate_profile(&profile, &version, "1.21.4", "neoforge-21.1.265").is_err());
        assert_eq!(
            generated_artifacts(&profile),
            ["net/neoforged/neoforge/21.1.265/neoforge-21.1.265-client.jar"]
        );
    }

    #[test]
    fn extracts_bundled_libraries_and_checks_their_hash() {
        let dir = tempfile::tempdir().unwrap();
        let jar = dir.path().join("installer.jar");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&jar).unwrap());
        let path = "net/minecraftforge/forge/1.20.1-47.4.28/forge-1.20.1-47.4.28-universal.jar";
        zip.start_file(
            format!("maven/{path}"),
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zip.write_all(b"universal").unwrap();
        zip.finish().unwrap();
        let library = |sha1: &str| -> Library {
            serde_json::from_value(serde_json::json!({
                "name": "net.minecraftforge:forge:1.20.1-47.4.28:universal",
                "downloads": { "artifact": { "path": path, "sha1": sha1, "size": 9, "url": "" } }
            }))
            .unwrap()
        };
        let libraries_dir = dir.path().join("libraries");
        let good = library(&sha1_hex(b"universal"));
        let extracted =
            extract_bundled_libraries(&jar, std::slice::from_ref(&good), &libraries_dir).unwrap();
        assert!(extracted.contains(&libraries_dir.join(path)));
        assert_eq!(
            std::fs::read(libraries_dir.join(path)).unwrap(),
            b"universal"
        );
        let bad = library(&sha1_hex(b"other"));
        assert!(extract_bundled_libraries(&jar, &[bad], &libraries_dir).is_err());
    }
}

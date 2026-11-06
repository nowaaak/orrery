use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{JavaInstall, JavaRequirement, java_binary, make_executable};
use crate::error::IoResultExt;
use crate::minecraft::maven::safe_join;
use crate::net::{Checksum, Download, VerifyMode, download_all};
use crate::platform::{Arch, Os, Platform};
use crate::{CoreError, CoreResult, storage};

const RUNTIME_INDEX_URL: &str = "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";
const MARKER_FILE: &str = ".installed";

type RuntimeIndex = HashMap<String, HashMap<String, Vec<RuntimeEntry>>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RuntimeEntry {
    manifest: ManifestRef,
    version: RuntimeVersion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManifestRef {
    sha1: String,
    size: u64,
    url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RuntimeVersion {
    name: String,
}

#[derive(Debug, Deserialize)]
struct RuntimeManifest {
    files: HashMap<String, RuntimeFile>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum RuntimeFile {
    Directory,
    File {
        #[serde(default)]
        executable: bool,
        downloads: FileDownloads,
    },
    Link {
        target: String,
    },
}

#[derive(Debug, Deserialize)]
struct FileDownloads {
    raw: RawDownload,
}

#[derive(Debug, Deserialize)]
struct RawDownload {
    sha1: String,
    size: u64,
    url: String,
}

pub(super) fn platform_key(platform: Platform) -> Option<&'static str> {
    match (platform.os, platform.arch) {
        (Os::Windows, Arch::X86_64) => Some("windows-x64"),
        (Os::Windows, Arch::X86) => Some("windows-x86"),
        (Os::Windows, Arch::Aarch64) => Some("windows-arm64"),
        (Os::MacOs, Arch::X86_64) => Some("mac-os"),
        (Os::MacOs, Arch::Aarch64) => Some("mac-os-arm64"),
        (Os::Linux, Arch::X86_64) => Some("linux"),
        (Os::Linux, Arch::X86) => Some("linux-i386"),
        _ => None,
    }
}

pub(super) async fn install(
    install: &JavaInstall<'_>,
    requirement: &JavaRequirement,
    platform_key: &str,
) -> CoreResult<Option<PathBuf>> {
    let index_path = install.paths.java_runtime_index_file();
    let prefer_cache = install.mode == VerifyMode::Quick;
    let mut index = runtime_index(install, &index_path, prefer_cache).await?;
    if prefer_cache && find_entry(&index, platform_key, &requirement.component).is_none() {
        index = runtime_index(install, &index_path, false).await?;
    }
    let Some(entry) = find_entry(&index, platform_key, &requirement.component) else {
        return Ok(None);
    };

    let home = install
        .paths
        .runtimes_dir()
        .join(&requirement.component)
        .join(platform_key);
    let java = runtime_java(&home, install.platform.os);
    let marker = home.join(MARKER_FILE);
    let installed = tokio::fs::read_to_string(&marker).await.ok();
    if install.mode == VerifyMode::Quick
        && installed.as_deref() == Some(entry.version.name.as_str())
        && tokio::fs::metadata(&java).await.is_ok()
    {
        return Ok(Some(java));
    }

    let manifest_bytes = install.http.get_bytes(&entry.manifest.url).await?;
    let actual = crate::net::sha1_hex(&manifest_bytes);
    if actual != entry.manifest.sha1 {
        return Err(CoreError::ChecksumMismatch {
            url: entry.manifest.url.clone(),
            expected: entry.manifest.sha1.clone(),
            actual,
        });
    }
    let manifest: RuntimeManifest =
        serde_json::from_slice(&manifest_bytes).map_err(|source| CoreError::Json {
            path: home.clone(),
            source,
        })?;

    let mut downloads = Vec::new();
    let mut executables = Vec::new();
    let mut links = Vec::new();
    for (relative, file) in &manifest.files {
        let path = safe_join(&home, relative)?;
        match file {
            RuntimeFile::Directory => {
                tokio::fs::create_dir_all(&path).await.at_path(&path)?;
            }
            RuntimeFile::File {
                executable,
                downloads: file_downloads,
            } => {
                if *executable {
                    executables.push(path.clone());
                }
                downloads.push(Download {
                    url: file_downloads.raw.url.clone(),
                    path,
                    checksum: Some(Checksum::Sha1(file_downloads.raw.sha1.clone())),
                    size: Some(file_downloads.raw.size),
                });
            }
            RuntimeFile::Link { target } => links.push((path, target.clone())),
        }
    }

    if let Err(err) = tokio::fs::remove_file(&marker).await
        && err.kind() != std::io::ErrorKind::NotFound
    {
        return Err(err).at_path(&marker);
    }
    download_all(
        install.http,
        downloads,
        install.concurrency,
        install.mode,
        install.progress,
    )
    .await?;
    for path in &executables {
        make_executable(path)?;
    }
    create_links(&links)?;
    tokio::fs::write(&marker, &entry.version.name)
        .await
        .at_path(&marker)?;
    Ok(Some(java))
}

fn find_entry<'a>(
    index: &'a RuntimeIndex,
    platform_key: &str,
    component: &str,
) -> Option<&'a RuntimeEntry> {
    index
        .get(platform_key)
        .and_then(|components| components.get(component))
        .and_then(|entries| entries.first())
}

async fn runtime_index(
    install: &JavaInstall<'_>,
    path: &Path,
    prefer_cache: bool,
) -> CoreResult<RuntimeIndex> {
    if prefer_cache && let Ok(Some(cached)) = storage::read_json::<RuntimeIndex>(path).await {
        return Ok(cached);
    }
    match install
        .http
        .get_json::<RuntimeIndex>(RUNTIME_INDEX_URL)
        .await
    {
        Ok(index) => {
            storage::write_json_atomic(path, &index).await?;
            Ok(index)
        }
        Err(err) => match storage::read_json::<RuntimeIndex>(path).await? {
            Some(cached) => {
                tracing::warn!(error = %err, "using cached java runtime index");
                Ok(cached)
            }
            None => Err(err),
        },
    }
}

fn runtime_java(home: &Path, os: Os) -> PathBuf {
    match os {
        Os::MacOs => java_binary(&home.join("jre.bundle").join("Contents").join("Home"), os),
        Os::Windows | Os::Linux => java_binary(home, os),
    }
}

#[cfg(unix)]
fn create_links(links: &[(PathBuf, String)]) -> CoreResult<()> {
    for (path, target) in links {
        if std::fs::symlink_metadata(path).is_ok() {
            std::fs::remove_file(path).at_path(path)?;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).at_path(parent)?;
        }
        std::os::unix::fs::symlink(target, path).at_path(path)?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn create_links(links: &[(PathBuf, String)]) -> CoreResult<()> {
    if !links.is_empty() {
        tracing::debug!(
            count = links.len(),
            "skipping symlinks in java runtime on this platform"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_runtime_manifest_entries() {
        let manifest: RuntimeManifest = serde_json::from_str(
            r#"{"files":{
                "bin":{"type":"directory"},
                "bin/java":{"type":"file","executable":true,"downloads":{"raw":{"sha1":"a","size":1,"url":"u"}}},
                "lib/link":{"type":"link","target":"../bin/java"}
            }}"#,
        )
        .unwrap();
        assert_eq!(manifest.files.len(), 3);
        assert!(matches!(
            manifest.files.get("bin/java"),
            Some(RuntimeFile::File {
                executable: true,
                ..
            })
        ));
    }

    #[test]
    fn platform_keys_match_mojang_naming() {
        let key = |os, arch| platform_key(Platform { os, arch });
        assert_eq!(key(Os::Windows, Arch::X86_64), Some("windows-x64"));
        assert_eq!(key(Os::MacOs, Arch::Aarch64), Some("mac-os-arm64"));
        assert_eq!(key(Os::Linux, Arch::Aarch64), None);
    }
}

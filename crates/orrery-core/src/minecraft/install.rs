use std::path::{Path, PathBuf};

use super::assets;
use super::libraries::resolve_libraries;
use super::manifest::MetaStore;
use super::natives::extract_natives;
use super::rules::RuleContext;
use super::version::VersionJson;
use crate::error::IoResultExt;
use crate::java::{JavaInstall, JavaRequirement, ensure_java, x64_on_apple_silicon};
use crate::net::{Checksum, Download, Http, VerifyMode, download_all};
use crate::paths::LauncherPaths;
use crate::platform::Platform;
use crate::progress::{Progress, Stage};
use crate::storage::temp_sibling;
use crate::{CoreError, CoreResult};

#[derive(Debug, Clone)]
pub struct InstalledVersion {
    pub version: VersionJson,
    pub classpath: Vec<PathBuf>,
    pub natives_dir: PathBuf,
    pub java: PathBuf,
    pub logging_argument: Option<String>,
}

pub struct Installer<'a> {
    pub http: &'a Http,
    pub paths: &'a LauncherPaths,
    pub meta: &'a MetaStore,
    pub concurrency: usize,
    pub platform: Platform,
    pub profile_client_jar: bool,
}

async fn copy_client_jar(source: &Path, target: PathBuf, mode: VerifyMode) -> CoreResult<PathBuf> {
    let expected = tokio::fs::metadata(source).await.at_path(source)?.len();
    let current = tokio::fs::metadata(&target).await.ok().map(|m| m.len());
    if mode == VerifyMode::Quick && current == Some(expected) {
        return Ok(target);
    }
    if let Some(parent) = target.parent() {
        tokio::fs::create_dir_all(parent).await.at_path(parent)?;
    }
    let temp = temp_sibling(&target);
    tokio::fs::copy(source, &temp).await.at_path(&temp)?;
    tokio::fs::rename(&temp, &target).await.at_path(&target)?;
    Ok(target)
}

impl Installer<'_> {
    pub async fn install(
        &self,
        version_id: &str,
        java_override: Option<PathBuf>,
        mode: VerifyMode,
        progress: &Progress,
    ) -> CoreResult<InstalledVersion> {
        progress.set_stage(Stage::FetchingMetadata);
        let resolved = self.meta.resolve(version_id).await?;
        let version = resolved.json;
        progress.check_cancelled()?;

        let ctx = RuleContext::new(self.platform);
        let libraries = resolve_libraries(&version, &ctx, &self.paths.libraries_dir())?;

        progress.set_stage(Stage::DownloadingLibraries);
        let client = version
            .downloads
            .as_ref()
            .and_then(|d| d.client.as_ref())
            .ok_or_else(|| {
                CoreError::NotFound(format!("client download for version {}", resolved.base_id))
            })?;
        let client_jar = self.paths.client_jar(&resolved.base_id);
        let mut downloads = libraries.downloads.clone();
        downloads.push(Download {
            url: client.url.clone(),
            path: client_jar.clone(),
            checksum: Some(Checksum::Sha1(client.sha1.clone())),
            size: Some(client.size),
        });
        download_all(self.http, downloads, self.concurrency, mode, progress).await?;

        progress.set_stage(Stage::DownloadingAssets);
        let assets_dir = self.paths.assets_dir();
        let index_ref = version.asset_index.as_ref().ok_or_else(|| {
            CoreError::NotFound(format!("asset index for version {}", version.id))
        })?;
        let index_download = assets::index_download(&assets_dir, index_ref)?;
        let index_path = index_download.path.clone();
        let mut asset_downloads = Vec::new();
        let logging_argument = match version.logging.as_ref().and_then(|l| l.client.as_ref()) {
            Some(logging) => {
                let file = &logging.file;
                let path = super::maven::safe_join(&assets_dir.join("log_configs"), &file.id)?;
                asset_downloads.push(Download {
                    url: file.url.clone(),
                    path: path.clone(),
                    checksum: Some(Checksum::Sha1(file.sha1.clone())),
                    size: Some(file.size),
                });
                Some(logging.argument.replace("${path}", &path.to_string_lossy()))
            }
            None => None,
        };
        download_all(self.http, vec![index_download], 1, mode, progress).await?;
        let index = assets::read_index(&index_path).await?;
        asset_downloads.extend(assets::object_downloads(&assets_dir, &index)?);
        download_all(self.http, asset_downloads, self.concurrency, mode, progress).await?;

        progress.set_stage(Stage::InstallingJava);
        let java = match java_override {
            Some(path) => {
                if !tokio::fs::metadata(&path).await.is_ok_and(|m| m.is_file()) {
                    return Err(CoreError::NotFound(format!(
                        "Java executable {}",
                        path.display()
                    )));
                }
                path
            }
            None => {
                let requirement = version
                    .java_version
                    .as_ref()
                    .map(|java| JavaRequirement {
                        component: java.component.clone(),
                        major_version: java.major_version,
                    })
                    .unwrap_or_else(JavaRequirement::legacy);
                let has_arm_natives = libraries
                    .natives
                    .iter()
                    .any(|native| native.path.to_string_lossy().contains("arm64"));
                let java_platform = if has_arm_natives {
                    self.platform
                } else {
                    x64_on_apple_silicon(self.platform)
                };
                if java_platform != self.platform {
                    tracing::info!(version = %version.id, "using x86_64 Java via Rosetta for a version without arm64 natives");
                }
                let install = JavaInstall {
                    http: self.http,
                    paths: self.paths,
                    platform: java_platform,
                    mode,
                    concurrency: self.concurrency,
                    progress,
                };
                ensure_java(&install, &requirement).await?
            }
        };

        progress.set_stage(Stage::ExtractingNatives);
        let natives_dir = self.paths.natives_dir(&version.id);
        extract_natives(
            libraries.natives.clone(),
            natives_dir.clone(),
            mode == VerifyMode::Full,
        )
        .await?;

        let client_jar = if self.profile_client_jar && version.id != resolved.base_id {
            copy_client_jar(&client_jar, self.paths.client_jar(&version.id), mode).await?
        } else {
            client_jar
        };
        let mut classpath = libraries.classpath;
        classpath.push(client_jar);
        Ok(InstalledVersion {
            version,
            classpath,
            natives_dir,
            java,
            logging_argument,
        })
    }
}

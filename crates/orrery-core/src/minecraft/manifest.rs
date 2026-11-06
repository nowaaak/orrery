use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::version::VersionJson;
use crate::net::{Http, sha1_hex};
use crate::paths::LauncherPaths;
use crate::{CoreError, CoreResult, storage};

pub const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const FIRST_SUPPORTED_RELEASE_TIME: &str = "2014-09-02T08:24:35+00:00";
const MANIFEST_MAX_AGE: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionManifest {
    pub latest: LatestVersions,
    pub versions: Vec<ManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestVersions {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
    pub release_time: String,
    pub sha1: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum VersionKind {
    Release,
    Snapshot,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct VersionSummary {
    pub id: String,
    pub kind: VersionKind,
    pub release_time: String,
}

impl VersionManifest {
    pub fn supported_versions(&self, include_snapshots: bool) -> Vec<VersionSummary> {
        self.versions
            .iter()
            .filter(|entry| entry.release_time.as_str() >= FIRST_SUPPORTED_RELEASE_TIME)
            .filter_map(|entry| {
                let kind = match entry.kind.as_str() {
                    "release" => VersionKind::Release,
                    "snapshot" if include_snapshots => VersionKind::Snapshot,
                    _ => return None,
                };
                Some(VersionSummary {
                    id: entry.id.clone(),
                    kind,
                    release_time: entry.release_time.clone(),
                })
            })
            .collect()
    }

    pub fn find(&self, id: &str) -> Option<&ManifestEntry> {
        self.versions.iter().find(|entry| entry.id == id)
    }

    pub fn is_supported(&self, id: &str) -> bool {
        self.find(id).is_some_and(|entry| {
            matches!(entry.kind.as_str(), "release" | "snapshot")
                && entry.release_time.as_str() >= FIRST_SUPPORTED_RELEASE_TIME
        })
    }
}

pub fn validate_version_id(id: &str) -> CoreResult<()> {
    let valid = !id.is_empty()
        && id.len() <= 128
        && !id.contains("..")
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+' | ' '));
    if valid {
        Ok(())
    } else {
        Err(CoreError::InvalidInput(format!(
            "invalid version id {id:?}"
        )))
    }
}

pub struct MetaStore {
    http: Http,
    paths: LauncherPaths,
}

impl MetaStore {
    pub fn new(http: Http, paths: LauncherPaths) -> Self {
        Self { http, paths }
    }

    pub async fn manifest(&self, refresh: bool) -> CoreResult<VersionManifest> {
        let path = self.paths.version_manifest_file();
        let cached = storage::read_json::<VersionManifest>(&path)
            .await
            .ok()
            .flatten();
        let fresh = !refresh && cached.is_some() && is_younger_than(&path, MANIFEST_MAX_AGE).await;
        if fresh && let Some(cached) = cached {
            return Ok(cached);
        }
        match self.http.get_json::<VersionManifest>(MANIFEST_URL).await {
            Ok(manifest) => {
                storage::write_json_atomic(&path, &manifest).await?;
                Ok(manifest)
            }
            Err(err) => match cached {
                Some(cached) => {
                    tracing::warn!(error = %err, "using cached version manifest");
                    Ok(cached)
                }
                None => Err(err),
            },
        }
    }

    pub async fn vanilla_version(&self, id: &str) -> CoreResult<VersionJson> {
        validate_version_id(id)?;
        let path = self.paths.version_json_file(id);
        if let Ok(bytes) = tokio::fs::read(&path).await {
            if let Ok(version) = serde_json::from_slice::<VersionJson>(&bytes) {
                return Ok(version);
            }
            tracing::warn!(version = id, "cached version json is invalid, refetching");
        }

        let manifest = self.manifest(false).await?;
        let entry = match manifest.find(id) {
            Some(entry) => entry.clone(),
            None => self
                .manifest(true)
                .await?
                .find(id)
                .cloned()
                .ok_or_else(|| CoreError::NotFound(format!("Minecraft version {id}")))?,
        };
        let bytes = self.http.get_bytes(&entry.url).await?;
        let actual = sha1_hex(&bytes);
        if actual != entry.sha1 {
            return Err(CoreError::ChecksumMismatch {
                url: entry.url,
                expected: entry.sha1,
                actual,
            });
        }
        let version =
            serde_json::from_slice::<VersionJson>(&bytes).map_err(|source| CoreError::Json {
                path: path.clone(),
                source,
            })?;
        storage::write_bytes_atomic(&path, &bytes).await?;
        Ok(version)
    }

    pub async fn stored_version(&self, id: &str) -> CoreResult<Option<VersionJson>> {
        validate_version_id(id)?;
        storage::read_json(&self.paths.version_json_file(id)).await
    }

    pub async fn store_version(&self, version: &VersionJson) -> CoreResult<()> {
        validate_version_id(&version.id)?;
        storage::write_json_atomic(&self.paths.version_json_file(&version.id), version).await
    }

    pub async fn resolve(&self, id: &str) -> CoreResult<ResolvedVersion> {
        let mut version = match self.stored_version(id).await? {
            Some(version) if version.inherits_from.is_some() => version,
            _ => self.vanilla_version(id).await?,
        };
        let mut base_id = version.id.clone();
        let mut depth = 0;
        while let Some(parent_id) = version.inherits_from.clone() {
            depth += 1;
            if depth > 4 {
                return Err(CoreError::InvalidInput(format!(
                    "version {id} has too many inheritance levels"
                )));
            }
            let parent = self.vanilla_version(&parent_id).await?;
            base_id = parent.id.clone();
            version = version.merge_onto(parent);
        }
        Ok(ResolvedVersion {
            json: version,
            base_id,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedVersion {
    pub json: VersionJson,
    pub base_id: String,
}

async fn is_younger_than(path: &std::path::Path, max_age: Duration) -> bool {
    tokio::fs::metadata(path)
        .await
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|modified| SystemTime::now().duration_since(modified).ok())
        .is_some_and(|age| age < max_age)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, kind: &str, time: &str) -> ManifestEntry {
        ManifestEntry {
            id: id.into(),
            kind: kind.into(),
            url: String::new(),
            release_time: time.into(),
            sha1: String::new(),
        }
    }

    #[test]
    fn supported_versions_start_at_1_8() {
        let manifest = VersionManifest {
            latest: LatestVersions {
                release: "1.21".into(),
                snapshot: "24w01a".into(),
            },
            versions: vec![
                entry("24w01a", "snapshot", "2024-01-03T12:00:00+00:00"),
                entry("1.21", "release", "2024-06-13T08:24:03+00:00"),
                entry("1.8", "release", "2014-09-02T08:24:35+00:00"),
                entry("1.7.10", "release", "2014-05-14T17:29:23+00:00"),
                entry("b1.7.3", "old_beta", "2011-07-08T00:00:00+00:00"),
            ],
        };

        let releases: Vec<_> = manifest
            .supported_versions(false)
            .into_iter()
            .map(|v| v.id)
            .collect();
        assert_eq!(releases, ["1.21", "1.8"]);
        assert_eq!(manifest.supported_versions(true).len(), 3);
        assert!(manifest.is_supported("1.8"));
        assert!(!manifest.is_supported("1.7.10"));
    }

    #[test]
    fn version_ids_are_validated() {
        assert!(validate_version_id("1.20.1").is_ok());
        assert!(validate_version_id("1.14 Pre-Release 1").is_ok());
        assert!(validate_version_id("fabric-loader-0.16.0-1.20.1").is_ok());
        assert!(validate_version_id("../evil").is_err());
        assert!(validate_version_id("a/b").is_err());
        assert!(validate_version_id("").is_err());
    }
}

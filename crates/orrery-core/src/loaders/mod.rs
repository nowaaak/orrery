pub mod forge;
pub mod installer;
pub mod neoforge;
pub mod processors;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::minecraft::libraries::join_url;
use crate::minecraft::manifest::{MetaStore, validate_version_id};
use crate::minecraft::maven::MavenCoordinate;
use crate::minecraft::version::{Artifact, LibraryDownloads, VersionJson};
use crate::net::Http;
use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ModLoader {
    Fabric,
    LegacyFabric,
    Quilt,
    Forge,
    NeoForge,
}

impl ModLoader {
    fn meta_url(self) -> Option<&'static str> {
        match self {
            Self::Fabric => Some("https://meta.fabricmc.net/v2"),
            Self::LegacyFabric => Some("https://meta.legacyfabric.net/v2"),
            Self::Quilt => Some("https://meta.quiltmc.org/v3"),
            Self::Forge | Self::NeoForge => None,
        }
    }

    fn default_maven(self) -> &'static str {
        match self {
            Self::Fabric => "https://maven.fabricmc.net/",
            Self::LegacyFabric => "https://maven.legacyfabric.net/",
            Self::Quilt => "https://maven.quiltmc.org/repository/release/",
            Self::Forge => "https://maven.minecraftforge.net/",
            Self::NeoForge => "https://maven.neoforged.net/releases/",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Fabric => "Fabric",
            Self::LegacyFabric => "Legacy Fabric",
            Self::Quilt => "Quilt",
            Self::Forge => "Forge",
            Self::NeoForge => "NeoForge",
        }
    }

    pub fn modrinth_loaders(self) -> &'static [&'static str] {
        match self {
            Self::Fabric => &["fabric"],
            Self::LegacyFabric => &["legacy-fabric"],
            Self::Quilt => &["quilt", "fabric"],
            Self::Forge => &["forge"],
            Self::NeoForge => &["neoforge"],
        }
    }

    pub fn runs_fabric_mods(self) -> bool {
        matches!(self, Self::Fabric | Self::LegacyFabric | Self::Quilt)
    }

    pub fn uses_installer(self) -> bool {
        matches!(self, Self::Forge | Self::NeoForge)
    }
}

#[derive(Debug, Deserialize)]
struct LoaderEntry {
    loader: LoaderInfo,
}

#[derive(Debug, Deserialize)]
struct LoaderInfo {
    version: String,
    #[serde(default)]
    stable: Option<bool>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct LoaderVersion {
    pub version: String,
    pub stable: bool,
}

fn encode_segment(value: &str) -> String {
    value.replace(' ', "%20")
}

fn is_stable(info: &LoaderInfo) -> bool {
    info.stable.unwrap_or_else(|| !info.version.contains('-'))
}

fn native_classifiers(template: &str) -> Vec<String> {
    if template.contains("${arch}") {
        vec![template.replace("${arch}", "64")]
    } else {
        vec![template.to_string()]
    }
}

pub fn fabric_needs_legacy(game_version: &str) -> bool {
    let mut parts = game_version.split('.').map(str::parse::<u32>);
    matches!(
        (parts.next(), parts.next()),
        (Some(Ok(1)), Some(Ok(minor))) if minor < 14
    )
}

pub fn profile_id(loader: ModLoader, game_version: &str, loader_version: &str) -> String {
    match loader {
        ModLoader::Fabric => format!("fabric-loader-{loader_version}-{game_version}"),
        ModLoader::LegacyFabric => {
            format!("legacy-fabric-loader-{loader_version}-{game_version}")
        }
        ModLoader::Quilt => format!("quilt-loader-{loader_version}-{game_version}"),
        ModLoader::Forge => format!("{game_version}-forge-{loader_version}"),
        ModLoader::NeoForge => format!("neoforge-{loader_version}"),
    }
}

pub(crate) async fn remote_sha1(http: &Http, url: &str) -> CoreResult<String> {
    let body = http.get_bytes(url).await?;
    let hash = String::from_utf8_lossy(&body)
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if hash.len() != 40 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(CoreError::InvalidInput(format!(
            "invalid checksum file {url}"
        )));
    }
    Ok(hash)
}

fn meta_url(loader: ModLoader) -> CoreResult<&'static str> {
    loader.meta_url().ok_or_else(|| {
        CoreError::Unsupported(format!(
            "{} profiles come from its installer",
            loader.display_name()
        ))
    })
}

pub async fn loader_versions(
    http: &Http,
    loader: ModLoader,
    game_version: &str,
) -> CoreResult<Vec<LoaderVersion>> {
    match loader {
        ModLoader::NeoForge => return neoforge::loader_versions(http, game_version).await,
        ModLoader::Forge => return forge::loader_versions(http, game_version).await,
        ModLoader::Fabric | ModLoader::LegacyFabric | ModLoader::Quilt => {}
    }
    validate_version_id(game_version)?;
    let url = format!(
        "{}/versions/loader/{}",
        meta_url(loader)?,
        encode_segment(game_version)
    );
    let entries: Vec<LoaderEntry> = http.get_json(&url).await?;
    Ok(entries
        .into_iter()
        .map(|entry| LoaderVersion {
            stable: is_stable(&entry.loader),
            version: entry.loader.version,
        })
        .collect())
}

pub async fn ensure_profile(
    http: &Http,
    meta: &MetaStore,
    loader: ModLoader,
    game_version: &str,
    loader_version: &str,
) -> CoreResult<String> {
    let meta_url = meta_url(loader)?;
    validate_version_id(game_version)?;
    validate_version_id(loader_version)?;
    let id = profile_id(loader, game_version, loader_version);
    if meta.stored_version(&id).await?.is_some() {
        return Ok(id);
    }

    let name = loader.display_name();
    let url = format!(
        "{}/versions/loader/{}/{}/profile/json",
        meta_url,
        encode_segment(game_version),
        encode_segment(loader_version)
    );
    let mut profile: VersionJson = http.get_json(&url).await.map_err(|err| match err {
        CoreError::HttpStatus {
            status: 400 | 404, ..
        } => CoreError::NotFound(format!(
            "{name} {loader_version} for Minecraft {game_version}"
        )),
        other => other,
    })?;
    if profile.inherits_from.as_deref() != Some(game_version) {
        return Err(CoreError::InvalidInput(format!(
            "{name} profile {} does not inherit from {game_version}",
            profile.id
        )));
    }
    profile.id = id.clone();

    for library in &mut profile.libraries {
        if library.downloads.is_some() || library.sha1.is_some() {
            continue;
        }
        let coordinate = MavenCoordinate::parse(&library.name)?;
        let base = library.url.as_deref().unwrap_or(loader.default_maven());
        if let Some(natives) = &library.natives {
            let mut classifiers = std::collections::HashMap::new();
            for template in natives.values() {
                for classifier in native_classifiers(template) {
                    let url = join_url(
                        base,
                        &coordinate.with_classifier(&classifier).relative_path(),
                    );
                    let sha1 = remote_sha1(http, &format!("{url}.sha1")).await?;
                    classifiers.insert(
                        classifier,
                        Artifact {
                            path: None,
                            sha1,
                            size: 0,
                            url,
                        },
                    );
                }
            }
            library.downloads = Some(LibraryDownloads {
                artifact: None,
                classifiers: Some(classifiers),
            });
            continue;
        }
        let sha1_url = format!("{}.sha1", join_url(base, &coordinate.relative_path()));
        library.sha1 = Some(remote_sha1(http, &sha1_url).await?);
    }

    meta.store_version(&profile).await?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_ids_are_stable_and_valid() {
        let fabric = profile_id(ModLoader::Fabric, "1.20.1", "0.16.0");
        assert_eq!(fabric, "fabric-loader-0.16.0-1.20.1");
        assert!(validate_version_id(&fabric).is_ok());
        assert_eq!(
            profile_id(ModLoader::Quilt, "1.20.1", "0.26.4"),
            "quilt-loader-0.26.4-1.20.1"
        );
        let neoforge = profile_id(ModLoader::NeoForge, "1.21.1", "21.1.265");
        assert_eq!(neoforge, "neoforge-21.1.265");
        assert!(validate_version_id(&neoforge).is_ok());
        let legacy = profile_id(ModLoader::LegacyFabric, "1.8.9", "0.20.1");
        assert_eq!(legacy, "legacy-fabric-loader-0.20.1-1.8.9");
        assert!(validate_version_id(&legacy).is_ok());
        let forge = profile_id(ModLoader::Forge, "1.20.1", "47.4.28");
        assert_eq!(forge, "1.20.1-forge-47.4.28");
        assert!(validate_version_id(&forge).is_ok());
    }

    #[test]
    fn old_minecraft_versions_need_legacy_fabric() {
        assert!(fabric_needs_legacy("1.8.9"));
        assert!(fabric_needs_legacy("1.13.2"));
        assert!(!fabric_needs_legacy("1.14"));
        assert!(!fabric_needs_legacy("1.20.1"));
        assert!(!fabric_needs_legacy("26.3"));
        assert!(!fabric_needs_legacy("18w43b"));
        assert_eq!(native_classifiers("natives-windows"), ["natives-windows"]);
        assert_eq!(
            native_classifiers("natives-windows-${arch}"),
            ["natives-windows-64"]
        );
    }

    #[test]
    fn quilt_stability_is_derived_from_the_version() {
        let entries: Vec<LoaderEntry> = serde_json::from_str(
            r#"[{"loader":{"version":"0.27.0-beta.1"}},{"loader":{"version":"0.26.4"}},{"loader":{"version":"0.16.0","stable":false}}]"#,
        )
        .unwrap();
        let stable: Vec<bool> = entries.iter().map(|e| is_stable(&e.loader)).collect();
        assert_eq!(stable, [false, true, false]);
    }

    #[test]
    fn quilt_also_accepts_fabric_mods() {
        assert_eq!(ModLoader::Quilt.modrinth_loaders(), ["quilt", "fabric"]);
        assert_eq!(ModLoader::Fabric.modrinth_loaders(), ["fabric"]);
        assert_eq!(ModLoader::NeoForge.modrinth_loaders(), ["neoforge"]);
    }
}

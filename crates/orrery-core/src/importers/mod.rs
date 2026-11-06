mod copy;
mod formats;
mod ini;
mod modrinth;
mod official;
mod service;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub use copy::{CopyPlan, Selection, check_archive, execute_copy, extract_archive, plan_copy};
pub(crate) use formats::curseforge_loader;
pub(crate) use service::loader_spec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ImportSource {
    Prism,
    MultiMc,
    ModrinthApp,
    CurseForge,
    AtLauncher,
    GdLauncher,
    Official,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum ForeignLoader {
    Vanilla,
    Fabric { version: String },
    Quilt { version: String },
    Forge { version: String },
    NeoForge { version: String },
    Other { name: String },
}

impl ForeignLoader {
    pub fn from_parts(kind: &str, version: Option<&str>) -> Self {
        let version = version.unwrap_or_default().trim().to_string();
        match kind.trim().to_ascii_lowercase().as_str() {
            "" | "vanilla" | "minecraft" => Self::Vanilla,
            "fabric" => Self::Fabric { version },
            "quilt" => Self::Quilt { version },
            "forge" => Self::Forge { version },
            "neoforge" | "neo_forge" => Self::NeoForge { version },
            other => Self::Other {
                name: other.to_string(),
            },
        }
    }

    pub fn uses_mods(&self) -> bool {
        !matches!(self, Self::Vanilla)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ForeignSettings {
    pub java_path: Option<String>,
    pub jvm_args: Option<String>,
    pub min_memory_mb: Option<u32>,
    pub max_memory_mb: Option<u32>,
    pub commands: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ForeignInstance {
    pub source: ImportSource,
    pub path: String,
    pub profile: Option<String>,
    pub name: String,
    pub game_dir: String,
    pub minecraft_version: Option<String>,
    pub latest: Option<String>,
    pub loader: ForeignLoader,
    pub settings: ForeignSettings,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImportTarget {
    pub path: String,
    #[serde(default)]
    pub profile: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImportFolder {
    pub name: String,
    pub files: u32,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum ImportWarning {
    LoaderNotSupported { loader: ForeignLoader },
    UnknownVersion,
    JavaNotTaken { path: String },
    JvmArgsNotTaken { args: String },
    CommandsNotTaken { commands: Vec<String> },
    SkippedLinks { count: u32 },
    SkippedUnsafe { count: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImportPreview {
    pub instance: ForeignInstance,
    pub minecraft_version: Option<String>,
    pub folders: Vec<ImportFolder>,
    pub files: u32,
    pub bytes: u64,
    pub warnings: Vec<ImportWarning>,
}

#[derive(Debug, Clone, Default)]
pub struct Locations {
    pub home: Option<PathBuf>,
    pub roaming: Option<PathBuf>,
    pub data: Option<PathBuf>,
    pub config: Option<PathBuf>,
    pub documents: Option<PathBuf>,
}

impl Locations {
    pub fn current() -> Self {
        let env = |key: &str| {
            std::env::var_os(key)
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
        };
        let home = env("HOME").or_else(|| env("USERPROFILE"));
        if cfg!(windows) {
            let roaming = env("APPDATA");
            Self {
                documents: home.as_ref().map(|h| h.join("Documents")),
                data: roaming.clone(),
                config: roaming.clone(),
                roaming,
                home,
            }
        } else if cfg!(target_os = "macos") {
            let support = home.as_ref().map(|h| h.join("Library/Application Support"));
            Self {
                documents: home.as_ref().map(|h| h.join("Documents")),
                data: support.clone(),
                config: support.clone(),
                roaming: support,
                home,
            }
        } else {
            let data =
                env("XDG_DATA_HOME").or_else(|| home.as_ref().map(|h| h.join(".local/share")));
            let config =
                env("XDG_CONFIG_HOME").or_else(|| home.as_ref().map(|h| h.join(".config")));
            Self {
                documents: home.as_ref().map(|h| h.join("Documents")),
                roaming: config.clone(),
                data,
                config,
                home,
            }
        }
    }
}

pub(crate) fn read_text(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        let units: Vec<u16> = bytes[2..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_le_bytes(*pair))
            .collect();
        return String::from_utf16(&units).ok();
    }
    let text = String::from_utf8_lossy(&bytes).into_owned();
    Some(text.trim_start_matches('\u{feff}').to_string())
}

pub(crate) fn read_json(path: &Path) -> Option<serde_json::Value> {
    serde_json::from_str(&read_text(path)?).ok()
}

pub(crate) fn display(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

pub(crate) fn folder_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Import".into())
}

pub fn inspect_instance(path: &Path, locations: &Locations) -> Vec<ForeignInstance> {
    if path.join("launcher_profiles.json").is_file() {
        return official::read_root(path);
    }
    if let Some(found) = formats::read_instance(path) {
        return vec![found];
    }
    if let Some(found) = modrinth::read_profile_folder(path, locations) {
        return vec![found];
    }
    Vec::new()
}

pub fn inspect(path: &Path, locations: &Locations) -> Vec<ForeignInstance> {
    let direct = inspect_instance(path, locations);
    if !direct.is_empty() {
        return direct;
    }
    if path.join("app.db").is_file() {
        return modrinth::read_database(path);
    }
    let mut found = Vec::new();
    for dir in [
        path.join("instances"),
        path.join("Instances"),
        path.join("profiles"),
        path.to_path_buf(),
    ] {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let child = entry.path();
            if child.is_dir() {
                found.extend(inspect_instance(&child, locations));
            }
        }
        if !found.is_empty() {
            break;
        }
    }
    found
}

fn scan_dir(dir: &Path, locations: &Locations, out: &mut Vec<ForeignInstance>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut children: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|p| p.is_dir())
        .collect();
    children.sort();
    for child in children {
        out.extend(inspect_instance(&child, locations));
    }
}

pub fn scan_defaults(locations: &Locations) -> Vec<ForeignInstance> {
    let mut found = Vec::new();
    for root in formats::prism_roots(locations) {
        for dir in formats::prism_instance_dirs(&root) {
            scan_dir(&dir, locations, &mut found);
        }
    }
    for dir in formats::instance_dirs(locations) {
        scan_dir(&dir, locations, &mut found);
    }
    found.extend(modrinth::scan(locations));
    for root in official::roots(locations) {
        found.extend(official::read_root(&root));
    }
    let mut seen = std::collections::HashSet::new();
    found.retain(|instance| seen.insert((instance.path.clone(), instance.profile.clone())));
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_loader_names() {
        assert_eq!(
            ForeignLoader::from_parts("Fabric", Some("0.16.9")),
            ForeignLoader::Fabric {
                version: "0.16.9".into()
            }
        );
        assert_eq!(
            ForeignLoader::from_parts("NeoForge", Some("21.1.1")),
            ForeignLoader::NeoForge {
                version: "21.1.1".into()
            }
        );
        assert_eq!(ForeignLoader::from_parts("", None), ForeignLoader::Vanilla);
        assert_eq!(
            ForeignLoader::from_parts("LegacyFabric", Some("1")),
            ForeignLoader::Other {
                name: "legacyfabric".into()
            }
        );
    }

    #[test]
    fn reads_utf16_and_bom_text() {
        let dir = tempfile::tempdir().unwrap();
        let utf16 = dir.path().join("a.json");
        let mut bytes = vec![0xFF, 0xFE];
        for unit in "{\"a\":1}".encode_utf16() {
            bytes.extend(unit.to_le_bytes());
        }
        std::fs::write(&utf16, bytes).unwrap();
        assert_eq!(read_json(&utf16).unwrap()["a"], 1);
        let bom = dir.path().join("b.json");
        std::fs::write(&bom, b"\xEF\xBB\xBF{\"b\":2}").unwrap();
        assert_eq!(read_json(&bom).unwrap()["b"], 2);
    }
}

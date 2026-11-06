use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{
    ForeignInstance, ForeignLoader, ForeignSettings, ImportSource, Locations, display, read_json,
};

pub const COPIED: [&str; 6] = [
    "saves",
    "resourcepacks",
    "shaderpacks",
    "screenshots",
    "options.txt",
    "servers.dat",
];
pub const COPIED_WITH_MODS: [&str; 8] = [
    "saves",
    "resourcepacks",
    "shaderpacks",
    "screenshots",
    "options.txt",
    "servers.dat",
    "mods",
    "config",
];
const PROFILE_FILES: [&str; 2] = [
    "launcher_profiles.json",
    "launcher_profiles_microsoft_store.json",
];

pub fn roots(locations: &Locations) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if cfg!(windows) {
        roots.extend(locations.roaming.as_ref().map(|r| r.join(".minecraft")));
    } else if cfg!(target_os = "macos") {
        roots.extend(locations.data.as_ref().map(|d| d.join("minecraft")));
    } else {
        roots.extend(locations.home.as_ref().map(|h| h.join(".minecraft")));
        roots.extend(
            locations
                .home
                .as_ref()
                .map(|h| h.join(".var/app/com.mojang.Minecraft/.minecraft")),
        );
    }
    roots
}

fn inherited(root: &Path, id: &str) -> Option<String> {
    if id.contains(['/', '\\']) || id.contains("..") {
        return None;
    }
    let json = read_json(&root.join("versions").join(id).join(format!("{id}.json")))?;
    json.get("inheritsFrom")
        .and_then(Value::as_str)
        .map(str::to_string)
}

pub fn parse_version_id(root: &Path, id: &str) -> (Option<String>, ForeignLoader) {
    let split_loader = |rest: &str| -> Option<(String, String)> {
        let (loader, game) = rest.rsplit_once('-')?;
        Some((game.to_string(), loader.to_string()))
    };
    if let Some(rest) = id.strip_prefix("fabric-loader-")
        && let Some((game, loader)) = split_loader(rest)
    {
        return (Some(game), ForeignLoader::Fabric { version: loader });
    }
    if let Some(rest) = id.strip_prefix("quilt-loader-")
        && let Some((game, loader)) = split_loader(rest)
    {
        return (Some(game), ForeignLoader::Quilt { version: loader });
    }
    if let Some((game, forge)) = id.split_once("-forge-") {
        return (
            Some(game.to_string()),
            ForeignLoader::Forge {
                version: forge.to_string(),
            },
        );
    }
    if let Some(version) = id.strip_prefix("neoforge-") {
        return (
            inherited(root, id),
            ForeignLoader::NeoForge {
                version: version.to_string(),
            },
        );
    }
    if id.to_ascii_lowercase().contains("forge") {
        return (
            inherited(root, id),
            ForeignLoader::Forge {
                version: id.to_string(),
            },
        );
    }
    match inherited(root, id) {
        Some(parent) => (
            Some(parent),
            ForeignLoader::Other {
                name: id.to_string(),
            },
        ),
        None => (Some(id.to_string()), ForeignLoader::Vanilla),
    }
}

pub fn read_root(root: &Path) -> Vec<ForeignInstance> {
    let mut found = Vec::new();
    for file in PROFILE_FILES {
        let Some(json) = read_json(&root.join(file)) else {
            continue;
        };
        let Some(profiles) = json.get("profiles").and_then(Value::as_object) else {
            continue;
        };
        for (key, profile) in profiles {
            let text = |name: &str| {
                profile
                    .get(name)
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|t| !t.is_empty())
                    .map(str::to_string)
            };
            let kind = text("type").unwrap_or_else(|| "custom".into());
            let latest = match kind.as_str() {
                "latest-release" => Some("release".to_string()),
                "latest-snapshot" => Some("snapshot".to_string()),
                _ => None,
            };
            let (minecraft_version, loader) = match (&latest, text("lastVersionId")) {
                (None, Some(id)) => parse_version_id(root, &id),
                _ => (None, ForeignLoader::Vanilla),
            };
            let game_dir = text("gameDir")
                .map(PathBuf::from)
                .filter(|dir| dir.is_absolute())
                .unwrap_or_else(|| root.to_path_buf());
            let name = text("name").unwrap_or_else(|| match latest.as_deref() {
                Some("snapshot") => "Latest snapshot".into(),
                _ => "Latest release".into(),
            });
            found.push(ForeignInstance {
                source: ImportSource::Official,
                path: display(root),
                profile: Some(format!("{file}#{key}")),
                name,
                game_dir: display(&game_dir),
                minecraft_version,
                latest,
                loader,
                settings: ForeignSettings {
                    java_path: text("javaDir"),
                    jvm_args: text("javaArgs"),
                    min_memory_mb: None,
                    max_memory_mb: None,
                    commands: Vec::new(),
                },
            });
        }
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_installer_version_ids() {
        let root = tempfile::tempdir().unwrap();
        let neo = root.path().join("versions/neoforge-21.1.77");
        std::fs::create_dir_all(&neo).unwrap();
        std::fs::write(
            neo.join("neoforge-21.1.77.json"),
            r#"{"inheritsFrom":"1.21.1"}"#,
        )
        .unwrap();
        let parse = |id: &str| parse_version_id(root.path(), id);
        assert_eq!(
            parse("fabric-loader-0.16.9-1.21.1"),
            (
                Some("1.21.1".into()),
                ForeignLoader::Fabric {
                    version: "0.16.9".into()
                }
            )
        );
        assert_eq!(
            parse("quilt-loader-0.26.0-1.20.4"),
            (
                Some("1.20.4".into()),
                ForeignLoader::Quilt {
                    version: "0.26.0".into()
                }
            )
        );
        assert_eq!(
            parse("1.20.1-forge-47.2.0"),
            (
                Some("1.20.1".into()),
                ForeignLoader::Forge {
                    version: "47.2.0".into()
                }
            )
        );
        assert_eq!(
            parse("neoforge-21.1.77"),
            (
                Some("1.21.1".into()),
                ForeignLoader::NeoForge {
                    version: "21.1.77".into()
                }
            )
        );
        assert_eq!(
            parse("1.21.4"),
            (Some("1.21.4".into()), ForeignLoader::Vanilla)
        );
    }

    #[test]
    fn reads_launcher_profiles() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("launcher_profiles.json"),
            r#"{"profiles":{"a":{"name":"","type":"latest-release","lastVersionId":"latest-release"},"b":{"name":"Fabric","type":"custom","lastVersionId":"fabric-loader-0.16.9-1.21.1","javaArgs":"-Xmx4G","gameDir":"relative/ignored"}}}"#,
        )
        .unwrap();
        let found = read_root(root.path());
        assert_eq!(found.len(), 2);
        let fabric = found.iter().find(|i| i.name == "Fabric").unwrap();
        assert_eq!(fabric.profile.as_deref(), Some("launcher_profiles.json#b"));
        assert_eq!(fabric.game_dir, display(root.path()));
        assert_eq!(fabric.settings.jvm_args.as_deref(), Some("-Xmx4G"));
        let latest = found.iter().find(|i| i.name == "Latest release").unwrap();
        assert_eq!(latest.latest.as_deref(), Some("release"));
        assert_eq!(latest.minecraft_version, None);
    }
}

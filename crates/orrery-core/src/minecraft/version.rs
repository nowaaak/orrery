use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::rules::Rule;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionJson {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inherits_from: Option<String>,
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main_class: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minecraft_arguments: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Arguments>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_index: Option<AssetIndexRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assets: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub downloads: Option<VersionDownloads>,
    #[serde(default)]
    pub libraries: Vec<Library>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub java_version: Option<JavaVersion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logging: Option<Logging>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_time: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<Argument>,
    #[serde(default)]
    pub jvm: Vec<Argument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Argument {
    Plain(String),
    Conditional {
        rules: Vec<Rule>,
        value: ArgumentValue,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ArgumentValue {
    Single(String),
    Many(Vec<String>),
}

impl ArgumentValue {
    pub fn values(&self) -> Vec<&str> {
        match self {
            Self::Single(value) => vec![value.as_str()],
            Self::Many(values) => values.iter().map(String::as_str).collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndexRef {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDownloads {
    #[serde(default)]
    pub client: Option<FileDownload>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_mappings: Option<FileDownload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDownload {
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    #[serde(default)]
    pub path: Option<String>,
    pub sha1: String,
    #[serde(default)]
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub downloads: Option<LibraryDownloads>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub natives: Option<HashMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extract: Option<Extract>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<Rule>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LibraryDownloads {
    #[serde(default)]
    pub artifact: Option<Artifact>,
    #[serde(default)]
    pub classifiers: Option<HashMap<String, Artifact>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Extract {
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersion {
    pub component: String,
    pub major_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Logging {
    #[serde(default)]
    pub client: Option<LoggingClient>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingClient {
    pub argument: String,
    pub file: LoggingFile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingFile {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

impl VersionJson {
    pub fn merge_onto(self, parent: VersionJson) -> VersionJson {
        let mut seen = HashSet::new();
        let libraries = self
            .libraries
            .into_iter()
            .chain(parent.libraries)
            .filter(|lib| seen.insert(library_identity(&lib.name)))
            .collect();

        let arguments = match (parent.arguments, self.arguments) {
            (Some(parent), Some(child)) => Some(Arguments {
                game: parent.game.into_iter().chain(child.game).collect(),
                jvm: parent.jvm.into_iter().chain(child.jvm).collect(),
            }),
            (parent, child) => child.or(parent),
        };

        VersionJson {
            id: self.id,
            inherits_from: None,
            kind: self.kind.or(parent.kind),
            main_class: self.main_class.or(parent.main_class),
            minecraft_arguments: self.minecraft_arguments.or(parent.minecraft_arguments),
            arguments,
            asset_index: self.asset_index.or(parent.asset_index),
            assets: self.assets.or(parent.assets),
            downloads: self.downloads.or(parent.downloads),
            libraries,
            java_version: self.java_version.or(parent.java_version),
            logging: self
                .logging
                .filter(|logging| logging.client.is_some())
                .or(parent.logging),
            release_time: self.release_time.or(parent.release_time),
        }
    }
}

fn library_identity(name: &str) -> String {
    let parts: Vec<&str> = name.split(':').collect();
    match parts.as_slice() {
        [group, artifact, _version] => format!("{group}:{artifact}"),
        [group, artifact, _version, classifier, ..] => format!("{group}:{artifact}:{classifier}"),
        _ => name.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lib(name: &str) -> Library {
        Library {
            name: name.into(),
            downloads: None,
            url: None,
            sha1: None,
            size: None,
            natives: None,
            extract: None,
            rules: None,
        }
    }

    fn version(id: &str) -> VersionJson {
        serde_json::from_value(serde_json::json!({ "id": id })).unwrap()
    }

    #[test]
    fn child_overrides_main_class_and_prepends_unique_libraries() {
        let mut parent = version("1.20.1");
        parent.main_class = Some("net.minecraft.client.main.Main".into());
        parent.libraries = vec![
            lib("org.ow2.asm:asm:9.3"),
            lib("com.mojang:brigadier:1.1.8"),
        ];
        parent.arguments = Some(Arguments {
            game: vec![Argument::Plain("--username".into())],
            jvm: vec![Argument::Plain("-cp".into())],
        });

        let mut child = version("fabric-loader-0.16.0-1.20.1");
        child.inherits_from = Some("1.20.1".into());
        child.main_class = Some("net.fabricmc.loader.impl.launch.knot.KnotClient".into());
        child.libraries = vec![
            lib("org.ow2.asm:asm:9.7"),
            lib("net.fabricmc:fabric-loader:0.16.0"),
        ];
        child.arguments = Some(Arguments {
            game: vec![],
            jvm: vec![Argument::Plain("-DFabricMcEmu=x".into())],
        });

        let merged = child.merge_onto(parent);
        assert_eq!(merged.id, "fabric-loader-0.16.0-1.20.1");
        assert_eq!(
            merged.main_class.as_deref(),
            Some("net.fabricmc.loader.impl.launch.knot.KnotClient")
        );
        let names: Vec<_> = merged.libraries.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "org.ow2.asm:asm:9.7",
                "net.fabricmc:fabric-loader:0.16.0",
                "com.mojang:brigadier:1.1.8"
            ]
        );
        let arguments = merged.arguments.unwrap();
        assert_eq!(arguments.jvm.len(), 2);
        assert_eq!(arguments.game.len(), 1);
        assert!(merged.inherits_from.is_none());
    }

    #[test]
    fn native_classifiers_are_kept_as_separate_libraries() {
        let mut child = version("x");
        child.libraries = vec![
            lib("org.lwjgl:lwjgl:3.3.1"),
            lib("org.lwjgl:lwjgl:3.3.1:natives-windows"),
        ];
        let merged = child.merge_onto(version("parent"));
        assert_eq!(merged.libraries.len(), 2);
    }

    #[test]
    fn an_empty_child_logging_block_keeps_the_parent_config() {
        let parent: VersionJson = serde_json::from_value(serde_json::json!({
            "id": "1.20.1",
            "logging": { "client": {
                "argument": "-Dlog4j.configurationFile=${path}",
                "file": { "id": "client-1.12.xml", "sha1": "x", "size": 1, "url": "https://example.com/c.xml" }
            } }
        }))
        .unwrap();
        let child: VersionJson = serde_json::from_value(
            serde_json::json!({ "id": "1.20.1-forge-47.4.28", "logging": {} }),
        )
        .unwrap();
        let merged = child.merge_onto(parent);
        assert!(merged.logging.and_then(|l| l.client).is_some());
    }
}

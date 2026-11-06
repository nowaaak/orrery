use std::io::Read;
use std::path::Path;

use serde::Serialize;
use ts_rs::TS;

use crate::error::IoResultExt;
use crate::{CoreError, CoreResult};

const MARKER: &[u8] = b"OptiFine_";
const CHANGELOG: &str = "changelog.txt";
const MAX_CHANGELOG_BYTES: u64 = 1024 * 1024;
const FORGE_NOTE: &str = "compatible with Forge ";
const MAX_CLASS_BYTES: u64 = 4 * 1024 * 1024;
const VERSION_CLASS: &str = "Config.class";
const VERSION_CLASS_SUFFIX: &str = "optifine/Config.class";
const PACKAGES: [&str; 2] = ["net/optifine/", "optifine/"];

fn is_version_class(name: &str) -> bool {
    name == VERSION_CLASS || name.ends_with(VERSION_CLASS_SUFFIX)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OptifineJar {
    pub minecraft_version: String,
    pub edition: String,
    pub release: String,
    pub preview: bool,
    pub forge_version: Option<String>,
}

impl OptifineJar {
    pub fn label(&self) -> String {
        let label = format!("{} {}", self.edition.replace('_', " "), self.release);
        if self.preview {
            format!("{label} preview")
        } else {
            label
        }
    }
}

fn is_release(token: &str) -> bool {
    let mut chars = token.chars();
    chars.next().is_some_and(|c| c.is_ascii_uppercase())
        && chars.clone().next().is_some()
        && chars.all(|c| c.is_ascii_digit())
}

pub fn parse_version(text: &str) -> Option<OptifineJar> {
    let rest = text.strip_prefix("OptiFine_")?;
    let (minecraft_version, rest) = rest.split_once('_')?;
    let valid_minecraft = minecraft_version.starts_with(|c: char| c.is_ascii_digit())
        && minecraft_version
            .chars()
            .all(|c| c.is_ascii_digit() || c == '.');
    if !valid_minecraft {
        return None;
    }
    let tokens: Vec<&str> = rest.split('_').collect();
    let release_at = tokens.iter().position(|token| is_release(token))?;
    if release_at == 0 {
        return None;
    }
    let trailing = &tokens[release_at + 1..];
    if trailing.iter().any(|token| !token.starts_with("pre")) {
        return None;
    }
    Some(OptifineJar {
        minecraft_version: minecraft_version.to_string(),
        edition: tokens[..release_at].join("_"),
        release: tokens[release_at].to_string(),
        preview: !trailing.is_empty(),
        forge_version: None,
    })
}

pub fn forge_compatibility(changelog: &str) -> Option<String> {
    let mut lines = changelog.lines().skip_while(|line| line.trim().is_empty());
    lines.next()?;
    lines
        .take_while(|line| !line.starts_with("OptiFine "))
        .find_map(|line| {
            let note = line.trim().trim_start_matches('-').trim();
            let version = note.strip_prefix(FORGE_NOTE)?.trim();
            let numeric = !version.is_empty()
                && version
                    .split('.')
                    .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()));
            numeric.then(|| version.to_string())
        })
}

pub fn find_version(bytes: &[u8]) -> Option<OptifineJar> {
    let mut start = 0;
    while let Some(offset) = bytes[start..]
        .windows(MARKER.len())
        .position(|window| window == MARKER)
    {
        let begin = start + offset;
        let end = bytes[begin..]
            .iter()
            .position(|b| !(b.is_ascii_alphanumeric() || *b == b'_' || *b == b'.'))
            .map_or(bytes.len(), |len| begin + len);
        if let Some(found) = std::str::from_utf8(&bytes[begin..end])
            .ok()
            .map(|text| text.trim_end_matches(['.', '_']))
            .and_then(parse_version)
        {
            return Some(found);
        }
        start = begin + MARKER.len();
    }
    None
}

fn from_file_name(path: &Path) -> Option<OptifineJar> {
    let stem = path.file_stem()?.to_str()?;
    let stem = stem.strip_prefix("preview_").unwrap_or(stem);
    parse_version(stem)
}

pub fn read(
    archive: &mut zip::ZipArchive<std::fs::File>,
    path: &Path,
) -> CoreResult<Option<OptifineJar>> {
    let is_optifine = archive.file_names().any(|name| {
        name.is_ok_and(|name| PACKAGES.iter().any(|package| name.starts_with(package)))
    });
    if !is_optifine {
        return Ok(None);
    }
    let classes: Vec<String> = archive
        .file_names()
        .filter_map(|name| name.ok().map(|name| name.into_owned()))
        .filter(|name| is_version_class(name))
        .collect();
    for class in classes {
        let Ok(entry) = archive.by_name(&class) else {
            continue;
        };
        let mut bytes = Vec::new();
        entry
            .take(MAX_CLASS_BYTES)
            .read_to_end(&mut bytes)
            .at_path(path)?;
        if let Some(found) = find_version(&bytes) {
            return Ok(Some(with_forge(archive, found)));
        }
    }
    let found = from_file_name(path).ok_or_else(|| CoreError::Archive {
        path: path.to_path_buf(),
        message: "this looks like OptiFine, but its version could not be read".into(),
    })?;
    Ok(Some(with_forge(archive, found)))
}

fn with_forge(archive: &mut zip::ZipArchive<std::fs::File>, mut jar: OptifineJar) -> OptifineJar {
    if let Ok(entry) = archive.by_name(CHANGELOG) {
        let mut text = String::new();
        if entry
            .take(MAX_CHANGELOG_BYTES)
            .read_to_string(&mut text)
            .is_ok()
        {
            jar.forge_version = forge_compatibility(&text);
        }
    }
    jar
}

pub fn inspect(path: &Path) -> CoreResult<Option<OptifineJar>> {
    let file = std::fs::File::open(path).at_path(path)?;
    let Ok(mut archive) = zip::ZipArchive::new(file) else {
        return Ok(None);
    };
    read(&mut archive, path)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn parses_release_and_preview_versions() {
        let release = parse_version("OptiFine_1.20.1_HD_U_I6").unwrap();
        assert_eq!(release.minecraft_version, "1.20.1");
        assert_eq!(release.edition, "HD_U");
        assert_eq!(release.release, "I6");
        assert!(!release.preview);
        assert_eq!(release.label(), "HD U I6");

        let preview = parse_version("OptiFine_1.21.4_HD_U_J3_pre12").unwrap();
        assert!(preview.preview);
        assert_eq!(preview.label(), "HD U J3 preview");

        assert_eq!(
            parse_version("OptiFine_1.8.9_HD_U_M5").unwrap().release,
            "M5"
        );
        assert_eq!(parse_version("OptiFine_"), None);
        assert_eq!(parse_version("OptiFine_HD_U_I6"), None);
        assert_eq!(parse_version("OptiFine_1.20.1_I6"), None);
        assert_eq!(parse_version("OptiFine_1.20.1_HD_U_I6_extra"), None);
    }

    #[test]
    fn reads_the_forge_version_of_the_newest_release_only() {
        let changelog = "OptiFine 1.20.1_HD_U_I6\r\n - fixed powder snow\r\n - compatible with Forge 47.2.18\r\n - compatible with NeoForge 47.1.56\r\n(21.12.2023)\r\n\r\nOptiFine 1.20.1_HD_U_I5\r\n - compatible with Forge 47.0.35\r\n";
        assert_eq!(forge_compatibility(changelog).as_deref(), Some("47.2.18"));
        let none = "OptiFine 1.21_HD_U_J1\n - not compatible with Forge\n\nOptiFine 1.21_HD_U_I9\n - compatible with Forge 51.0.1\n";
        assert_eq!(forge_compatibility(none), None);
        assert_eq!(forge_compatibility(""), None);
    }

    #[test]
    fn finds_the_version_inside_class_bytes() {
        let mut class = b"\xCA\xFE\xBA\xBE\x01\x00\x09OptiFine_\x01\x00".to_vec();
        class.extend_from_slice(b"\x01\x00\x17OptiFine_1.20.1_HD_U_I6\x01\x00\x04HD_U");
        assert_eq!(find_version(&class).unwrap().minecraft_version, "1.20.1");
        assert_eq!(find_version(b"no marker here"), None);
    }

    fn jar(dir: &Path, name: &str, files: &[(&str, &[u8])]) -> std::path::PathBuf {
        let path = dir.join(name);
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        for (entry, content) in files {
            zip.start_file(*entry, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(content).unwrap();
        }
        zip.finish().unwrap();
        path
    }

    #[test]
    fn inspects_jars() {
        let dir = tempfile::tempdir().unwrap();
        let modern = jar(
            dir.path(),
            "renamed.jar",
            &[
                ("optifine/Installer.class", b""),
                (
                    "notch/net/optifine/Config.class",
                    b"..OptiFine_1.20.1_HD_U_I6..",
                ),
                ("notch/net/minecraftforge/common/ForgeConfig.class", b""),
            ],
        );
        assert_eq!(inspect(&modern).unwrap().unwrap().release, "I6");

        let by_name = jar(
            dir.path(),
            "preview_OptiFine_1.21.4_HD_U_J3_pre12.jar",
            &[("net/optifine/Other.class", b"")],
        );
        assert!(inspect(&by_name).unwrap().unwrap().preview);

        let unknown = jar(dir.path(), "x.jar", &[("optifine/Installer.class", b"")]);
        assert!(inspect(&unknown).is_err());

        let other = jar(dir.path(), "sodium.jar", &[("fabric.mod.json", b"{}")]);
        assert_eq!(inspect(&other).unwrap(), None);

        std::fs::write(dir.path().join("broken.jar"), b"not a zip").unwrap();
        assert_eq!(inspect(&dir.path().join("broken.jar")).unwrap(), None);
    }
}

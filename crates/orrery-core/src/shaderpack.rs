use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use sha2::{Digest, Sha512};

use crate::error::IoResultExt;
use crate::extras::{ModUpdate, uses_shaders};
use crate::game_options::{self, HardwareTier};
use crate::instances::mods::{ContentKind, InstalledMod, ModsDir};
use crate::modrinth::install::{ModTarget, install};
use crate::modrinth::service::content_loaders;
use crate::progress::Stage;
use crate::services::Services;
use crate::storage;
use crate::tasks::{TaskId, TaskKind, TaskSpec};
use crate::{CoreError, CoreResult};

pub const BUNDLED_SHADER_ID: &str = "orrery-shaders";
pub const SHADERPACK_VERSION: &str = "1.0.0";
pub const SHADERPACK_TITLE: &str = "Orrery Shaders";

const IRIS_SLUG: &str = "iris";
const IRIS_CONFIG: &str = "config/iris.properties";
const PACK_PROPERTIES: &str = "shaders/shaders.properties";
const FILES: &[(&str, &[u8])] = include!(concat!(env!("OUT_DIR"), "/shaderpack_files.rs"));

pub struct BundledPack {
    pub bytes: Vec<u8>,
    pub sha512: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activation {
    Select,
    KeepSelection,
}

pub fn file_name() -> String {
    format!("Orrery-Shaders-{SHADERPACK_VERSION}.zip")
}

pub fn bundled() -> CoreResult<&'static BundledPack> {
    static PACK: OnceLock<BundledPack> = OnceLock::new();
    if let Some(pack) = PACK.get() {
        return Ok(pack);
    }
    let pack = build_pack()?;
    Ok(PACK.get_or_init(|| pack))
}

fn build_pack() -> CoreResult<BundledPack> {
    let archive = PathBuf::from(file_name());
    let zip_error = |err: zip::result::ZipError| CoreError::Archive {
        path: archive.clone(),
        message: err.to_string(),
    };
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .last_modified_time(zip::DateTime::DEFAULT)
        .unix_permissions(0o644);
    for (name, bytes) in FILES {
        zip.start_file(*name, options).map_err(zip_error)?;
        zip.write_all(bytes).at_path(&archive)?;
    }
    let bytes = zip.finish().map_err(zip_error)?.into_inner();
    let sha512 = hex::encode(Sha512::digest(&bytes));
    Ok(BundledPack { bytes, sha512 })
}

fn pack_text(name: &str) -> &'static str {
    FILES
        .iter()
        .find(|(file, _)| *file == name)
        .and_then(|(_, bytes)| std::str::from_utf8(bytes).ok())
        .unwrap_or_default()
}

fn profile_name(tier: HardwareTier) -> &'static str {
    match tier {
        HardwareTier::Low => "LOW",
        HardwareTier::Medium => "MEDIUM",
        HardwareTier::High => "HIGH",
    }
}

fn property_key(line: &str) -> Option<&str> {
    let line = line.trim_start();
    if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
        return None;
    }
    let end = line
        .find(|c: char| matches!(c, '=' | ':') || c.is_whitespace())
        .unwrap_or(line.len());
    Some(&line[..end])
}

fn property_value(line: &str) -> &str {
    let line = line.trim_start();
    let key_end = line
        .find(|c: char| matches!(c, '=' | ':') || c.is_whitespace())
        .unwrap_or(line.len());
    let rest = line[key_end..].trim_start();
    rest.strip_prefix(['=', ':']).unwrap_or(rest).trim()
}

fn unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next) = chars.next() {
                out.push(next);
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn read_property(text: &str, key: &str) -> Option<String> {
    text.lines()
        .find(|line| property_key(line) == Some(key))
        .map(|line| unescape(property_value(line)))
}

fn set_properties(text: &str, values: &[(&str, &str)]) -> String {
    let mut written = vec![false; values.len()];
    let mut out = String::new();
    for line in text.lines() {
        let index = property_key(line).and_then(|key| values.iter().position(|(k, _)| *k == key));
        match index {
            Some(index) if written[index] => {}
            Some(index) => {
                let (key, value) = values[index];
                out.push_str(&format!("{key}={value}\n"));
                written[index] = true;
            }
            None => {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    for ((key, value), done) in values.iter().zip(written) {
        if !done {
            out.push_str(&format!("{key}={value}\n"));
        }
    }
    out
}

fn profile_values(properties: &str, profile: &str) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();
    collect_profile(properties, profile, &mut values, 0);
    values
}

fn collect_profile(
    properties: &str,
    profile: &str,
    values: &mut BTreeMap<String, String>,
    depth: u32,
) {
    let key = format!("profile.{profile}");
    let Some(line) = properties
        .lines()
        .find(|line| property_key(line) == Some(key.as_str()))
    else {
        return;
    };
    for token in property_value(line).split_whitespace() {
        if token.starts_with("!program.") {
            continue;
        }
        if let Some(parent) = token.strip_prefix("profile.") {
            if depth < 8 {
                collect_profile(properties, parent, values, depth + 1);
            }
        } else if let Some(option) = token.strip_prefix('!') {
            values.insert(option.to_string(), "false".to_string());
        } else if let Some((option, value)) = token.split_once(['=', ':']) {
            values.insert(option.to_string(), value.to_string());
        } else {
            values.insert(token.to_string(), "true".to_string());
        }
    }
}

async fn read_text(path: &Path) -> CoreResult<Option<String>> {
    match tokio::fs::read_to_string(path).await {
        Ok(text) => Ok(Some(text)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err).at_path(path),
    }
}

async fn update_iris_config(
    game_dir: &Path,
    file_name: &str,
    previous: Option<&str>,
    activation: Activation,
) -> CoreResult<()> {
    let path = game_dir.join(IRIS_CONFIG);
    let text = read_text(&path).await?.unwrap_or_default();
    let values: &[(&str, &str)] = match activation {
        Activation::Select => &[("shaderPack", file_name), ("enableShaders", "true")],
        Activation::KeepSelection => {
            let selected = read_property(&text, "shaderPack");
            if previous.is_none() || selected.as_deref() != previous {
                return Ok(());
            }
            &[("shaderPack", file_name)]
        }
    };
    storage::write_bytes_atomic(&path, set_properties(&text, values).as_bytes()).await
}

async fn prepare_options(
    shader_dir: &Path,
    file_name: &str,
    previous: Option<&str>,
    tier: HardwareTier,
) -> CoreResult<()> {
    let target = shader_dir.join(format!("{file_name}.txt"));
    if tokio::fs::try_exists(&target).await.at_path(&target)? {
        return Ok(());
    }
    if let Some(previous) = previous.filter(|previous| *previous != file_name) {
        let old = shader_dir.join(format!("{previous}.txt"));
        if tokio::fs::try_exists(&old).await.at_path(&old)? {
            return tokio::fs::rename(&old, &target).await.at_path(&target);
        }
    }
    let values = profile_values(pack_text(PACK_PROPERTIES), profile_name(tier));
    let text: String = values
        .iter()
        .map(|(key, value)| format!("{key}={value}\n"))
        .collect();
    storage::write_bytes_atomic(&target, text.as_bytes()).await
}

pub async fn install_bundled(
    mods: &ModsDir,
    activation: Activation,
    tier: HardwareTier,
) -> CoreResult<InstalledMod> {
    let pack = bundled()?;
    let file_name = file_name();
    let shader_dir = mods.dir(ContentKind::Shader);
    storage::write_bytes_atomic(&shader_dir.join(&file_name), &pack.bytes).await?;

    let mut index = mods.load().await?;
    let previous = index
        .mods
        .iter()
        .position(|m| m.project_id == BUNDLED_SHADER_ID)
        .map(|position| index.mods.remove(position));
    if let Some(old) = &previous {
        if old.file_name != file_name {
            mods.delete_files(old).await?;
        } else if !old.enabled {
            mods.delete_disabled_copy(old).await?;
        }
    }
    let previous_name = previous.as_ref().map(|old| old.file_name.as_str());

    let entry = InstalledMod {
        project_id: BUNDLED_SHADER_ID.to_string(),
        version_id: SHADERPACK_VERSION.to_string(),
        title: SHADERPACK_TITLE.to_string(),
        version_number: SHADERPACK_VERSION.to_string(),
        file_name: file_name.clone(),
        sha512: pack.sha512.clone(),
        enabled: true,
        dependency: false,
        icon_url: None,
        kind: ContentKind::Shader,
        game_versions: Vec::new(),
        loaders: Vec::new(),
        pack: None,
        pinned: false,
    };
    index.mods.push(entry.clone());
    index.mods.sort_by_key(|m| m.title.to_lowercase());
    mods.save(&index).await?;

    prepare_options(&shader_dir, &file_name, previous_name, tier).await?;
    update_iris_config(mods.game_dir(), &file_name, previous_name, activation).await?;
    Ok(entry)
}

pub fn bundled_update(installed: &[InstalledMod]) -> CoreResult<Option<ModUpdate>> {
    let Some(entry) = installed.iter().find(|m| m.project_id == BUNDLED_SHADER_ID) else {
        return Ok(None);
    };
    if entry.sha512 == bundled()?.sha512 {
        return Ok(None);
    }
    Ok(Some(ModUpdate {
        project_id: BUNDLED_SHADER_ID.to_string(),
        title: SHADERPACK_TITLE.to_string(),
        current_version: entry.version_number.clone(),
        version_id: SHADERPACK_VERSION.to_string(),
        version_number: SHADERPACK_VERSION.to_string(),
    }))
}

impl Services {
    pub async fn install_orrery_shaders(&self, instance_id: &str) -> CoreResult<TaskId> {
        let instance = self.instances.get(instance_id).await?;
        content_loaders(&instance, ContentKind::Shader)?;
        self.ensure_stopped(&instance)?;
        let services = self.clone();
        self.tasks.spawn(
            TaskSpec {
                kind: TaskKind::InstallOrreryShaders,
                subject: SHADERPACK_TITLE.to_string(),
                instance_id: Some(instance.id.clone()),
            },
            move |progress| async move {
                let _guard = services.mods_lock.lock().await;
                let mods = services.mods_dir(&instance)?;
                if !uses_shaders(&mods.load().await?.mods) {
                    let target = ModTarget {
                        game_version: &instance.minecraft_version,
                        loaders: content_loaders(&instance, ContentKind::Mod)?,
                        kind: ContentKind::Mod,
                    };
                    install(
                        &services.http,
                        &services.modrinth,
                        &mods,
                        &target,
                        IRIS_SLUG,
                        None,
                        &progress,
                    )
                    .await?;
                }
                progress.check_cancelled()?;
                progress.set_stage(Stage::ExtractingFiles);
                install_bundled(
                    &mods,
                    Activation::Select,
                    game_options::detect_hardware_tier(),
                )
                .await?;
                progress.set_stage(Stage::Finished);
                Ok(())
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::instances::mods::ModsFile;

    fn pack_files() -> impl Iterator<Item = &'static str> {
        FILES.iter().map(|(name, _)| *name)
    }

    fn lang_keys(name: &str) -> BTreeSet<&'static str> {
        pack_text(name)
            .lines()
            .filter_map(|line| line.split_once('=').map(|(key, _)| key.trim()))
            .collect()
    }

    fn defined_options() -> BTreeSet<String> {
        FILES
            .iter()
            .filter(|(name, _)| name.ends_with(".glsl"))
            .flat_map(|(name, _)| pack_text(name).lines())
            .filter_map(|line| {
                let line = line.trim_start();
                if let Some(rest) = line.strip_prefix("#define ") {
                    rest.split_whitespace().next().map(str::to_string)
                } else if line.starts_with("const ") && line.contains("//") {
                    line.split_whitespace().nth(2).map(str::to_string)
                } else {
                    None
                }
            })
            .collect()
    }

    fn screen_and_profile_options() -> BTreeSet<String> {
        let properties = pack_text(PACK_PROPERTIES);
        let mut options = BTreeSet::new();
        for line in properties.lines() {
            let Some(key) = property_key(line) else {
                continue;
            };
            if key == "screen" || key.starts_with("screen.") || key == "sliders" {
                options.extend(
                    property_value(line)
                        .split_whitespace()
                        .filter(|token| !token.starts_with('[') && *token != "<empty>")
                        .filter(|token| *token != "<profile>")
                        .map(str::to_string),
                );
            }
            if let Some(profile) = key.strip_prefix("profile.") {
                options.extend(profile_values(properties, profile).into_keys());
            }
        }
        options
    }

    #[test]
    fn pack_is_deterministic_and_complete() {
        let first = build_pack().unwrap();
        let second = build_pack().unwrap();
        assert_eq!(first.sha512, second.sha512);
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(first.bytes)).unwrap();
        assert_eq!(archive.len(), FILES.len());
        assert!(archive.by_name(PACK_PROPERTIES).is_ok());
    }

    #[test]
    fn every_program_has_both_stages_and_includes_resolve() {
        let files: BTreeSet<&str> = pack_files().collect();
        for name in &files {
            if let Some(stem) = name.strip_suffix(".vsh") {
                assert!(files.contains(format!("{stem}.fsh").as_str()), "{stem}.fsh");
            }
            if let Some(stem) = name.strip_suffix(".fsh") {
                assert!(files.contains(format!("{stem}.vsh").as_str()), "{stem}.vsh");
            }
            for line in pack_text(name).lines() {
                if let Some(include) = line.trim().strip_prefix("#include \"/") {
                    let target = format!("shaders/{}", include.trim_end_matches('"'));
                    assert!(files.contains(target.as_str()), "{name} includes {target}");
                }
            }
        }
        for name in files.iter().filter(|name| name.ends_with("sh")) {
            assert!(
                pack_text(name).starts_with("#version "),
                "{name} must start with #version"
            );
        }
    }

    #[test]
    fn options_are_defined_and_translated() {
        let defined = defined_options();
        let english = lang_keys("shaders/lang/en_us.lang");
        let german = lang_keys("shaders/lang/de_de.lang");
        let options = screen_and_profile_options();
        assert!(!options.is_empty());
        for option in &options {
            assert!(defined.contains(option), "{option} is not defined");
            let key = format!("option.{option}");
            assert!(english.contains(key.as_str()), "{key} missing in en_us");
            assert!(german.contains(key.as_str()), "{key} missing in de_de");
        }
        assert_eq!(english, german, "language files have different keys");
        for profile in ["LOW", "MEDIUM", "HIGH", "ULTRA"] {
            assert!(!profile_values(pack_text(PACK_PROPERTIES), profile).is_empty());
        }
    }

    #[test]
    fn properties_are_replaced_in_place() {
        let text =
            "#Iris config\nmaxShadowRenderDistance=32\nshaderPack=Other.zip\nshaderPack=Dup.zip\n";
        let updated = set_properties(
            text,
            &[("shaderPack", "Orrery.zip"), ("enableShaders", "true")],
        );
        assert_eq!(
            updated,
            "#Iris config\nmaxShadowRenderDistance=32\nshaderPack=Orrery.zip\nenableShaders=true\n"
        );
        assert_eq!(
            read_property("shaderPack = My\\:Pack.zip\n", "shaderPack").as_deref(),
            Some("My:Pack.zip")
        );
        assert_eq!(read_property("# shaderPack=x\n", "shaderPack"), None);
    }

    #[test]
    fn profiles_inherit_and_toggle_options() {
        let properties =
            "profile.LOW=A=1 !B C !program.composite1\nprofile.HIGH=profile.LOW A:3 B\n";
        let low = profile_values(properties, "LOW");
        assert_eq!(low["A"], "1");
        assert_eq!(low["B"], "false");
        assert_eq!(low["C"], "true");
        assert_eq!(low.len(), 3);
        let high = profile_values(properties, "HIGH");
        assert_eq!(high["A"], "3");
        assert_eq!(high["B"], "true");
    }

    fn mods_dir(root: &Path) -> ModsDir {
        ModsDir::new(root.to_path_buf(), &root.join("minecraft"))
    }

    #[tokio::test]
    async fn install_selects_pack_and_writes_profile() {
        let dir = tempfile::tempdir().unwrap();
        let mods = mods_dir(dir.path());
        let config = dir.path().join("minecraft").join(IRIS_CONFIG);
        std::fs::create_dir_all(config.parent().unwrap()).unwrap();
        std::fs::write(&config, "colorSpace=SRGB\nenableShaders=false\n").unwrap();

        let entry = install_bundled(&mods, Activation::Select, HardwareTier::Low)
            .await
            .unwrap();
        let shader_dir = mods.dir(ContentKind::Shader);
        assert!(shader_dir.join(&entry.file_name).exists());
        assert_eq!(mods.load().await.unwrap().mods, vec![entry.clone()]);

        let iris = std::fs::read_to_string(&config).unwrap();
        assert_eq!(
            iris,
            format!(
                "colorSpace=SRGB\nenableShaders=true\nshaderPack={}\n",
                entry.file_name
            )
        );
        let options =
            std::fs::read_to_string(shader_dir.join(format!("{}.txt", entry.file_name))).unwrap();
        let low = profile_values(pack_text(PACK_PROPERTIES), "LOW");
        for (key, value) in &low {
            assert!(options.contains(&format!("{key}={value}\n")));
        }
        assert!(bundled_update(&[entry]).unwrap().is_none());
    }

    #[tokio::test]
    async fn update_replaces_old_version_and_keeps_player_choices() {
        let dir = tempfile::tempdir().unwrap();
        let mods = mods_dir(dir.path());
        let shader_dir = mods.dir(ContentKind::Shader);
        std::fs::create_dir_all(&shader_dir).unwrap();
        let old_name = "Orrery-Shaders-0.9.0.zip";
        std::fs::write(shader_dir.join(old_name), b"old").unwrap();
        std::fs::write(shader_dir.join(format!("{old_name}.txt")), "BLOOM=false\n").unwrap();
        let old = InstalledMod {
            project_id: BUNDLED_SHADER_ID.into(),
            version_id: "0.9.0".into(),
            title: SHADERPACK_TITLE.into(),
            version_number: "0.9.0".into(),
            file_name: old_name.into(),
            sha512: "old".into(),
            enabled: true,
            dependency: false,
            icon_url: None,
            kind: ContentKind::Shader,
            game_versions: Vec::new(),
            loaders: Vec::new(),
            pack: None,
            pinned: false,
        };
        assert!(
            bundled_update(std::slice::from_ref(&old))
                .unwrap()
                .is_some()
        );
        mods.save(&ModsFile {
            schema: 1,
            mods: vec![old],
        })
        .await
        .unwrap();
        let config = dir.path().join("minecraft").join(IRIS_CONFIG);
        std::fs::create_dir_all(config.parent().unwrap()).unwrap();
        std::fs::write(
            &config,
            format!("shaderPack={old_name}\nenableShaders=false\n"),
        )
        .unwrap();

        let entry = install_bundled(&mods, Activation::KeepSelection, HardwareTier::High)
            .await
            .unwrap();
        assert!(!shader_dir.join(old_name).exists());
        assert_eq!(
            std::fs::read_to_string(shader_dir.join(format!("{}.txt", entry.file_name))).unwrap(),
            "BLOOM=false\n"
        );
        assert_eq!(
            std::fs::read_to_string(&config).unwrap(),
            format!("shaderPack={}\nenableShaders=false\n", entry.file_name)
        );

        std::fs::write(&config, "shaderPack=Another.zip\n").unwrap();
        install_bundled(&mods, Activation::KeepSelection, HardwareTier::High)
            .await
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(&config).unwrap(),
            "shaderPack=Another.zip\n"
        );
        assert_eq!(mods.load().await.unwrap().mods.len(), 1);
    }
}

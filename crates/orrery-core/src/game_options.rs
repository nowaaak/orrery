use std::io::{ErrorKind, Read};
use std::num::NonZero;
use std::path::{Path, PathBuf};

use tokio::io::AsyncWriteExt;

use crate::tuning::{self, VersionTier};
use crate::{CoreError, CoreResult};

pub const OPTIONS_FILE: &str = "options.txt";
const MAX_VERSION_JSON_BYTES: u64 = 64 * 1024;
const LEGACY_MOUSE_BASE: i32 = -100;
const LEGACY_KEY_CODES: &[(&str, i32)] = &[
    ("escape", 1),
    ("1", 2),
    ("2", 3),
    ("3", 4),
    ("4", 5),
    ("5", 6),
    ("6", 7),
    ("7", 8),
    ("8", 9),
    ("9", 10),
    ("0", 11),
    ("minus", 12),
    ("equal", 13),
    ("backspace", 14),
    ("tab", 15),
    ("q", 16),
    ("w", 17),
    ("e", 18),
    ("r", 19),
    ("t", 20),
    ("y", 21),
    ("u", 22),
    ("i", 23),
    ("o", 24),
    ("p", 25),
    ("left.bracket", 26),
    ("right.bracket", 27),
    ("enter", 28),
    ("left.control", 29),
    ("a", 30),
    ("s", 31),
    ("d", 32),
    ("f", 33),
    ("g", 34),
    ("h", 35),
    ("j", 36),
    ("k", 37),
    ("l", 38),
    ("semicolon", 39),
    ("apostrophe", 40),
    ("grave.accent", 41),
    ("left.shift", 42),
    ("backslash", 43),
    ("z", 44),
    ("x", 45),
    ("c", 46),
    ("v", 47),
    ("b", 48),
    ("n", 49),
    ("m", 50),
    ("comma", 51),
    ("period", 52),
    ("slash", 53),
    ("right.shift", 54),
    ("keypad.multiply", 55),
    ("left.alt", 56),
    ("space", 57),
    ("caps.lock", 58),
    ("f1", 59),
    ("f2", 60),
    ("f3", 61),
    ("f4", 62),
    ("f5", 63),
    ("f6", 64),
    ("f7", 65),
    ("f8", 66),
    ("f9", 67),
    ("f10", 68),
    ("num.lock", 69),
    ("scroll.lock", 70),
    ("keypad.7", 71),
    ("keypad.8", 72),
    ("keypad.9", 73),
    ("keypad.subtract", 74),
    ("keypad.4", 75),
    ("keypad.5", 76),
    ("keypad.6", 77),
    ("keypad.add", 78),
    ("keypad.1", 79),
    ("keypad.2", 80),
    ("keypad.3", 81),
    ("keypad.0", 82),
    ("keypad.decimal", 83),
    ("f11", 87),
    ("f12", 88),
    ("f13", 100),
    ("f14", 101),
    ("f15", 102),
    ("f16", 103),
    ("f17", 104),
    ("f18", 105),
    ("f19", 113),
    ("keypad.equal", 141),
    ("keypad.enter", 156),
    ("right.control", 157),
    ("keypad.divide", 181),
    ("print.screen", 183),
    ("right.alt", 184),
    ("pause", 197),
    ("home", 199),
    ("up", 200),
    ("page.up", 201),
    ("left", 203),
    ("right", 205),
    ("end", 207),
    ("down", 208),
    ("page.down", 209),
    ("insert", 210),
    ("delete", 211),
    ("left.win", 219),
    ("right.win", 220),
    ("menu", 221),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareTier {
    Low,
    Medium,
    High,
}

pub fn hardware_tier(memory_mb: Option<u32>, threads: Option<usize>) -> HardwareTier {
    let memory_mb = memory_mb.unwrap_or(0);
    let threads = threads.unwrap_or(1);
    if memory_mb < 8192 || threads <= 4 {
        HardwareTier::Low
    } else if memory_mb < 16384 || threads <= 8 {
        HardwareTier::Medium
    } else {
        HardwareTier::High
    }
}

pub fn detect_hardware_tier() -> HardwareTier {
    hardware_tier(
        tuning::system_memory_mb(),
        std::thread::available_parallelism().ok().map(NonZero::get),
    )
}

pub fn default_options(
    minecraft_version: &str,
    data_version: Option<u32>,
    tier: HardwareTier,
) -> Option<String> {
    let version_tier = tuning::version_tier(minecraft_version);
    let mut lines = vec![("graphicsPreset", "\"custom\"")];
    match tier {
        HardwareTier::High => return None,
        HardwareTier::Medium => {
            lines.push(("renderDistance", "10"));
            lines.push(("simulationDistance", "8"));
            lines.push(("biomeBlendRadius", "1"));
        }
        HardwareTier::Low => {
            lines.push(("renderDistance", "8"));
            lines.push(("simulationDistance", "6"));
            lines.push(("biomeBlendRadius", "0"));
            lines.push(("particles", "1"));
            lines.push(("entityShadows", "false"));
        }
    }
    lines.retain(|(key, _)| match *key {
        "graphicsPreset" | "simulationDistance" => version_tier == VersionTier::Current,
        "biomeBlendRadius" => version_tier != VersionTier::Legacy,
        _ => true,
    });

    let mut text = String::new();
    if let Some(data_version) = data_version {
        text.push_str(&format!("version:{data_version}\n"));
    }
    for (key, value) in lines {
        text.push_str(&format!("{key}:{value}\n"));
    }
    Some(text)
}

pub fn transfer_options(text: &str, from_version: &str, to_version: &str) -> String {
    let to_legacy_keys = tuning::version_tier(from_version) != VersionTier::Legacy
        && tuning::version_tier(to_version) == VersionTier::Legacy;
    let region_upper_case = tuning::release_minor(to_version).is_some_and(|minor| minor < 11);
    let mut out = String::new();
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else {
            out.push_str(line);
            out.push('\n');
            continue;
        };
        let value = match key {
            "resourcePacks" | "incompatibleResourcePacks" => continue,
            "version" if to_legacy_keys => continue,
            "lang" => language_code(value, region_upper_case),
            key if to_legacy_keys && key.starts_with("key_") => match legacy_key_code(value) {
                Some(code) => code.to_string(),
                None => continue,
            },
            _ => value.to_string(),
        };
        out.push_str(key);
        out.push(':');
        out.push_str(&value);
        out.push('\n');
    }
    out
}

fn language_code(code: &str, region_upper_case: bool) -> String {
    match code.split_once('_') {
        Some((language, region)) if region_upper_case => format!(
            "{}_{}",
            language.to_ascii_lowercase(),
            region.to_ascii_uppercase()
        ),
        _ => code.to_ascii_lowercase(),
    }
}

fn legacy_key_code(value: &str) -> Option<i32> {
    let name = value.split(':').next()?;
    if let Some(button) = name.strip_prefix("key.mouse.") {
        let index = match button {
            "left" => 0,
            "right" => 1,
            "middle" => 2,
            number => match number.parse::<i32>().ok()? {
                number @ 4..=8 => number - 1,
                _ => return None,
            },
        };
        return Some(LEGACY_MOUSE_BASE + index);
    }
    let key = name.strip_prefix("key.keyboard.")?;
    if key == "unknown" {
        return Some(0);
    }
    LEGACY_KEY_CODES
        .iter()
        .find(|(known, _)| *known == key)
        .map(|(_, code)| *code)
}

pub fn read_version_json(client_jar: &Path) -> Option<serde_json::Value> {
    let file = std::fs::File::open(client_jar).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;
    let entry = archive.by_name("version.json").ok()?;
    let mut text = String::new();
    entry
        .take(MAX_VERSION_JSON_BYTES)
        .read_to_string(&mut text)
        .ok()?;
    serde_json::from_str(&text).ok()
}

pub fn data_version(version_json: &serde_json::Value) -> Option<u32> {
    u32::try_from(version_json.get("world_version")?.as_u64()?).ok()
}

pub fn read_data_version(client_jar: &Path) -> Option<u32> {
    data_version(&read_version_json(client_jar)?)
}

pub async fn write_defaults_if_missing(
    game_dir: &Path,
    minecraft_version: &str,
    client_jar: PathBuf,
) -> CoreResult<bool> {
    let path = game_dir.join(OPTIONS_FILE);
    if tokio::fs::try_exists(&path).await.unwrap_or(true) {
        return Ok(false);
    }
    let (data_version, tier) = tokio::task::spawn_blocking(move || {
        (read_data_version(&client_jar), detect_hardware_tier())
    })
    .await
    .map_err(|err| CoreError::Unsupported(format!("hardware detection panicked: {err}")))?;
    let Some(text) = default_options(minecraft_version, data_version, tier) else {
        return Ok(false);
    };
    let io_error = |source| CoreError::Io {
        path: path.clone(),
        source,
    };
    let mut file = match tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .await
    {
        Ok(file) => file,
        Err(err) if err.kind() == ErrorKind::AlreadyExists => return Ok(false),
        Err(err) => return Err(io_error(err)),
    };
    file.write_all(text.as_bytes()).await.map_err(io_error)?;
    file.flush().await.map_err(io_error)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiers_follow_memory_and_threads() {
        assert_eq!(hardware_tier(Some(4096), Some(16)), HardwareTier::Low);
        assert_eq!(hardware_tier(Some(32768), Some(4)), HardwareTier::Low);
        assert_eq!(hardware_tier(Some(16384), Some(8)), HardwareTier::Medium);
        assert_eq!(hardware_tier(Some(32768), Some(16)), HardwareTier::High);
        assert_eq!(hardware_tier(None, None), HardwareTier::Low);
    }

    #[test]
    fn options_only_contain_keys_the_version_knows() {
        let legacy = default_options("1.8.9", None, HardwareTier::Low).unwrap();
        assert_eq!(
            legacy,
            "renderDistance:8\nparticles:1\nentityShadows:false\n"
        );
        let modern = default_options("1.16.5", Some(2586), HardwareTier::Medium).unwrap();
        assert_eq!(
            modern,
            "version:2586\nrenderDistance:10\nbiomeBlendRadius:1\n"
        );
        let current = default_options("26.3", Some(5023), HardwareTier::Medium).unwrap();
        assert!(current.contains(
            "graphicsPreset:\"custom\"
renderDistance:10
simulationDistance:8
"
        ));
        assert!(default_options("1.20.1", Some(3465), HardwareTier::High).is_none());
    }

    #[test]
    fn downgrade_to_legacy_converts_keys_and_language() {
        let modern = "version:3953\nlang:de_de\nnarrator:0\nresourcePacks:[\"vanilla\",\"file/Orrery Menu\"]\nincompatibleResourcePacks:[]\nkey_key.attack:key.mouse.left\nkey_key.use:key.mouse.right\nkey_key.pickItem:key.mouse.middle\nkey_key.togglePerspective:key.mouse.5\nkey_key.jump:key.keyboard.space\nkey_key.sneak:key.keyboard.left.shift\nkey_key.drop:key.keyboard.q\nkey_key.chat:key.keyboard.t\nkey_key.screenshot:key.keyboard.f2\nkey_key.playerlist:key.keyboard.unknown\nkey_key.socialInteractions:key.keyboard.f25\n";
        assert_eq!(
            transfer_options(modern, "1.21", "1.8.9"),
            "lang:de_DE\nnarrator:0\nkey_key.attack:-100\nkey_key.use:-99\nkey_key.pickItem:-98\nkey_key.togglePerspective:-96\nkey_key.jump:57\nkey_key.sneak:42\nkey_key.drop:16\nkey_key.chat:20\nkey_key.screenshot:60\nkey_key.playerlist:0\n"
        );
    }

    #[test]
    fn upgrade_keeps_values_for_the_game_data_fixer() {
        let legacy =
            "lang:de_DE\nkey_key.jump:57\nkey_key.attack:-100\nresourcePacks:[\"Faithful\"]\n";
        assert_eq!(
            transfer_options(legacy, "1.8.9", "1.21"),
            "lang:de_de\nkey_key.jump:57\nkey_key.attack:-100\n"
        );
    }

    #[test]
    fn transfer_between_modern_versions_keeps_keys() {
        let current = "version:3465\nlang:en_us\nkey_key.jump:key.keyboard.space\n";
        assert_eq!(transfer_options(current, "1.20.1", "1.16.5"), current);
        assert_eq!(
            transfer_options("lang:de_de\n", "1.16.5", "1.12.2"),
            "lang:de_de\n"
        );
    }

    #[tokio::test]
    async fn existing_options_are_never_touched() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(OPTIONS_FILE);
        std::fs::write(&path, "renderDistance:32\n").unwrap();
        let written = write_defaults_if_missing(dir.path(), "1.20.1", dir.path().join("x.jar"))
            .await
            .unwrap();
        assert!(!written);
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "renderDistance:32\n"
        );
    }
}

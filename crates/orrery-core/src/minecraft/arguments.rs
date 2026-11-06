use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::rules::{Features, RuleContext, rules_allow};
use super::version::{Argument, VersionJson};
use crate::platform::{Arch, Os, Platform};
use crate::servers::ServerAddress;
use crate::{CoreError, CoreResult};

#[derive(Debug, Clone)]
pub struct LaunchCredentials {
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    pub user_type: String,
    pub xuid: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

pub struct LaunchSpec<'a> {
    pub version: &'a VersionJson,
    pub platform: Platform,
    pub classpath: &'a [PathBuf],
    pub natives_dir: &'a Path,
    pub game_dir: &'a Path,
    pub assets_dir: &'a Path,
    pub libraries_dir: &'a Path,
    pub credentials: &'a LaunchCredentials,
    pub min_memory_mb: u32,
    pub max_memory_mb: u32,
    pub extra_jvm_args: &'a [String],
    pub resolution: Option<Resolution>,
    pub logging_argument: Option<String>,
    pub launcher_name: &'a str,
    pub launcher_version: &'a str,
    pub quick_play_world: Option<&'a str>,
    pub server: Option<&'a ServerAddress>,
    pub extra_game_args: &'a [String],
}

pub fn build_arguments(spec: &LaunchSpec<'_>) -> CoreResult<Vec<String>> {
    let main_class = spec.version.main_class.as_deref().ok_or_else(|| {
        CoreError::InvalidInput(format!("version {} has no main class", spec.version.id))
    })?;
    let ctx = RuleContext {
        platform: spec.platform,
        features: Features {
            has_custom_resolution: spec.resolution.is_some(),
            is_quick_play_singleplayer: spec.quick_play_world.is_some(),
            is_quick_play_multiplayer: spec.server.is_some(),
        },
    };
    let values = placeholder_values(spec);

    let mut args = vec![
        format!("-Xms{}M", spec.min_memory_mb),
        format!("-Xmx{}M", spec.max_memory_mb),
    ];
    args.extend(spec.extra_jvm_args.iter().cloned());

    match spec
        .version
        .arguments
        .as_ref()
        .filter(|a| !a.jvm.is_empty())
    {
        Some(arguments) => args.extend(expand(&arguments.jvm, &ctx, &values)),
        None => args.extend(
            legacy_jvm_arguments(spec.platform)
                .iter()
                .map(|arg| substitute(arg, &values)),
        ),
    }
    if let Some(logging) = &spec.logging_argument {
        args.push(logging.clone());
    }
    args.push(main_class.to_string());

    let mut has_resolution_args = false;
    if let Some(arguments) = spec
        .version
        .arguments
        .as_ref()
        .filter(|a| !a.game.is_empty())
    {
        let game = expand(&arguments.game, &ctx, &values);
        has_resolution_args = game.iter().any(|arg| arg == "--width");
        args.extend(game);
    } else if let Some(legacy) = &spec.version.minecraft_arguments {
        args.extend(
            legacy
                .split_whitespace()
                .map(|arg| substitute(arg, &values)),
        );
    }
    if let (Some(resolution), false) = (spec.resolution, has_resolution_args) {
        args.extend([
            "--width".to_string(),
            resolution.width.to_string(),
            "--height".to_string(),
            resolution.height.to_string(),
        ]);
    }
    if let Some(server) = spec.server
        && !args.iter().any(|arg| arg == "--quickPlayMultiplayer")
    {
        args.extend([
            "--server".to_string(),
            server.host.clone(),
            "--port".to_string(),
            server.port.to_string(),
        ]);
    }
    args.extend(spec.extra_game_args.iter().cloned());
    Ok(args)
}

fn legacy_jvm_arguments(platform: Platform) -> Vec<&'static str> {
    let mut args = Vec::new();
    match platform.os {
        Os::MacOs => args.push("-XstartOnFirstThread"),
        Os::Windows => args.push(
            "-XX:HeapDumpPath=MojangTricksIntelDriversForPerformance_javaw.exe_minecraft.exe.heapdump",
        ),
        Os::Linux => {}
    }
    if platform.arch == Arch::X86 {
        args.push("-Xss1M");
    }
    args.extend([
        "-Djava.library.path=${natives_directory}",
        "-Dminecraft.launcher.brand=${launcher_name}",
        "-Dminecraft.launcher.version=${launcher_version}",
        "-cp",
        "${classpath}",
    ]);
    args
}

fn expand(
    arguments: &[Argument],
    ctx: &RuleContext,
    values: &HashMap<&str, String>,
) -> Vec<String> {
    let mut out = Vec::new();
    for argument in arguments {
        match argument {
            Argument::Plain(value) => out.push(substitute(value, values)),
            Argument::Conditional { rules, value } => {
                if rules_allow(Some(rules), ctx) {
                    out.extend(value.values().into_iter().map(|v| substitute(v, values)));
                }
            }
        }
    }
    out
}

fn substitute(template: &str, values: &HashMap<&str, String>) -> String {
    let mut result = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("${") {
        result.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find('}') {
            Some(end) => {
                let key = &after[..end];
                match values.get(key) {
                    Some(value) => result.push_str(value),
                    None => {
                        tracing::warn!(placeholder = key, "unknown launch argument placeholder");
                        result.push_str(&rest[start..start + 2 + end + 1]);
                    }
                }
                rest = &after[end + 1..];
            }
            None => {
                result.push_str(&rest[start..]);
                rest = "";
            }
        }
    }
    result.push_str(rest);
    result
}

fn placeholder_values<'a>(spec: &LaunchSpec<'_>) -> HashMap<&'a str, String> {
    let credentials = spec.credentials;
    let classpath: Vec<String> = spec
        .classpath
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    let assets_index = spec
        .version
        .asset_index
        .as_ref()
        .map(|index| index.id.clone())
        .or_else(|| spec.version.assets.clone())
        .unwrap_or_default();
    let resolution = spec.resolution.unwrap_or(Resolution {
        width: 854,
        height: 480,
    });
    let path = |p: &Path| p.to_string_lossy().into_owned();

    HashMap::from([
        ("auth_player_name", credentials.username.clone()),
        (
            "quickPlaySingleplayer",
            spec.quick_play_world.unwrap_or_default().to_string(),
        ),
        (
            "quickPlayMultiplayer",
            spec.server.map(ToString::to_string).unwrap_or_default(),
        ),
        ("auth_uuid", credentials.uuid.clone()),
        ("auth_access_token", credentials.access_token.clone()),
        (
            "auth_session",
            format!("token:{}:{}", credentials.access_token, credentials.uuid),
        ),
        (
            "auth_xuid",
            credentials.xuid.clone().unwrap_or_else(|| "0".into()),
        ),
        ("clientid", String::new()),
        ("user_type", credentials.user_type.clone()),
        ("user_properties", "{}".into()),
        ("version_name", spec.version.id.clone()),
        (
            "version_type",
            spec.version
                .kind
                .clone()
                .unwrap_or_else(|| "release".into()),
        ),
        ("game_directory", path(spec.game_dir)),
        ("assets_root", path(spec.assets_dir)),
        ("game_assets", path(spec.assets_dir)),
        ("assets_index_name", assets_index),
        ("natives_directory", path(spec.natives_dir)),
        ("library_directory", path(spec.libraries_dir)),
        ("launcher_name", spec.launcher_name.to_string()),
        ("launcher_version", spec.launcher_version.to_string()),
        (
            "classpath",
            classpath.join(spec.platform.classpath_separator()),
        ),
        (
            "classpath_separator",
            spec.platform.classpath_separator().to_string(),
        ),
        ("resolution_width", resolution.width.to_string()),
        ("resolution_height", resolution.height.to_string()),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn credentials() -> LaunchCredentials {
        LaunchCredentials {
            username: "Steve".into(),
            uuid: "uuid-1".into(),
            access_token: "secret-token".into(),
            user_type: "msa".into(),
            xuid: None,
        }
    }

    fn spec<'a>(
        version: &'a VersionJson,
        platform: Platform,
        classpath: &'a [PathBuf],
        credentials: &'a LaunchCredentials,
        resolution: Option<Resolution>,
    ) -> LaunchSpec<'a> {
        LaunchSpec {
            version,
            platform,
            classpath,
            natives_dir: Path::new("natives"),
            game_dir: Path::new("game"),
            assets_dir: Path::new("assets"),
            libraries_dir: Path::new("libraries"),
            credentials,
            min_memory_mb: 512,
            max_memory_mb: 2048,
            extra_jvm_args: &[],
            resolution,
            logging_argument: Some("-Dlog4j.configurationFile=log.xml".into()),
            launcher_name: "orrery",
            launcher_version: "0.1.0",
            quick_play_world: None,
            server: None,
            extra_game_args: &[],
        }
    }

    fn legacy_version() -> VersionJson {
        serde_json::from_value(serde_json::json!({
            "id": "1.8.9",
            "type": "release",
            "mainClass": "net.minecraft.client.main.Main",
            "assets": "1.8",
            "minecraftArguments": "--username ${auth_player_name} --version ${version_name} --gameDir ${game_directory} --assetsDir ${assets_root} --assetIndex ${assets_index_name} --uuid ${auth_uuid} --accessToken ${auth_access_token} --userProperties ${user_properties} --userType ${user_type}"
        }))
        .unwrap()
    }

    fn modern_version() -> VersionJson {
        serde_json::from_value(serde_json::json!({
            "id": "1.20.1",
            "type": "release",
            "mainClass": "net.minecraft.client.main.Main",
            "assetIndex": { "id": "5", "sha1": "x", "size": 1, "url": "u" },
            "arguments": {
                "game": [
                    "--username", "${auth_player_name}",
                    "--assetIndex", "${assets_index_name}",
                    { "rules": [{ "action": "allow", "features": { "is_demo_user": true } }], "value": "--demo" },
                    { "rules": [{ "action": "allow", "features": { "has_custom_resolution": true } }], "value": ["--width", "${resolution_width}", "--height", "${resolution_height}"] },
                    { "rules": [{ "action": "allow", "features": { "is_quick_play_singleplayer": true } }], "value": ["--quickPlaySingleplayer", "${quickPlaySingleplayer}"] },
                    { "rules": [{ "action": "allow", "features": { "is_quick_play_multiplayer": true } }], "value": ["--quickPlayMultiplayer", "${quickPlayMultiplayer}"] }
                ],
                "jvm": [
                    { "rules": [{ "action": "allow", "os": { "name": "osx" } }], "value": ["-XstartOnFirstThread"] },
                    "-Djava.library.path=${natives_directory}",
                    "-cp", "${classpath}"
                ]
            }
        }))
        .unwrap()
    }

    #[test]
    fn legacy_arguments_on_windows() {
        let version = legacy_version();
        let classpath = vec![PathBuf::from("a.jar"), PathBuf::from("b.jar")];
        let creds = credentials();
        let platform = Platform {
            os: Os::Windows,
            arch: Arch::X86_64,
        };
        let args = build_arguments(&spec(&version, platform, &classpath, &creds, None)).unwrap();

        assert_eq!(args[0], "-Xms512M");
        assert_eq!(args[1], "-Xmx2048M");
        assert!(args.iter().any(|a| a.starts_with("-XX:HeapDumpPath=")));
        assert!(args.contains(&"-Djava.library.path=natives".to_string()));
        let cp = args.iter().position(|a| a == "-cp").unwrap();
        assert_eq!(args[cp + 1], "a.jar;b.jar");
        let main = args
            .iter()
            .position(|a| a == "net.minecraft.client.main.Main")
            .unwrap();
        assert_eq!(args[main - 1], "-Dlog4j.configurationFile=log.xml");
        assert_eq!(&args[main + 1..main + 3], ["--username", "Steve"]);
        assert!(args.contains(&"1.8".to_string()));
        assert!(!args.iter().any(|a| a.contains("${")));
    }

    #[test]
    fn modern_arguments_respect_rules_and_resolution() {
        let version = modern_version();
        let classpath = vec![PathBuf::from("a.jar"), PathBuf::from("b.jar")];
        let creds = credentials();
        let mac = Platform {
            os: Os::MacOs,
            arch: Arch::Aarch64,
        };
        let resolution = Some(Resolution {
            width: 1280,
            height: 720,
        });
        let args = build_arguments(&spec(&version, mac, &classpath, &creds, resolution)).unwrap();

        assert!(args.contains(&"-XstartOnFirstThread".to_string()));
        assert!(args.contains(&"a.jar:b.jar".to_string()));
        assert!(!args.contains(&"--demo".to_string()));
        let width = args.iter().position(|a| a == "--width").unwrap();
        assert_eq!(args[width + 1], "1280");
        assert_eq!(args.iter().filter(|a| *a == "--width").count(), 1);
        assert!(args.contains(&"5".to_string()));
    }

    #[test]
    fn quick_play_adds_the_world_only_when_requested() {
        let version = modern_version();
        let creds = credentials();
        let platform = Platform {
            os: Os::Windows,
            arch: Arch::X86_64,
        };
        let plain = build_arguments(&spec(&version, platform, &[], &creds, None)).unwrap();
        assert!(!plain.iter().any(|a| a.starts_with("--quickPlay")));

        let mut with_world = spec(&version, platform, &[], &creds, None);
        with_world.quick_play_world = Some("My World");
        let args = build_arguments(&with_world).unwrap();
        let flag = args
            .iter()
            .position(|a| a == "--quickPlaySingleplayer")
            .unwrap();
        assert_eq!(args[flag + 1], "My World");

        let legacy = legacy_version();
        let mut legacy_spec = spec(&legacy, platform, &[], &creds, None);
        legacy_spec.quick_play_world = Some("My World");
        let legacy_args = build_arguments(&legacy_spec).unwrap();
        assert!(!legacy_args.iter().any(|a| a == "--quickPlaySingleplayer"));
    }

    #[test]
    fn legacy_versions_get_resolution_appended() {
        let version = legacy_version();
        let creds = credentials();
        let platform = Platform {
            os: Os::Linux,
            arch: Arch::X86_64,
        };
        let resolution = Some(Resolution {
            width: 800,
            height: 600,
        });
        let args = build_arguments(&spec(&version, platform, &[], &creds, resolution)).unwrap();
        assert_eq!(
            &args[args.len() - 4..],
            ["--width", "800", "--height", "600"]
        );
    }

    #[test]
    fn server_join_uses_quick_play_or_legacy_flags() {
        let creds = credentials();
        let platform = Platform {
            os: Os::Windows,
            arch: Arch::X86_64,
        };
        let server = ServerAddress::parse("play.example.org:25570").unwrap();
        let extra = vec!["--fullscreen".to_string()];

        let modern = modern_version();
        let mut modern_spec = spec(&modern, platform, &[], &creds, None);
        modern_spec.server = Some(&server);
        modern_spec.extra_game_args = &extra;
        let args = build_arguments(&modern_spec).unwrap();
        let flag = args
            .iter()
            .position(|a| a == "--quickPlayMultiplayer")
            .unwrap();
        assert_eq!(args[flag + 1], "play.example.org:25570");
        assert!(!args.iter().any(|a| a == "--server"));
        assert_eq!(args.last().unwrap(), "--fullscreen");

        let legacy = legacy_version();
        let mut legacy_spec = spec(&legacy, platform, &[], &creds, None);
        legacy_spec.server = Some(&server);
        let args = build_arguments(&legacy_spec).unwrap();
        assert_eq!(
            &args[args.len() - 4..],
            ["--server", "play.example.org", "--port", "25570"]
        );
        assert!(!args.iter().any(|a| a.starts_with("--quickPlay")));

        let plain = build_arguments(&spec(&modern, platform, &[], &creds, None)).unwrap();
        assert!(!plain.iter().any(|a| a == "--quickPlayMultiplayer"));
    }

    #[test]
    fn unknown_placeholders_are_kept() {
        let values = HashMap::from([("a", "1".to_string())]);
        assert_eq!(substitute("${a}-${b}-${", &values), "1-${b}-${");
    }
}

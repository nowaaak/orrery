use super::model::LaunchSettings;
use crate::{CoreError, CoreResult};

const JVM_OPTION_VARS: [&str; 3] = ["JAVA_TOOL_OPTIONS", "_JAVA_OPTIONS", "JDK_JAVA_OPTIONS"];
const AUTH_MARKERS: [&str; 3] = ["authlib", "yggdrasil", "-dminecraft.api."];
const SESSION_GAME_ARGS: [&str; 10] = [
    "--username",
    "--uuid",
    "--accessToken",
    "--session",
    "--userType",
    "--xuid",
    "--clientId",
    "--userProperties",
    "--profileProperties",
    "--demo",
];

fn rejected(what: &str, value: &str) -> CoreError {
    CoreError::InvalidInput(format!(
        "{what} {value:?} would change how the game signs in and is not allowed"
    ))
}

fn touches_auth(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    AUTH_MARKERS.iter().any(|marker| lower.contains(marker))
}

pub fn check_jvm_args(args: &[String]) -> CoreResult<()> {
    match args.iter().find(|arg| touches_auth(arg)) {
        Some(arg) => Err(rejected("JVM argument", arg)),
        None => Ok(()),
    }
}

pub fn check_launch(launch: &LaunchSettings) -> CoreResult<()> {
    for arg in &launch.extra_game_args {
        let name = arg.split_once('=').map_or(arg.as_str(), |(name, _)| name);
        if SESSION_GAME_ARGS.contains(&name) {
            return Err(rejected("game argument", arg));
        }
    }
    for var in &launch.env {
        if JVM_OPTION_VARS.contains(&var.key.as_str()) && touches_auth(&var.value) {
            return Err(rejected("environment variable", &var.key));
        }
    }
    if let Some(wrapper) = &launch.wrapper
        && touches_auth(wrapper)
    {
        return Err(rejected("wrapper command", wrapper));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instances::EnvVar;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn auth_overrides_in_jvm_args_are_rejected() {
        for bad in [
            "-javaagent:authlib-injector-1.2.5.jar=https://example.org",
            "-javaagent:C:\\tools\\AuthLib.jar",
            "-Dminecraft.api.auth.host=https://example.org",
            "-Dminecraft.api.session.host=https://example.org",
            "-Dauthlibinjector.side=client",
            "-Dyggdrasil.server=https://example.org",
        ] {
            assert!(check_jvm_args(&args(&[bad])).is_err(), "{bad}");
        }
        assert!(check_jvm_args(&args(&["-XX:+UseG1GC", "-javaagent:profiler.jar"])).is_ok());
    }

    #[test]
    fn session_game_args_are_rejected() {
        for bad in [
            "--username",
            "--accessToken",
            "--demo",
            "--uuid=abc",
            "--session",
        ] {
            let launch = LaunchSettings {
                extra_game_args: args(&[bad]),
                ..LaunchSettings::default()
            };
            assert!(check_launch(&launch).is_err(), "{bad}");
        }
        let fine = LaunchSettings {
            extra_game_args: args(&["--width", "1280", "--fullscreen"]),
            ..LaunchSettings::default()
        };
        assert!(check_launch(&fine).is_ok());
    }

    #[test]
    fn auth_overrides_through_env_and_wrapper_are_rejected() {
        let env = |key: &str, value: &str| LaunchSettings {
            env: vec![EnvVar {
                key: key.into(),
                value: value.into(),
            }],
            ..LaunchSettings::default()
        };
        assert!(
            check_launch(&env("JAVA_TOOL_OPTIONS", "-javaagent:authlib-injector.jar")).is_err()
        );
        assert!(check_launch(&env("_JAVA_OPTIONS", "-Dminecraft.api.auth.host=x")).is_err());
        assert!(check_launch(&env("JAVA_TOOL_OPTIONS", "-Xss4m")).is_ok());
        assert!(check_launch(&env("OTHER", "authlib")).is_ok());
        let wrapper = LaunchSettings {
            wrapper: Some("java -javaagent:authlib-injector.jar".into()),
            ..LaunchSettings::default()
        };
        assert!(check_launch(&wrapper).is_err());
    }
}

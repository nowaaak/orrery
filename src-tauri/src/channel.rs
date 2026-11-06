use serde::Serialize;
use ts_rs::TS;

const DEV_SUFFIXES: [&str; 2] = [".dev", ".uitest"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Channel {
    Dev,
    Release,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AppInfo {
    pub version: String,
    pub channel: Channel,
}

pub fn current() -> Channel {
    if cfg!(debug_assertions) {
        Channel::Dev
    } else {
        Channel::Release
    }
}

pub fn check_identity(identifier: &str, channel: Channel) -> Result<(), String> {
    let dev_identity = DEV_SUFFIXES
        .iter()
        .any(|suffix| identifier.ends_with(suffix));
    if channel == Channel::Dev && !dev_identity {
        return Err(format!(
            "this development build uses the release identifier {identifier} and would change the data of the installed Orrery. Start it with `npm run app` instead of `npm run tauri dev`."
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev_builds_need_a_dev_identity() {
        assert!(check_identity("io.github.nowaaak.orrery", Channel::Dev).is_err());
        assert!(check_identity("io.github.nowaaak.orrery.dev", Channel::Dev).is_ok());
        assert!(check_identity("io.github.nowaaak.orrery.uitest", Channel::Dev).is_ok());
        assert!(check_identity("io.github.nowaaak.orrery", Channel::Release).is_ok());
    }
}

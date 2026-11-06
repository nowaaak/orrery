use serde::Deserialize;

use super::LoaderVersion;
use crate::CoreResult;
use crate::minecraft::manifest::validate_version_id;
use crate::net::Http;

const MAVEN: &str = "https://maven.neoforged.net/releases/";
const VERSIONS_URL: &str =
    "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge";

#[derive(Debug, Deserialize)]
struct MavenVersions {
    versions: Vec<String>,
}

fn numeric_parts(loader_version: &str) -> Option<Vec<u32>> {
    if loader_version.contains('+') {
        return None;
    }
    let core = loader_version.split('-').next()?;
    core.split('.').map(|part| part.parse().ok()).collect()
}

pub fn minecraft_version(loader_version: &str) -> Option<String> {
    match numeric_parts(loader_version)?.as_slice() {
        [major, minor, patch, _] if *major >= 26 => Some(match patch {
            0 => format!("{major}.{minor}"),
            _ => format!("{major}.{minor}.{patch}"),
        }),
        [major, minor, _] if (20..26).contains(major) => Some(match minor {
            0 => format!("1.{major}"),
            _ => format!("1.{major}.{minor}"),
        }),
        _ => None,
    }
}

pub fn installer_url(loader_version: &str) -> String {
    format!(
        "{MAVEN}net/neoforged/neoforge/{loader_version}/neoforge-{loader_version}-installer.jar"
    )
}

pub fn select_versions(all: Vec<String>, game_version: &str) -> Vec<LoaderVersion> {
    let mut matching: Vec<(Vec<u32>, String)> = all
        .into_iter()
        .filter(|version| minecraft_version(version).as_deref() == Some(game_version))
        .filter_map(|version| numeric_parts(&version).map(|parts| (parts, version)))
        .collect();
    matching.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| a.1.contains('-').cmp(&b.1.contains('-')))
    });
    matching
        .into_iter()
        .map(|(_, version)| LoaderVersion {
            stable: !version.contains('-'),
            version,
        })
        .collect()
}

pub async fn loader_versions(http: &Http, game_version: &str) -> CoreResult<Vec<LoaderVersion>> {
    validate_version_id(game_version)?;
    let list: MavenVersions = http.get_json(VERSIONS_URL).await?;
    Ok(select_versions(list.versions, game_version))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_loader_versions_to_minecraft_versions() {
        let cases = [
            ("20.2.93", Some("1.20.2")),
            ("20.4.0-beta", Some("1.20.4")),
            ("21.0.167", Some("1.21")),
            ("21.1.265", Some("1.21.1")),
            ("21.11.48", Some("1.21.11")),
            ("26.1.2.126", Some("26.1.2")),
            ("26.3.0.101", Some("26.3")),
            ("26.3.0.0-beta", Some("26.3")),
            ("26.1.0.0-alpha.1+snapshot-1", None),
            ("0.25w14craftmine.3-beta", None),
            ("47.1.0", None),
        ];
        for (loader, minecraft) in cases {
            assert_eq!(minecraft_version(loader).as_deref(), minecraft, "{loader}");
        }
    }

    #[test]
    fn selects_matching_versions_newest_first() {
        let all = ["21.1.9", "21.1.10", "21.1.1", "21.0.3", "21.1.11-beta"]
            .map(String::from)
            .to_vec();
        let selected = select_versions(all, "1.21.1");
        let names: Vec<&str> = selected.iter().map(|v| v.version.as_str()).collect();
        assert_eq!(names, ["21.1.11-beta", "21.1.10", "21.1.9", "21.1.1"]);
        assert!(!selected[0].stable);
        assert!(selected[1].stable);
    }
}

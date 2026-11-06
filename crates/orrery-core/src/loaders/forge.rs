use super::LoaderVersion;
use crate::minecraft::manifest::validate_version_id;
use crate::net::Http;
use crate::{CoreError, CoreResult};

const MAVEN: &str = "https://maven.minecraftforge.net/";
const METADATA_URL: &str =
    "https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml";
const LEGACY_GAME_VERSIONS: [&str; 2] = ["1.8.9", "1.12.2"];

fn numeric_parts(version: &str) -> Option<Vec<u32>> {
    version.split('.').map(|part| part.parse().ok()).collect()
}

pub fn supported(game_version: &str) -> bool {
    if LEGACY_GAME_VERSIONS.contains(&game_version) {
        return true;
    }
    match numeric_parts(game_version).as_deref() {
        Some([1, minor, ..]) => *minor >= 17,
        Some([major, ..]) => *major >= 26,
        _ => false,
    }
}

pub fn installer_url(maven_version: &str) -> String {
    format!("{MAVEN}net/minecraftforge/forge/{maven_version}/forge-{maven_version}-installer.jar")
}

pub fn parse_metadata(xml: &str) -> Vec<String> {
    xml.split("<version>")
        .skip(1)
        .filter_map(|rest| {
            rest.split_once("</version>")
                .map(|(v, _)| v.trim().to_string())
        })
        .collect()
}

fn loader_part<'a>(maven_version: &'a str, game_version: &str) -> Option<&'a str> {
    let rest = maven_version
        .strip_prefix(game_version)?
        .strip_prefix('-')?;
    let rest = rest
        .strip_suffix(game_version)
        .and_then(|r| r.strip_suffix('-'))
        .unwrap_or(rest);
    numeric_parts(rest).map(|_| rest)
}

pub fn select_versions(all: Vec<String>, game_version: &str) -> Vec<LoaderVersion> {
    if !supported(game_version) {
        return Vec::new();
    }
    let mut matching: Vec<(Vec<u32>, String)> = all
        .iter()
        .filter_map(|full| loader_part(full, game_version))
        .filter_map(|version| numeric_parts(version).map(|parts| (parts, version.to_string())))
        .collect();
    matching.sort_by(|a, b| b.0.cmp(&a.0));
    matching.dedup_by(|a, b| a.1 == b.1);
    matching
        .into_iter()
        .map(|(_, version)| LoaderVersion {
            stable: true,
            version,
        })
        .collect()
}

pub fn find_maven_version(
    all: &[String],
    game_version: &str,
    loader_version: &str,
) -> Option<String> {
    all.iter()
        .find(|full| loader_part(full, game_version) == Some(loader_version))
        .cloned()
}

async fn metadata(http: &Http) -> CoreResult<Vec<String>> {
    let bytes = http.get_bytes(METADATA_URL).await?;
    let xml = String::from_utf8(bytes)
        .map_err(|_| CoreError::InvalidInput("the Forge version list is not UTF-8".into()))?;
    Ok(parse_metadata(&xml))
}

pub async fn loader_versions(http: &Http, game_version: &str) -> CoreResult<Vec<LoaderVersion>> {
    validate_version_id(game_version)?;
    Ok(select_versions(metadata(http).await?, game_version))
}

pub async fn maven_version(
    http: &Http,
    game_version: &str,
    loader_version: &str,
) -> CoreResult<String> {
    if !LEGACY_GAME_VERSIONS.contains(&game_version) {
        return Ok(format!("{game_version}-{loader_version}"));
    }
    find_maven_version(&metadata(http).await?, game_version, loader_version).ok_or_else(|| {
        CoreError::NotFound(format!(
            "Forge {loader_version} for Minecraft {game_version}"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_versions_from_maven_metadata() {
        let xml = "<metadata><versioning><versions>\n<version>1.20.1-47.4.5</version>\n<version>1.8.9-11.15.1.2318-1.8.9</version></versions></versioning></metadata>";
        assert_eq!(
            parse_metadata(xml),
            ["1.20.1-47.4.5", "1.8.9-11.15.1.2318-1.8.9"]
        );
    }

    #[test]
    fn selects_numeric_versions_for_the_game_newest_first() {
        let all = [
            "1.20.1-47.4.5",
            "1.20.1-47.4.28",
            "1.20.1-47.10.0",
            "1.20.10-99.0.0",
            "1.20.1-47.4.0-pre",
            "1.21.1-52.1.2",
        ]
        .map(String::from)
        .to_vec();
        let names: Vec<String> = select_versions(all, "1.20.1")
            .into_iter()
            .map(|v| v.version)
            .collect();
        assert_eq!(names, ["47.10.0", "47.4.28", "47.4.5"]);
    }

    #[test]
    fn legacy_versions_drop_the_repeated_minecraft_suffix() {
        let all = [
            "1.8.9-11.15.1.2318-1.8.9",
            "1.8.9-11.15.0.1657",
            "1.12.2-14.23.5.2864",
            "1.12.2-14.23.0.2486-1.12.2",
        ]
        .map(String::from)
        .to_vec();
        let names: Vec<String> = select_versions(all.clone(), "1.8.9")
            .into_iter()
            .map(|v| v.version)
            .collect();
        assert_eq!(names, ["11.15.1.2318", "11.15.0.1657"]);
        assert_eq!(
            find_maven_version(&all, "1.8.9", "11.15.1.2318").as_deref(),
            Some("1.8.9-11.15.1.2318-1.8.9")
        );
        assert_eq!(
            find_maven_version(&all, "1.12.2", "14.23.5.2864").as_deref(),
            Some("1.12.2-14.23.5.2864")
        );
        assert_eq!(find_maven_version(&all, "1.8.9", "11.15.1.9999"), None);
    }

    #[test]
    fn offers_legacy_and_modern_installers_only() {
        assert!(supported("1.20.1"));
        assert!(supported("1.17"));
        assert!(supported("26.3"));
        assert!(supported("1.12.2"));
        assert!(supported("1.8.9"));
        assert!(!supported("1.16.5"));
        assert!(!supported("1.7.10"));
        assert!(!supported("25w14a"));
        assert!(select_versions(vec!["1.16.5-36.2.39".into()], "1.16.5").is_empty());
    }
}

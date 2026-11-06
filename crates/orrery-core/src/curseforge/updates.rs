use std::collections::HashMap;

use super::client::{CurseForgeClient, loader_types};
use super::models::{CfMod, FileIndex, parse_project_key};
use crate::CoreResult;
use crate::extras::ModUpdate;
use crate::instances::mods::{ContentKind, InstalledMod};
use crate::loaders::ModLoader;

const RELEASE: u8 = 1;

pub fn newest_index<'a>(
    indexes: &'a [FileIndex],
    game_version: &str,
    loader: Option<ModLoader>,
    kind: ContentKind,
) -> Option<&'a FileIndex> {
    let candidates: Vec<&FileIndex> = indexes
        .iter()
        .filter(|index| {
            index.game_version == game_version
                && match (kind, loader) {
                    (ContentKind::Mod, Some(loader)) => index
                        .mod_loader
                        .is_some_and(|value| loader_types(loader).contains(&value)),
                    (ContentKind::Mod, None) => false,
                    (ContentKind::ResourcePack | ContentKind::Shader, _) => true,
                }
        })
        .collect();
    candidates
        .iter()
        .filter(|index| index.release_type == RELEASE)
        .max_by_key(|index| index.file_id)
        .or_else(|| candidates.iter().max_by_key(|index| index.file_id))
        .copied()
}

pub async fn check(
    client: &CurseForgeClient,
    game_version: &str,
    loader: Option<ModLoader>,
    items: &[&InstalledMod],
) -> CoreResult<Vec<ModUpdate>> {
    let keyed: Vec<(u32, &InstalledMod)> = items
        .iter()
        .filter_map(|item| Some((parse_project_key(&item.project_id)?, *item)))
        .collect();
    if keyed.is_empty() {
        return Ok(Vec::new());
    }
    let ids: Vec<u32> = keyed.iter().map(|(id, _)| *id).collect();
    let projects: HashMap<u32, CfMod> = client
        .projects(&ids)
        .await?
        .into_iter()
        .map(|project| (project.id, project))
        .collect();
    let found: Vec<(&InstalledMod, u32)> = keyed
        .iter()
        .filter_map(|(mod_id, installed)| {
            let current: u32 = installed.version_id.parse().unwrap_or(0);
            let index = newest_index(
                &projects.get(mod_id)?.latest_files_indexes,
                game_version,
                loader,
                installed.kind,
            )?;
            (index.file_id > current).then_some((*installed, index.file_id))
        })
        .collect();
    if found.is_empty() {
        return Ok(Vec::new());
    }
    let file_ids: Vec<u32> = found.iter().map(|(_, id)| *id).collect();
    let names: HashMap<u32, String> = client
        .files(&file_ids)
        .await?
        .into_iter()
        .map(|file| (file.id, file.display_name))
        .collect();
    Ok(found
        .into_iter()
        .map(|(installed, file_id)| ModUpdate {
            project_id: installed.project_id.clone(),
            title: installed.title.clone(),
            current_version: installed.version_number.clone(),
            version_id: file_id.to_string(),
            version_number: names
                .get(&file_id)
                .cloned()
                .unwrap_or_else(|| file_id.to_string()),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::net::Http;

    fn index(game_version: &str, file_id: u32, release_type: u8, loader: Option<u8>) -> FileIndex {
        FileIndex {
            game_version: game_version.into(),
            file_id,
            release_type,
            mod_loader: loader,
        }
    }

    #[test]
    fn prefers_the_newest_release_for_the_loader() {
        let indexes = vec![
            index("1.20.1", 10, 1, Some(1)),
            index("1.20.1", 30, 2, Some(1)),
            index("1.20.1", 20, 1, Some(1)),
            index("1.20.1", 40, 1, Some(4)),
            index("1.19.2", 50, 1, Some(1)),
        ];
        let pick = |loader, kind| {
            newest_index(&indexes, "1.20.1", loader, kind).map(|index| index.file_id)
        };
        assert_eq!(pick(Some(ModLoader::Forge), ContentKind::Mod), Some(20));
        assert_eq!(pick(Some(ModLoader::Quilt), ContentKind::Mod), Some(40));
        assert_eq!(pick(Some(ModLoader::NeoForge), ContentKind::Mod), None);
        assert_eq!(pick(None, ContentKind::ResourcePack), Some(40));
    }

    #[test]
    fn falls_back_to_betas() {
        let indexes = vec![index("1.20.1", 30, 2, Some(1))];
        assert_eq!(
            newest_index(&indexes, "1.20.1", Some(ModLoader::Forge), ContentKind::Mod)
                .map(|index| index.file_id),
            Some(30)
        );
    }

    fn installed(project_id: &str, version_id: &str) -> InstalledMod {
        InstalledMod {
            project_id: project_id.into(),
            version_id: version_id.into(),
            title: "Mod".into(),
            version_number: "old".into(),
            file_name: "mod.jar".into(),
            sha512: "0".repeat(128),
            enabled: true,
            dependency: false,
            icon_url: None,
            kind: ContentKind::Mod,
            game_versions: Vec::new(),
            loaders: Vec::new(),
            pack: None,
            pinned: false,
        }
    }

    #[tokio::test]
    async fn reports_newer_files_only() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/mods"))
            .and(body_json(serde_json::json!({ "modIds": [5, 6] })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [
                    { "id": 5, "name": "A", "latestFilesIndexes": [
                        { "gameVersion": "1.20.1", "fileId": 60, "filename": "a.jar", "releaseType": 1, "modLoader": 1 }
                    ] },
                    { "id": 6, "name": "B", "latestFilesIndexes": [
                        { "gameVersion": "1.20.1", "fileId": 70, "filename": "b.jar", "releaseType": 1, "modLoader": 1 }
                    ] }
                ]
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/mods/files"))
            .and(body_json(serde_json::json!({ "fileIds": [60] })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [{ "id": 60, "modId": 5, "displayName": "A 2.0", "fileName": "a.jar", "releaseType": 1 }]
            })))
            .mount(&server)
            .await;
        let client = CurseForgeClient::with_base(
            Http::new("orrery-test").unwrap(),
            &server.uri(),
            Some("key".into()),
        );
        let a = installed("curseforge-5", "50");
        let b = installed("curseforge-6", "70");
        let other = installed("AANobbMI", "x");
        let updates = check(&client, "1.20.1", Some(ModLoader::Forge), &[&a, &b, &other])
            .await
            .unwrap();
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].project_id, "curseforge-5");
        assert_eq!(updates[0].version_id, "60");
        assert_eq!(updates[0].version_number, "A 2.0");
    }
}

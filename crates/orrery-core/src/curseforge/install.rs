use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

use super::client::{CurseForgeClient, is_allowed_download, loader_names};
use super::models::{CfFile, CfMod, project_key};
use crate::instances::mods::{ContentKind, InstalledMod, ModsDir, validate_content_file_name};
use crate::loaders::ModLoader;
use crate::modpacks::sha512_file;
use crate::modrinth::ModrinthClient;
use crate::net::{Checksum, Download, Http, VerifyMode, download_all};
use crate::progress::{Progress, Stage};
use crate::{CoreError, CoreResult};

const MAX_RESOLVED_PROJECTS: usize = 64;
const MODRINTH_CDN: &str = "https://cdn.modrinth.com/";
const DOWNLOAD_CONCURRENCY: usize = 4;

pub struct CfTarget<'a> {
    pub game_version: &'a str,
    pub loader: Option<ModLoader>,
    pub kind: ContentKind,
}

impl CfTarget<'_> {
    pub fn accepts(&self, file: &CfFile) -> bool {
        if !file.is_installable() || !file.game_versions.iter().any(|v| v == self.game_version) {
            return false;
        }
        match (self.kind, self.loader) {
            (ContentKind::Mod, Some(loader)) => file.game_versions.iter().any(|version| {
                loader_names(loader)
                    .iter()
                    .any(|name| name.eq_ignore_ascii_case(version))
            }),
            (ContentKind::Mod, None) => false,
            (ContentKind::ResourcePack | ContentKind::Shader, _) => true,
        }
    }

    pub(crate) fn recorded_loaders(&self, file: &CfFile) -> Vec<String> {
        file.game_versions
            .iter()
            .filter_map(|version| match version.to_ascii_lowercase().as_str() {
                "forge" => Some("forge"),
                "neoforge" => Some("neoforge"),
                "quilt" => Some("quilt"),
                "fabric" if self.loader == Some(ModLoader::LegacyFabric) => Some("legacy-fabric"),
                "fabric" => Some("fabric"),
                _ => None,
            })
            .map(str::to_string)
            .collect()
    }
}

pub(crate) fn minecraft_versions(file: &CfFile) -> Vec<String> {
    file.game_versions
        .iter()
        .filter(|version| {
            version.starts_with(|c: char| c.is_ascii_digit())
                && version.contains('.')
                && version
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-'))
        })
        .cloned()
        .collect()
}

pub fn pick_file(files: Vec<CfFile>, target: &CfTarget<'_>) -> Option<CfFile> {
    let mut files: Vec<CfFile> = files.into_iter().filter(|f| target.accepts(f)).collect();
    files.sort_by(|a, b| b.file_date.cmp(&a.file_date).then(b.id.cmp(&a.id)));
    match files.iter().position(CfFile::is_release) {
        Some(index) => Some(files.swap_remove(index)),
        None => files.into_iter().next(),
    }
}

fn normalized_title(title: &str) -> String {
    title
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

#[derive(Debug, Clone)]
pub struct PlannedFile {
    pub project: CfMod,
    pub file: CfFile,
    pub dependency: bool,
}

pub async fn resolve(
    client: &CurseForgeClient,
    target: &CfTarget<'_>,
    installed: &[InstalledMod],
    mod_id: u32,
    file_id: Option<u32>,
) -> CoreResult<Vec<PlannedFile>> {
    let installed_titles: HashSet<String> = installed
        .iter()
        .filter(|m| m.kind == target.kind)
        .map(|m| normalized_title(&m.title))
        .collect();
    let mut queue = VecDeque::from([(mod_id, file_id, false)]);
    let mut seen = HashSet::new();
    let mut planned = Vec::new();
    while let Some((mod_id, file_id, dependency)) = queue.pop_front() {
        if !seen.insert(mod_id) {
            continue;
        }
        if planned.len() >= MAX_RESOLVED_PROJECTS {
            return Err(CoreError::InvalidInput(
                "too many dependencies to resolve".into(),
            ));
        }
        let key = project_key(mod_id);
        if dependency && installed.iter().any(|m| m.project_id == key) {
            continue;
        }
        let project = client.project(mod_id).await?;
        if dependency && installed_titles.contains(&normalized_title(&project.name)) {
            continue;
        }
        let file = match file_id {
            Some(file_id) => client
                .files(&[file_id])
                .await?
                .into_iter()
                .find(|file| file.id == file_id && file.mod_id == mod_id && file.is_installable())
                .ok_or_else(|| CoreError::NotFound(format!("CurseForge file {file_id}")))?,
            None => {
                let files = client
                    .project_files(mod_id, Some(target.game_version))
                    .await?;
                pick_file(files, target).ok_or_else(|| {
                    CoreError::NotFound(format!(
                        "a file of {} for Minecraft {}",
                        project.name, target.game_version
                    ))
                })?
            }
        };
        if target.kind == ContentKind::Mod {
            for required in file.required_mods() {
                queue.push_back((required, None, true));
            }
        }
        planned.push(PlannedFile {
            project,
            file,
            dependency,
        });
    }
    Ok(planned)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSource {
    pub url: String,
    pub checksum: Checksum,
}

#[derive(Debug, Default)]
pub struct Sources {
    pub found: HashMap<u32, FileSource>,
    pub blocked: Vec<u32>,
}

pub async fn find_sources(modrinth: &ModrinthClient, files: &[CfFile]) -> CoreResult<Sources> {
    let mut sources = Sources::default();
    let mut missing: Vec<(u32, String)> = Vec::new();
    for file in files {
        match (file.download_url.as_deref(), file.sha1()) {
            (Some(url), Some(sha1)) if is_allowed_download(url) => {
                sources.found.insert(
                    file.id,
                    FileSource {
                        url: url.to_string(),
                        checksum: Checksum::Sha1(sha1),
                    },
                );
            }
            (_, Some(sha1)) => missing.push((file.id, sha1)),
            (_, None) => sources.blocked.push(file.id),
        }
    }
    if missing.is_empty() {
        return Ok(sources);
    }
    let hashes: Vec<String> = missing.iter().map(|(_, sha1)| sha1.clone()).collect();
    let versions = modrinth.versions_for_sha1(&hashes).await?;
    for (file_id, sha1) in missing {
        let source = versions.get(&sha1).and_then(|version| {
            version.files.iter().find_map(|file| {
                let same = file
                    .hashes
                    .get("sha1")
                    .is_some_and(|hash| hash.eq_ignore_ascii_case(&sha1));
                let sha512 = file.hashes.get("sha512").filter(|hash| {
                    hash.len() == 128 && hash.chars().all(|c| c.is_ascii_hexdigit())
                })?;
                (same && file.url.starts_with(MODRINTH_CDN)).then(|| FileSource {
                    url: file.url.clone(),
                    checksum: Checksum::Sha512(sha512.to_ascii_lowercase()),
                })
            })
        });
        match source {
            Some(source) => {
                sources.found.insert(file_id, source);
            }
            None => sources.blocked.push(file_id),
        }
    }
    Ok(sources)
}

pub(crate) async fn content_hash(path: PathBuf, checksum: &Checksum) -> CoreResult<String> {
    match checksum {
        Checksum::Sha512(hash) => Ok(hash.clone()),
        Checksum::Sha1(_) | Checksum::Sha256(_) => sha512_of(path).await,
    }
}

pub(crate) async fn sha512_of(path: PathBuf) -> CoreResult<String> {
    tokio::task::spawn_blocking(move || sha512_file(&path))
        .await
        .map_err(|err| CoreError::Unsupported(format!("content hash panicked: {err}")))?
}

pub struct Clients<'a> {
    pub http: &'a Http,
    pub curseforge: &'a CurseForgeClient,
    pub modrinth: &'a ModrinthClient,
}

impl Clients<'_> {
    pub(crate) async fn download(
        &self,
        downloads: Vec<Download>,
        concurrency: usize,
        progress: &Progress,
    ) -> CoreResult<()> {
        let (curseforge, other): (Vec<Download>, Vec<Download>) = downloads
            .into_iter()
            .partition(|download| is_allowed_download(&download.url));
        download_all(
            self.curseforge.downloads(),
            curseforge,
            concurrency,
            VerifyMode::Full,
            progress,
        )
        .await?;
        download_all(self.http, other, concurrency, VerifyMode::Full, progress).await
    }
}

pub async fn install(
    clients: &Clients<'_>,
    mods: &ModsDir,
    target: &CfTarget<'_>,
    mod_id: u32,
    file_id: Option<u32>,
    progress: &Progress,
) -> CoreResult<Vec<InstalledMod>> {
    progress.set_stage(Stage::FetchingMetadata);
    let index = mods.load().await?;
    let plan = resolve(clients.curseforge, target, &index.mods, mod_id, file_id).await?;
    progress.check_cancelled()?;
    for item in &plan {
        validate_content_file_name(&item.file.file_name, target.kind)?;
    }
    let files: Vec<CfFile> = plan.iter().map(|item| item.file.clone()).collect();
    let sources = find_sources(clients.modrinth, &files).await?;
    if let Some(blocked) = plan
        .iter()
        .find(|item| sources.blocked.contains(&item.file.id))
    {
        return Err(CoreError::Unsupported(format!(
            "{} can only be downloaded on the CurseForge website",
            blocked.project.name
        )));
    }

    let dir = mods.dir(target.kind);
    let downloads = plan
        .iter()
        .map(|item| {
            let source = &sources.found[&item.file.id];
            Download {
                url: source.url.clone(),
                path: dir.join(&item.file.file_name),
                checksum: Some(source.checksum.clone()),
                size: Some(item.file.file_length).filter(|size| *size > 0),
            }
        })
        .collect();
    progress.set_stage(Stage::DownloadingMods);
    clients
        .download(downloads, DOWNLOAD_CONCURRENCY, progress)
        .await?;

    let mut entries = Vec::with_capacity(plan.len());
    for item in &plan {
        let source = &sources.found[&item.file.id];
        let sha512 = content_hash(dir.join(&item.file.file_name), &source.checksum).await?;
        let key = project_key(item.project.id);
        let previous = index.mods.iter().find(|m| m.project_id == key);
        entries.push(InstalledMod {
            project_id: key.clone(),
            version_id: item.file.id.to_string(),
            title: item.project.name.clone(),
            version_number: item.file.display_name.clone(),
            file_name: item.file.file_name.clone(),
            sha512,
            enabled: true,
            dependency: item.dependency && previous.is_none_or(|p| p.dependency),
            icon_url: item.project.icon_url(),
            kind: target.kind,
            game_versions: minecraft_versions(&item.file),
            loaders: target.recorded_loaders(&item.file),
            pack: None,
            pinned: previous.is_some_and(|p| p.pinned),
        });
    }
    mods.commit(index, &entries).await?;
    progress.set_stage(Stage::Finished);
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    fn file(id: u32, mod_id: u32, versions: &[&str], date: &str, release_type: u8) -> CfFile {
        serde_json::from_value(serde_json::json!({
            "id": id,
            "modId": mod_id,
            "displayName": format!("File {id}"),
            "fileName": format!("file-{id}.jar"),
            "releaseType": release_type,
            "fileDate": date,
            "fileLength": 3,
            "downloadUrl": format!("https://edge.forgecdn.net/files/{id}/file-{id}.jar"),
            "hashes": [{ "value": format!("{:040x}", id), "algo": 1 }],
            "gameVersions": versions,
            "dependencies": []
        }))
        .unwrap()
    }

    fn forge_target() -> CfTarget<'static> {
        CfTarget {
            game_version: "1.20.1",
            loader: Some(ModLoader::Forge),
            kind: ContentKind::Mod,
        }
    }

    fn installed(project_id: &str, title: &str) -> InstalledMod {
        InstalledMod {
            project_id: project_id.into(),
            version_id: "v".into(),
            title: title.into(),
            version_number: "1".into(),
            file_name: "x.jar".into(),
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

    #[test]
    fn picks_the_newest_matching_release() {
        let files = vec![
            file(1, 5, &["1.20.1", "Forge"], "2024-01-01", 1),
            file(2, 5, &["1.20.1", "Forge"], "2024-03-01", 2),
            file(3, 5, &["1.20.1", "Fabric"], "2024-04-01", 1),
            file(4, 5, &["1.20.1", "Forge"], "2024-02-01", 1),
            file(5, 5, &["1.19.2", "Forge"], "2024-05-01", 1),
        ];
        assert_eq!(pick_file(files, &forge_target()).unwrap().id, 4);
    }

    #[test]
    fn falls_back_to_betas_and_respects_quilt() {
        let files = vec![file(2, 5, &["1.20.1", "Forge"], "2024-03-01", 2)];
        assert_eq!(pick_file(files, &forge_target()).unwrap().id, 2);
        let quilt = CfTarget {
            loader: Some(ModLoader::Quilt),
            ..forge_target()
        };
        let files = vec![file(3, 5, &["1.20.1", "Fabric"], "2024-04-01", 1)];
        assert_eq!(pick_file(files, &quilt).unwrap().id, 3);
    }

    #[test]
    fn resource_packs_ignore_loaders() {
        let target = CfTarget {
            game_version: "1.20.1",
            loader: None,
            kind: ContentKind::ResourcePack,
        };
        assert!(target.accepts(&file(1, 5, &["1.20.1"], "2024", 1)));
        let mut server_pack = file(2, 5, &["1.20.1"], "2024", 1);
        server_pack.is_server_pack = Some(true);
        assert!(!target.accepts(&server_pack));
    }

    #[test]
    fn records_versions_and_loaders() {
        let file = file(1, 5, &["1.20.1", "Forge", "Client", "Java 17"], "2024", 1);
        assert_eq!(minecraft_versions(&file), vec!["1.20.1"]);
        assert_eq!(forge_target().recorded_loaders(&file), vec!["forge"]);
        let legacy = CfTarget {
            game_version: "1.8.9",
            loader: Some(ModLoader::LegacyFabric),
            kind: ContentKind::Mod,
        };
        let fabric = self::file(2, 5, &["1.8.9", "Fabric"], "2024", 1);
        assert_eq!(legacy.recorded_loaders(&fabric), vec!["legacy-fabric"]);
    }

    fn project_json(id: u32, name: &str) -> serde_json::Value {
        serde_json::json!({ "id": id, "name": name, "slug": name.to_lowercase() })
    }

    fn file_json(id: u32, mod_id: u32, dependencies: serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "modId": mod_id,
            "displayName": format!("File {id}"),
            "fileName": format!("file-{id}.jar"),
            "releaseType": 1,
            "fileDate": "2024-01-01",
            "downloadUrl": null,
            "hashes": [],
            "gameVersions": ["1.20.1", "Forge"],
            "dependencies": dependencies
        })
    }

    async fn mock_project(server: &MockServer, id: u32, name: &str, files: serde_json::Value) {
        Mock::given(method("GET"))
            .and(path(format!("/v1/mods/{id}")))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "data": project_json(id, name) })),
            )
            .mount(server)
            .await;
        Mock::given(method("GET"))
            .and(path(format!("/v1/mods/{id}/files")))
            .and(query_param("gameVersion", "1.20.1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": files,
                "pagination": { "index": 0, "pageSize": 50, "resultCount": 1, "totalCount": 1 }
            })))
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn resolves_required_dependencies_once() {
        let server = MockServer::start().await;
        let deps = serde_json::json!([
            { "modId": 7, "relationType": 3 },
            { "modId": 8, "relationType": 2 },
            { "modId": 9, "relationType": 3 },
            { "modId": 10, "relationType": 3 }
        ]);
        mock_project(
            &server,
            5,
            "Main",
            serde_json::json!([file_json(50, 5, deps)]),
        )
        .await;
        mock_project(
            &server,
            7,
            "Library",
            serde_json::json!([file_json(
                70,
                7,
                serde_json::json!([{ "modId": 5, "relationType": 3 }])
            )]),
        )
        .await;
        mock_project(
            &server,
            9,
            "Just Enough Items",
            serde_json::json!([file_json(90, 9, serde_json::json!([]))]),
        )
        .await;
        let client = CurseForgeClient::with_base(
            Http::new("orrery-test").unwrap(),
            &server.uri(),
            Some("key".into()),
        );
        let installed = vec![
            installed("curseforge-10", "Already"),
            installed("u6VqBqgw", "Just Enough Items"),
        ];
        let plan = resolve(&client, &forge_target(), &installed, 5, None)
            .await
            .unwrap();
        let ids: Vec<(u32, bool)> = plan.iter().map(|p| (p.file.id, p.dependency)).collect();
        assert_eq!(ids, vec![(50, false), (70, true)]);
    }

    #[tokio::test]
    async fn blocked_files_are_looked_up_on_modrinth() {
        let server = MockServer::start().await;
        let sha1 = "a".repeat(40);
        let sha512 = "b".repeat(128);
        Mock::given(method("POST"))
            .and(path("/version_files"))
            .and(body_json(serde_json::json!({
                "hashes": [sha1.clone(), "c".repeat(40)],
                "algorithm": "sha1"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                sha1.clone(): {
                    "id": "ver", "project_id": "proj", "name": "v", "version_number": "1",
                    "version_type": "release", "date_published": "2024",
                    "files": [{
                        "hashes": { "sha1": sha1.clone(), "sha512": sha512.clone() },
                        "url": "https://cdn.modrinth.com/data/proj/versions/ver/a.jar",
                        "filename": "a.jar", "primary": true, "size": 3
                    }]
                }
            })))
            .mount(&server)
            .await;
        let modrinth = ModrinthClient::with_base(Http::new("orrery-test").unwrap(), &server.uri());
        let open = file(1, 5, &["1.20.1"], "2024", 1);
        let mut blocked = file(2, 5, &["1.20.1"], "2024", 1);
        blocked.download_url = None;
        blocked.hashes[0].value = sha1.clone();
        let mut unknown = file(3, 5, &["1.20.1"], "2024", 1);
        unknown.download_url = None;
        unknown.hashes[0].value = "c".repeat(40);
        let mut foreign = file(4, 5, &["1.20.1"], "2024", 1);
        foreign.download_url = Some("https://example.com/x.jar".into());
        foreign.hashes.clear();

        let sources = find_sources(&modrinth, &[open, blocked, unknown, foreign])
            .await
            .unwrap();
        assert_eq!(
            sources.found[&1].checksum,
            Checksum::Sha1(format!("{:040x}", 1))
        );
        assert_eq!(
            sources.found[&2],
            FileSource {
                url: "https://cdn.modrinth.com/data/proj/versions/ver/a.jar".into(),
                checksum: Checksum::Sha512(sha512),
            }
        );
        assert_eq!(sources.blocked, vec![4, 3]);
    }

    #[tokio::test]
    async fn open_files_need_no_modrinth_request() {
        let server = MockServer::start().await;
        let modrinth = ModrinthClient::with_base(Http::new("orrery-test").unwrap(), &server.uri());
        let sources = find_sources(&modrinth, &[file(1, 5, &["1.20.1"], "2024", 1)])
            .await
            .unwrap();
        assert_eq!(sources.found.len(), 1);
        assert!(server.received_requests().await.unwrap().is_empty());
    }
}

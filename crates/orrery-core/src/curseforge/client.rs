use std::collections::HashMap;

use reqwest::header::HeaderValue;
use serde::de::DeserializeOwned;

use super::models::{CfFile, CfMod, Data, FingerprintMatches, Page};
use crate::instances::mods::ContentKind;
use crate::loaders::ModLoader;
use crate::modrinth::models::{ModSearchHit, ModSearchPage};
use crate::net::{Http, ensure_success, http_error, with_retry};
use crate::{CoreError, CoreResult};

const DEFAULT_API: &str = "https://api.curseforge.com";
const KEY_HEADER: &str = "x-api-key";
const GAME_ID: u32 = 432;
pub const PAGE_SIZE: u32 = 20;
pub const MODPACK_CLASS: u32 = 4471;
const MAX_RESULTS: u32 = 10_000;
const FILES_PAGE_SIZE: u32 = 50;
const MAX_FILE_PAGES: u32 = 20;
const IDS_PER_REQUEST: usize = 100;
const SORT_POPULARITY: &str = "2";
const DOWNLOAD_HOSTS: [&str; 2] = ["edge.forgecdn.net", "mediafilez.forgecdn.net"];

#[derive(Clone)]
pub struct CurseForgeClient {
    http: Http,
    downloads: Http,
    base: String,
    key: Option<HeaderValue>,
}

pub fn class_id(kind: ContentKind) -> u32 {
    match kind {
        ContentKind::Mod => 6,
        ContentKind::ResourcePack => 12,
        ContentKind::Shader => 6552,
    }
}

pub fn loader_type(loader: ModLoader) -> u8 {
    match loader {
        ModLoader::Forge => 1,
        ModLoader::Fabric | ModLoader::LegacyFabric | ModLoader::Quilt => 4,
        ModLoader::NeoForge => 6,
    }
}

pub fn loader_types(loader: ModLoader) -> &'static [u8] {
    match loader {
        ModLoader::Forge => &[1],
        ModLoader::Fabric | ModLoader::LegacyFabric => &[4],
        ModLoader::Quilt => &[5, 4],
        ModLoader::NeoForge => &[6],
    }
}

pub fn loader_names(loader: ModLoader) -> &'static [&'static str] {
    match loader {
        ModLoader::Forge => &["Forge"],
        ModLoader::Fabric | ModLoader::LegacyFabric => &["Fabric"],
        ModLoader::Quilt => &["Quilt", "Fabric"],
        ModLoader::NeoForge => &["NeoForge"],
    }
}

pub fn is_allowed_download(url: &str) -> bool {
    url.strip_prefix("https://")
        .and_then(|rest| rest.split(['/', '?', '#']).next())
        .is_some_and(|host| DOWNLOAD_HOSTS.contains(&host))
}

fn sensitive_key(key: Option<String>) -> Option<HeaderValue> {
    let key = key?;
    let key = key.trim();
    if key.is_empty() {
        return None;
    }
    match HeaderValue::from_str(key) {
        Ok(mut value) => {
            value.set_sensitive(true);
            Some(value)
        }
        Err(_) => {
            tracing::warn!("ignoring a CurseForge API key with invalid characters");
            None
        }
    }
}

impl CurseForgeClient {
    pub fn new(http: Http, key: Option<String>) -> Self {
        Self::with_base(http, DEFAULT_API, key)
    }

    pub fn with_base(http: Http, base: &str, key: Option<String>) -> Self {
        let key = sensitive_key(key);
        let downloads = match &key {
            Some(value) => http
                .with_header(KEY_HEADER, value.clone(), &DOWNLOAD_HOSTS)
                .unwrap_or_else(|err| {
                    tracing::warn!(error = %err, "CurseForge downloads go without the API key");
                    http.clone()
                }),
            None => http.clone(),
        };
        Self {
            http,
            downloads,
            base: base.trim_end_matches('/').to_string(),
            key,
        }
    }

    pub fn is_available(&self) -> bool {
        self.key.is_some()
    }

    pub fn downloads(&self) -> &Http {
        &self.downloads
    }

    fn key(&self) -> CoreResult<&HeaderValue> {
        self.key
            .as_ref()
            .ok_or_else(|| CoreError::Unsupported("this build has no CurseForge API key".into()))
    }

    async fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> CoreResult<T> {
        let key = self.key()?;
        let url = format!("{}{path}", self.base);
        with_retry(|| async {
            let response = self
                .http
                .client()
                .get(&url)
                .header(KEY_HEADER, key.clone())
                .query(query)
                .send()
                .await
                .map_err(|source| http_error(&url, source))?;
            ensure_success(&url, response)?
                .json::<T>()
                .await
                .map_err(|source| http_error(&url, source))
        })
        .await
        .map_err(not_found(path))
    }

    async fn post<T: DeserializeOwned>(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> CoreResult<T> {
        let key = self.key()?;
        let url = format!("{}{path}", self.base);
        with_retry(|| async {
            let response = self
                .http
                .client()
                .post(&url)
                .header(KEY_HEADER, key.clone())
                .json(body)
                .send()
                .await
                .map_err(|source| http_error(&url, source))?;
            ensure_success(&url, response)?
                .json::<T>()
                .await
                .map_err(|source| http_error(&url, source))
        })
        .await
    }

    pub async fn search(
        &self,
        class_id: u32,
        query: &str,
        game_version: Option<&str>,
        loader: Option<ModLoader>,
        offset: u32,
    ) -> CoreResult<ModSearchPage> {
        if offset.saturating_add(PAGE_SIZE) > MAX_RESULTS {
            return Ok(ModSearchPage {
                hits: Vec::new(),
                offset,
                total: MAX_RESULTS,
            });
        }
        let mut params = vec![
            ("gameId", GAME_ID.to_string()),
            ("classId", class_id.to_string()),
            ("searchFilter", query.trim().to_string()),
            ("sortField", SORT_POPULARITY.to_string()),
            ("sortOrder", "desc".to_string()),
            ("index", offset.to_string()),
            ("pageSize", PAGE_SIZE.to_string()),
        ];
        if let Some(game_version) = game_version {
            params.push(("gameVersion", game_version.to_string()));
            if let Some(loader) = loader {
                params.push(("modLoaderType", loader_type(loader).to_string()));
            }
        }
        let page: Page<CfMod> = self.get("/v1/mods/search", &params).await?;
        let total = page.pagination.total_count.min(u64::from(MAX_RESULTS)) as u32;
        Ok(ModSearchPage {
            hits: page.data.into_iter().map(ModSearchHit::from).collect(),
            offset: page.pagination.index,
            total,
        })
    }

    pub async fn project(&self, mod_id: u32) -> CoreResult<CfMod> {
        let response: Data<CfMod> = self.get(&format!("/v1/mods/{mod_id}"), &[]).await?;
        Ok(response.data)
    }

    pub async fn projects(&self, mod_ids: &[u32]) -> CoreResult<Vec<CfMod>> {
        let mut projects = Vec::new();
        for chunk in mod_ids.chunks(IDS_PER_REQUEST) {
            let body = serde_json::json!({ "modIds": chunk });
            let response: Data<Vec<CfMod>> = self.post("/v1/mods", &body).await?;
            projects.extend(response.data);
        }
        Ok(projects)
    }

    pub async fn project_files(
        &self,
        mod_id: u32,
        game_version: Option<&str>,
    ) -> CoreResult<Vec<CfFile>> {
        let path = format!("/v1/mods/{mod_id}/files");
        let mut files = Vec::new();
        for page in 0..MAX_FILE_PAGES {
            let mut params = vec![
                ("index", (page * FILES_PAGE_SIZE).to_string()),
                ("pageSize", FILES_PAGE_SIZE.to_string()),
            ];
            if let Some(game_version) = game_version {
                params.push(("gameVersion", game_version.to_string()));
            }
            let response: Page<CfFile> = self.get(&path, &params).await?;
            let received = response.pagination.result_count;
            files.extend(response.data);
            if received < FILES_PAGE_SIZE || files.len() as u64 >= response.pagination.total_count {
                break;
            }
        }
        Ok(files)
    }

    pub async fn files(&self, file_ids: &[u32]) -> CoreResult<Vec<CfFile>> {
        let mut files = Vec::new();
        for chunk in file_ids.chunks(IDS_PER_REQUEST) {
            let body = serde_json::json!({ "fileIds": chunk });
            let response: Data<Vec<CfFile>> = self.post("/v1/mods/files", &body).await?;
            files.extend(response.data);
        }
        Ok(files)
    }

    pub async fn files_for_fingerprints(
        &self,
        fingerprints: &[u32],
    ) -> CoreResult<HashMap<u32, CfFile>> {
        let mut found = HashMap::new();
        for chunk in fingerprints.chunks(IDS_PER_REQUEST) {
            let body = serde_json::json!({ "fingerprints": chunk });
            let response: Data<FingerprintMatches> = self.post("/v1/fingerprints", &body).await?;
            for matched in response.data.exact_matches {
                if let Ok(fingerprint) = u32::try_from(matched.file.file_fingerprint)
                    && matched.file.mod_id == matched.id
                {
                    found.insert(fingerprint, matched.file);
                }
            }
        }
        Ok(found)
    }
}

fn not_found(path: &str) -> impl Fn(CoreError) -> CoreError + '_ {
    move |err| match err {
        CoreError::HttpStatus { status: 404, .. } => {
            CoreError::NotFound(format!("CurseForge resource {path}"))
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use wiremock::matchers::{body_json, header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    const KEY: &str = "test-key";

    fn client(server: &MockServer, key: Option<&str>) -> CurseForgeClient {
        CurseForgeClient::with_base(
            Http::new("orrery-test").unwrap(),
            &server.uri(),
            key.map(str::to_string),
        )
    }

    fn file_json(id: u32, mod_id: u32, fingerprint: u64) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "modId": mod_id,
            "displayName": format!("File {id}"),
            "fileName": format!("file-{id}.jar"),
            "releaseType": 1,
            "fileDate": "2024-01-01T00:00:00Z",
            "fileLength": 3,
            "downloadUrl": format!("https://edge.forgecdn.net/files/{id}/file-{id}.jar"),
            "hashes": [
                { "value": "A9993E364706816ABA3E25717850C26C9CD0D89D", "algo": 1 },
                { "value": "900150983cd24fb0d6963f7d28e17f72", "algo": 2 }
            ],
            "gameVersions": ["1.20.1", "Forge"],
            "dependencies": [
                { "modId": 7, "relationType": 3 },
                { "modId": 8, "relationType": 2 }
            ],
            "fileFingerprint": fingerprint
        })
    }

    #[tokio::test]
    async fn without_a_key_nothing_is_requested() {
        let server = MockServer::start().await;
        let client = client(&server, None);
        assert!(!client.is_available());
        let err = client.project(1).await.unwrap_err();
        assert!(matches!(err, CoreError::Unsupported(_)));
        assert!(server.received_requests().await.unwrap().is_empty());
    }

    #[test]
    fn blank_keys_count_as_missing() {
        let http = Http::new("orrery-test").unwrap();
        assert!(!CurseForgeClient::new(http.clone(), Some("  ".into())).is_available());
        assert!(!CurseForgeClient::new(http, Some("bad\nkey".into())).is_available());
    }

    #[tokio::test]
    async fn search_sends_the_key_and_filters() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/mods/search"))
            .and(header(KEY_HEADER, KEY))
            .and(query_param("gameId", "432"))
            .and(query_param("classId", "6"))
            .and(query_param("searchFilter", "jei"))
            .and(query_param("gameVersion", "1.20.1"))
            .and(query_param("modLoaderType", "1"))
            .and(query_param("index", "20"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [{
                    "id": 238222,
                    "name": "Just Enough Items",
                    "slug": "jei",
                    "summary": "View items",
                    "downloadCount": 3.5e8,
                    "authors": [{ "name": "mezz" }],
                    "logo": { "thumbnailUrl": "https://media.forgecdn.net/x.png" }
                }],
                "pagination": { "index": 20, "pageSize": 20, "resultCount": 1, "totalCount": 25000 }
            })))
            .mount(&server)
            .await;
        let page = client(&server, Some(KEY))
            .search(
                class_id(ContentKind::Mod),
                " jei ",
                Some("1.20.1"),
                Some(ModLoader::Forge),
                20,
            )
            .await
            .unwrap();
        assert_eq!(page.total, MAX_RESULTS);
        assert_eq!(page.offset, 20);
        let hit = &page.hits[0];
        assert_eq!(hit.project_id, "curseforge-238222");
        assert_eq!(hit.author, "mezz");
        assert_eq!(hit.downloads, 350_000_000);
        assert_eq!(
            hit.icon_url.as_deref(),
            Some("https://media.forgecdn.net/x.png")
        );
    }

    #[tokio::test]
    async fn search_stops_at_the_result_limit() {
        let server = MockServer::start().await;
        let page = client(&server, Some(KEY))
            .search(6, "x", None, None, MAX_RESULTS)
            .await
            .unwrap();
        assert!(page.hits.is_empty());
        assert!(server.received_requests().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn reads_files_with_hashes_and_dependencies() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/mods/files"))
            .and(header(KEY_HEADER, KEY))
            .and(body_json(serde_json::json!({ "fileIds": [11] })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [file_json(11, 5, 99)]
            })))
            .mount(&server)
            .await;
        let files = client(&server, Some(KEY)).files(&[11]).await.unwrap();
        let file = &files[0];
        assert_eq!(
            file.sha1().as_deref(),
            Some("a9993e364706816aba3e25717850c26c9cd0d89d")
        );
        assert_eq!(file.required_mods().collect::<Vec<_>>(), vec![7]);
        assert_eq!(file.release_label(), "release");
        assert!(file.is_installable());
    }

    #[tokio::test]
    async fn pages_through_project_files() {
        let server = MockServer::start().await;
        let first: Vec<_> = (0..FILES_PAGE_SIZE)
            .map(|i| file_json(i + 1, 5, 0))
            .collect();
        Mock::given(method("GET"))
            .and(path("/v1/mods/5/files"))
            .and(query_param("index", "0"))
            .and(query_param("gameVersion", "1.20.1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": first,
                "pagination": { "index": 0, "pageSize": 50, "resultCount": 50, "totalCount": 51 }
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/mods/5/files"))
            .and(query_param("index", "50"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [file_json(99, 5, 0)],
                "pagination": { "index": 50, "pageSize": 50, "resultCount": 1, "totalCount": 51 }
            })))
            .mount(&server)
            .await;
        let files = client(&server, Some(KEY))
            .project_files(5, Some("1.20.1"))
            .await
            .unwrap();
        assert_eq!(files.len(), 51);
        assert_eq!(files[50].id, 99);
    }

    #[tokio::test]
    async fn matches_fingerprints_to_files() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/fingerprints"))
            .and(body_json(serde_json::json!({ "fingerprints": [99, 100] })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "isCacheBuilt": true,
                    "exactMatches": [
                        { "id": 5, "file": file_json(11, 5, 99), "latestFiles": [] },
                        { "id": 6, "file": file_json(12, 7, 100), "latestFiles": [] }
                    ],
                    "unmatchedFingerprints": []
                }
            })))
            .mount(&server)
            .await;
        let found = client(&server, Some(KEY))
            .files_for_fingerprints(&[99, 100])
            .await
            .unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[&99].id, 11);
    }

    #[tokio::test]
    async fn missing_projects_are_not_found() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/mods/404"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        let err = client(&server, Some(KEY)).project(404).await.unwrap_err();
        assert!(matches!(err, CoreError::NotFound(_)));
    }

    #[tokio::test]
    async fn the_key_is_not_part_of_errors() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&server)
            .await;
        let err = client(&server, Some("secret-value"))
            .project(1)
            .await
            .unwrap_err();
        assert!(!format!("{err} {err:?}").contains("secret-value"));
    }

    #[test]
    fn only_the_curseforge_cdn_is_allowed() {
        assert!(is_allowed_download(
            "https://edge.forgecdn.net/files/1/2/a.jar"
        ));
        assert!(is_allowed_download(
            "https://mediafilez.forgecdn.net/files/1/2/a.jar"
        ));
        assert!(!is_allowed_download("http://edge.forgecdn.net/files/a.jar"));
        assert!(!is_allowed_download(
            "https://edge.forgecdn.net.evil.io/a.jar"
        ));
        assert!(!is_allowed_download("https://user@edge.forgecdn.net/a.jar"));
        assert!(!is_allowed_download("https://cdn.modrinth.com/a.jar"));
    }

    #[test]
    fn maps_loaders() {
        assert_eq!(loader_type(ModLoader::NeoForge), 6);
        assert_eq!(loader_type(ModLoader::Quilt), 4);
        assert_eq!(loader_names(ModLoader::Quilt), &["Quilt", "Fabric"]);
        assert_eq!(class_id(ContentKind::Shader), 6552);
    }
}

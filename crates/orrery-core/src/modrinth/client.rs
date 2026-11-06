use std::collections::HashMap;

use super::models::{ModSearchPage, Project, SearchResponse, Version};
use crate::net::{Http, ensure_success, http_error, with_retry};
use crate::{CoreError, CoreResult};

const DEFAULT_API: &str = "https://api.modrinth.com/v2";
pub const PAGE_SIZE: u32 = 20;
const PROJECTS_PER_REQUEST: usize = 100;

#[derive(Clone)]
pub struct ModrinthClient {
    http: Http,
    base: String,
}

fn encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn validate_id(id: &str) -> CoreResult<()> {
    let valid = !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if valid {
        Ok(())
    } else {
        Err(CoreError::InvalidInput(format!(
            "invalid Modrinth id {id:?}"
        )))
    }
}

impl ModrinthClient {
    pub fn new(http: Http) -> Self {
        Self::with_base(http, DEFAULT_API)
    }

    pub fn with_base(http: Http, base: &str) -> Self {
        Self {
            http,
            base: base.trim_end_matches('/').to_string(),
        }
    }

    pub async fn search(
        &self,
        query: &str,
        game_version: Option<&str>,
        project_type: &str,
        loaders: &[&str],
        offset: u32,
    ) -> CoreResult<ModSearchPage> {
        let mut facets = vec![vec![format!("project_type:{project_type}")]];
        if let Some(game_version) = game_version {
            facets.push(vec![format!("versions:{game_version}")]);
        }
        if !loaders.is_empty() {
            facets.push(loaders.iter().map(|l| format!("categories:{l}")).collect());
        }
        let facets = serde_json::to_string(&facets).unwrap_or_default();
        let url = format!(
            "{}/search?query={}&facets={}&offset={offset}&limit={PAGE_SIZE}&index=relevance",
            self.base,
            encode(query.trim()),
            encode(&facets)
        );
        let response: SearchResponse = self.http.get_json(&url).await?;
        Ok(ModSearchPage {
            hits: response.hits.into_iter().map(Into::into).collect(),
            offset: response.offset,
            total: response.total_hits,
        })
    }

    pub async fn project_versions(
        &self,
        project_id: &str,
        game_version: &str,
        loaders: &[&str],
    ) -> CoreResult<Vec<Version>> {
        validate_id(project_id)?;
        let loaders = serde_json::json!(loaders).to_string();
        let game_versions = serde_json::json!([game_version]).to_string();
        let url = format!(
            "{}/project/{project_id}/version?loaders={}&game_versions={}",
            self.base,
            encode(&loaders),
            encode(&game_versions)
        );
        self.http
            .get_json(&url)
            .await
            .map_err(not_found(project_id))
    }

    pub async fn modpack_versions(
        &self,
        project_id: &str,
        loaders: &[&str],
    ) -> CoreResult<Vec<Version>> {
        validate_id(project_id)?;
        let loaders = serde_json::json!(loaders).to_string();
        let url = format!(
            "{}/project/{project_id}/version?loaders={}",
            self.base,
            encode(&loaders)
        );
        self.http
            .get_json(&url)
            .await
            .map_err(not_found(project_id))
    }

    pub async fn projects(&self, ids: &[String]) -> CoreResult<Vec<Project>> {
        let mut projects = Vec::new();
        for chunk in ids.chunks(PROJECTS_PER_REQUEST) {
            for id in chunk {
                validate_id(id)?;
            }
            let ids = serde_json::json!(chunk).to_string();
            let url = format!("{}/projects?ids={}", self.base, encode(&ids));
            projects.extend(self.http.get_json::<Vec<Project>>(&url).await?);
        }
        Ok(projects)
    }

    pub async fn versions_for_hashes(
        &self,
        sha512_hashes: &[String],
    ) -> CoreResult<HashMap<String, Version>> {
        self.versions_for(sha512_hashes, "sha512").await
    }

    pub async fn versions_for_sha1(
        &self,
        sha1_hashes: &[String],
    ) -> CoreResult<HashMap<String, Version>> {
        self.versions_for(sha1_hashes, "sha1").await
    }

    async fn versions_for(
        &self,
        hashes: &[String],
        algorithm: &str,
    ) -> CoreResult<HashMap<String, Version>> {
        if hashes.is_empty() {
            return Ok(HashMap::new());
        }
        let url = format!("{}/version_files", self.base);
        let body = serde_json::json!({
            "hashes": hashes,
            "algorithm": algorithm,
        });
        self.post_json(&url, &body).await
    }

    pub async fn version(&self, version_id: &str) -> CoreResult<Version> {
        validate_id(version_id)?;
        let url = format!("{}/version/{version_id}", self.base);
        self.http
            .get_json(&url)
            .await
            .map_err(not_found(version_id))
    }

    pub async fn project(&self, project_id: &str) -> CoreResult<Project> {
        validate_id(project_id)?;
        let url = format!("{}/project/{project_id}", self.base);
        self.http
            .get_json(&url)
            .await
            .map_err(not_found(project_id))
    }

    pub async fn latest_for_hashes(
        &self,
        sha512_hashes: &[String],
        game_version: &str,
        loaders: &[&str],
    ) -> CoreResult<HashMap<String, Version>> {
        if sha512_hashes.is_empty() {
            return Ok(HashMap::new());
        }
        let url = format!("{}/version_files/update", self.base);
        let body = serde_json::json!({
            "hashes": sha512_hashes,
            "algorithm": "sha512",
            "loaders": loaders,
            "game_versions": [game_version],
        });
        self.post_json(&url, &body).await
    }

    async fn post_json<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        body: &serde_json::Value,
    ) -> CoreResult<T> {
        with_retry(|| async {
            let response = self
                .http
                .client()
                .post(url)
                .json(body)
                .send()
                .await
                .map_err(|source| http_error(url, source))?;
            ensure_success(url, response)?
                .json::<T>()
                .await
                .map_err(|source| http_error(url, source))
        })
        .await
    }
}

fn not_found(id: &str) -> impl Fn(CoreError) -> CoreError + '_ {
    move |err| match err {
        CoreError::HttpStatus { status: 404, .. } => {
            CoreError::NotFound(format!("Modrinth project or version {id}"))
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_query_components() {
        assert_eq!(encode("sodium extra"), "sodium%20extra");
        assert_eq!(encode("[\"a\"]"), "%5B%22a%22%5D");
        assert_eq!(encode("Größe"), "Gr%C3%B6%C3%9Fe");
    }

    #[test]
    fn ids_are_validated() {
        assert!(validate_id("AANobbMI").is_ok());
        assert!(validate_id("../x").is_err());
        assert!(validate_id("").is_err());
    }
}

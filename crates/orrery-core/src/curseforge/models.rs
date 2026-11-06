use serde::Deserialize;

use crate::modrinth::models::ModSearchHit;

pub const PROJECT_PREFIX: &str = "curseforge-";
pub const RELATION_REQUIRED: u8 = 3;
const HASH_SHA1: u8 = 1;
const RELEASE: u8 = 1;
const BETA: u8 = 2;

pub fn project_key(mod_id: u32) -> String {
    format!("{PROJECT_PREFIX}{mod_id}")
}

pub fn parse_project_key(key: &str) -> Option<u32> {
    key.strip_prefix(PROJECT_PREFIX)
        .filter(|id| !id.is_empty() && id.len() <= 10 && id.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|id| id.parse().ok())
}

#[derive(Debug, Clone, Deserialize)]
pub struct Data<T> {
    pub data: T,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Page<T> {
    pub data: Vec<T>,
    pub pagination: Pagination,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pagination {
    pub index: u32,
    pub result_count: u32,
    pub total_count: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CfMod {
    pub id: u32,
    pub name: String,
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub download_count: f64,
    #[serde(default)]
    pub class_id: Option<u32>,
    #[serde(default)]
    pub authors: Vec<CfAuthor>,
    #[serde(default)]
    pub logo: Option<CfLogo>,
    #[serde(default)]
    pub links: Option<CfLinks>,
    #[serde(default)]
    pub allow_mod_distribution: Option<bool>,
    #[serde(default)]
    pub latest_files_indexes: Vec<FileIndex>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileIndex {
    pub game_version: String,
    pub file_id: u32,
    pub release_type: u8,
    #[serde(default)]
    pub mod_loader: Option<u8>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CfAuthor {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CfLogo {
    #[serde(default)]
    pub thumbnail_url: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CfLinks {
    #[serde(default)]
    pub website_url: Option<String>,
}

impl CfMod {
    pub fn icon_url(&self) -> Option<String> {
        self.logo
            .as_ref()
            .and_then(|logo| logo.thumbnail_url.clone().or_else(|| logo.url.clone()))
            .filter(|url| url.starts_with("https://"))
    }

    pub fn website_url(&self) -> Option<&str> {
        self.links
            .as_ref()
            .and_then(|links| links.website_url.as_deref())
            .filter(|url| url.starts_with("https://www.curseforge.com/"))
    }
}

impl From<CfMod> for ModSearchHit {
    fn from(project: CfMod) -> Self {
        let icon_url = project.icon_url();
        Self {
            project_id: project_key(project.id),
            slug: project.slug,
            title: project.name,
            description: project.summary,
            author: project
                .authors
                .into_iter()
                .next()
                .map(|author| author.name)
                .unwrap_or_default(),
            downloads: project.download_count.max(0.0) as u64,
            icon_url,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CfFile {
    pub id: u32,
    pub mod_id: u32,
    pub display_name: String,
    pub file_name: String,
    pub release_type: u8,
    #[serde(default)]
    pub file_date: String,
    #[serde(default)]
    pub file_length: u64,
    #[serde(default)]
    pub download_url: Option<String>,
    #[serde(default)]
    pub hashes: Vec<CfHash>,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<CfDependency>,
    #[serde(default)]
    pub file_fingerprint: u64,
    #[serde(default)]
    pub is_available: Option<bool>,
    #[serde(default)]
    pub is_server_pack: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CfHash {
    pub value: String,
    pub algo: u8,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CfDependency {
    pub mod_id: u32,
    pub relation_type: u8,
}

impl CfFile {
    pub fn sha1(&self) -> Option<String> {
        self.hashes
            .iter()
            .find(|hash| hash.algo == HASH_SHA1)
            .map(|hash| hash.value.to_ascii_lowercase())
            .filter(|hash| hash.len() == 40 && hash.chars().all(|c| c.is_ascii_hexdigit()))
    }

    pub fn is_release(&self) -> bool {
        self.release_type == RELEASE
    }

    pub fn release_label(&self) -> &'static str {
        match self.release_type {
            RELEASE => "release",
            BETA => "beta",
            _ => "alpha",
        }
    }

    pub fn required_mods(&self) -> impl Iterator<Item = u32> + '_ {
        self.dependencies
            .iter()
            .filter(|dep| dep.relation_type == RELATION_REQUIRED)
            .map(|dep| dep.mod_id)
    }

    pub fn is_installable(&self) -> bool {
        self.is_available != Some(false) && self.is_server_pack != Some(true)
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FingerprintMatches {
    #[serde(default)]
    pub exact_matches: Vec<FingerprintMatch>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FingerprintMatch {
    pub id: u32,
    pub file: CfFile,
}

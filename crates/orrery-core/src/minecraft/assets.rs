use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use super::version::AssetIndexRef;
use crate::error::IoResultExt;
use crate::net::{Checksum, Download};
use crate::{CoreError, CoreResult};

const RESOURCES_URL: &str = "https://resources.download.minecraft.net";

#[derive(Debug, Deserialize)]
pub struct AssetIndex {
    pub objects: HashMap<String, AssetObject>,
    #[serde(default, rename = "virtual")]
    pub is_virtual: bool,
    #[serde(default)]
    pub map_to_resources: bool,
}

#[derive(Debug, Deserialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}

pub fn index_download(assets_dir: &Path, index: &AssetIndexRef) -> CoreResult<Download> {
    if !is_simple_name(&index.id) {
        return Err(CoreError::InvalidInput(format!(
            "invalid asset index id {:?}",
            index.id
        )));
    }
    Ok(Download {
        url: index.url.clone(),
        path: assets_dir
            .join("indexes")
            .join(format!("{}.json", index.id)),
        checksum: Some(Checksum::Sha1(index.sha1.clone())),
        size: Some(index.size),
    })
}

pub async fn read_index(path: &Path) -> CoreResult<AssetIndex> {
    let bytes = tokio::fs::read(path).await.at_path(path)?;
    let index: AssetIndex = serde_json::from_slice(&bytes).map_err(|source| CoreError::Json {
        path: path.to_path_buf(),
        source,
    })?;
    if index.is_virtual || index.map_to_resources {
        return Err(CoreError::Unsupported(
            "legacy virtual asset indexes (before 1.7.10) are not supported".into(),
        ));
    }
    Ok(index)
}

pub fn object_downloads(assets_dir: &Path, index: &AssetIndex) -> CoreResult<Vec<Download>> {
    let objects_dir = assets_dir.join("objects");
    index
        .objects
        .values()
        .map(|object| {
            let hash = object.hash.to_ascii_lowercase();
            if hash.len() != 40 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(CoreError::InvalidInput(format!(
                    "invalid asset hash {:?}",
                    object.hash
                )));
            }
            let prefix = &hash[..2];
            Ok(Download {
                url: format!("{RESOURCES_URL}/{prefix}/{hash}"),
                path: objects_dir.join(prefix).join(&hash),
                checksum: Some(Checksum::Sha1(hash.clone())),
                size: Some(object.size),
            })
        })
        .collect()
}

fn is_simple_name(value: &str) -> bool {
    !value.is_empty()
        && !value.contains("..")
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_downloads_use_hash_prefix_layout() {
        let index: AssetIndex = serde_json::from_str(
            r#"{"objects":{"minecraft/sounds/a.ogg":{"hash":"bdf48ef6b5d0d23bbb02e17d04865216179f510a","size":3}}}"#,
        )
        .unwrap();
        let downloads = object_downloads(Path::new("assets"), &index).unwrap();
        assert_eq!(
            downloads[0].url,
            "https://resources.download.minecraft.net/bd/bdf48ef6b5d0d23bbb02e17d04865216179f510a"
        );
        assert!(downloads[0].path.ends_with(Path::new(
            "objects/bd/bdf48ef6b5d0d23bbb02e17d04865216179f510a"
        )));
    }

    #[test]
    fn malformed_hash_is_rejected() {
        let index: AssetIndex =
            serde_json::from_str(r#"{"objects":{"x":{"hash":"../../etc","size":3}}}"#).unwrap();
        assert!(object_downloads(Path::new("assets"), &index).is_err());
    }
}

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Serialize, de::DeserializeOwned};

use crate::{CoreError, CoreResult, error::IoResultExt};

pub trait Versioned {
    const SCHEMA: u32;
    fn schema(&self) -> u32;
}

pub async fn read_json<T: DeserializeOwned>(path: &Path) -> CoreResult<Option<T>> {
    let bytes = match tokio::fs::read(path).await {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err).at_path(path),
    };
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|source| CoreError::Json {
            path: path.to_path_buf(),
            source,
        })
}

pub async fn read_versioned<T: DeserializeOwned + Versioned>(path: &Path) -> CoreResult<Option<T>> {
    let Some(value) = read_json::<T>(path).await? else {
        return Ok(None);
    };
    if value.schema() > T::SCHEMA {
        return Err(CoreError::UnsupportedSchema {
            path: path.to_path_buf(),
            found: value.schema(),
            supported: T::SCHEMA,
        });
    }
    Ok(Some(value))
}

pub async fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> CoreResult<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|source| CoreError::Json {
        path: path.to_path_buf(),
        source,
    })?;
    write_bytes_atomic(path, &bytes).await
}

pub async fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> CoreResult<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.at_path(parent)?;
    }
    let tmp = temp_sibling(path);
    tokio::fs::write(&tmp, bytes).await.at_path(&tmp)?;
    if let Err(err) = tokio::fs::rename(&tmp, path).await {
        let _ = tokio::fs::remove_file(&tmp).await;
        return Err(err).at_path(path);
    }
    Ok(())
}

pub fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}

pub fn temp_sibling(path: &Path) -> std::path::PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut name = path
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    name.push(format!(".{}-{n}.part", std::process::id()));
    path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Sample {
        schema: u32,
        name: String,
    }

    impl Versioned for Sample {
        const SCHEMA: u32 = 1;
        fn schema(&self) -> u32 {
            self.schema
        }
    }

    #[tokio::test]
    async fn missing_file_reads_as_none() {
        let dir = tempfile::tempdir().unwrap();
        let result: Option<Sample> = read_json(&dir.path().join("missing.json")).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn write_then_read_roundtrips_and_leaves_no_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("sample.json");
        let sample = Sample {
            schema: 1,
            name: "Grüße".into(),
        };
        write_json_atomic(&path, &sample).await.unwrap();
        write_json_atomic(&path, &sample).await.unwrap();

        let read: Sample = read_versioned(&path).await.unwrap().unwrap();
        assert_eq!(read, sample);
        let entries: Vec<_> = std::fs::read_dir(path.parent().unwrap()).unwrap().collect();
        assert_eq!(entries.len(), 1);
    }

    #[tokio::test]
    async fn newer_schema_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("future.json");
        std::fs::write(&path, r#"{"schema": 2, "name": "x"}"#).unwrap();
        let err = read_versioned::<Sample>(&path).await.unwrap_err();
        assert!(matches!(err, CoreError::UnsupportedSchema { found: 2, .. }));
    }
}

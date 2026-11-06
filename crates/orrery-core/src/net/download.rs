use std::path::{Path, PathBuf};

use futures::{StreamExt, TryStreamExt};
use sha1::Digest;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};

use super::Http;
use super::client::{ensure_success, http_error, with_retry};
use crate::error::IoResultExt;
use crate::progress::Progress;
use crate::storage::temp_sibling;
use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Checksum {
    Sha1(String),
    Sha256(String),
    Sha512(String),
}

impl Checksum {
    fn expected(&self) -> &str {
        match self {
            Self::Sha1(hash) | Self::Sha256(hash) | Self::Sha512(hash) => hash,
        }
    }

    fn hasher(&self) -> Hasher {
        match self {
            Self::Sha1(_) => Hasher::Sha1(sha1::Sha1::new()),
            Self::Sha256(_) => Hasher::Sha256(sha2::Sha256::new()),
            Self::Sha512(_) => Hasher::Sha512(sha2::Sha512::new()),
        }
    }
}

enum Hasher {
    Sha1(sha1::Sha1),
    Sha256(sha2::Sha256),
    Sha512(sha2::Sha512),
}

impl Hasher {
    fn update(&mut self, data: &[u8]) {
        match self {
            Self::Sha1(h) => h.update(data),
            Self::Sha256(h) => h.update(data),
            Self::Sha512(h) => h.update(data),
        }
    }

    fn finish(self) -> String {
        match self {
            Self::Sha1(h) => hex::encode(h.finalize()),
            Self::Sha256(h) => hex::encode(h.finalize()),
            Self::Sha512(h) => hex::encode(h.finalize()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Download {
    pub url: String,
    pub path: PathBuf,
    pub checksum: Option<Checksum>,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyMode {
    Quick,
    Full,
}

pub async fn download_all(
    http: &Http,
    downloads: Vec<Download>,
    concurrency: usize,
    mode: VerifyMode,
    progress: &Progress,
) -> CoreResult<()> {
    let mut unique = std::collections::HashSet::new();
    let downloads: Vec<Download> = downloads
        .into_iter()
        .filter(|d| unique.insert(d.path.clone()))
        .collect();

    let total_bytes = downloads.iter().filter_map(|d| d.size).sum();
    progress.add_totals(downloads.len() as u64, total_bytes);

    futures::stream::iter(downloads.into_iter().map(Ok))
        .try_for_each_concurrent(concurrency.max(1), |download| async move {
            progress.check_cancelled()?;
            if file_matches(
                &download.path,
                download.checksum.as_ref(),
                download.size,
                mode,
            )
            .await?
            {
                progress.add_bytes(download.size.unwrap_or(0));
            } else {
                with_retry(|| fetch_one(http, &download, progress)).await?;
            }
            progress.add_items(1);
            Ok(())
        })
        .await
}

pub async fn file_matches(
    path: &Path,
    checksum: Option<&Checksum>,
    size: Option<u64>,
    mode: VerifyMode,
) -> CoreResult<bool> {
    let metadata = match tokio::fs::metadata(path).await {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(err).at_path(path),
    };
    if !metadata.is_file() {
        return Ok(false);
    }
    if let Some(size) = size
        && metadata.len() != size
    {
        return Ok(false);
    }
    let needs_hash = mode == VerifyMode::Full || size.is_none();
    match checksum {
        Some(checksum) if needs_hash => Ok(hash_file(path, checksum)
            .await?
            .eq_ignore_ascii_case(checksum.expected())),
        _ => Ok(true),
    }
}

pub fn sha1_hex(data: &[u8]) -> String {
    hex::encode(sha1::Sha1::digest(data))
}

async fn hash_file(path: &Path, checksum: &Checksum) -> CoreResult<String> {
    let mut file = tokio::fs::File::open(path).await.at_path(path)?;
    let mut hasher = checksum.hasher();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).await.at_path(path)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finish())
}

struct TempFile(PathBuf);

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

async fn fetch_one(http: &Http, download: &Download, progress: &Progress) -> CoreResult<()> {
    let path = &download.path;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.at_path(parent)?;
    }
    let temp = TempFile(temp_sibling(path));
    let mut received: u64 = 0;
    let result = stream_to_file(http, download, &temp.0, progress, &mut received).await;
    if result.is_err() {
        progress.remove_bytes(received);
        return result;
    }
    if let Some(expected) = download.size
        && expected != received
    {
        progress.remove_bytes(received);
        return Err(CoreError::ChecksumMismatch {
            url: download.url.clone(),
            expected: format!("{expected} bytes"),
            actual: format!("{received} bytes"),
        });
    }
    if download.size.is_none() {
        progress.add_totals(0, received);
    }
    tokio::fs::rename(&temp.0, path).await.at_path(path)?;
    Ok(())
}

async fn stream_to_file(
    http: &Http,
    download: &Download,
    temp: &Path,
    progress: &Progress,
    received: &mut u64,
) -> CoreResult<()> {
    let url = &download.url;
    let response = http
        .client()
        .get(url)
        .send()
        .await
        .map_err(|source| http_error(url, source))?;
    let response = ensure_success(url, response)?;

    let file = tokio::fs::File::create(temp).await.at_path(temp)?;
    let mut writer = BufWriter::new(file);
    let mut hasher = download.checksum.as_ref().map(Checksum::hasher);
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        progress.check_cancelled()?;
        let chunk = chunk.map_err(|source| http_error(url, source))?;
        if let Some(hasher) = hasher.as_mut() {
            hasher.update(&chunk);
        }
        writer.write_all(&chunk).await.at_path(temp)?;
        *received += chunk.len() as u64;
        progress.add_bytes(chunk.len() as u64);
    }
    writer.flush().await.at_path(temp)?;
    drop(writer);

    if let (Some(hasher), Some(checksum)) = (hasher, download.checksum.as_ref()) {
        let actual = hasher.finish();
        if !actual.eq_ignore_ascii_case(checksum.expected()) {
            return Err(CoreError::ChecksumMismatch {
                url: url.clone(),
                expected: checksum.expected().to_string(),
                actual,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const BODY: &[u8] = b"hello launcher";
    const WRONG_SHA1: &str = "8cf3b2b5a4a8ab39e5e4c2a8b8c7f8a5ef1a6c3e";

    fn sha1_of(data: &[u8]) -> String {
        hex::encode(sha1::Sha1::digest(data))
    }

    async fn server_with_body(body: &'static [u8]) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/file"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(body))
            .mount(&server)
            .await;
        server
    }

    fn item(server: &MockServer, target: &Path, checksum: Option<Checksum>) -> Download {
        Download {
            url: format!("{}/file", server.uri()),
            path: target.to_path_buf(),
            checksum,
            size: Some(BODY.len() as u64),
        }
    }

    #[tokio::test]
    async fn downloads_and_verifies_sha1() {
        let server = server_with_body(BODY).await;
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("a").join("file.bin");
        let progress = Progress::new();
        let http = Http::new("test").unwrap();

        let download = item(&server, &target, Some(Checksum::Sha1(sha1_of(BODY))));
        download_all(&http, vec![download], 4, VerifyMode::Full, &progress)
            .await
            .unwrap();

        assert_eq!(std::fs::read(&target).unwrap(), BODY);
        let snapshot = progress.snapshot();
        assert_eq!(snapshot.items_done, 1);
        assert_eq!(snapshot.bytes_done, BODY.len() as u64);
    }

    #[tokio::test]
    async fn checksum_mismatch_leaves_no_files() {
        let server = server_with_body(BODY).await;
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("file.bin");
        let progress = Progress::new();
        let http = Http::new("test").unwrap();

        let download = item(&server, &target, Some(Checksum::Sha1(WRONG_SHA1.into())));
        let err = download_all(&http, vec![download], 1, VerifyMode::Quick, &progress)
            .await
            .unwrap_err();

        assert!(matches!(err, CoreError::ChecksumMismatch { .. }));
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
        assert_eq!(progress.snapshot().bytes_done, 0);
    }

    #[tokio::test]
    async fn existing_valid_file_is_skipped_and_corrupt_file_is_replaced() {
        let server = server_with_body(BODY).await;
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("file.bin");
        let http = Http::new("test").unwrap();
        let checksum = Some(Checksum::Sha1(sha1_of(BODY)));

        std::fs::write(&target, BODY).unwrap();
        download_all(
            &http,
            vec![item(&server, &target, checksum.clone())],
            1,
            VerifyMode::Full,
            &Progress::new(),
        )
        .await
        .unwrap();
        assert_eq!(server.received_requests().await.unwrap().len(), 0);

        std::fs::write(&target, b"hello launcheR").unwrap();
        download_all(
            &http,
            vec![item(&server, &target, checksum)],
            1,
            VerifyMode::Full,
            &Progress::new(),
        )
        .await
        .unwrap();
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
        assert_eq!(std::fs::read(&target).unwrap(), BODY);
    }

    #[tokio::test]
    async fn server_error_is_reported() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let http = Http::new("test").unwrap();
        let download = item(&server, &dir.path().join("x"), None);
        let err = download_all(
            &http,
            vec![download],
            1,
            VerifyMode::Quick,
            &Progress::new(),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, CoreError::HttpStatus { status: 404, .. }));
    }
}

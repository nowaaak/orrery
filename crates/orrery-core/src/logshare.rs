use std::path::Path;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::IoResultExt;
use crate::net::{Http, ensure_success, http_error};
use crate::redact::Redactor;
use crate::services::Services;
use crate::{CoreError, CoreResult};

pub const MCLOGS_API: &str = "https://api.mclo.gs";
const SOURCE: &str = "Orrery";
const LAUNCHER_LOG_PREFIX: &str = "launcher";
const LAUNCHER_LOG_LINES: usize = 5000;
const MAX_LINES: usize = 25_000;
const MAX_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SharedLog {
    pub url: String,
}

#[derive(Deserialize)]
struct PasteResponse {
    success: bool,
    url: Option<String>,
    error: Option<String>,
}

pub fn prepare(text: &str, redactor: &Redactor) -> String {
    truncate_tail(&redactor.redact(text))
}

pub async fn upload(
    http: &Http,
    api: &str,
    text: &str,
    redactor: &Redactor,
) -> CoreResult<SharedLog> {
    let content = prepare(text, redactor);
    if content.trim().is_empty() {
        return Err(CoreError::InvalidInput("the log is empty".into()));
    }
    let url = format!("{}/1/log", api.trim_end_matches('/'));
    let response = http
        .client()
        .post(&url)
        .json(&serde_json::json!({ "content": content, "source": SOURCE }))
        .send()
        .await
        .map_err(|source| http_error(&url, source))?;
    let paste: PasteResponse = ensure_success(&url, response)?
        .json()
        .await
        .map_err(|source| http_error(&url, source))?;
    match (paste.success, paste.url) {
        (true, Some(link)) if link.starts_with("https://") => Ok(SharedLog { url: link }),
        (_, _) => Err(CoreError::Unsupported(format!(
            "mclo.gs rejected the log: {}",
            paste.error.unwrap_or_else(|| "no link returned".into())
        ))),
    }
}

impl Services {
    pub async fn log_redactor(&self) -> Redactor {
        self.accounts
            .view()
            .await
            .accounts
            .iter()
            .fold(Redactor::system(), |redactor, account| {
                redactor.name(&account.username)
            })
    }

    pub async fn prepare_log_share(&self, text: &str) -> String {
        prepare(text, &self.log_redactor().await)
    }

    pub async fn share_log(&self, text: &str) -> CoreResult<SharedLog> {
        upload(&self.http, MCLOGS_API, text, &self.log_redactor().await).await
    }

    pub async fn launcher_log_tail(&self) -> CoreResult<Vec<String>> {
        let dir = self.paths.logs.clone();
        tokio::task::spawn_blocking(move || read_launcher_log(&dir))
            .await
            .map_err(|err| CoreError::Unsupported(format!("log reader panicked: {err}")))?
    }
}

fn read_launcher_log(dir: &Path) -> CoreResult<Vec<String>> {
    let newest = std::fs::read_dir(dir)
        .at_path(dir)?
        .flatten()
        .filter(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name.starts_with(LAUNCHER_LOG_PREFIX) && name.ends_with(".log")
        })
        .filter_map(|entry| Some((entry.metadata().ok()?.modified().ok()?, entry.path())))
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path);
    let Some(path) = newest else {
        return Ok(Vec::new());
    };
    let bytes = std::fs::read(&path).at_path(&path)?;
    let text = String::from_utf8_lossy(&bytes);
    let lines: Vec<&str> = text.lines().collect();
    let skip = lines.len().saturating_sub(LAUNCHER_LOG_LINES);
    Ok(lines[skip..].iter().map(|line| line.to_string()).collect())
}

fn truncate_tail(text: &str) -> String {
    let mut start = 0;
    let body = text.strip_suffix('\n').unwrap_or(text);
    if let Some((index, _)) = body.match_indices('\n').rev().nth(MAX_LINES - 1) {
        start = index + 1;
    }
    if text.len() - start > MAX_BYTES {
        start = text.len() - MAX_BYTES;
        while !text.is_char_boundary(start) {
            start += 1;
        }
        if let Some(newline) = text[start..].find('\n') {
            start += newline + 1;
        }
    }
    text[start..].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_partial_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn keeps_the_last_lines_within_limits() {
        let text: String = (0..MAX_LINES + 10).map(|i| format!("line {i}\n")).collect();
        let kept = truncate_tail(&text);
        assert_eq!(kept.lines().count(), MAX_LINES);
        assert!(kept.starts_with("line 10\n"));

        let long = "ä".repeat(MAX_BYTES / 2 + 10) + "\nend";
        let kept = truncate_tail(&long);
        assert_eq!(kept, "end");
    }

    #[test]
    fn reads_the_newest_launcher_log() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read_launcher_log(dir.path()).unwrap().is_empty());
        std::fs::write(dir.path().join("launcher.2026-11-01.log"), "old\n").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(dir.path().join("other.log"), "skip\n").unwrap();
        std::fs::write(dir.path().join("launcher.2026-11-02.log"), "a\nb\n").unwrap();
        assert_eq!(read_launcher_log(dir.path()).unwrap(), ["a", "b"]);
    }

    #[tokio::test]
    async fn uploads_the_redacted_log() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/1/log"))
            .and(body_partial_json(serde_json::json!({
                "content": "Connecting to <ip>\nuser <user>",
                "source": "Orrery"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "success": true,
                "id": "abc",
                "url": "https://mclo.gs/abc",
                "raw": "https://api.mclo.gs/1/raw/abc"
            })))
            .expect(1)
            .mount(&server)
            .await;
        let http = Http::new("orrery-test").unwrap();
        let redactor = Redactor::new().name("Michel");
        let shared = upload(
            &http,
            &server.uri(),
            "Connecting to 198.51.100.4\nuser Michel",
            &redactor,
        )
        .await
        .unwrap();
        assert_eq!(shared.url, "https://mclo.gs/abc");
    }

    #[tokio::test]
    async fn reports_rejections_and_empty_logs() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "success": false,
                "error": "Required field 'content' is empty."
            })))
            .mount(&server)
            .await;
        let http = Http::new("orrery-test").unwrap();
        let err = upload(&http, &server.uri(), "x", &Redactor::new())
            .await
            .unwrap_err();
        assert!(err.to_string().contains("content"), "{err}");
        assert!(
            upload(&http, &server.uri(), " \n", &Redactor::new())
                .await
                .is_err()
        );
    }
}

use std::net::{Ipv4Addr, Ipv6Addr};
use std::time::Duration;

use reqwest::Url;
use serde::Deserialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinSet;
use ts_rs::TS;

use super::AuthFailure;
use crate::{CoreError, CoreResult};

const MAX_REQUEST_BYTES: usize = 8 * 1024;
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_MESSAGE_CHARS: usize = 300;

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LoginPage {
    pub success: String,
    pub failure: String,
}

pub struct Loopback {
    listeners: Vec<TcpListener>,
    port: u16,
}

#[derive(Debug)]
enum Outcome {
    Ignore,
    Code(String),
    Failed(CoreError),
}

impl Loopback {
    pub async fn bind() -> CoreResult<Self> {
        let v4 = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .map_err(listen_error)?;
        let port = v4.local_addr().map_err(listen_error)?.port();
        let mut listeners = vec![v4];
        match TcpListener::bind((Ipv6Addr::LOCALHOST, port)).await {
            Ok(v6) => listeners.push(v6),
            Err(err) => tracing::debug!(error = %err, "no IPv6 loopback for the sign-in redirect"),
        }
        Ok(Self { listeners, port })
    }

    pub fn redirect_uri(&self) -> String {
        format!("http://localhost:{}", self.port)
    }

    pub async fn wait(&self, state: &str, page: &LoginPage) -> CoreResult<String> {
        let mut connections = JoinSet::new();
        loop {
            tokio::select! {
                accepted = self.accept() => {
                    let stream = accepted?;
                    let state = state.to_string();
                    let page = page.clone();
                    connections.spawn(async move {
                        tokio::time::timeout(CONNECTION_TIMEOUT, respond(stream, &state, &page))
                            .await
                            .unwrap_or(Outcome::Ignore)
                    });
                }
                Some(joined) = connections.join_next() => match joined {
                    Ok(Outcome::Code(code)) => return Ok(code),
                    Ok(Outcome::Failed(err)) => return Err(err),
                    Ok(Outcome::Ignore) | Err(_) => {}
                },
            }
        }
    }

    async fn accept(&self) -> CoreResult<TcpStream> {
        let accepts = self
            .listeners
            .iter()
            .map(|listener| Box::pin(listener.accept()));
        let (accepted, _, _) = futures::future::select_all(accepts).await;
        accepted.map(|(stream, _)| stream).map_err(listen_error)
    }
}

fn listen_error(err: std::io::Error) -> CoreError {
    AuthFailure::Service.error(format!("could not wait for the sign-in redirect: {err}"))
}

async fn respond(mut stream: TcpStream, state: &str, page: &LoginPage) -> Outcome {
    let Some(target) = read_target(&mut stream).await else {
        return Outcome::Ignore;
    };
    let outcome = classify(&target, state);
    let response = match &outcome {
        Outcome::Ignore => http_response("404 Not Found", ""),
        Outcome::Code(_) => http_response("200 OK", &page_html(&page.success)),
        Outcome::Failed(_) => http_response("200 OK", &page_html(&page.failure)),
    };
    if let Err(err) = stream.write_all(response.as_bytes()).await {
        tracing::debug!(error = %err, "could not answer the sign-in redirect");
    }
    let _ = stream.shutdown().await;
    outcome
}

async fn read_target(stream: &mut TcpStream) -> Option<String> {
    let mut buffer = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    let line_end = loop {
        if let Some(end) = buffer.windows(2).position(|pair| pair == b"\r\n") {
            break end;
        }
        if buffer.len() >= MAX_REQUEST_BYTES {
            return None;
        }
        let read = stream.read(&mut chunk).await.ok()?;
        if read == 0 {
            return None;
        }
        buffer.extend_from_slice(&chunk[..read]);
    };
    let line = std::str::from_utf8(&buffer[..line_end]).ok()?;
    let mut parts = line.split(' ');
    match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some("GET"), Some(target), Some(version), None) if version.starts_with("HTTP/1.") => {
            Some(target.to_string())
        }
        _ => None,
    }
}

fn classify(target: &str, expected_state: &str) -> Outcome {
    if !target.starts_with('/') {
        return Outcome::Ignore;
    }
    let Ok(url) = Url::parse(&format!("http://localhost{target}")) else {
        return Outcome::Ignore;
    };
    if url.path() != "/" {
        return Outcome::Ignore;
    }
    let (mut state, mut code, mut error, mut description) = (None, None, None, None);
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "state" => state = Some(value.into_owned()),
            "code" => code = Some(value.into_owned()),
            "error" => error = Some(value.into_owned()),
            "error_description" => description = Some(value.into_owned()),
            _ => {}
        }
    }
    if state.as_deref() != Some(expected_state) {
        return Outcome::Ignore;
    }
    if let Some(error) = error {
        let reason = match error.as_str() {
            "access_denied" => AuthFailure::Declined,
            _ => AuthFailure::Service,
        };
        return Outcome::Failed(reason.error(description.unwrap_or(error)));
    }
    match code {
        Some(code) if !code.is_empty() => Outcome::Code(code),
        _ => Outcome::Failed(
            AuthFailure::Service.error("the sign-in redirect contained no authorization code"),
        ),
    }
}

fn http_response(status: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

fn page_html(message: &str) -> String {
    let message: String = message.chars().take(MAX_MESSAGE_CHARS).collect();
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Orrery</title><style>:root{{color-scheme:light dark}}body{{margin:0;min-height:100vh;display:grid;place-items:center;font:16px/1.5 system-ui,sans-serif;background:#0b0b12;color:#f4f4f5}}@media (prefers-color-scheme:light){{body{{background:#f8f8fb;color:#18181b}}}}main{{max-width:28rem;padding:2rem;text-align:center}}h1{{font-size:1.25rem;margin:0 0 .5rem}}</style></head><body><main><h1>Orrery</h1><p>{}</p></main></body></html>",
        escape_html(&message)
    )
}

fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page() -> LoginPage {
        LoginPage {
            success: "Fertig <b>".into(),
            failure: "Fehlgeschlagen".into(),
        }
    }

    #[test]
    fn classifies_redirects() {
        assert!(
            matches!(classify("/?code=abc&state=s1", "s1"), Outcome::Code(code) if code == "abc")
        );
        assert!(matches!(
            classify("/?code=abc&state=other", "s1"),
            Outcome::Ignore
        ));
        assert!(matches!(classify("/?code=abc", "s1"), Outcome::Ignore));
        assert!(matches!(classify("/favicon.ico", "s1"), Outcome::Ignore));
        assert!(matches!(
            classify("http://evil/?code=abc&state=s1", "s1"),
            Outcome::Ignore
        ));
        match classify("/?error=access_denied&error_description=no&state=s1", "s1") {
            Outcome::Failed(err) => assert_eq!(err.kind(), "auth_declined"),
            other => panic!("unexpected {other:?}"),
        }
        match classify("/?state=s1", "s1") {
            Outcome::Failed(err) => assert_eq!(err.kind(), "auth_service"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn page_escapes_the_message() {
        let html = page_html("<script>x</script>");
        assert!(html.contains("&lt;script&gt;x&lt;/script&gt;"));
        assert!(!html.contains("<script>"));
    }

    #[tokio::test]
    async fn waits_for_the_matching_redirect() {
        let loopback = Loopback::bind().await.unwrap();
        let redirect = loopback.redirect_uri();
        let base = redirect.replace("localhost", "127.0.0.1");
        let client = reqwest::Client::new();

        let idle = TcpStream::connect(base.trim_start_matches("http://"))
            .await
            .unwrap();
        let sender = tokio::spawn(async move {
            let ignored = client
                .get(format!("{base}/?code=wrong&state=nope"))
                .send()
                .await
                .unwrap();
            assert_eq!(ignored.status(), 404);
            let favicon = client
                .get(format!("{base}/favicon.ico"))
                .send()
                .await
                .unwrap();
            assert_eq!(favicon.status(), 404);
            let done = client
                .get(format!("{base}/?code=right&state=s1"))
                .send()
                .await
                .unwrap();
            assert_eq!(done.status(), 200);
            done.text().await.unwrap()
        });

        let code = tokio::time::timeout(Duration::from_secs(5), loopback.wait("s1", &page()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(code, "right");
        let body = sender.await.unwrap();
        assert!(body.contains("Fertig &lt;b&gt;"), "{body}");
        drop(idle);
    }

    #[tokio::test]
    async fn reports_a_declined_sign_in() {
        let loopback = Loopback::bind().await.unwrap();
        let base = loopback.redirect_uri().replace("localhost", "127.0.0.1");
        let sender = tokio::spawn(async move {
            reqwest::get(format!("{base}/?error=access_denied&state=s1"))
                .await
                .unwrap()
                .text()
                .await
                .unwrap()
        });
        let err = loopback.wait("s1", &page()).await.unwrap_err();
        assert_eq!(err.kind(), "auth_declined");
        assert!(sender.await.unwrap().contains("Fehlgeschlagen"));
    }
}

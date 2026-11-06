use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue};
use serde::de::DeserializeOwned;

use crate::{CoreError, CoreResult};

const MAX_ATTEMPTS: u32 = 3;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const READ_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_REDIRECTS: usize = 5;

#[derive(Clone)]
pub struct Http {
    client: reqwest::Client,
    user_agent: Arc<str>,
}

fn builder(user_agent: &str) -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .user_agent(user_agent)
        .connect_timeout(CONNECT_TIMEOUT)
        .read_timeout(READ_TIMEOUT)
}

fn build(user_agent: &str, builder: reqwest::ClientBuilder) -> CoreResult<Http> {
    let client = builder.build().map_err(|source| CoreError::Http {
        url: String::new(),
        source,
    })?;
    Ok(Http {
        client,
        user_agent: user_agent.into(),
    })
}

pub fn redirect_allowed(url: &reqwest::Url, hosts: &[&str]) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.host_str().is_some_and(|host| hosts.contains(&host))
}

impl Http {
    pub fn new(user_agent: &str) -> CoreResult<Self> {
        build(user_agent, builder(user_agent))
    }

    pub fn with_header(
        &self,
        name: &'static str,
        mut value: HeaderValue,
        hosts: &'static [&'static str],
    ) -> CoreResult<Self> {
        value.set_sensitive(true);
        let mut headers = HeaderMap::new();
        headers.insert(name, value);
        let policy = reqwest::redirect::Policy::custom(move |attempt| {
            if attempt.previous().len() >= MAX_REDIRECTS {
                attempt.error("too many redirects")
            } else if redirect_allowed(attempt.url(), hosts) {
                attempt.follow()
            } else {
                attempt.stop()
            }
        });
        build(
            &self.user_agent,
            builder(&self.user_agent)
                .default_headers(headers)
                .redirect(policy),
        )
    }

    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    pub async fn get(&self, url: &str) -> CoreResult<reqwest::Response> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|source| http_error(url, source))?;
        ensure_success(url, response)
    }

    pub async fn get_json<T: DeserializeOwned>(&self, url: &str) -> CoreResult<T> {
        with_retry(|| async {
            let response = self.get(url).await?;
            response
                .json::<T>()
                .await
                .map_err(|source| http_error(url, source))
        })
        .await
    }

    pub async fn get_bytes(&self, url: &str) -> CoreResult<Vec<u8>> {
        with_retry(|| async {
            let response = self.get(url).await?;
            let bytes = response
                .bytes()
                .await
                .map_err(|source| http_error(url, source))?;
            Ok(bytes.to_vec())
        })
        .await
    }
}

pub fn http_error(url: &str, source: reqwest::Error) -> CoreError {
    CoreError::Http {
        url: url.to_string(),
        source: source.without_url(),
    }
}

pub fn ensure_success(url: &str, response: reqwest::Response) -> CoreResult<reqwest::Response> {
    let status = response.status();
    if status.is_success() {
        Ok(response)
    } else {
        Err(CoreError::HttpStatus {
            url: url.to_string(),
            status: status.as_u16(),
        })
    }
}

pub async fn with_retry<T, F, Fut>(mut attempt: F) -> CoreResult<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = CoreResult<T>>,
{
    let mut tries = 0;
    loop {
        tries += 1;
        match attempt().await {
            Ok(value) => return Ok(value),
            Err(err) if tries < MAX_ATTEMPTS && err.is_retryable() => {
                let delay = Duration::from_millis(500 * 2u64.pow(tries - 1));
                tracing::warn!(error = %err, attempt = tries, "retrying after error");
                tokio::time::sleep(delay).await;
            }
            Err(err) => return Err(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    const HOSTS: &[&str] = &["edge.forgecdn.net", "mediafilez.forgecdn.net"];

    #[test]
    fn redirects_only_go_to_listed_https_hosts() {
        let allowed = |url: &str| redirect_allowed(&reqwest::Url::parse(url).unwrap(), HOSTS);
        assert!(allowed("https://mediafilez.forgecdn.net/files/1/2/a.jar"));
        assert!(!allowed("http://mediafilez.forgecdn.net/files/1/2/a.jar"));
        assert!(!allowed("https://cdn.modrinth.com/a.jar"));
        assert!(!allowed("https://user@mediafilez.forgecdn.net/a.jar"));
        assert!(!allowed("https://mediafilez.forgecdn.net.evil.io/a.jar"));
    }

    #[tokio::test]
    async fn header_clients_send_the_header_and_stop_at_foreign_redirects() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/file"))
            .and(header("x-api-key", "secret"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/moved"))
            .respond_with(
                ResponseTemplate::new(302).insert_header("location", "https://example.com/x"),
            )
            .mount(&server)
            .await;
        let http = Http::new("orrery-test")
            .unwrap()
            .with_header("x-api-key", HeaderValue::from_static("secret"), HOSTS)
            .unwrap();
        let body = http
            .get_bytes(&format!("{}/file", server.uri()))
            .await
            .unwrap();
        assert_eq!(body, b"ok");
        let err = http
            .get(&format!("{}/moved", server.uri()))
            .await
            .unwrap_err();
        assert!(matches!(err, CoreError::HttpStatus { status: 302, .. }));
    }
}

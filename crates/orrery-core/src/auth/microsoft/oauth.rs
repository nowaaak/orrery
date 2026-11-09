use reqwest::Url;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::{AuthFailure, Endpoints, parse, read_body};
use crate::CoreResult;
use crate::encoding::base64url_unpadded;
use crate::net::{Http, http_error};

const SCOPE: &str = "XboxLive.signin offline_access";
const DEVICE_GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";

pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

impl Pkce {
    pub fn generate() -> Self {
        Self::from_verifier(format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        ))
    }

    fn from_verifier(verifier: String) -> Self {
        let challenge = base64url_unpadded(&Sha256::digest(verifier.as_bytes()));
        Self {
            verifier,
            challenge,
        }
    }
}

pub fn authorize_url(
    endpoints: &Endpoints,
    client_id: &str,
    redirect_uri: &str,
    challenge: &str,
    state: &str,
) -> CoreResult<String> {
    let base = format!("{}/authorize", endpoints.oauth);
    let mut url = Url::parse(&base).map_err(|err| {
        AuthFailure::Service.error(format!("invalid authorization endpoint {base}: {err}"))
    })?;
    url.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("response_mode", "query")
        .append_pair("scope", SCOPE)
        .append_pair("state", state)
        .append_pair("code_challenge", challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("prompt", "select_account");
    Ok(url.into())
}

pub async fn exchange_code(
    http: &Http,
    endpoints: &Endpoints,
    client_id: &str,
    code: &str,
    redirect_uri: &str,
    verifier: &str,
) -> CoreResult<TokenResponse> {
    let url = format!("{}/token", endpoints.oauth);
    let (status, body) = post_form(
        http,
        &url,
        &[
            ("grant_type", "authorization_code"),
            ("client_id", client_id),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("code_verifier", verifier),
            ("scope", SCOPE),
        ],
    )
    .await?;
    if status != 200 {
        return Err(oauth_failure(&body));
    }
    parse(&url, &body)
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    #[serde(default = "default_interval")]
    pub interval: u64,
}

fn default_interval() -> u64 {
    5
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
struct OAuthError {
    error: String,
    #[serde(default)]
    error_description: Option<String>,
}

pub enum PollResult {
    Pending,
    SlowDown,
    Done(TokenResponse),
}

async fn post_form(http: &Http, url: &str, form: &[(&str, &str)]) -> CoreResult<(u16, Vec<u8>)> {
    let response = http
        .client()
        .post(url)
        .form(form)
        .send()
        .await
        .map_err(|source| http_error(url, source))?;
    read_body(url, response).await
}

fn oauth_failure(body: &[u8]) -> crate::CoreError {
    match serde_json::from_slice::<OAuthError>(body) {
        Ok(err) => {
            let reason = match err.error.as_str() {
                "authorization_declined" | "access_denied" => AuthFailure::Declined,
                "expired_token" | "code_expired" => AuthFailure::Expired,
                "invalid_grant" => AuthFailure::SessionExpired,
                _ => AuthFailure::Service,
            };
            reason.error(err.error_description.unwrap_or(err.error))
        }
        Err(_) => AuthFailure::Service.error("Microsoft login returned an unexpected response"),
    }
}

pub async fn request_device_code(
    http: &Http,
    endpoints: &Endpoints,
    client_id: &str,
) -> CoreResult<DeviceCode> {
    let url = format!("{}/devicecode", endpoints.oauth);
    let (status, body) =
        post_form(http, &url, &[("client_id", client_id), ("scope", SCOPE)]).await?;
    if status != 200 {
        return Err(oauth_failure(&body));
    }
    parse(&url, &body)
}

pub async fn poll_token(
    http: &Http,
    endpoints: &Endpoints,
    client_id: &str,
    device_code: &str,
) -> CoreResult<PollResult> {
    let url = format!("{}/token", endpoints.oauth);
    let (status, body) = post_form(
        http,
        &url,
        &[
            ("grant_type", DEVICE_GRANT),
            ("client_id", client_id),
            ("device_code", device_code),
        ],
    )
    .await?;
    if status == 200 {
        return parse(&url, &body).map(PollResult::Done);
    }
    match serde_json::from_slice::<OAuthError>(&body) {
        Ok(err) if err.error == "authorization_pending" => Ok(PollResult::Pending),
        Ok(err) if err.error == "slow_down" => Ok(PollResult::SlowDown),
        _ => Err(oauth_failure(&body)),
    }
}

pub async fn refresh(
    http: &Http,
    endpoints: &Endpoints,
    client_id: &str,
    refresh_token: &str,
) -> CoreResult<TokenResponse> {
    let url = format!("{}/token", endpoints.oauth);
    let (status, body) = post_form(
        http,
        &url,
        &[
            ("grant_type", "refresh_token"),
            ("client_id", client_id),
            ("refresh_token", refresh_token),
            ("scope", SCOPE),
        ],
    )
    .await?;
    if status != 200 {
        return Err(oauth_failure(&body));
    }
    parse(&url, &body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_matches_the_rfc_7636_example() {
        let pkce = Pkce::from_verifier("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk".into());
        assert_eq!(
            pkce.challenge,
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn generated_verifiers_are_long_and_unique() {
        let first = Pkce::generate();
        let second = Pkce::generate();
        assert_eq!(first.verifier.len(), 64);
        assert!(first.verifier.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(first.verifier, second.verifier);
    }

    #[test]
    fn authorize_url_asks_for_an_account() {
        let url = authorize_url(
            &Endpoints::default(),
            "client",
            "http://localhost:51234",
            "challenge",
            "state-1",
        )
        .unwrap();
        let parsed = Url::parse(&url).unwrap();
        assert_eq!(
            parsed.as_str().split('?').next(),
            Some("https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize")
        );
        let pairs: Vec<(String, String)> = parsed.query_pairs().into_owned().collect();
        for expected in [
            ("client_id", "client"),
            ("response_type", "code"),
            ("redirect_uri", "http://localhost:51234"),
            ("scope", SCOPE),
            ("state", "state-1"),
            ("code_challenge", "challenge"),
            ("code_challenge_method", "S256"),
            ("prompt", "select_account"),
        ] {
            assert!(
                pairs.contains(&(expected.0.into(), expected.1.into())),
                "missing {expected:?} in {url}"
            );
        }
    }
}

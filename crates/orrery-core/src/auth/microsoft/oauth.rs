use serde::Deserialize;

use super::{AuthFailure, Endpoints, parse, read_body};
use crate::CoreResult;
use crate::net::{Http, http_error};

const SCOPE: &str = "XboxLive.signin offline_access";
const DEVICE_GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";

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

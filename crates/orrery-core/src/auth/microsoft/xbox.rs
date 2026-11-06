use serde::Deserialize;
use serde_json::json;

use super::{AuthFailure, Endpoints, parse, read_body};
use crate::CoreResult;
use crate::net::{Http, http_error};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XboxResponse {
    token: String,
    display_claims: DisplayClaims,
}

#[derive(Debug, Deserialize)]
struct DisplayClaims {
    xui: Vec<UserClaim>,
}

#[derive(Debug, Deserialize)]
struct UserClaim {
    uhs: String,
}

#[derive(Debug, Deserialize)]
struct XstsError {
    #[serde(rename = "XErr")]
    xerr: u64,
}

#[derive(Debug, Clone)]
pub struct XstsToken {
    pub token: String,
    pub user_hash: String,
}

async fn post_json(http: &Http, url: &str, body: serde_json::Value) -> CoreResult<(u16, Vec<u8>)> {
    let response = http
        .client()
        .post(url)
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|source| http_error(url, source))?;
    read_body(url, response).await
}

fn into_token(url: &str, body: &[u8]) -> CoreResult<XstsToken> {
    let response: XboxResponse = parse(url, body)?;
    let user_hash = response
        .display_claims
        .xui
        .into_iter()
        .next()
        .map(|claim| claim.uhs)
        .ok_or_else(|| AuthFailure::Service.error("Xbox Live response has no user hash"))?;
    Ok(XstsToken {
        token: response.token,
        user_hash,
    })
}

pub fn xsts_failure(code: u64) -> crate::CoreError {
    match code {
        2148916233 => {
            AuthFailure::NoXboxAccount.error("this Microsoft account has no Xbox profile")
        }
        2148916235 => {
            AuthFailure::RegionUnavailable.error("Xbox Live is not available in this region")
        }
        2148916236 | 2148916237 => {
            AuthFailure::AdultVerificationRequired.error("the account needs adult verification")
        }
        2148916238 => AuthFailure::ChildAccount
            .error("child accounts must be added to a Microsoft family by an adult"),
        other => AuthFailure::Service.error(format!("Xbox Live rejected the login (XErr {other})")),
    }
}

pub async fn authenticate(
    http: &Http,
    endpoints: &Endpoints,
    ms_access_token: &str,
) -> CoreResult<XstsToken> {
    let url = &endpoints.xbl;
    let (status, body) = post_json(
        http,
        url,
        json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={ms_access_token}")
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        }),
    )
    .await?;
    if status != 200 {
        return Err(
            AuthFailure::Service.error(format!("Xbox Live login failed with HTTP {status}"))
        );
    }
    into_token(url, &body)
}

pub async fn authorize(
    http: &Http,
    endpoints: &Endpoints,
    xbl_token: &str,
) -> CoreResult<XstsToken> {
    let url = &endpoints.xsts;
    let (status, body) = post_json(
        http,
        url,
        json!({
            "Properties": {
                "SandboxId": "RETAIL",
                "UserTokens": [xbl_token]
            },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        }),
    )
    .await?;
    if status == 401 {
        let code = serde_json::from_slice::<XstsError>(&body)
            .map(|e| e.xerr)
            .unwrap_or(0);
        return Err(xsts_failure(code));
    }
    if status != 200 {
        return Err(
            AuthFailure::Service.error(format!("XSTS authorization failed with HTTP {status}"))
        );
    }
    into_token(url, &body)
}

use serde::Deserialize;
use serde_json::json;

use super::xbox::XstsToken;
use super::{AuthFailure, Endpoints, parse, read_body};
use crate::CoreResult;
use crate::net::{Http, http_error};

#[derive(Debug, Clone, Deserialize)]
pub struct MinecraftToken {
    pub access_token: String,
    pub expires_in: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub skins: Vec<ProfileSkin>,
    #[serde(default)]
    pub capes: Vec<ProfileCape>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProfileSkin {
    pub state: String,
    pub url: String,
    #[serde(default)]
    pub variant: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProfileCape {
    pub id: String,
    pub state: String,
    pub url: String,
    #[serde(default)]
    pub alias: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Entitlements {
    #[serde(default)]
    items: Vec<Entitlement>,
}

#[derive(Debug, Deserialize)]
struct Entitlement {
    name: String,
}

const OWNERSHIP_ENTITLEMENTS: [&str; 2] = ["product_minecraft", "game_minecraft"];

const MULTIPART_BOUNDARY: &str = "orrery-skin-upload-7f3a9c";

impl Profile {
    pub fn active_skin(&self) -> Option<&ProfileSkin> {
        self.skins
            .iter()
            .find(|skin| skin.state == "ACTIVE")
            .or_else(|| self.skins.first())
    }
}

pub async fn login(
    http: &Http,
    endpoints: &Endpoints,
    xsts: &XstsToken,
) -> CoreResult<MinecraftToken> {
    let url = format!("{}/authentication/login_with_xbox", endpoints.minecraft);
    let response = http
        .client()
        .post(&url)
        .header("Accept", "application/json")
        .json(&json!({
            "identityToken": format!("XBL3.0 x={};{}", xsts.user_hash, xsts.token)
        }))
        .send()
        .await
        .map_err(|source| http_error(&url, source))?;
    let (status, body) = read_body(&url, response).await?;
    match status {
        200 => parse(&url, &body),
        403 => Err(AuthFailure::AppNotApproved.error(
            "Minecraft services rejected this app registration; it must be approved by Mojang first",
        )),
        other => Err(AuthFailure::Service.error(format!("Minecraft login failed with HTTP {other}"))),
    }
}

pub async fn verify_ownership(
    http: &Http,
    endpoints: &Endpoints,
    access_token: &str,
) -> CoreResult<()> {
    let url = format!(
        "{}/entitlements/license?requestId={}",
        endpoints.minecraft,
        uuid::Uuid::new_v4()
    );
    let response = http
        .client()
        .get(&url)
        .bearer_auth(access_token)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|source| http_error(&url, source))?;
    let (status, body) = read_body(&url, response).await?;
    match status {
        200 => {
            let entitlements: Entitlements = parse(&url, &body)?;
            if entitlements
                .items
                .iter()
                .any(|item| OWNERSHIP_ENTITLEMENTS.contains(&item.name.as_str()))
            {
                Ok(())
            } else {
                Err(AuthFailure::NoMinecraftProfile
                    .error("this account does not own Minecraft: Java Edition"))
            }
        }
        other => Err(AuthFailure::Service
            .error(format!("the game ownership check failed with HTTP {other}"))),
    }
}

async fn profile_response(url: &str, response: reqwest::Response) -> CoreResult<Profile> {
    let (status, body) = read_body(url, response).await?;
    match status {
        200 => parse(url, &body),
        401 => {
            Err(AuthFailure::SessionExpired
                .error("the Minecraft session expired, please try again"))
        }
        429 => Err(AuthFailure::Service.error(
            "Minecraft services allow only a few profile changes per minute, please wait a moment",
        )),
        other => {
            Err(AuthFailure::Service.error(format!("the profile change failed with HTTP {other}")))
        }
    }
}

pub fn skin_upload_body(png: &[u8], slim: bool) -> Vec<u8> {
    let variant = if slim { "slim" } else { "classic" };
    let mut body = Vec::with_capacity(png.len() + 512);
    body.extend(
        format!(
            "--{MULTIPART_BOUNDARY}\r\nContent-Disposition: form-data; name=\"variant\"\r\n\r\n{variant}\r\n--{MULTIPART_BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"skin.png\"\r\nContent-Type: image/png\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend(png);
    body.extend(format!("\r\n--{MULTIPART_BOUNDARY}--\r\n").as_bytes());
    body
}

pub async fn upload_skin(
    http: &Http,
    endpoints: &Endpoints,
    access_token: &str,
    png: &[u8],
    slim: bool,
) -> CoreResult<Profile> {
    let url = format!("{}/minecraft/profile/skins", endpoints.minecraft);
    let response = http
        .client()
        .post(&url)
        .bearer_auth(access_token)
        .header(
            "Content-Type",
            format!("multipart/form-data; boundary={MULTIPART_BOUNDARY}"),
        )
        .body(skin_upload_body(png, slim))
        .send()
        .await
        .map_err(|source| http_error(&url, source))?;
    profile_response(&url, response).await
}

pub async fn set_cape(
    http: &Http,
    endpoints: &Endpoints,
    access_token: &str,
    cape_id: Option<&str>,
) -> CoreResult<Profile> {
    let url = format!("{}/minecraft/profile/capes/active", endpoints.minecraft);
    let request = match cape_id {
        Some(id) => http.client().put(&url).json(&json!({ "capeId": id })),
        None => http.client().delete(&url),
    };
    let response = request
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|source| http_error(&url, source))?;
    profile_response(&url, response).await
}

pub async fn profile(
    http: &Http,
    endpoints: &Endpoints,
    access_token: &str,
) -> CoreResult<Profile> {
    let url = format!("{}/minecraft/profile", endpoints.minecraft);
    let response = http
        .client()
        .get(&url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|source| http_error(&url, source))?;
    let (status, body) = read_body(&url, response).await?;
    match status {
        200 => parse(&url, &body),
        404 => Err(AuthFailure::NoMinecraftProfile
            .error("this account does not own Minecraft: Java Edition or has no profile yet")),
        other => {
            Err(AuthFailure::Service.error(format!("profile request failed with HTTP {other}")))
        }
    }
}

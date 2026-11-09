mod loopback;
mod minecraft;
mod oauth;
mod session;
mod xbox;

use serde::Serialize;
use ts_rs::TS;

pub use loopback::LoginPage;
pub use session::{
    BrowserLoginInfo, DeviceCodeInfo, LoginEvent, LoginListener, LoginStatus, MicrosoftAuth,
};

#[derive(Debug, Clone)]
pub struct Endpoints {
    pub oauth: String,
    pub xbl: String,
    pub xsts: String,
    pub minecraft: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        Self {
            oauth: "https://login.microsoftonline.com/consumers/oauth2/v2.0".into(),
            xbl: "https://user.auth.xboxlive.com/user/authenticate".into(),
            xsts: "https://xsts.auth.xboxlive.com/xsts/authorize".into(),
            minecraft: "https://api.minecraftservices.com".into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum AuthFailure {
    NotConfigured,
    Declined,
    Expired,
    NoXboxAccount,
    ChildAccount,
    RegionUnavailable,
    AdultVerificationRequired,
    AppNotApproved,
    NoMinecraftProfile,
    SessionExpired,
    Service,
}

impl AuthFailure {
    pub fn code(self) -> &'static str {
        match self {
            Self::NotConfigured => "auth_not_configured",
            Self::Declined => "auth_declined",
            Self::Expired => "auth_expired",
            Self::NoXboxAccount => "auth_no_xbox_account",
            Self::ChildAccount => "auth_child_account",
            Self::RegionUnavailable => "auth_region_unavailable",
            Self::AdultVerificationRequired => "auth_adult_verification",
            Self::AppNotApproved => "auth_app_not_approved",
            Self::NoMinecraftProfile => "auth_no_minecraft_profile",
            Self::SessionExpired => "auth_session_expired",
            Self::Service => "auth_service",
        }
    }

    pub fn error(self, message: impl Into<String>) -> crate::CoreError {
        crate::CoreError::Auth {
            reason: self,
            message: message.into(),
        }
    }
}

async fn read_body(url: &str, response: reqwest::Response) -> crate::CoreResult<(u16, Vec<u8>)> {
    let status = response.status().as_u16();
    let body = response
        .bytes()
        .await
        .map_err(|source| crate::net::http_error(url, source))?;
    Ok((status, body.to_vec()))
}

fn parse<T: serde::de::DeserializeOwned>(url: &str, body: &[u8]) -> crate::CoreResult<T> {
    serde_json::from_slice(body)
        .map_err(|err| AuthFailure::Service.error(format!("unexpected response from {url}: {err}")))
}

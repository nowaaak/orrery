use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tokio_util::sync::CancellationToken;
use ts_rs::TS;

use super::minecraft::{self, Profile};
use super::oauth::{self, PollResult};
use super::{AuthFailure, Endpoints, xbox};
use crate::CoreResult;
use crate::auth::account::{Account, AccountKind, Skin};
use crate::auth::secrets::SecretStore;
use crate::auth::store::AccountStore;
use crate::minecraft::arguments::LaunchCredentials;
use crate::net::Http;
use crate::storage::unix_now;

const REFRESH_MARGIN_SECS: i64 = 5 * 60;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DeviceCodeInfo {
    pub login_id: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(export)]
pub enum LoginStatus {
    Success { account: Account },
    Failed { kind: String, message: String },
    Cancelled,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LoginEvent {
    pub login_id: String,
    pub status: LoginStatus,
}

pub type LoginListener = Arc<dyn Fn(LoginEvent) + Send + Sync>;

#[derive(Clone)]
struct Session {
    access_token: String,
    expires_at: i64,
    uuid: String,
    name: String,
}

pub struct MicrosoftAuth {
    http: Http,
    endpoints: Endpoints,
    client_id: Option<String>,
    secrets: Arc<dyn SecretStore>,
    accounts: Arc<AccountStore>,
    listener: LoginListener,
    sessions: Mutex<HashMap<String, Session>>,
    refresh_locks: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
    logins: Mutex<HashMap<String, CancellationToken>>,
}

struct Authenticated {
    refresh_token: String,
    profile: Profile,
    session: Session,
}

fn secret_key(account_id: &str) -> String {
    format!("msa:{account_id}")
}

fn session_key(account_id: &str) -> String {
    format!("msa-session:{account_id}")
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

impl MicrosoftAuth {
    pub fn new(
        http: Http,
        endpoints: Endpoints,
        client_id: Option<String>,
        secrets: Arc<dyn SecretStore>,
        accounts: Arc<AccountStore>,
        listener: LoginListener,
    ) -> Arc<Self> {
        Arc::new(Self {
            http,
            endpoints,
            client_id: client_id.filter(|id| !id.trim().is_empty()),
            secrets,
            accounts,
            listener,
            sessions: Mutex::new(HashMap::new()),
            refresh_locks: Mutex::new(HashMap::new()),
            logins: Mutex::new(HashMap::new()),
        })
    }

    pub fn is_configured(&self) -> bool {
        self.client_id.is_some()
    }

    fn client_id(&self) -> CoreResult<&str> {
        self.client_id.as_deref().ok_or_else(|| {
            AuthFailure::NotConfigured
                .error("this build has no Microsoft client id (set MS_CLIENT_ID at build time)")
        })
    }

    pub async fn begin_login(self: &Arc<Self>) -> CoreResult<DeviceCodeInfo> {
        let client_id = self.client_id()?.to_string();
        let code = oauth::request_device_code(&self.http, &self.endpoints, &client_id).await?;
        let login_id = uuid::Uuid::new_v4().to_string();
        let cancel = CancellationToken::new();
        lock(&self.logins).insert(login_id.clone(), cancel.clone());

        let info = DeviceCodeInfo {
            login_id: login_id.clone(),
            user_code: code.user_code.clone(),
            verification_uri: code.verification_uri.clone(),
            expires_in: code.expires_in,
        };

        let auth = self.clone();
        tokio::spawn(async move {
            let status = tokio::select! {
                result = auth.complete_login(&client_id, code) => match result {
                    Ok(account) => LoginStatus::Success { account },
                    Err(err) => {
                        tracing::warn!(error = %err, "microsoft login failed");
                        LoginStatus::Failed { kind: err.kind().to_string(), message: err.to_string() }
                    }
                },
                () = cancel.cancelled() => LoginStatus::Cancelled,
            };
            lock(&auth.logins).remove(&login_id);
            (auth.listener)(LoginEvent { login_id, status });
        });
        Ok(info)
    }

    pub fn cancel_login(&self, login_id: &str) {
        if let Some(token) = lock(&self.logins).get(login_id) {
            token.cancel();
        }
    }

    async fn complete_login(
        &self,
        client_id: &str,
        code: oauth::DeviceCode,
    ) -> CoreResult<Account> {
        let deadline = unix_now() + code.expires_in as i64;
        let mut interval = code.interval.max(1);
        let tokens = loop {
            tokio::time::sleep(Duration::from_secs(interval)).await;
            if unix_now() > deadline {
                return Err(AuthFailure::Expired.error("the login code expired"));
            }
            match oauth::poll_token(&self.http, &self.endpoints, client_id, &code.device_code)
                .await?
            {
                PollResult::Pending => {}
                PollResult::SlowDown => interval += 5,
                PollResult::Done(tokens) => break tokens,
            }
        };

        let authenticated = self.authenticate(tokens).await?;
        let account = self
            .accounts
            .upsert(Account {
                id: uuid::Uuid::new_v4().to_string(),
                kind: AccountKind::Microsoft,
                uuid: authenticated.profile.id.clone(),
                username: authenticated.profile.name.clone(),
                skin: skin_of(&authenticated.profile),
                added_at: unix_now(),
            })
            .await?;
        self.secrets
            .set(&secret_key(&account.id), &authenticated.refresh_token)?;
        self.remember(&account.id, authenticated.session);
        Ok(account)
    }

    async fn authenticate(&self, tokens: oauth::TokenResponse) -> CoreResult<Authenticated> {
        let xbl = xbox::authenticate(&self.http, &self.endpoints, &tokens.access_token).await?;
        let xsts = xbox::authorize(&self.http, &self.endpoints, &xbl.token).await?;
        let minecraft = minecraft::login(&self.http, &self.endpoints, &xsts).await?;
        minecraft::verify_ownership(&self.http, &self.endpoints, &minecraft.access_token).await?;
        let profile =
            minecraft::profile(&self.http, &self.endpoints, &minecraft.access_token).await?;
        let session = Session {
            access_token: minecraft.access_token,
            expires_at: unix_now() + minecraft.expires_in as i64,
            uuid: profile.id.clone(),
            name: profile.name.clone(),
        };
        Ok(Authenticated {
            refresh_token: tokens.refresh_token,
            profile,
            session,
        })
    }

    fn refresh_lock(&self, account_id: &str) -> Arc<tokio::sync::Mutex<()>> {
        lock(&self.refresh_locks)
            .entry(account_id.to_string())
            .or_default()
            .clone()
    }

    async fn valid_session(&self, account: &Account) -> CoreResult<Session> {
        let refresh_lock = self.refresh_lock(&account.id);
        let _guard = refresh_lock.lock().await;

        let cached = lock(&self.sessions).get(&account.id).cloned();
        if let Some(session) = cached
            && session.expires_at - REFRESH_MARGIN_SECS > unix_now()
        {
            return Ok(session);
        }
        self.verified_session(account).await
    }

    async fn verified_session(&self, account: &Account) -> CoreResult<Session> {
        match self.refresh_account(account).await {
            Ok(refreshed) => Ok(refreshed.session),
            Err(err) => {
                lock(&self.sessions).remove(&account.id);
                Err(err)
            }
        }
    }

    fn remember(&self, account_id: &str, session: Session) {
        lock(&self.sessions).insert(account_id.to_string(), session);
    }

    pub async fn discard_stored_sessions(&self) {
        for account in self.accounts.view().await.accounts {
            if let Err(err) = self.secrets.delete(&session_key(&account.id)) {
                tracing::warn!(error = %err, "could not remove a stored Minecraft session");
            }
        }
    }

    async fn store_profile(&self, account: &Account, profile: &Profile) -> CoreResult<()> {
        self.accounts
            .update(&account.id, |stored| {
                stored.username = profile.name.clone();
                stored.skin = skin_of(profile);
            })
            .await
            .map(|_| ())
    }

    pub async fn profile(&self, account: &Account) -> CoreResult<Profile> {
        let session = self.valid_session(account).await?;
        let profile =
            minecraft::profile(&self.http, &self.endpoints, &session.access_token).await?;
        self.store_profile(account, &profile).await?;
        Ok(profile)
    }

    pub async fn upload_skin(
        &self,
        account: &Account,
        png: &[u8],
        slim: bool,
    ) -> CoreResult<Profile> {
        let session = self.valid_session(account).await?;
        let profile = minecraft::upload_skin(
            &self.http,
            &self.endpoints,
            &session.access_token,
            png,
            slim,
        )
        .await?;
        self.store_profile(account, &profile).await?;
        Ok(profile)
    }

    pub async fn set_cape(&self, account: &Account, cape_id: Option<&str>) -> CoreResult<Profile> {
        let session = self.valid_session(account).await?;
        let profile =
            minecraft::set_cape(&self.http, &self.endpoints, &session.access_token, cape_id)
                .await?;
        self.store_profile(account, &profile).await?;
        Ok(profile)
    }

    pub async fn launch_credentials(&self, account: &Account) -> CoreResult<LaunchCredentials> {
        let refresh_lock = self.refresh_lock(&account.id);
        let _guard = refresh_lock.lock().await;
        let session = self.verified_session(account).await?;
        Ok(LaunchCredentials {
            username: session.name,
            uuid: session.uuid,
            access_token: session.access_token,
            user_type: "msa".into(),
            xuid: None,
        })
    }

    async fn refresh_account(&self, account: &Account) -> CoreResult<Authenticated> {
        let client_id = self.client_id()?;
        let key = secret_key(&account.id);
        let refresh_token = self.secrets.get(&key)?.ok_or_else(|| {
            AuthFailure::SessionExpired.error("no stored login, please sign in again")
        })?;
        let tokens = oauth::refresh(&self.http, &self.endpoints, client_id, &refresh_token).await?;
        self.secrets.set(&key, &tokens.refresh_token)?;
        let authenticated = self.authenticate(tokens).await?;
        if authenticated.profile.id != account.uuid {
            return Err(
                AuthFailure::Service.error("the stored login belongs to a different profile")
            );
        }
        let profile = authenticated.profile.clone();
        self.accounts
            .update(&account.id, |stored| {
                stored.username = profile.name.clone();
                stored.skin = skin_of(&profile);
            })
            .await?;
        self.remember(&account.id, authenticated.session.clone());
        Ok(authenticated)
    }

    pub fn forget(&self, account_id: &str) -> CoreResult<()> {
        lock(&self.sessions).remove(account_id);
        self.secrets.delete(&session_key(account_id))?;
        self.secrets.delete(&secret_key(account_id))
    }
}

fn skin_of(profile: &Profile) -> Option<Skin> {
    profile.active_skin().map(|skin| Skin::Remote {
        url: skin.url.replace("http://", "https://"),
        slim: skin.variant.as_deref() == Some("SLIM"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::secrets::MemorySecrets;
    use wiremock::matchers::{body_string_contains, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn mock_chain(server: &MockServer, xsts_status: u16, xsts_body: serde_json::Value) {
        Mock::given(method("POST"))
            .and(path("/oauth/devicecode"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "device_code": "dev", "user_code": "ABCD-1234",
                "verification_uri": "https://microsoft.com/link", "expires_in": 900, "interval": 1
            })))
            .mount(server)
            .await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .and(body_string_contains("device_code"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "ms-access", "refresh_token": "refresh-1"
            })))
            .mount(server)
            .await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .and(body_string_contains("grant_type=refresh_token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "ms-access-2", "refresh_token": "refresh-2"
            })))
            .mount(server)
            .await;
        Mock::given(method("POST"))
            .and(path("/xbl"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "Token": "xbl-token", "DisplayClaims": { "xui": [{ "uhs": "hash" }] }
            })))
            .mount(server)
            .await;
        Mock::given(method("POST"))
            .and(path("/xsts"))
            .respond_with(ResponseTemplate::new(xsts_status).set_body_json(xsts_body))
            .mount(server)
            .await;
        Mock::given(method("POST"))
            .and(path("/mc/authentication/login_with_xbox"))
            .and(body_string_contains("XBL3.0 x=hash;xsts-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "mc-token", "expires_in": 86400
            })))
            .mount(server)
            .await;
        Mock::given(method("GET"))
            .and(path("/mc/entitlements/license"))
            .and(header("Authorization", "Bearer mc-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [{ "name": "product_minecraft", "signature": "x" }, { "name": "game_minecraft", "signature": "y" }],
                "signature": "z", "keyId": "1"
            })))
            .mount(server)
            .await;
        Mock::given(method("GET"))
            .and(path("/mc/minecraft/profile"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "0123456789abcdef0123456789abcdef", "name": "Alex",
                "skins": [{ "state": "ACTIVE", "url": "http://textures.minecraft.net/texture/x", "variant": "SLIM" }]
            })))
            .mount(server)
            .await;
    }

    fn endpoints(server: &MockServer) -> Endpoints {
        Endpoints {
            oauth: format!("{}/oauth", server.uri()),
            xbl: format!("{}/xbl", server.uri()),
            xsts: format!("{}/xsts", server.uri()),
            minecraft: format!("{}/mc", server.uri()),
        }
    }

    async fn setup(
        server: &MockServer,
    ) -> (
        Arc<MicrosoftAuth>,
        Arc<MemorySecrets>,
        tokio::sync::mpsc::UnboundedReceiver<LoginEvent>,
        tempfile::TempDir,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let accounts = Arc::new(
            AccountStore::load(dir.path().join("accounts.json"))
                .await
                .unwrap(),
        );
        let secrets = Arc::new(MemorySecrets::default());
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let auth = MicrosoftAuth::new(
            Http::new("test").unwrap(),
            endpoints(server),
            Some("client".into()),
            secrets.clone(),
            accounts,
            Arc::new(move |event| {
                let _ = tx.send(event);
            }),
        );
        (auth, secrets, rx, dir)
    }

    #[tokio::test]
    async fn device_code_login_then_refresh_rotates_token() {
        let server = MockServer::start().await;
        let xsts_ok = serde_json::json!({ "Token": "xsts-token", "DisplayClaims": { "xui": [{ "uhs": "hash" }] } });
        mock_chain(&server, 200, xsts_ok).await;
        let (auth, secrets, mut rx, _dir) = setup(&server).await;

        let info = auth.begin_login().await.unwrap();
        assert_eq!(info.user_code, "ABCD-1234");
        let event = tokio::time::timeout(Duration::from_secs(10), rx.recv())
            .await
            .unwrap()
            .unwrap();
        let LoginStatus::Success { account } = event.status else {
            panic!("login failed: {:?}", event.status);
        };
        assert_eq!(account.username, "Alex");
        assert_eq!(
            account.skin,
            Some(Skin::Remote {
                url: "https://textures.minecraft.net/texture/x".into(),
                slim: true
            })
        );
        let requests = server.received_requests().await.unwrap();
        assert!(requests.iter().any(|request| {
            request.url.path() == "/mc/entitlements/license"
                && request.url.query_pairs().any(|(name, value)| {
                    name == "requestId" && uuid::Uuid::parse_str(&value).is_ok()
                })
        }));
        let key = secret_key(&account.id);
        assert_eq!(secrets.get(&key).unwrap().as_deref(), Some("refresh-1"));

        assert_eq!(secrets.get(&session_key(&account.id)).unwrap(), None);
        let credentials = auth.launch_credentials(&account).await.unwrap();
        assert_eq!(credentials.access_token, "mc-token");
        assert_eq!(credentials.user_type, "msa");
        assert_eq!(secrets.get(&key).unwrap().as_deref(), Some("refresh-2"));
        assert_eq!(secrets.get(&session_key(&account.id)).unwrap(), None);

        auth.forget(&account.id).unwrap();
        assert_eq!(secrets.get(&key).unwrap(), None);
        assert!(lock(&auth.sessions).is_empty());
    }

    async fn signed_in(
        server: &MockServer,
    ) -> (
        Arc<MicrosoftAuth>,
        Arc<MemorySecrets>,
        Account,
        tempfile::TempDir,
    ) {
        let (auth, secrets, mut rx, dir) = setup(server).await;
        auth.begin_login().await.unwrap();
        let event = tokio::time::timeout(Duration::from_secs(10), rx.recv())
            .await
            .unwrap()
            .unwrap();
        let LoginStatus::Success { account } = event.status else {
            panic!("login failed: {:?}", event.status);
        };
        (auth, secrets, account, dir)
    }

    fn unreachable_endpoints() -> Endpoints {
        Endpoints {
            oauth: "http://127.0.0.1:9/oauth".into(),
            xbl: "http://127.0.0.1:9/xbl".into(),
            xsts: "http://127.0.0.1:9/xsts".into(),
            minecraft: "http://127.0.0.1:9/mc".into(),
        }
    }

    #[tokio::test]
    async fn every_launch_needs_an_online_check() {
        let server = MockServer::start().await;
        let xsts_ok = serde_json::json!({ "Token": "xsts-token", "DisplayClaims": { "xui": [{ "uhs": "hash" }] } });
        mock_chain(&server, 200, xsts_ok).await;
        let (auth, secrets, account, dir) = signed_in(&server).await;
        assert!(lock(&auth.sessions).contains_key(&account.id));

        let restarted = MicrosoftAuth::new(
            Http::new("test").unwrap(),
            unreachable_endpoints(),
            Some("client".into()),
            secrets.clone(),
            Arc::new(
                AccountStore::load(dir.path().join("accounts.json"))
                    .await
                    .unwrap(),
            ),
            Arc::new(|_| {}),
        );
        lock(&restarted.sessions).insert(
            account.id.clone(),
            lock(&auth.sessions).get(&account.id).cloned().unwrap(),
        );
        assert!(restarted.launch_credentials(&account).await.is_err());
        assert!(lock(&restarted.sessions).is_empty());
    }

    #[tokio::test]
    async fn losing_the_game_blocks_the_next_launch() {
        let server = MockServer::start().await;
        let xsts_ok = serde_json::json!({ "Token": "xsts-token", "DisplayClaims": { "xui": [{ "uhs": "hash" }] } });
        mock_chain(&server, 200, xsts_ok).await;
        let (auth, _secrets, account, _dir) = signed_in(&server).await;

        Mock::given(method("GET"))
            .and(path("/mc/entitlements/license"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [], "signature": "z", "keyId": "1"
            })))
            .with_priority(1)
            .mount(&server)
            .await;
        let err = auth.launch_credentials(&account).await.unwrap_err();
        assert_eq!(err.kind(), "auth_no_minecraft_profile");
        assert!(lock(&auth.sessions).is_empty());
    }

    #[tokio::test]
    async fn launch_uses_the_verified_profile() {
        let server = MockServer::start().await;
        let xsts_ok = serde_json::json!({ "Token": "xsts-token", "DisplayClaims": { "xui": [{ "uhs": "hash" }] } });
        mock_chain(&server, 200, xsts_ok).await;
        let (auth, _secrets, account, _dir) = signed_in(&server).await;

        let mut edited = account.clone();
        edited.username = "Someone".into();
        edited.uuid = "ffffffffffffffffffffffffffffffff".into();
        let err = auth.launch_credentials(&edited).await.unwrap_err();
        assert!(err.to_string().contains("different profile"), "{err}");

        let mut renamed = account.clone();
        renamed.username = "Someone".into();
        let credentials = auth.launch_credentials(&renamed).await.unwrap();
        assert_eq!(credentials.username, "Alex");
        assert_eq!(credentials.uuid, "0123456789abcdef0123456789abcdef");
    }

    #[tokio::test]
    async fn stored_sessions_from_older_versions_are_removed() {
        let server = MockServer::start().await;
        let xsts_ok = serde_json::json!({ "Token": "xsts-token", "DisplayClaims": { "xui": [{ "uhs": "hash" }] } });
        mock_chain(&server, 200, xsts_ok).await;
        let (auth, secrets, account, _dir) = signed_in(&server).await;
        secrets
            .set(
                &session_key(&account.id),
                "{\"access_token\":\"old\",\"expires_at\":0}",
            )
            .unwrap();
        auth.discard_stored_sessions().await;
        assert_eq!(secrets.get(&session_key(&account.id)).unwrap(), None);
        assert!(secrets.get(&secret_key(&account.id)).unwrap().is_some());
    }

    #[tokio::test]
    async fn uploads_skins_and_switches_capes() {
        let server = MockServer::start().await;
        let xsts_ok = serde_json::json!({ "Token": "xsts-token", "DisplayClaims": { "xui": [{ "uhs": "hash" }] } });
        mock_chain(&server, 200, xsts_ok).await;
        let uploaded = serde_json::json!({
            "id": "0123456789abcdef0123456789abcdef",
            "name": "Alex",
            "skins": [{ "state": "ACTIVE", "url": "http://textures.minecraft.net/texture/new", "variant": "CLASSIC" }],
            "capes": [{ "id": "cape-1", "state": "ACTIVE", "url": "http://textures.minecraft.net/texture/cape", "alias": "Migrator" }]
        });
        Mock::given(method("POST"))
            .and(path("/mc/minecraft/profile/skins"))
            .and(body_string_contains("name=\"variant\"\r\n\r\nclassic"))
            .and(body_string_contains("filename=\"skin.png\""))
            .respond_with(ResponseTemplate::new(200).set_body_json(uploaded.clone()))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("PUT"))
            .and(path("/mc/minecraft/profile/capes/active"))
            .and(body_string_contains("cape-1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(uploaded.clone()))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/mc/minecraft/profile/capes/active"))
            .respond_with(ResponseTemplate::new(429))
            .expect(1)
            .mount(&server)
            .await;
        let (auth, _secrets, mut rx, _dir) = setup(&server).await;
        auth.begin_login().await.unwrap();
        let event = tokio::time::timeout(Duration::from_secs(10), rx.recv())
            .await
            .unwrap()
            .unwrap();
        let LoginStatus::Success { account } = event.status else {
            panic!("login failed: {:?}", event.status);
        };

        let png = *b"PNG-DATA";
        let profile = auth.upload_skin(&account, &png, false).await.unwrap();
        assert_eq!(profile.capes[0].alias.as_deref(), Some("Migrator"));
        let stored = auth.accounts.get(&account.id).await.unwrap();
        assert_eq!(
            stored.skin,
            Some(Skin::Remote {
                url: "https://textures.minecraft.net/texture/new".into(),
                slim: false
            })
        );
        auth.set_cape(&account, Some("cape-1")).await.unwrap();
        let limited = auth.set_cape(&account, None).await.unwrap_err();
        assert!(limited.to_string().contains("per minute"), "{limited}");
    }

    #[test]
    fn skin_upload_body_is_multipart() {
        let body = String::from_utf8(minecraft::skin_upload_body(b"PNG", true)).unwrap();
        assert!(body.starts_with("--orrery-skin-upload-7f3a9c\r\n"));
        assert!(body.contains("name=\"variant\"\r\n\r\nslim\r\n"));
        assert!(body.contains("Content-Type: image/png\r\n\r\nPNG\r\n"));
        assert!(body.ends_with("--orrery-skin-upload-7f3a9c--\r\n"));
    }

    #[tokio::test]
    async fn xsts_child_account_error_is_reported() {
        let server = MockServer::start().await;
        mock_chain(&server, 401, serde_json::json!({ "XErr": 2148916238u64 })).await;
        let (auth, _secrets, mut rx, _dir) = setup(&server).await;

        auth.begin_login().await.unwrap();
        let event = tokio::time::timeout(Duration::from_secs(10), rx.recv())
            .await
            .unwrap()
            .unwrap();
        match event.status {
            LoginStatus::Failed { kind, .. } => assert_eq!(kind, "auth_child_account"),
            other => panic!("unexpected status {other:?}"),
        }
    }

    #[tokio::test]
    async fn accounts_without_the_game_cannot_sign_in() {
        let server = MockServer::start().await;
        let xsts_ok = serde_json::json!({ "Token": "xsts-token", "DisplayClaims": { "xui": [{ "uhs": "hash" }] } });
        mock_chain(&server, 200, xsts_ok).await;
        Mock::given(method("GET"))
            .and(path("/mc/entitlements/license"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [], "signature": "z", "keyId": "1"
            })))
            .with_priority(1)
            .mount(&server)
            .await;
        let (auth, _secrets, mut rx, _dir) = setup(&server).await;

        auth.begin_login().await.unwrap();
        let event = tokio::time::timeout(Duration::from_secs(10), rx.recv())
            .await
            .unwrap()
            .unwrap();
        match event.status {
            LoginStatus::Failed { kind, .. } => assert_eq!(kind, "auth_no_minecraft_profile"),
            other => panic!("unexpected status {other:?}"),
        }
        assert!(auth.accounts.view().await.accounts.is_empty());
        assert!(lock(&auth.sessions).is_empty());
    }

    #[tokio::test]
    async fn missing_client_id_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let accounts = Arc::new(AccountStore::load(dir.path().join("a.json")).await.unwrap());
        let auth = MicrosoftAuth::new(
            Http::new("test").unwrap(),
            Endpoints::default(),
            None,
            Arc::new(MemorySecrets::default()),
            accounts,
            Arc::new(|_| {}),
        );
        assert!(!auth.is_configured());
        let err = auth.begin_login().await.unwrap_err();
        assert_eq!(err.kind(), "auth_not_configured");
    }
}

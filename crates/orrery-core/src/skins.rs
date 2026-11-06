use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use ts_rs::TS;

use crate::error::IoResultExt;
use crate::net::{Http, ensure_success, http_error};
use crate::services::Services;
use crate::storage::{self, Versioned, unix_now};
use crate::{CoreError, CoreResult};

const LIBRARY_FILE: &str = "library.json";
const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
const MAX_SKIN_BYTES: u64 = 512 * 1024;
const MAX_TEXTURE_BYTES: usize = 1024 * 1024;
const MAX_NAME_CHARS: usize = 48;
const TEXTURE_PREFIX: &str = "https://textures.minecraft.net/texture/";
pub const MOJANG_API: &str = "https://api.mojang.com";
pub const MOJANG_SESSION: &str = "https://sessionserver.mojang.com";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LibrarySkin {
    pub id: String,
    pub name: String,
    pub slim: bool,
    pub added_at: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct LibraryFile {
    schema: u32,
    #[serde(default)]
    skins: Vec<LibrarySkin>,
}

impl Versioned for LibraryFile {
    const SCHEMA: u32 = 1;
    fn schema(&self) -> u32 {
        self.schema
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProfileCapeView {
    pub id: String,
    pub alias: Option<String>,
    pub url: String,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProfileView {
    pub skin_url: Option<String>,
    pub slim: bool,
    pub capes: Vec<ProfileCapeView>,
}

pub struct SkinLibrary {
    dir: PathBuf,
    lock: Mutex<()>,
}

fn validate_id(id: &str) -> CoreResult<()> {
    uuid::Uuid::parse_str(id)
        .map(|_| ())
        .map_err(|_| CoreError::InvalidInput(format!("invalid skin id {id:?}")))
}

fn clean_name(name: &str) -> CoreResult<String> {
    let name = name.trim();
    if name.is_empty()
        || name.chars().count() > MAX_NAME_CHARS
        || name.chars().any(char::is_control)
    {
        return Err(CoreError::InvalidInput(format!(
            "skin names must have between 1 and {MAX_NAME_CHARS} characters"
        )));
    }
    Ok(name.to_string())
}

impl SkinLibrary {
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            lock: Mutex::new(()),
        }
    }

    async fn load(&self) -> CoreResult<LibraryFile> {
        Ok(
            storage::read_versioned::<LibraryFile>(&self.dir.join(LIBRARY_FILE))
                .await?
                .unwrap_or(LibraryFile {
                    schema: LibraryFile::SCHEMA,
                    skins: Vec::new(),
                }),
        )
    }

    fn png_path(&self, id: &str) -> CoreResult<PathBuf> {
        validate_id(id)?;
        Ok(self.dir.join(format!("{id}.png")))
    }

    pub async fn list(&self) -> CoreResult<Vec<LibrarySkin>> {
        Ok(self.load().await?.skins)
    }

    pub async fn get(&self, id: &str) -> CoreResult<LibrarySkin> {
        self.list()
            .await?
            .into_iter()
            .find(|skin| skin.id == id)
            .ok_or_else(|| CoreError::NotFound(format!("skin {id}")))
    }

    pub async fn bytes(&self, id: &str) -> CoreResult<Vec<u8>> {
        let path = self.png_path(id)?;
        tokio::fs::read(&path).await.at_path(&path)
    }

    pub async fn add(&self, name: &str, png: &[u8], slim: bool) -> CoreResult<LibrarySkin> {
        let name = clean_name(name)?;
        validate_skin_png(png)?;
        let _guard = self.lock.lock().await;
        let mut file = self.load().await?;
        let skin = LibrarySkin {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            slim,
            added_at: unix_now(),
        };
        storage::write_bytes_atomic(&self.png_path(&skin.id)?, png).await?;
        file.skins.insert(0, skin.clone());
        storage::write_json_atomic(&self.dir.join(LIBRARY_FILE), &file).await?;
        Ok(skin)
    }

    pub async fn update(&self, id: &str, name: &str, slim: bool) -> CoreResult<LibrarySkin> {
        let name = clean_name(name)?;
        let _guard = self.lock.lock().await;
        let mut file = self.load().await?;
        let skin = file
            .skins
            .iter_mut()
            .find(|skin| skin.id == id)
            .ok_or_else(|| CoreError::NotFound(format!("skin {id}")))?;
        skin.name = name;
        skin.slim = slim;
        let updated = skin.clone();
        storage::write_json_atomic(&self.dir.join(LIBRARY_FILE), &file).await?;
        Ok(updated)
    }

    pub async fn remove(&self, id: &str) -> CoreResult<()> {
        let path = self.png_path(id)?;
        let _guard = self.lock.lock().await;
        let mut file = self.load().await?;
        let before = file.skins.len();
        file.skins.retain(|skin| skin.id != id);
        if file.skins.len() == before {
            return Err(CoreError::NotFound(format!("skin {id}")));
        }
        storage::write_json_atomic(&self.dir.join(LIBRARY_FILE), &file).await?;
        match tokio::fs::remove_file(&path).await {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err).at_path(&path),
        }
    }
}

pub fn decode_base64(input: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut buffer = 0u32;
    let mut bits = 0;
    for byte in input.bytes().filter(|b| !b.is_ascii_whitespace()) {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' => break,
            _ => return None,
        };
        buffer = (buffer << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    Some(out)
}

pub fn validate_texture_url(url: &str) -> CoreResult<String> {
    let url = url.trim().replacen("http://", "https://", 1);
    let hash = url.strip_prefix(TEXTURE_PREFIX).unwrap_or_default();
    if hash.is_empty() || hash.len() > 128 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(CoreError::InvalidInput(format!(
            "only Minecraft texture URLs can be loaded, not {url:?}"
        )));
    }
    Ok(url)
}

pub async fn fetch_texture(http: &Http, url: &str) -> CoreResult<Vec<u8>> {
    let url = validate_texture_url(url)?;
    let bytes = http.get_bytes(&url).await?;
    if bytes.len() > MAX_TEXTURE_BYTES || !bytes.starts_with(b"\x89PNG") {
        return Err(CoreError::InvalidInput(
            "the texture is not a PNG image".into(),
        ));
    }
    Ok(bytes)
}

#[derive(Debug, Deserialize)]
struct NameLookup {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct SessionProfile {
    #[serde(default)]
    properties: Vec<SessionProperty>,
}

#[derive(Debug, Deserialize)]
struct SessionProperty {
    name: String,
    value: String,
}

pub struct PlayerSkin {
    pub name: String,
    pub url: String,
    pub slim: bool,
}

pub async fn player_skin(
    http: &Http,
    api: &str,
    session: &str,
    player: &str,
) -> CoreResult<PlayerSkin> {
    validate_player_name(player)?;
    let lookup_url = format!("{api}/users/profiles/minecraft/{player}");
    let response = http
        .client()
        .get(&lookup_url)
        .send()
        .await
        .map_err(|source| http_error(&lookup_url, source))?;
    if matches!(response.status().as_u16(), 204 | 404) {
        return Err(CoreError::NotFound(format!("player {player}")));
    }
    let lookup: NameLookup = ensure_success(&lookup_url, response)?
        .json()
        .await
        .map_err(|source| http_error(&lookup_url, source))?;
    if lookup.id.len() != 32 || !lookup.id.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(CoreError::InvalidInput(
            "Mojang returned an invalid profile id".into(),
        ));
    }
    let profile_url = format!("{session}/session/minecraft/profile/{}", lookup.id);
    let profile: SessionProfile = http.get_json(&profile_url).await?;
    let textures = profile
        .properties
        .iter()
        .find(|property| property.name == "textures")
        .and_then(|property| decode_base64(&property.value))
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .ok_or_else(|| CoreError::NotFound(format!("skin of {player}")))?;
    let skin = &textures["textures"]["SKIN"];
    let url = skin["url"]
        .as_str()
        .ok_or_else(|| CoreError::NotFound(format!("skin of {player}")))?;
    Ok(PlayerSkin {
        name: lookup.name,
        url: validate_texture_url(url)?,
        slim: skin["metadata"]["model"] == "slim",
    })
}

fn validate_player_name(name: &str) -> CoreResult<()> {
    let valid = (3..=16).contains(&name.len())
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if valid {
        Ok(())
    } else {
        Err(CoreError::InvalidInput(
            "player names need 3 to 16 characters: letters, digits or underscore".into(),
        ))
    }
}

fn validate_skin_png(bytes: &[u8]) -> CoreResult<()> {
    if bytes.len() as u64 > MAX_SKIN_BYTES {
        return Err(CoreError::InvalidInput("skin file is too large".into()));
    }
    if bytes.len() < 24 || bytes[..8] != PNG_SIGNATURE || &bytes[12..16] != b"IHDR" {
        return Err(CoreError::InvalidInput("skin must be a PNG image".into()));
    }
    let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    match (width, height) {
        (64, 64) | (64, 32) => Ok(()),
        _ => Err(CoreError::InvalidInput(format!(
            "skin must be 64x64 or 64x32 pixels, got {width}x{height}"
        ))),
    }
}

async fn read_skin_file(path: &Path) -> CoreResult<Vec<u8>> {
    let metadata = tokio::fs::metadata(path).await.at_path(path)?;
    if !metadata.is_file() || metadata.len() > MAX_SKIN_BYTES {
        return Err(CoreError::InvalidInput("skin file is too large".into()));
    }
    tokio::fs::read(path).await.at_path(path)
}

impl Services {
    pub async fn import_skin_file(&self, path: &Path, slim: bool) -> CoreResult<LibrarySkin> {
        let bytes = read_skin_file(path).await?;
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("Skin")
            .chars()
            .take(MAX_NAME_CHARS)
            .collect::<String>();
        self.skin_library.add(&name, &bytes, slim).await
    }

    pub async fn import_player_skin(&self, player: &str) -> CoreResult<LibrarySkin> {
        let skin = player_skin(&self.http, MOJANG_API, MOJANG_SESSION, player.trim()).await?;
        let bytes = fetch_texture(&self.http, &skin.url).await?;
        self.skin_library.add(&skin.name, &bytes, skin.slim).await
    }

    pub async fn apply_library_skin(&self, account_id: &str, skin_id: &str) -> CoreResult<()> {
        let skin = self.skin_library.get(skin_id).await?;
        let bytes = self.skin_library.bytes(skin_id).await?;
        let account = self.accounts.get(account_id).await?;
        self.microsoft
            .upload_skin(&account, &bytes, skin.slim)
            .await?;
        Ok(())
    }

    pub async fn account_profile(&self, account_id: &str) -> CoreResult<ProfileView> {
        let account = self.accounts.get(account_id).await?;
        let profile = self.microsoft.profile(&account).await?;
        let active = profile.active_skin();
        Ok(ProfileView {
            skin_url: active.map(|skin| skin.url.replacen("http://", "https://", 1)),
            slim: active.is_some_and(|skin| skin.variant.as_deref() == Some("SLIM")),
            capes: profile
                .capes
                .iter()
                .map(|cape| ProfileCapeView {
                    id: cape.id.clone(),
                    alias: cape.alias.clone(),
                    url: cape.url.replacen("http://", "https://", 1),
                    active: cape.state == "ACTIVE",
                })
                .collect(),
        })
    }

    pub async fn set_account_cape(
        &self,
        account_id: &str,
        cape_id: Option<&str>,
    ) -> CoreResult<()> {
        let account = self.accounts.get(account_id).await?;
        self.microsoft.set_cape(&account, cape_id).await.map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_header(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = PNG_SIGNATURE.to_vec();
        bytes.extend([0, 0, 0, 13]);
        bytes.extend(b"IHDR");
        bytes.extend(width.to_be_bytes());
        bytes.extend(height.to_be_bytes());
        bytes
    }

    #[test]
    fn player_names_are_validated() {
        assert!(validate_player_name("Steve_01").is_ok());
        assert!(validate_player_name("ab").is_err());
        assert!(validate_player_name("Jürgen").is_err());
        assert!(validate_player_name("a_really_long_name_here").is_err());
    }

    #[test]
    fn skin_dimensions_are_checked() {
        assert!(validate_skin_png(&png_header(64, 64)).is_ok());
        assert!(validate_skin_png(&png_header(64, 32)).is_ok());
        assert!(validate_skin_png(&png_header(128, 128)).is_err());
        assert!(validate_skin_png(b"not a png at all, definitely not").is_err());
    }
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        bytes.extend(width.to_be_bytes());
        bytes.extend(height.to_be_bytes());
        bytes.extend([8, 6, 0, 0, 0]);
        bytes
    }

    #[tokio::test]
    async fn library_adds_updates_and_removes() {
        let dir = tempfile::tempdir().unwrap();
        let library = SkinLibrary::new(dir.path().join("library"));
        let first = library.add(" Steve ", &png(64, 64), false).await.unwrap();
        let second = library.add("Alex", &png(64, 64), true).await.unwrap();
        assert_eq!(first.name, "Steve");
        let listed = library.list().await.unwrap();
        assert_eq!(listed[0].id, second.id);
        assert_eq!(library.bytes(&first.id).await.unwrap(), png(64, 64));

        let renamed = library.update(&first.id, "Steve 2", true).await.unwrap();
        assert!(renamed.slim);
        assert!(library.add("x", &png(32, 32), false).await.is_err());
        assert!(library.add("  ", &png(64, 64), false).await.is_err());
        assert!(library.bytes("../escape").await.is_err());

        library.remove(&first.id).await.unwrap();
        assert!(library.remove(&first.id).await.is_err());
        assert_eq!(library.list().await.unwrap().len(), 1);
    }

    #[test]
    fn decodes_base64_and_checks_texture_urls() {
        assert_eq!(decode_base64("aGVsbG8gd29ybGQ=").unwrap(), b"hello world");
        assert_eq!(decode_base64("aGk").unwrap(), b"hi");
        assert!(decode_base64("a$b").is_none());
        assert_eq!(
            validate_texture_url("http://textures.minecraft.net/texture/abc123").unwrap(),
            "https://textures.minecraft.net/texture/abc123"
        );
        for bad in [
            "https://evil.example/texture/abc",
            "https://textures.minecraft.net/texture/../x",
            "https://textures.minecraft.net/texture/",
            "https://textures.minecraft.net.evil.example/texture/ab",
        ] {
            assert!(validate_texture_url(bad).is_err(), "{bad}");
        }
    }

    #[tokio::test]
    async fn looks_up_player_skins() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users/profiles/minecraft/Notch"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "069a79f444e94726a5befca90e38aaf5",
                "name": "Notch"
            })))
            .mount(&server)
            .await;
        let textures = r#"{"textures":{"SKIN":{"url":"http://textures.minecraft.net/texture/292009a4925b58f02c77dadc3ecef07ea4c7472f64e0fdc32ce5522489362680","metadata":{"model":"slim"}}}}"#;
        let encoded = {
            const TABLE: &[u8] =
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
            let bytes = textures.as_bytes();
            let mut out = String::new();
            for chunk in bytes.chunks(3) {
                let b = [
                    chunk[0],
                    *chunk.get(1).unwrap_or(&0),
                    *chunk.get(2).unwrap_or(&0),
                ];
                let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
                for (i, shift) in [18, 12, 6, 0].into_iter().enumerate() {
                    out.push(if i <= chunk.len() {
                        char::from(TABLE[((n >> shift) & 63) as usize])
                    } else {
                        '='
                    });
                }
            }
            out
        };
        Mock::given(method("GET"))
            .and(path(
                "/session/minecraft/profile/069a79f444e94726a5befca90e38aaf5",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "069a79f444e94726a5befca90e38aaf5",
                "name": "Notch",
                "properties": [{ "name": "textures", "value": encoded }]
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/users/profiles/minecraft/Nobody"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        let http = Http::new("test").unwrap();
        let skin = player_skin(&http, &server.uri(), &server.uri(), "Notch")
            .await
            .unwrap();
        assert_eq!(skin.name, "Notch");
        assert!(skin.slim);
        assert!(
            skin.url
                .starts_with("https://textures.minecraft.net/texture/2920")
        );
        assert!(matches!(
            player_skin(&http, &server.uri(), &server.uri(), "Nobody").await,
            Err(CoreError::NotFound(_))
        ));
        assert!(
            player_skin(&http, &server.uri(), &server.uri(), "bad name")
                .await
                .is_err()
        );
    }
}

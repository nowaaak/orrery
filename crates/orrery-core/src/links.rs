use std::path::{Path, PathBuf};

use reqwest::Url;
use serde::Serialize;
use ts_rs::TS;

use crate::error::IoResultExt;
use crate::instances::validate_instance_id;
use crate::servers::ServerAddress;
use crate::storage;
use crate::worlds::validate_world_folder;
use crate::{CoreError, CoreResult};

const KEY_FILE: &str = "link.key";
const KEY_PARAM: &str = "key";
const MAX_SHORTCUT_NAME_CHARS: usize = 60;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum LinkAction {
    Launch {
        instance_id: String,
        world: Option<String>,
        server: Option<String>,
    },
    Open {
        instance_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DeepLink {
    pub action: LinkAction,
    pub trusted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutFormat {
    WindowsUrl,
    LinuxDesktop,
    MacWebloc,
}

pub async fn load_or_create_key(config_dir: &Path) -> CoreResult<String> {
    let path = config_dir.join(KEY_FILE);
    match tokio::fs::read_to_string(&path).await {
        Ok(key) if is_valid_key(key.trim()) => return Ok(key.trim().to_string()),
        Ok(_) => tracing::warn!("link key is malformed, creating a new one"),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(err).at_path(&path),
    }
    let key = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    storage::write_bytes_atomic(&path, key.as_bytes()).await?;
    Ok(key)
}

fn is_valid_key(key: &str) -> bool {
    key.len() == 64 && key.chars().all(|c| c.is_ascii_hexdigit())
}

fn invalid(url: &str) -> CoreError {
    CoreError::InvalidInput(format!("unsupported link {url:?}"))
}

pub fn parse(url: &str, scheme: &str, key: &str) -> CoreResult<DeepLink> {
    let parsed = Url::parse(url).map_err(|_| invalid(url))?;
    if parsed.scheme() != scheme {
        return Err(invalid(url));
    }
    let instance_id = parsed.path().trim_matches('/').to_string();
    validate_instance_id(&instance_id)?;
    let mut world = None;
    let mut server = None;
    let mut trusted = false;
    for (name, value) in parsed.query_pairs() {
        match name.as_ref() {
            "world" => {
                validate_world_folder(&value)?;
                world = Some(value.into_owned());
            }
            "server" => {
                ServerAddress::parse(&value)?;
                server = Some(value.into_owned());
            }
            KEY_PARAM => trusted = is_valid_key(key) && value == key,
            _ => return Err(invalid(url)),
        }
    }
    let action = match parsed.host_str() {
        Some("launch") => {
            if world.is_some() && server.is_some() {
                return Err(invalid(url));
            }
            LinkAction::Launch {
                instance_id,
                world,
                server,
            }
        }
        Some("instance") if world.is_none() && server.is_none() => LinkAction::Open { instance_id },
        _ => return Err(invalid(url)),
    };
    Ok(DeepLink { action, trusted })
}

pub fn launch_url(
    scheme: &str,
    key: &str,
    instance_id: &str,
    world: Option<&str>,
    server: Option<&str>,
) -> CoreResult<String> {
    validate_instance_id(instance_id)?;
    let mut url =
        Url::parse(&format!("{scheme}://launch/{instance_id}")).map_err(|_| invalid(scheme))?;
    {
        let mut query = url.query_pairs_mut();
        if let Some(world) = world {
            validate_world_folder(world)?;
            query.append_pair("world", world);
        }
        if let Some(server) = server {
            ServerAddress::parse(server)?;
            query.append_pair("server", server);
        }
        query.append_pair(KEY_PARAM, key);
    }
    Ok(url.into())
}

fn shortcut_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '-'
            } else {
                c
            }
        })
        .collect();
    let trimmed = cleaned.trim().trim_matches('.').trim();
    let short: String = trimmed.chars().take(MAX_SHORTCUT_NAME_CHARS).collect();
    if short.is_empty() {
        "Orrery".into()
    } else {
        short
    }
}

pub fn shortcut_content(format: ShortcutFormat, name: &str, url: &str, icon: &Path) -> Vec<u8> {
    match format {
        ShortcutFormat::WindowsUrl => format!(
            "[InternetShortcut]\r\nURL={url}\r\nIconFile={}\r\nIconIndex=0\r\n",
            icon.display()
        )
        .into_bytes(),
        ShortcutFormat::LinuxDesktop => format!(
            "[Desktop Entry]\nType=Application\nName={name}\nExec=xdg-open \"{url}\"\nIcon={}\nTerminal=false\n",
            icon.display()
        )
        .into_bytes(),
        ShortcutFormat::MacWebloc => format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n<dict>\n\t<key>URL</key>\n\t<string>{}</string>\n</dict>\n</plist>\n",
            url.replace('&', "&amp;")
        )
        .into_bytes(),
    }
}

fn extension(format: ShortcutFormat) -> &'static str {
    match format {
        ShortcutFormat::WindowsUrl => "url",
        ShortcutFormat::LinuxDesktop => "desktop",
        ShortcutFormat::MacWebloc => "webloc",
    }
}

pub async fn create_shortcut(
    dir: &Path,
    format: ShortcutFormat,
    name: &str,
    url: &str,
    icon: &Path,
) -> CoreResult<PathBuf> {
    let base = shortcut_name(name);
    let ext = extension(format);
    let mut path = dir.join(format!("{base}.{ext}"));
    let mut counter = 2;
    while tokio::fs::metadata(&path).await.is_ok() {
        path = dir.join(format!("{base} ({counter}).{ext}"));
        counter += 1;
    }
    let content = shortcut_content(format, &base, url, icon);
    storage::write_bytes_atomic(&path, &content).await?;
    #[cfg(unix)]
    if format == ShortcutFormat::LinuxDesktop {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .await
            .at_path(&path)?;
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn roundtrips_launch_links() {
        let url = launch_url("orrery", KEY, "survival", Some("Meine Welt & mehr"), None).unwrap();
        assert!(url.starts_with("orrery://launch/survival?world="));
        assert_eq!(
            parse(&url, "orrery", KEY).unwrap(),
            DeepLink {
                action: LinkAction::Launch {
                    instance_id: "survival".into(),
                    world: Some("Meine Welt & mehr".into()),
                    server: None,
                },
                trusted: true,
            }
        );
        let server =
            launch_url("orrery", KEY, "pvp", None, Some("play.example.org:25566")).unwrap();
        let parsed = parse(&server, "orrery", KEY).unwrap();
        assert!(matches!(
            parsed.action,
            LinkAction::Launch { server: Some(ref s), .. } if s == "play.example.org:25566"
        ));
    }

    #[test]
    fn links_without_the_key_are_untrusted() {
        let parsed = parse("orrery://launch/survival", "orrery", KEY).unwrap();
        assert!(!parsed.trusted);
        let wrong = parse("orrery://launch/survival?key=abc", "orrery", KEY).unwrap();
        assert!(!wrong.trusted);
        let open = parse("orrery://instance/survival/", "orrery", KEY).unwrap();
        assert_eq!(
            open.action,
            LinkAction::Open {
                instance_id: "survival".into()
            }
        );
    }

    #[test]
    fn rejects_bad_links() {
        for url in [
            "https://launch/survival",
            "orrery-dev://launch/survival",
            "orrery://launch/a/b",
            "orrery://launch/Bad%20Id",
            "orrery://launch/survival?world=../x",
            "orrery://launch/survival?server=bad%20host",
            "orrery://launch/survival?world=a&server=b",
            "orrery://launch/survival?unknown=1",
            "orrery://delete/survival",
            "orrery://instance/survival?world=a",
            "not a url",
        ] {
            assert!(parse(url, "orrery", KEY).is_err(), "{url}");
        }
    }

    #[test]
    fn builds_shortcut_files() {
        let windows = String::from_utf8(shortcut_content(
            ShortcutFormat::WindowsUrl,
            "Survival",
            "orrery://launch/survival?key=k",
            Path::new(r"C:\Program Files\Orrery\Orrery.exe"),
        ))
        .unwrap();
        assert!(windows.contains("URL=orrery://launch/survival?key=k\r\n"));
        assert!(windows.contains(r"IconFile=C:\Program Files\Orrery\Orrery.exe"));
        let mac = String::from_utf8(shortcut_content(
            ShortcutFormat::MacWebloc,
            "x",
            "orrery://launch/a?world=b&key=k",
            Path::new("/x"),
        ))
        .unwrap();
        assert!(mac.contains("world=b&amp;key=k"));
        assert_eq!(shortcut_name(" a/b:c? "), "a-b-c-");
        assert_eq!(shortcut_name("..."), "Orrery");
    }

    #[tokio::test]
    async fn creates_unique_shortcuts_and_a_stable_key() {
        let dir = tempfile::tempdir().unwrap();
        let first = create_shortcut(
            dir.path(),
            ShortcutFormat::WindowsUrl,
            "Survival",
            "orrery://launch/survival",
            Path::new("o.exe"),
        )
        .await
        .unwrap();
        let second = create_shortcut(
            dir.path(),
            ShortcutFormat::WindowsUrl,
            "Survival",
            "orrery://launch/survival",
            Path::new("o.exe"),
        )
        .await
        .unwrap();
        assert_eq!(first.file_name().unwrap(), "Survival.url");
        assert_eq!(second.file_name().unwrap(), "Survival (2).url");

        let key = load_or_create_key(dir.path()).await.unwrap();
        assert!(is_valid_key(&key));
        assert_eq!(load_or_create_key(dir.path()).await.unwrap(), key);
        std::fs::write(dir.path().join(KEY_FILE), "short").unwrap();
        assert_ne!(load_or_create_key(dir.path()).await.unwrap(), key);
    }
}

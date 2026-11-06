use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::ServerAddress;
use super::playtime::server_key;
use crate::error::IoResultExt;
use crate::nbt::{self, Tag};
use crate::services::Services;
use crate::storage;
use crate::{CoreError, CoreResult};

pub const SERVERS_FILE: &str = "servers.dat";
const BACKUP_FILE: &str = "servers.dat.orrery-backup";
const LIST_KEY: &str = "servers";
const MAX_NAME_CHARS: usize = 128;
const MAX_ICON_CHARS: usize = 512 * 1024;
const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SavedServer {
    pub index: u32,
    pub name: String,
    pub address: String,
    pub icon: Option<String>,
    pub play_secs: u64,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ServerInput {
    pub name: String,
    pub address: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum ServerChange {
    Add {
        server: ServerInput,
    },
    Edit {
        index: u32,
        expected_address: String,
        server: ServerInput,
    },
    Remove {
        index: u32,
        expected_address: String,
    },
    Move {
        index: u32,
        expected_address: String,
        to: u32,
    },
}

struct ServerFile {
    name: String,
    root: Tag,
}

impl ServerFile {
    fn empty() -> Self {
        Self {
            name: String::new(),
            root: Tag::Compound(vec![(LIST_KEY.into(), Tag::List(10, Vec::new()))]),
        }
    }

    fn parse(bytes: &[u8]) -> CoreResult<Self> {
        let (name, root) = nbt::read(bytes).ok_or_else(|| {
            CoreError::InvalidInput("servers.dat is not a valid server list".into())
        })?;
        let mut file = Self { name, root };
        if file.root.get(LIST_KEY).and_then(Tag::as_list).is_none() {
            file.root.set(LIST_KEY, Tag::List(10, Vec::new()));
        }
        Ok(file)
    }

    fn entries(&self) -> &[Tag] {
        self.root
            .get(LIST_KEY)
            .and_then(Tag::as_list)
            .unwrap_or_default()
    }

    fn entries_mut(&mut self) -> CoreResult<&mut Vec<Tag>> {
        if let Tag::Compound(entries) = &mut self.root
            && let Some((_, Tag::List(_, items))) = entries.iter_mut().find(|(k, _)| k == LIST_KEY)
        {
            return Ok(items);
        }
        Err(CoreError::InvalidInput(
            "servers.dat has no server list".into(),
        ))
    }

    fn visible(&self) -> Vec<SavedServer> {
        self.entries()
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.get("hidden").and_then(Tag::as_i64).unwrap_or(0) == 0)
            .map(|(index, entry)| SavedServer {
                index: index as u32,
                name: entry
                    .get("name")
                    .and_then(Tag::as_str)
                    .unwrap_or_default()
                    .to_string(),
                address: entry
                    .get("ip")
                    .and_then(Tag::as_str)
                    .unwrap_or_default()
                    .to_string(),
                icon: entry
                    .get("icon")
                    .and_then(Tag::as_str)
                    .filter(|icon| is_base64(icon))
                    .map(|icon| format!("data:image/png;base64,{icon}")),
                play_secs: 0,
            })
            .collect()
    }

    fn checked_index(&self, index: u32, expected_address: &str) -> CoreResult<usize> {
        let index = index as usize;
        match self.entries().get(index) {
            Some(entry) if entry.get("ip").and_then(Tag::as_str) == Some(expected_address) => {
                Ok(index)
            }
            _ => Err(CoreError::InvalidInput(
                "the server list changed, reload it and try again".into(),
            )),
        }
    }

    fn apply(&mut self, change: ServerChange) -> CoreResult<()> {
        match change {
            ServerChange::Add { server } => {
                let (name, address) = validate(server)?;
                self.entries_mut()?.push(Tag::Compound(vec![
                    ("name".into(), Tag::String(name)),
                    ("ip".into(), Tag::String(address)),
                ]));
            }
            ServerChange::Edit {
                index,
                expected_address,
                server,
            } => {
                let index = self.checked_index(index, &expected_address)?;
                let (name, address) = validate(server)?;
                let entry = &mut self.entries_mut()?[index];
                if address != expected_address {
                    entry.remove("icon");
                }
                entry.set("name", Tag::String(name));
                entry.set("ip", Tag::String(address));
            }
            ServerChange::Remove {
                index,
                expected_address,
            } => {
                let index = self.checked_index(index, &expected_address)?;
                self.entries_mut()?.remove(index);
            }
            ServerChange::Move {
                index,
                expected_address,
                to,
            } => {
                let index = self.checked_index(index, &expected_address)?;
                let entries = self.entries_mut()?;
                let to = (to as usize).min(entries.len() - 1);
                let entry = entries.remove(index);
                entries.insert(to, entry);
            }
        }
        Ok(())
    }

    fn bytes(&self) -> Vec<u8> {
        nbt::write(&self.name, &self.root)
    }
}

fn validate(server: ServerInput) -> CoreResult<(String, String)> {
    let name = server.name.trim().to_string();
    if name.is_empty()
        || name.chars().count() > MAX_NAME_CHARS
        || name.chars().any(char::is_control)
    {
        return Err(CoreError::InvalidInput(format!(
            "server names must have between 1 and {MAX_NAME_CHARS} characters"
        )));
    }
    let address = server.address.trim().to_string();
    ServerAddress::parse(&address)?;
    Ok((name, address))
}

fn is_base64(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ICON_CHARS
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'='))
}

async fn read_file(path: &Path) -> CoreResult<Option<ServerFile>> {
    match tokio::fs::metadata(path).await {
        Ok(metadata) if metadata.len() > MAX_FILE_BYTES => {
            return Err(CoreError::InvalidInput("servers.dat is too large".into()));
        }
        Ok(_) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err).at_path(path),
    }
    let bytes = tokio::fs::read(path).await.at_path(path)?;
    ServerFile::parse(&bytes).map(Some)
}

pub async fn list(game_dir: &Path) -> CoreResult<Vec<SavedServer>> {
    Ok(read_file(&game_dir.join(SERVERS_FILE))
        .await?
        .map(|file| file.visible())
        .unwrap_or_default())
}

pub async fn change(game_dir: &Path, change: ServerChange) -> CoreResult<Vec<SavedServer>> {
    let path = game_dir.join(SERVERS_FILE);
    let existing = read_file(&path).await?;
    let mut file = match existing {
        Some(file) => {
            let backup: PathBuf = game_dir.join(BACKUP_FILE);
            tokio::fs::copy(&path, &backup).await.at_path(&backup)?;
            file
        }
        None => ServerFile::empty(),
    };
    file.apply(change)?;
    storage::write_bytes_atomic(&path, &file.bytes()).await?;
    Ok(file.visible())
}

impl Services {
    pub async fn list_servers(&self, instance_id: &str) -> CoreResult<Vec<SavedServer>> {
        let servers = list(&self.instances.game_dir(instance_id)?).await?;
        self.with_playtime(instance_id, servers).await
    }

    async fn with_playtime(
        &self,
        instance_id: &str,
        mut servers: Vec<SavedServer>,
    ) -> CoreResult<Vec<SavedServer>> {
        let totals = match self.server_playtime.totals(instance_id).await {
            Ok(totals) => totals,
            Err(err) => {
                tracing::warn!(error = %err, "failed to read the server playtime");
                return Ok(servers);
            }
        };
        for server in &mut servers {
            server.play_secs = server_key(&server.address)
                .and_then(|key| totals.get(&key).copied())
                .unwrap_or_default();
        }
        Ok(servers)
    }

    pub async fn change_servers(
        &self,
        instance_id: &str,
        server_change: ServerChange,
    ) -> CoreResult<Vec<SavedServer>> {
        let instance = self.instances.get(instance_id).await?;
        if self.games.is_running(&instance.id) {
            return Err(CoreError::InvalidInput(
                "stop the game before changing the server list".into(),
            ));
        }
        let game_dir = self.instances.game_dir(&instance.id)?;
        tokio::fs::create_dir_all(&game_dir)
            .await
            .at_path(&game_dir)?;
        let servers = change(&game_dir, server_change).await?;
        self.with_playtime(&instance.id, servers).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(name: &str, address: &str) -> ServerInput {
        ServerInput {
            name: name.into(),
            address: address.into(),
        }
    }

    fn vanilla_file() -> Vec<u8> {
        let entry = |name: &str, ip: &str, extra: Vec<(String, Tag)>| {
            let mut fields = vec![
                ("ip".to_string(), Tag::String(ip.into())),
                ("name".to_string(), Tag::String(name.into())),
            ];
            fields.extend(extra);
            Tag::Compound(fields)
        };
        let root = Tag::Compound(vec![(
            "servers".into(),
            Tag::List(
                10,
                vec![
                    entry(
                        "Hypixel 🎮",
                        "mc.hypixel.net",
                        vec![
                            ("icon".into(), Tag::String("iVBORw0KGgo=".into())),
                            ("acceptTextures".into(), Tag::Byte(1)),
                        ],
                    ),
                    entry(
                        "Quick",
                        "play.example.org",
                        vec![("hidden".into(), Tag::Byte(1))],
                    ),
                    entry("Local", "localhost:25566", Vec::new()),
                ],
            ),
        )]);
        nbt::write("", &root)
    }

    #[tokio::test]
    async fn reads_the_vanilla_list_and_hides_hidden_entries() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(SERVERS_FILE), vanilla_file()).unwrap();
        let servers = list(dir.path()).await.unwrap();
        assert_eq!(servers.len(), 2);
        assert_eq!(servers[0].name, "Hypixel 🎮");
        assert_eq!(
            servers[0].icon.as_deref(),
            Some("data:image/png;base64,iVBORw0KGgo=")
        );
        assert_eq!(servers[1].index, 2);
        assert_eq!(servers[1].address, "localhost:25566");
        assert!(list(&dir.path().join("missing")).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn changes_keep_unknown_fields_and_write_a_backup() {
        let dir = tempfile::tempdir().unwrap();
        let original = vanilla_file();
        std::fs::write(dir.path().join(SERVERS_FILE), &original).unwrap();

        let servers = change(
            dir.path(),
            ServerChange::Add {
                server: input(" Orrery Test ", " localhost "),
            },
        )
        .await
        .unwrap();
        assert_eq!(servers.last().unwrap().name, "Orrery Test");
        assert_eq!(
            std::fs::read(dir.path().join(BACKUP_FILE)).unwrap(),
            original
        );

        change(
            dir.path(),
            ServerChange::Edit {
                index: 0,
                expected_address: "mc.hypixel.net".into(),
                server: input("Hypixel", "mc.hypixel.net"),
            },
        )
        .await
        .unwrap();
        let (_, root) = nbt::read(&std::fs::read(dir.path().join(SERVERS_FILE)).unwrap()).unwrap();
        let first = &root.get("servers").unwrap().as_list().unwrap()[0];
        assert_eq!(first.get("acceptTextures"), Some(&Tag::Byte(1)));
        assert!(first.get("icon").is_some());
        assert_eq!(root.get("servers").unwrap().as_list().unwrap().len(), 4);

        let moved = change(
            dir.path(),
            ServerChange::Move {
                index: 3,
                expected_address: "localhost".into(),
                to: 0,
            },
        )
        .await
        .unwrap();
        assert_eq!(moved[0].name, "Orrery Test");

        let edited = change(
            dir.path(),
            ServerChange::Edit {
                index: 1,
                expected_address: "mc.hypixel.net".into(),
                server: input("Other", "other.example.org"),
            },
        )
        .await
        .unwrap();
        assert_eq!(edited[1].icon, None);

        let removed = change(
            dir.path(),
            ServerChange::Remove {
                index: 0,
                expected_address: "localhost".into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(removed.len(), 2);
    }

    #[tokio::test]
    async fn rejects_stale_indices_and_bad_input() {
        let dir = tempfile::tempdir().unwrap();
        let created = change(
            dir.path(),
            ServerChange::Add {
                server: input("Neu", "localhost:25565"),
            },
        )
        .await
        .unwrap();
        assert_eq!(created.len(), 1);
        assert!(!dir.path().join(BACKUP_FILE).exists());

        for bad in [
            ServerChange::Remove {
                index: 0,
                expected_address: "other".into(),
            },
            ServerChange::Remove {
                index: 5,
                expected_address: "localhost:25565".into(),
            },
            ServerChange::Add {
                server: input(" ", "localhost"),
            },
            ServerChange::Add {
                server: input("x", "bad host"),
            },
        ] {
            assert!(change(dir.path(), bad).await.is_err());
        }
        assert_eq!(list(dir.path()).await.unwrap().len(), 1);

        std::fs::write(dir.path().join(SERVERS_FILE), b"garbage").unwrap();
        assert!(list(dir.path()).await.is_err());
    }
}

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::mpsc;
use ts_rs::TS;

use crate::{CoreError, CoreResult};

const OP_HANDSHAKE: u32 = 0;
const OP_FRAME: u32 = 1;
const OP_CLOSE: u32 = 2;
const MAX_FRAME_BYTES: usize = 64 * 1024;
const MAX_PIPES: u8 = 10;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const RETRY_DELAY: Duration = Duration::from_secs(20);
const LARGE_IMAGE: &str = "orrery";
const MAX_TEXT_CHARS: usize = 120;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum PresenceLevel {
    Off,
    #[default]
    Basic,
    Detailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GamePresence {
    pub instance_name: String,
    pub minecraft_version: String,
    pub loader: Option<String>,
    pub world: Option<String>,
    pub server: Option<String>,
    pub dimension: Option<String>,
    pub started_at: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameLocation {
    pub world: Option<String>,
    pub server: Option<String>,
    pub dimension: Option<String>,
}

enum Message {
    Started {
        instance_id: String,
        game: GamePresence,
        level: PresenceLevel,
    },
    Moved {
        instance_id: String,
        location: GameLocation,
    },
    Stopped {
        instance_id: String,
    },
    Level(PresenceLevel),
}

#[derive(Clone)]
pub struct Presence {
    tx: mpsc::UnboundedSender<Message>,
}

impl Presence {
    pub fn start(client_id: String) -> Self {
        Self::with_endpoints(client_id, default_endpoints())
    }

    pub fn with_endpoints(client_id: String, endpoints: Vec<PathBuf>) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(run(client_id, endpoints, rx));
        Self { tx }
    }

    pub fn game_started(&self, instance_id: &str, game: GamePresence, level: PresenceLevel) {
        let _ = self.tx.send(Message::Started {
            instance_id: instance_id.to_string(),
            game,
            level,
        });
    }

    pub fn game_moved(&self, instance_id: &str, location: GameLocation) {
        let _ = self.tx.send(Message::Moved {
            instance_id: instance_id.to_string(),
            location,
        });
    }

    pub fn game_stopped(&self, instance_id: &str) {
        let _ = self.tx.send(Message::Stopped {
            instance_id: instance_id.to_string(),
        });
    }

    pub fn set_level(&self, level: PresenceLevel) {
        let _ = self.tx.send(Message::Level(level));
    }
}

fn clip(text: &str) -> String {
    let mut clipped: String = text.chars().take(MAX_TEXT_CHARS).collect();
    while clipped.chars().count() < 2 {
        clipped.push(' ');
    }
    clipped
}

pub fn activity(game: &GamePresence, level: PresenceLevel) -> Option<Value> {
    if level == PresenceLevel::Off {
        return None;
    }
    let details = match &game.loader {
        Some(loader) => format!("Minecraft {} with {loader}", game.minecraft_version),
        None => format!("Minecraft {}", game.minecraft_version),
    };
    let mut activity = json!({
        "details": clip(&details),
        "timestamps": { "start": game.started_at },
        "assets": {
            "large_image": LARGE_IMAGE,
            "large_text": clip(&game.instance_name),
        },
    });
    if level == PresenceLevel::Detailed {
        let place = match (&game.server, &game.world) {
            (Some(server), _) => Some(format!("Playing on {server}")),
            (None, Some(world)) => Some(format!("Playing in {world}")),
            (None, None) => None,
        };
        let state = match (place, game.dimension.as_deref().map(dimension_name)) {
            (Some(place), Some(dimension)) => Some(format!("{place} · {dimension}")),
            (place, _) => place,
        };
        if let Some(state) = state {
            activity["state"] = Value::String(clip(&state));
        }
    }
    Some(activity)
}

pub fn dimension_name(id: &str) -> String {
    let path = id.rsplit_once(':').map_or(id, |(_, path)| path);
    match path {
        "overworld" => "Overworld".into(),
        "the_nether" => "The Nether".into(),
        "the_end" => "The End".into(),
        other => other
            .rsplit('/')
            .next()
            .unwrap_or(other)
            .split('_')
            .filter(|word| !word.is_empty())
            .map(|word| {
                let mut chars = word.chars();
                chars.next().map_or_else(String::new, |first| {
                    first.to_uppercase().chain(chars).collect()
                })
            })
            .collect::<Vec<_>>()
            .join(" "),
    }
}

pub fn encode_frame(op: u32, payload: &Value) -> Vec<u8> {
    let body = payload.to_string().into_bytes();
    let mut frame = Vec::with_capacity(body.len() + 8);
    frame.extend(op.to_le_bytes());
    frame.extend((body.len() as u32).to_le_bytes());
    frame.extend(body);
    frame
}

pub async fn read_frame(reader: &mut (impl AsyncRead + Unpin)) -> std::io::Result<(u32, Value)> {
    let mut header = [0u8; 8];
    reader.read_exact(&mut header).await?;
    let op = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
    let len = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;
    if len > MAX_FRAME_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Discord frame is too large",
        ));
    }
    let mut body = vec![0; len];
    reader.read_exact(&mut body).await?;
    let value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    Ok((op, value))
}

fn default_endpoints() -> Vec<PathBuf> {
    let mut bases: Vec<PathBuf> = Vec::new();
    if cfg!(windows) {
        bases.push(PathBuf::from(r"\\.\pipe"));
    } else {
        for key in ["XDG_RUNTIME_DIR", "TMPDIR", "TMP", "TEMP"] {
            if let Some(dir) = std::env::var_os(key) {
                let dir = PathBuf::from(dir);
                bases.push(dir.join("app").join("com.discordapp.Discord"));
                bases.push(dir.join("snap.discord"));
                bases.push(dir.clone());
            }
        }
        bases.push(PathBuf::from("/tmp"));
    }
    let mut endpoints = Vec::new();
    for base in bases {
        for index in 0..MAX_PIPES {
            let path = base.join(format!("discord-ipc-{index}"));
            if !endpoints.contains(&path) {
                endpoints.push(path);
            }
        }
    }
    endpoints
}

#[cfg(windows)]
async fn open(path: &PathBuf) -> std::io::Result<tokio::net::windows::named_pipe::NamedPipeClient> {
    tokio::net::windows::named_pipe::ClientOptions::new().open(path)
}

#[cfg(unix)]
async fn open(path: &PathBuf) -> std::io::Result<tokio::net::UnixStream> {
    tokio::net::UnixStream::connect(path).await
}

async fn connect(client_id: &str, endpoints: &[PathBuf]) -> CoreResult<Connection> {
    for path in endpoints {
        let Ok(Ok(stream)) = tokio::time::timeout(CONNECT_TIMEOUT, open(path)).await else {
            continue;
        };
        let mut connection = Connection {
            stream: Box::new(stream),
        };
        match tokio::time::timeout(CONNECT_TIMEOUT, connection.handshake(client_id)).await {
            Ok(Ok(())) => return Ok(connection),
            Ok(Err(err)) => {
                tracing::debug!(endpoint = %path.display(), error = %err, "Discord handshake failed");
                return Err(err);
            }
            Err(_) => continue,
        }
    }
    Err(CoreError::Unreachable("Discord is not running".into()))
}

trait Stream: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Stream for T {}

struct Connection {
    stream: Box<dyn Stream>,
}

impl Connection {
    async fn send(&mut self, op: u32, payload: &Value) -> std::io::Result<()> {
        self.stream.write_all(&encode_frame(op, payload)).await?;
        self.stream.flush().await
    }

    async fn handshake(&mut self, client_id: &str) -> CoreResult<()> {
        let io = |err: std::io::Error| CoreError::Unreachable(format!("Discord: {err}"));
        self.send(OP_HANDSHAKE, &json!({ "v": 1, "client_id": client_id }))
            .await
            .map_err(io)?;
        let (op, payload) = read_frame(&mut self.stream).await.map_err(io)?;
        if op == OP_FRAME && payload["evt"] == "READY" {
            Ok(())
        } else {
            Err(CoreError::Unreachable(format!(
                "Discord refused the connection: {}",
                payload["message"].as_str().unwrap_or("unknown reason")
            )))
        }
    }

    async fn set_activity(&mut self, activity: Option<Value>) -> std::io::Result<()> {
        let payload = json!({
            "cmd": "SET_ACTIVITY",
            "args": { "pid": std::process::id(), "activity": activity },
            "nonce": uuid::Uuid::new_v4().to_string(),
        });
        self.send(OP_FRAME, &payload).await?;
        loop {
            let (op, reply) = read_frame(&mut self.stream).await?;
            match op {
                OP_CLOSE => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::ConnectionAborted,
                        "Discord closed the connection",
                    ));
                }
                OP_FRAME if reply["nonce"] == payload["nonce"] => {
                    if reply["evt"] == "ERROR" {
                        tracing::debug!(reply = %reply, "Discord rejected the activity");
                    }
                    return Ok(());
                }
                _ => continue,
            }
        }
    }
}

struct State {
    games: HashMap<String, (GamePresence, u64)>,
    level: PresenceLevel,
    counter: u64,
}

impl State {
    fn current(&self) -> Option<Value> {
        self.games
            .values()
            .max_by_key(|(_, order)| *order)
            .and_then(|(game, _)| activity(game, self.level))
    }
}

async fn run(client_id: String, endpoints: Vec<PathBuf>, mut rx: mpsc::UnboundedReceiver<Message>) {
    let mut state = State {
        games: HashMap::new(),
        level: PresenceLevel::Basic,
        counter: 0,
    };
    let mut connection: Option<Connection> = None;
    let mut dirty = false;
    loop {
        let message = if dirty && connection.is_none() {
            match tokio::time::timeout(RETRY_DELAY, rx.recv()).await {
                Ok(message) => message,
                Err(_) => {
                    dirty = sync(&client_id, &endpoints, &mut connection, &state).await;
                    continue;
                }
            }
        } else {
            rx.recv().await
        };
        let Some(message) = message else {
            return;
        };
        match message {
            Message::Started {
                instance_id,
                game,
                level,
            } => {
                state.counter += 1;
                state.level = level;
                state.games.insert(instance_id, (game, state.counter));
            }
            Message::Moved {
                instance_id,
                location,
            } => {
                let Some((game, _)) = state.games.get_mut(&instance_id) else {
                    continue;
                };
                let unchanged = game.world == location.world
                    && game.server == location.server
                    && game.dimension == location.dimension;
                game.world = location.world;
                game.server = location.server;
                game.dimension = location.dimension;
                if unchanged || state.level != PresenceLevel::Detailed {
                    continue;
                }
            }
            Message::Stopped { instance_id } => {
                state.games.remove(&instance_id);
            }
            Message::Level(level) => state.level = level,
        }
        dirty = sync(&client_id, &endpoints, &mut connection, &state).await;
    }
}

async fn sync(
    client_id: &str,
    endpoints: &[PathBuf],
    connection: &mut Option<Connection>,
    state: &State,
) -> bool {
    let activity = state.current();
    if activity.is_none() && connection.is_none() {
        return false;
    }
    for _ in 0..2 {
        if connection.is_none() {
            match connect(client_id, endpoints).await {
                Ok(fresh) => *connection = Some(fresh),
                Err(err) => {
                    tracing::debug!(error = %err, "Discord presence is not available");
                    return activity.is_some();
                }
            }
        }
        let Some(active) = connection.as_mut() else {
            return true;
        };
        match active.set_activity(activity.clone()).await {
            Ok(()) => {
                if activity.is_none() {
                    *connection = None;
                }
                return false;
            }
            Err(err) => {
                tracing::debug!(error = %err, "lost the Discord connection");
                *connection = None;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(world: Option<&str>, server: Option<&str>) -> GamePresence {
        GamePresence {
            instance_name: "Survival".into(),
            minecraft_version: "1.21.1".into(),
            loader: Some("Fabric".into()),
            world: world.map(str::to_string),
            server: server.map(str::to_string),
            dimension: None,
            started_at: 1_700_000_000,
        }
    }

    #[test]
    fn names_dimensions() {
        assert_eq!(dimension_name("minecraft:overworld"), "Overworld");
        assert_eq!(dimension_name("minecraft:the_nether"), "The Nether");
        assert_eq!(dimension_name("minecraft:the_end"), "The End");
        assert_eq!(dimension_name("aether:the_aether"), "The Aether");
        assert_eq!(dimension_name("mod:caves/deep_dark"), "Deep Dark");
        assert_eq!(dimension_name("plain"), "Plain");
    }

    #[test]
    fn adds_the_dimension_to_detailed_activities() {
        let nether = GamePresence {
            dimension: Some("minecraft:the_nether".into()),
            ..game(Some("Welt"), None)
        };
        assert_eq!(
            activity(&nether, PresenceLevel::Detailed).unwrap()["state"],
            "Playing in Welt · The Nether"
        );
        assert!(
            activity(&nether, PresenceLevel::Basic)
                .unwrap()
                .get("state")
                .is_none()
        );
        let menu = GamePresence {
            dimension: Some("minecraft:overworld".into()),
            ..game(None, None)
        };
        assert!(
            activity(&menu, PresenceLevel::Detailed)
                .unwrap()
                .get("state")
                .is_none()
        );
    }

    #[test]
    fn builds_activities_per_level() {
        assert_eq!(activity(&game(None, None), PresenceLevel::Off), None);
        let basic = activity(&game(Some("Welt"), None), PresenceLevel::Basic).unwrap();
        assert_eq!(basic["details"], "Minecraft 1.21.1 with Fabric");
        assert_eq!(basic["assets"]["large_text"], "Survival");
        assert_eq!(basic["timestamps"]["start"], 1_700_000_000);
        assert!(basic.get("state").is_none());
        let detailed = activity(
            &game(None, Some("play.example.org")),
            PresenceLevel::Detailed,
        )
        .unwrap();
        assert_eq!(detailed["state"], "Playing on play.example.org");
        let short = GamePresence {
            instance_name: "A".into(),
            ..game(None, None)
        };
        assert_eq!(
            activity(&short, PresenceLevel::Basic).unwrap()["assets"]["large_text"],
            "A "
        );
    }

    #[tokio::test]
    async fn frames_roundtrip() {
        let (mut client, mut server) = tokio::io::duplex(1024);
        let payload = json!({ "cmd": "SET_ACTIVITY", "nonce": "1" });
        client
            .write_all(&encode_frame(OP_FRAME, &payload))
            .await
            .unwrap();
        assert_eq!(read_frame(&mut server).await.unwrap(), (OP_FRAME, payload));

        let mut huge = Vec::new();
        huge.extend(OP_FRAME.to_le_bytes());
        huge.extend(u32::MAX.to_le_bytes());
        client.write_all(&huge).await.unwrap();
        assert!(read_frame(&mut server).await.is_err());
    }

    #[test]
    fn lists_ipc_endpoints() {
        let endpoints = default_endpoints();
        assert!(endpoints.len() >= 10);
        assert!(
            endpoints
                .iter()
                .all(|path| path.to_string_lossy().contains("discord-ipc-"))
        );
    }

    async fn fake_discord(
        mut stream: impl AsyncRead + AsyncWrite + Unpin,
        log: std::sync::Arc<std::sync::Mutex<Vec<Value>>>,
    ) {
        let (op, hello) = read_frame(&mut stream).await.unwrap();
        assert_eq!(op, OP_HANDSHAKE);
        assert_eq!(hello["client_id"], "123");
        stream
            .write_all(&encode_frame(
                OP_FRAME,
                &json!({ "cmd": "DISPATCH", "evt": "READY" }),
            ))
            .await
            .unwrap();
        while let Ok((_, frame)) = read_frame(&mut stream).await {
            let nonce = frame["nonce"].clone();
            log.lock().unwrap().push(frame["args"]["activity"].clone());
            stream
                .write_all(&encode_frame(
                    OP_FRAME,
                    &json!({ "cmd": "SET_ACTIVITY", "nonce": nonce }),
                ))
                .await
                .unwrap();
        }
    }

    async fn wait_for(log: &std::sync::Arc<std::sync::Mutex<Vec<Value>>>, count: usize) {
        for _ in 0..200 {
            if log.lock().unwrap().len() >= count {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!(
            "Discord received {} of {count} updates",
            log.lock().unwrap().len()
        );
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn talks_to_a_named_pipe() {
        use tokio::net::windows::named_pipe::ServerOptions;
        let name = format!(r"\\.\pipe\orrery-test-{}", uuid::Uuid::new_v4().simple());
        let server = ServerOptions::new()
            .first_pipe_instance(true)
            .create(&name)
            .unwrap();
        let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let server_log = log.clone();
        tokio::spawn(async move {
            server.connect().await.unwrap();
            fake_discord(server, server_log).await;
        });
        exercise(PathBuf::from(&name), &log).await;
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn talks_to_a_unix_socket() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("discord-ipc-0");
        let listener = tokio::net::UnixListener::bind(&path).unwrap();
        let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let server_log = log.clone();
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            fake_discord(stream, server_log).await;
        });
        exercise(path, &log).await;
    }

    async fn exercise(endpoint: PathBuf, log: &std::sync::Arc<std::sync::Mutex<Vec<Value>>>) {
        let presence =
            Presence::with_endpoints("123".into(), vec![PathBuf::from("missing"), endpoint]);
        presence.game_started("a", game(None, Some("localhost")), PresenceLevel::Detailed);
        wait_for(log, 1).await;
        let end = GameLocation {
            world: Some("Welt".into()),
            server: None,
            dimension: Some("minecraft:the_end".into()),
        };
        presence.game_moved("a", end.clone());
        wait_for(log, 2).await;
        presence.game_moved("a", end);
        presence.game_moved("missing", GameLocation::default());
        presence.set_level(PresenceLevel::Basic);
        wait_for(log, 3).await;
        presence.game_stopped("a");
        wait_for(log, 4).await;
        let log = log.lock().unwrap();
        assert_eq!(log.len(), 4);
        assert_eq!(log[0]["state"], "Playing on localhost");
        assert_eq!(log[1]["state"], "Playing in Welt · The End");
        assert!(log[2].get("state").is_none());
        assert_eq!(log[3], Value::Null);
    }
}

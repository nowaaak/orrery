use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use ts_rs::TS;

use crate::error::IoResultExt;
use crate::{CoreError, CoreResult};

pub const PROTOCOL_VERSION: u32 = 1;
const HELLO_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_LINE_BYTES: usize = 64 * 1024;
const MAX_TEXT_CHARS: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameStatus {
    pub screen: String,
    #[serde(default)]
    pub world: Option<String>,
    #[serde(default)]
    pub server: Option<String>,
    #[serde(default)]
    pub dimension: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum BridgeEvent {
    Connected {
        instance_id: String,
        mod_version: String,
    },
    Status {
        instance_id: String,
        status: GameStatus,
    },
    Screenshot {
        instance_id: String,
        file: String,
    },
    Disconnected {
        instance_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgeTicket {
    pub port: u16,
    pub token: String,
}

impl BridgeTicket {
    pub fn jvm_args(&self) -> Vec<String> {
        vec![
            format!("-Dorrery.bridge=127.0.0.1:{}", self.port),
            format!("-Dorrery.bridge.token={}", self.token),
        ]
    }
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Incoming {
    Hello {
        version: u32,
        token: String,
        #[serde(default)]
        mod_version: String,
    },
    Status(GameStatus),
    Screenshot {
        file: String,
    },
    #[serde(other)]
    Unknown,
}

pub type BridgeListener = Arc<dyn Fn(BridgeEvent) + Send + Sync>;

struct Session {
    cancel: CancellationToken,
    outbox: Option<mpsc::UnboundedSender<Value>>,
}

pub struct BridgeHub {
    listener: BridgeListener,
    sessions: Arc<Mutex<HashMap<String, Session>>>,
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

fn clip(text: String) -> String {
    text.chars()
        .filter(|c| !c.is_control())
        .take(MAX_TEXT_CHARS)
        .collect()
}

fn sanitize(status: GameStatus) -> GameStatus {
    GameStatus {
        screen: clip(status.screen),
        world: status.world.map(clip),
        server: status.server.map(clip),
        dimension: status.dimension.map(clip),
    }
}

fn screenshot_name(file: &str) -> Option<String> {
    let valid = !file.is_empty()
        && file.chars().count() <= MAX_TEXT_CHARS
        && !file.starts_with('.')
        && file.to_ascii_lowercase().ends_with(".png")
        && !file
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\' | ':'));
    valid.then(|| file.to_string())
}

impl BridgeHub {
    pub fn new(listener: BridgeListener) -> Self {
        Self {
            listener,
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn open(&self, instance_id: &str) -> CoreResult<BridgeTicket> {
        self.close(instance_id);
        let socket = TcpListener::bind("127.0.0.1:0").await.map_err(|err| {
            CoreError::Unsupported(format!("cannot open the companion bridge: {err}"))
        })?;
        let port = socket
            .local_addr()
            .map_err(|err| {
                CoreError::Unsupported(format!("cannot open the companion bridge: {err}"))
            })?
            .port();
        let token = format!("{}", uuid::Uuid::new_v4().simple());
        let cancel = CancellationToken::new();
        lock(&self.sessions).insert(
            instance_id.to_string(),
            Session {
                cancel: cancel.clone(),
                outbox: None,
            },
        );
        tokio::spawn(accept_loop(
            socket,
            instance_id.to_string(),
            token.clone(),
            cancel,
            self.listener.clone(),
            self.sessions.clone(),
        ));
        Ok(BridgeTicket { port, token })
    }

    pub fn close(&self, instance_id: &str) {
        if let Some(session) = lock(&self.sessions).remove(instance_id) {
            session.cancel.cancel();
        }
    }

    pub fn is_connected(&self, instance_id: &str) -> bool {
        lock(&self.sessions)
            .get(instance_id)
            .is_some_and(|session| session.outbox.is_some())
    }

    pub fn send(&self, instance_id: &str, message: Value) -> bool {
        lock(&self.sessions)
            .get(instance_id)
            .and_then(|session| session.outbox.as_ref())
            .is_some_and(|outbox| outbox.send(message).is_ok())
    }

    pub fn toast(&self, instance_id: &str, title: &str, body: &str) -> bool {
        self.send(
            instance_id,
            json!({ "type": "toast", "title": clip(title.into()), "body": clip(body.into()) }),
        )
    }

    pub fn toast_all(&self, title: &str, body: &str) -> usize {
        let connected: Vec<String> = lock(&self.sessions)
            .iter()
            .filter(|(_, session)| session.outbox.is_some())
            .map(|(id, _)| id.clone())
            .collect();
        connected
            .iter()
            .filter(|id| self.toast(id, title, body))
            .count()
    }
}

async fn accept_loop(
    socket: TcpListener,
    instance_id: String,
    token: String,
    cancel: CancellationToken,
    listener: BridgeListener,
    sessions: Arc<Mutex<HashMap<String, Session>>>,
) {
    loop {
        let stream = tokio::select! {
            () = cancel.cancelled() => return,
            accepted = socket.accept() => match accepted {
                Ok((stream, peer)) if peer.ip().is_loopback() => stream,
                Ok(_) => continue,
                Err(err) => {
                    tracing::warn!(error = %err, "companion bridge accept failed");
                    return;
                }
            },
        };
        let result = tokio::select! {
            () = cancel.cancelled() => return,
            result = serve(stream, &instance_id, &token, &listener, &sessions) => result,
        };
        if let Some(session) = lock(&sessions).get_mut(&instance_id) {
            session.outbox = None;
        }
        match result {
            Ok(true) => listener(BridgeEvent::Disconnected {
                instance_id: instance_id.clone(),
            }),
            Ok(false) => {}
            Err(err) => tracing::debug!(error = %err, "companion bridge connection ended"),
        }
    }
}

async fn next_line(
    reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
) -> std::io::Result<Option<String>> {
    let mut line = Vec::new();
    loop {
        let available = reader.fill_buf().await?;
        if available.is_empty() {
            return Ok((!line.is_empty()).then(|| String::from_utf8_lossy(&line).into_owned()));
        }
        if let Some(end) = available.iter().position(|byte| *byte == b'\n') {
            line.extend_from_slice(&available[..end]);
            reader.consume(end + 1);
            return Ok(Some(String::from_utf8_lossy(&line).into_owned()));
        }
        let length = available.len();
        line.extend_from_slice(available);
        reader.consume(length);
        if line.len() > MAX_LINE_BYTES {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "bridge message is too long",
            ));
        }
    }
}

async fn serve(
    stream: TcpStream,
    instance_id: &str,
    token: &str,
    listener: &BridgeListener,
    sessions: &Arc<Mutex<HashMap<String, Session>>>,
) -> CoreResult<bool> {
    let io = |err: std::io::Error| CoreError::Unreachable(format!("companion bridge: {err}"));
    let (read, mut write) = stream.into_split();
    let mut reader = BufReader::new(read);
    let hello = tokio::time::timeout(HELLO_TIMEOUT, next_line(&mut reader))
        .await
        .map_err(|_| CoreError::Unreachable("companion bridge: no hello".into()))?
        .map_err(io)?
        .unwrap_or_default();
    let mod_version = match serde_json::from_str::<Incoming>(&hello) {
        Ok(Incoming::Hello {
            version,
            token: offered,
            mod_version,
        }) if version == PROTOCOL_VERSION && offered == token => clip(mod_version),
        _ => {
            tracing::warn!("rejected a companion bridge connection");
            return Ok(false);
        }
    };
    let welcome =
        json!({ "type": "welcome", "version": PROTOCOL_VERSION, "instanceId": instance_id });
    write
        .write_all(format!("{welcome}\n").as_bytes())
        .await
        .map_err(io)?;

    let (tx, mut rx) = mpsc::unbounded_channel::<Value>();
    match lock(sessions).get_mut(instance_id) {
        Some(session) => session.outbox = Some(tx),
        None => return Ok(false),
    }
    listener(BridgeEvent::Connected {
        instance_id: instance_id.to_string(),
        mod_version,
    });
    let writer = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if write
                .write_all(format!("{message}\n").as_bytes())
                .await
                .is_err()
            {
                break;
            }
        }
    });
    while let Some(line) = next_line(&mut reader).await.map_err(io)? {
        match serde_json::from_str::<Incoming>(&line) {
            Ok(Incoming::Status(status)) => listener(BridgeEvent::Status {
                instance_id: instance_id.to_string(),
                status: sanitize(status),
            }),
            Ok(Incoming::Screenshot { file }) => {
                if let Some(file) = screenshot_name(&file) {
                    listener(BridgeEvent::Screenshot {
                        instance_id: instance_id.to_string(),
                        file,
                    });
                }
            }
            _ => {}
        }
    }
    writer.abort();
    Ok(true)
}

pub fn java_path(path: &std::path::Path) -> String {
    let text = path.to_string_lossy();
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = text.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        text.into_owned()
    }
}

pub fn companion_jar_name(minecraft_version: &str) -> String {
    format!("orrery-companion-{minecraft_version}.jar")
}

#[derive(Debug, Clone, Deserialize)]
pub struct CompanionManifest {
    pub version: String,
    pub jars: Vec<CompanionJar>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CompanionJar {
    pub minecraft: String,
    pub file: String,
    pub sha512: String,
}

pub async fn companion_jar(
    dir: &std::path::Path,
    minecraft_version: &str,
) -> CoreResult<Option<std::path::PathBuf>> {
    let manifest_path = dir.join("companion.json");
    let manifest: CompanionManifest = match tokio::fs::read(&manifest_path).await {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|source| CoreError::Json {
            path: manifest_path.clone(),
            source,
        })?,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err).at_path(&manifest_path),
    };
    let Some(entry) = manifest
        .jars
        .iter()
        .find(|jar| jar.minecraft == minecraft_version)
    else {
        return Ok(None);
    };
    if entry.file != companion_jar_name(&entry.minecraft) {
        return Err(CoreError::InvalidInput(format!(
            "unexpected companion file {:?}",
            entry.file
        )));
    }
    let path = dir.join(&entry.file);
    let hash_path = path.clone();
    let actual = tokio::task::spawn_blocking(move || crate::modpacks::sha512_file(&hash_path))
        .await
        .map_err(|err| CoreError::Unsupported(format!("hashing panicked: {err}")))??;
    if !actual.eq_ignore_ascii_case(&entry.sha512) {
        return Err(CoreError::ChecksumMismatch {
            url: path.display().to_string(),
            expected: entry.sha512.clone(),
            actual,
        });
    }
    Ok(Some(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn connect(
        ticket: &BridgeTicket,
    ) -> (
        BufReader<tokio::net::tcp::OwnedReadHalf>,
        tokio::net::tcp::OwnedWriteHalf,
    ) {
        let stream = TcpStream::connect(("127.0.0.1", ticket.port))
            .await
            .unwrap();
        let (read, write) = stream.into_split();
        (BufReader::new(read), write)
    }

    #[tokio::test]
    async fn accepts_the_token_and_relays_messages() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = events.clone();
        let hub = BridgeHub::new(Arc::new(move |event| sink.lock().unwrap().push(event)));
        let ticket = hub.open("survival").await.unwrap();
        assert_eq!(
            ticket.jvm_args()[0],
            format!("-Dorrery.bridge=127.0.0.1:{}", ticket.port)
        );

        let (mut reader, mut writer) = connect(&ticket).await;
        writer
            .write_all(
                format!(
                    "{}\n",
                    json!({ "type": "hello", "version": 1, "token": ticket.token, "mod_version": "0.1.0" })
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        let mut welcome = String::new();
        reader.read_line(&mut welcome).await.unwrap();
        assert!(welcome.contains("\"welcome\""));
        for _ in 0..50 {
            if hub.is_connected("survival") {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(hub.toast("survival", "Hallo", "Welt"));
        let mut toast = String::new();
        reader.read_line(&mut toast).await.unwrap();
        assert!(toast.contains("\"toast\"") && toast.contains("Hallo"));
        assert_eq!(hub.toast_all("Mods", "Fertig"), 1);
        let mut broadcast = String::new();
        reader.read_line(&mut broadcast).await.unwrap();
        assert!(broadcast.contains("Fertig"));

        writer
            .write_all(
                concat!(
                    "{\"type\":\"status\",\"screen\":\"world\",\"world\":\"Meine\\u0007 Welt\"}\n",
                    "{\"type\":\"unknown\"}\n",
                    "{\"type\":\"screenshot\",\"file\":\"../evil.png\"}\n",
                    "{\"type\":\"screenshot\",\"file\":\"2026-11-02_10.00.00.png\"}\n",
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        drop(writer);
        drop(reader);
        for _ in 0..100 {
            if events.lock().unwrap().len() >= 4 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let events = events.lock().unwrap();
        assert_eq!(events.len(), 4);
        assert!(
            matches!(&events[0], BridgeEvent::Connected { mod_version, .. } if mod_version == "0.1.0")
        );
        assert!(matches!(
            &events[1],
            BridgeEvent::Status { status, .. } if status.world.as_deref() == Some("Meine Welt")
        ));
        assert!(matches!(
            &events[2],
            BridgeEvent::Screenshot { file, .. } if file == "2026-11-02_10.00.00.png"
        ));
        assert!(matches!(&events[3], BridgeEvent::Disconnected { .. }));
        hub.close("survival");
        assert!(!hub.is_connected("survival"));
    }

    #[tokio::test]
    async fn drops_overlong_lines() {
        let hub = BridgeHub::new(Arc::new(|_| {}));
        let ticket = hub.open("long").await.unwrap();
        let (mut reader, mut writer) = connect(&ticket).await;
        let _ = writer.write_all(&vec![b'a'; MAX_LINE_BYTES * 2]).await;
        let mut reply = String::new();
        let read = tokio::time::timeout(Duration::from_secs(2), reader.read_line(&mut reply))
            .await
            .unwrap()
            .unwrap_or(0);
        assert_eq!(read, 0);
        assert!(!hub.is_connected("long"));
    }

    #[tokio::test]
    async fn rejects_wrong_tokens() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = events.clone();
        let hub = BridgeHub::new(Arc::new(move |event| sink.lock().unwrap().push(event)));
        let ticket = hub.open("pvp").await.unwrap();
        let (mut reader, mut writer) = connect(&ticket).await;
        writer
            .write_all(b"{\"type\":\"hello\",\"version\":1,\"token\":\"wrong\"}\n")
            .await
            .unwrap();
        let mut reply = String::new();
        let read = tokio::time::timeout(Duration::from_secs(2), reader.read_line(&mut reply))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(read, 0);
        assert!(events.lock().unwrap().is_empty());
        assert!(!hub.is_connected("pvp"));
    }

    #[test]
    fn accepts_only_plain_screenshot_names() {
        assert_eq!(
            screenshot_name("2026-11-02_10.00.00_1.png").as_deref(),
            Some("2026-11-02_10.00.00_1.png")
        );
        assert!(screenshot_name("SHOT.PNG").is_some());
        for bad in [
            "",
            ".png",
            "a.jpg",
            "../a.png",
            "dir\\a.png",
            "C:a.png",
            "a\n.png",
        ] {
            assert_eq!(screenshot_name(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn strips_verbatim_prefixes_for_java() {
        use std::path::Path;
        assert_eq!(
            java_path(Path::new(r"\\?\E:\Orrery\resources\companion\a.jar")),
            r"E:\Orrery\resources\companion\a.jar"
        );
        assert_eq!(
            java_path(Path::new(r"\\?\UNC\server\share\a.jar")),
            r"\\server\share\a.jar"
        );
        assert_eq!(
            java_path(Path::new("/opt/orrery/a.jar")),
            "/opt/orrery/a.jar"
        );
    }

    #[tokio::test]
    async fn verifies_bundled_jars() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(companion_jar(dir.path(), "1.21.11").await.unwrap(), None);
        std::fs::write(dir.path().join("orrery-companion-1.21.11.jar"), b"jar").unwrap();
        let hash =
            crate::modpacks::sha512_file(&dir.path().join("orrery-companion-1.21.11.jar")).unwrap();
        let manifest = json!({
            "version": "0.1.0",
            "jars": [
                { "minecraft": "1.21.11", "file": "orrery-companion-1.21.11.jar", "sha512": hash },
                { "minecraft": "1.20.1", "file": "orrery-companion-1.20.1.jar", "sha512": "00" },
                { "minecraft": "26.3", "file": "../evil.jar", "sha512": "00" }
            ]
        });
        std::fs::write(dir.path().join("companion.json"), manifest.to_string()).unwrap();
        assert!(
            companion_jar(dir.path(), "1.21.11")
                .await
                .unwrap()
                .is_some()
        );
        assert_eq!(companion_jar(dir.path(), "1.21.1").await.unwrap(), None);
        assert!(companion_jar(dir.path(), "1.20.1").await.is_err());
        assert!(companion_jar(dir.path(), "26.3").await.is_err());
    }
}

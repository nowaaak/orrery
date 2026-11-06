use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::{DEFAULT_PORT, ServerAddress};
use crate::CoreResult;
use crate::instances::InstanceStore;
use crate::storage;

pub const PLAYTIME_FILE: &str = "server-playtime.json";
const MAX_SERVERS: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub server: String,
    pub secs: u64,
}

pub fn server_key(address: &str) -> Option<String> {
    let parsed = ServerAddress::parse(address).ok()?;
    let host = parsed.host.trim_end_matches('.').to_ascii_lowercase();
    if host.is_empty() {
        return None;
    }
    Some(match (parsed.port, host.contains(':')) {
        (DEFAULT_PORT, _) => host,
        (port, true) => format!("[{host}]:{port}"),
        (port, false) => format!("{host}:{port}"),
    })
}

pub async fn read(instance_dir: &Path) -> CoreResult<BTreeMap<String, u64>> {
    Ok(storage::read_json(&instance_dir.join(PLAYTIME_FILE))
        .await?
        .unwrap_or_default())
}

pub struct ServerPlaytime {
    instances: Arc<InstanceStore>,
    sessions: Mutex<HashMap<String, (String, Instant)>>,
    write_lock: tokio::sync::Mutex<()>,
}

impl ServerPlaytime {
    pub fn new(instances: Arc<InstanceStore>) -> Self {
        Self {
            instances,
            sessions: Mutex::new(HashMap::new()),
            write_lock: tokio::sync::Mutex::new(()),
        }
    }

    fn sessions(&self) -> std::sync::MutexGuard<'_, HashMap<String, (String, Instant)>> {
        self.sessions.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn observe(&self, instance_id: &str, server: Option<&str>) -> Option<Segment> {
        self.observe_at(instance_id, server, Instant::now())
    }

    fn observe_at(&self, instance_id: &str, server: Option<&str>, now: Instant) -> Option<Segment> {
        let key = server.and_then(server_key);
        let mut sessions = self.sessions();
        if let (Some((current, _)), Some(key)) = (sessions.get(instance_id), &key)
            && current == key
        {
            return None;
        }
        let finished = sessions
            .remove(instance_id)
            .map(|(server, since)| Segment {
                server,
                secs: now.saturating_duration_since(since).as_secs(),
            })
            .filter(|segment| segment.secs > 0);
        if let Some(key) = key {
            sessions.insert(instance_id.to_string(), (key, now));
        }
        finished
    }

    pub fn track(self: &Arc<Self>, instance_id: &str, server: Option<&str>) {
        let Some(segment) = self.observe(instance_id, server) else {
            return;
        };
        let tracker = self.clone();
        let instance_id = instance_id.to_string();
        tokio::spawn(async move { tracker.record(&instance_id, segment).await });
    }

    pub async fn finish(&self, instance_id: &str) {
        if let Some(segment) = self.observe(instance_id, None) {
            self.record(instance_id, segment).await;
        }
    }

    pub async fn record(&self, instance_id: &str, segment: Segment) {
        if let Err(err) = self.add(instance_id, &segment).await {
            tracing::warn!(error = %err, "failed to store the server playtime");
        }
    }

    async fn add(&self, instance_id: &str, segment: &Segment) -> CoreResult<()> {
        let _guard = self.write_lock.lock().await;
        let dir = self.instances.instance_dir(instance_id)?;
        if tokio::fs::metadata(&dir).await.is_err() {
            return Ok(());
        }
        let mut totals = read(&dir).await?;
        let total = totals.entry(segment.server.clone()).or_default();
        *total = total.saturating_add(segment.secs);
        while totals.len() > MAX_SERVERS {
            let Some(smallest) = totals
                .iter()
                .filter(|(server, _)| **server != segment.server)
                .min_by_key(|(_, secs)| **secs)
                .map(|(server, _)| server.clone())
            else {
                break;
            };
            totals.remove(&smallest);
        }
        storage::write_json_atomic(&dir.join(PLAYTIME_FILE), &totals).await
    }

    pub async fn totals(&self, instance_id: &str) -> CoreResult<BTreeMap<String, u64>> {
        let mut totals = read(&self.instances.instance_dir(instance_id)?).await?;
        if let Some((server, since)) = self.sessions().get(instance_id) {
            let total = totals.entry(server.clone()).or_default();
            *total = total.saturating_add(since.elapsed().as_secs());
        }
        Ok(totals)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn normalizes_server_keys() {
        assert_eq!(
            server_key("Play.Example.org").as_deref(),
            Some("play.example.org")
        );
        assert_eq!(
            server_key("play.example.org:25565").as_deref(),
            Some("play.example.org")
        );
        assert_eq!(server_key("example.org.").as_deref(), Some("example.org"));
        assert_eq!(
            server_key("localhost:25566").as_deref(),
            Some("localhost:25566")
        );
        assert_eq!(server_key("[::1]:25570").as_deref(), Some("[::1]:25570"));
        assert_eq!(server_key("not a server"), None);
    }

    #[tokio::test]
    async fn adds_up_time_per_server() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("survival")).unwrap();
        let playtime = ServerPlaytime::new(Arc::new(InstanceStore::new(dir.path().to_path_buf())));
        let start = Instant::now();
        let at = |secs| start + Duration::from_secs(secs);

        assert_eq!(
            playtime.observe_at("survival", Some("Example.org"), at(0)),
            None
        );
        assert_eq!(
            playtime.observe_at("survival", Some("example.org:25565"), at(30)),
            None
        );
        let first = playtime
            .observe_at("survival", Some("localhost:25566"), at(90))
            .unwrap();
        assert_eq!(
            first,
            Segment {
                server: "example.org".into(),
                secs: 90
            }
        );
        let second = playtime.observe_at("survival", None, at(100)).unwrap();
        assert_eq!(second.secs, 10);
        assert_eq!(playtime.observe_at("survival", None, at(200)), None);

        playtime.record("survival", first.clone()).await;
        playtime.record("survival", first).await;
        playtime.record("survival", second).await;
        playtime
            .record(
                "missing",
                Segment {
                    server: "a".into(),
                    secs: 5,
                },
            )
            .await;
        let totals = playtime.totals("survival").await.unwrap();
        assert_eq!(totals.get("example.org"), Some(&180));
        assert_eq!(totals.get("localhost:25566"), Some(&10));
        assert!(!dir.path().join("missing").exists());
    }
}

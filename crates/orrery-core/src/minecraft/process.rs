use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use ts_rs::TS;

use super::crash::CrashReport;
use crate::error::IoResultExt;
use crate::{CoreError, CoreResult};

const LOG_FLUSH_INTERVAL: Duration = Duration::from_millis(100);
const LOG_BATCH_LIMIT: usize = 500;
const LOG_TAIL_LINES: usize = 5000;
const LOG_DRAIN_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_LINE_CHARS: usize = 4096;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(export)]
pub enum GameState {
    Running {
        pid: u32,
    },
    Exited {
        code: Option<i32>,
        crash: Option<CrashReport>,
    },
}

pub struct ExitInfo {
    pub code: Option<i32>,
    pub played_secs: u64,
    pub stopped_by_user: bool,
    pub log_tail: Vec<String>,
}

pub type ExitHook =
    Box<dyn FnOnce(ExitInfo) -> Pin<Box<dyn Future<Output = Option<CrashReport>> + Send>> + Send>;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameStateEvent {
    pub instance_id: String,
    pub state: GameState,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameLogEvent {
    pub instance_id: String,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RunningGame {
    pub instance_id: String,
    pub pid: u32,
}

pub trait GameListener: Send + Sync {
    fn state(&self, event: GameStateEvent);
    fn log(&self, event: GameLogEvent);
}

struct Running {
    pid: u32,
    kill: CancellationToken,
}

pub struct GameRegistry {
    running: Mutex<HashMap<String, Running>>,
    logs: Mutex<HashMap<String, VecDeque<String>>>,
    listener: Arc<dyn GameListener>,
}

pub struct GameCommand<'a> {
    pub instance_id: &'a str,
    pub program: &'a Path,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub working_dir: &'a Path,
    pub on_exit: Option<ExitHook>,
}

impl GameRegistry {
    pub fn new(listener: Arc<dyn GameListener>) -> Arc<Self> {
        Arc::new(Self {
            running: Mutex::new(HashMap::new()),
            logs: Mutex::new(HashMap::new()),
            listener,
        })
    }

    pub fn is_running(&self, instance_id: &str) -> bool {
        lock(&self.running).contains_key(instance_id)
    }

    pub fn running(&self) -> Vec<RunningGame> {
        lock(&self.running)
            .iter()
            .map(|(instance_id, running)| RunningGame {
                instance_id: instance_id.clone(),
                pid: running.pid,
            })
            .collect()
    }

    pub fn log_tail(&self, instance_id: &str) -> Vec<String> {
        lock(&self.logs)
            .get(instance_id)
            .map(|lines| lines.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn kill(&self, instance_id: &str) -> CoreResult<()> {
        let running = lock(&self.running);
        let game = running.get(instance_id).ok_or_else(|| {
            CoreError::NotFound(format!("running game for instance {instance_id}"))
        })?;
        game.kill.cancel();
        Ok(())
    }

    pub fn spawn(self: &Arc<Self>, command: GameCommand<'_>) -> CoreResult<u32> {
        let instance_id = command.instance_id.to_string();
        if self.is_running(&instance_id) {
            return Err(CoreError::InvalidInput(format!(
                "instance {instance_id} is already running"
            )));
        }

        let mut process = tokio::process::Command::new(command.program);
        process
            .args(&command.args)
            .envs(command.env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
            .current_dir(command.working_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        process.creation_flags(CREATE_NO_WINDOW);

        let mut child = process.spawn().at_path(command.program)?;
        let started = Instant::now();
        let on_exit = command.on_exit;
        let pid = child.id().unwrap_or_default();
        let kill = CancellationToken::new();
        lock(&self.running).insert(
            instance_id.clone(),
            Running {
                pid,
                kill: kill.clone(),
            },
        );
        lock(&self.logs).insert(instance_id.clone(), VecDeque::new());
        self.listener.state(GameStateEvent {
            instance_id: instance_id.clone(),
            state: GameState::Running { pid },
        });

        let (tx, rx) = mpsc::unbounded_channel();
        if let Some(stdout) = child.stdout.take() {
            tokio::spawn(forward_lines(stdout, tx.clone()));
        }
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(forward_lines(stderr, tx));
        }
        let logs_done = tokio::spawn(self.clone().batch_logs(instance_id.clone(), rx));

        let registry = self.clone();
        tokio::spawn(async move {
            let mut stopped_by_user = false;
            let status = tokio::select! {
                status = child.wait() => status,
                () = kill.cancelled() => {
                    stopped_by_user = true;
                    if let Err(err) = child.start_kill() {
                        tracing::warn!(error = %err, "failed to kill game process");
                    }
                    child.wait().await
                }
            };
            let mut logs_done = logs_done;
            if tokio::time::timeout(LOG_DRAIN_TIMEOUT, &mut logs_done)
                .await
                .is_err()
            {
                tracing::warn!(instance = %instance_id, "game output stayed open after exit, stopped reading it");
                logs_done.abort();
            }
            let code = status.ok().and_then(|s| s.code());
            tracing::info!(instance = %instance_id, ?code, "game exited");
            let crash = match on_exit {
                Some(hook) => {
                    hook(ExitInfo {
                        code,
                        played_secs: started.elapsed().as_secs(),
                        stopped_by_user,
                        log_tail: registry.log_tail(&instance_id),
                    })
                    .await
                }
                None => None,
            };
            lock(&registry.running).remove(&instance_id);
            registry.listener.state(GameStateEvent {
                instance_id,
                state: GameState::Exited { code, crash },
            });
        });
        Ok(pid)
    }

    async fn batch_logs(
        self: Arc<Self>,
        instance_id: String,
        mut rx: mpsc::UnboundedReceiver<String>,
    ) {
        let mut ticker = tokio::time::interval(LOG_FLUSH_INTERVAL);
        let mut batch = Vec::new();
        loop {
            tokio::select! {
                line = rx.recv() => match line {
                    Some(line) => {
                        batch.push(line);
                        if batch.len() >= LOG_BATCH_LIMIT {
                            self.flush_logs(&instance_id, &mut batch);
                        }
                    }
                    None => break,
                },
                _ = ticker.tick() => self.flush_logs(&instance_id, &mut batch),
            }
        }
        self.flush_logs(&instance_id, &mut batch);
    }

    fn flush_logs(&self, instance_id: &str, batch: &mut Vec<String>) {
        if batch.is_empty() {
            return;
        }
        let lines = std::mem::take(batch);
        {
            let mut logs = lock(&self.logs);
            let tail = logs.entry(instance_id.to_string()).or_default();
            for line in &lines {
                if tail.len() == LOG_TAIL_LINES {
                    tail.pop_front();
                }
                tail.push_back(line.clone());
            }
        }
        self.listener.log(GameLogEvent {
            instance_id: instance_id.to_string(),
            lines,
        });
    }
}

async fn forward_lines(reader: impl AsyncRead + Unpin, tx: mpsc::UnboundedSender<String>) {
    let mut lines = BufReader::new(reader).lines();
    let mut parser = Log4jParser::default();
    loop {
        match lines.next_line().await {
            Ok(Some(line)) => {
                for mut out in parser.push(line) {
                    if out.len() > MAX_LINE_CHARS {
                        let cut = (0..=MAX_LINE_CHARS)
                            .rev()
                            .find(|i| out.is_char_boundary(*i))
                            .unwrap_or(0);
                        out.truncate(cut);
                    }
                    if tx.send(out).is_err() {
                        return;
                    }
                }
            }
            Ok(None) => break,
            Err(err) => {
                tracing::debug!(error = %err, "stopped reading game output");
                break;
            }
        }
    }
    for line in parser.finish() {
        let _ = tx.send(line);
    }
}

#[derive(Default)]
struct Log4jParser {
    event: Option<Vec<String>>,
}

impl Log4jParser {
    fn push(&mut self, line: String) -> Vec<String> {
        match &mut self.event {
            None if line.trim_start().starts_with("<log4j:Event") => {
                if line.contains("</log4j:Event>") {
                    format_event(&line)
                } else {
                    self.event = Some(vec![line]);
                    Vec::new()
                }
            }
            None => vec![line],
            Some(buffer) => {
                let done = line.contains("</log4j:Event>");
                buffer.push(line);
                if done {
                    let joined = buffer.join("\n");
                    self.event = None;
                    format_event(&joined)
                } else {
                    Vec::new()
                }
            }
        }
    }

    fn finish(&mut self) -> Vec<String> {
        self.event.take().unwrap_or_default()
    }
}

fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let key = format!(" {name}=\"");
    let start = tag.find(&key)? + key.len();
    let end = tag[start..].find('"')?;
    Some(&tag[start..start + end])
}

fn cdata_after<'a>(text: &'a str, element: &str) -> Option<&'a str> {
    let start = text.find(element)?;
    let rest = &text[start..];
    let open = rest.find("<![CDATA[")? + "<![CDATA[".len();
    let close = rest[open..].find("]]>")?;
    Some(&rest[open..open + close])
}

fn format_event(xml: &str) -> Vec<String> {
    let header_end = xml.find('>').unwrap_or(xml.len());
    let header = &xml[..header_end];
    let level = attribute(header, "level").unwrap_or("INFO");
    let thread = attribute(header, "thread").unwrap_or("main");
    let message = cdata_after(xml, "<log4j:Message>").unwrap_or_default();

    let mut lines: Vec<String> = Vec::new();
    for (index, part) in message.split('\n').enumerate() {
        if index == 0 {
            lines.push(format!("[{thread}/{level}]: {part}"));
        } else {
            lines.push(part.to_string());
        }
    }
    if let Some(throwable) = cdata_after(xml, "<log4j:Throwable>") {
        lines.extend(throwable.lines().map(str::to_string));
    }
    lines
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[derive(Default)]
    struct Recorder {
        states: Mutex<Vec<GameStateEvent>>,
        lines: Mutex<Vec<String>>,
    }

    impl GameListener for Recorder {
        fn state(&self, event: GameStateEvent) {
            self.states.lock().unwrap().push(event);
        }
        fn log(&self, event: GameLogEvent) {
            self.lines.lock().unwrap().extend(event.lines);
        }
    }

    fn shell(script: &str) -> (PathBuf, Vec<String>) {
        if cfg!(windows) {
            ("cmd".into(), vec!["/C".into(), script.into()])
        } else {
            ("sh".into(), vec!["-c".into(), script.into()])
        }
    }

    async fn wait_until_stopped(registry: &GameRegistry, id: &str) {
        for _ in 0..500 {
            if !registry.is_running(id) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("process did not exit");
    }

    #[test]
    fn log4j_events_are_converted_to_plain_lines() {
        let mut parser = Log4jParser::default();
        let mut out = Vec::new();
        for line in [
            "plain line",
            r#"<log4j:Event logger="fuu" timestamp="1" level="WARN" thread="Render thread">"#,
            "  <log4j:Message><![CDATA[first",
            "second]]></log4j:Message>",
            "  <log4j:Throwable><![CDATA[java.lang.Exception: boom",
            "\tat x.y(Z.java:1)",
            "]]></log4j:Throwable>",
            "</log4j:Event>",
        ] {
            out.extend(parser.push(line.to_string()));
        }
        assert_eq!(
            out,
            [
                "plain line",
                "[Render thread/WARN]: first",
                "second",
                "java.lang.Exception: boom",
                "\tat x.y(Z.java:1)",
            ]
        );
        assert!(parser.finish().is_empty());
    }

    #[tokio::test]
    async fn captures_output_and_exit_code() {
        let recorder = Arc::new(Recorder::default());
        let registry = GameRegistry::new(recorder.clone());
        let dir = tempfile::tempdir().unwrap();
        let (program, args) = shell("echo hello && exit 3");

        registry
            .spawn(GameCommand {
                instance_id: "a",
                program: &program,
                args,
                env: Vec::new(),
                working_dir: dir.path(),
                on_exit: None,
            })
            .unwrap();
        wait_until_stopped(&registry, "a").await;

        assert_eq!(
            recorder.lines.lock().unwrap().first().map(|l| l.trim()),
            Some("hello")
        );
        assert_eq!(registry.log_tail("a").len(), 1);
        let states = recorder.states.lock().unwrap();
        assert!(matches!(
            states.last().unwrap().state,
            GameState::Exited { code: Some(3), .. }
        ));
    }

    #[tokio::test]
    async fn passes_environment_to_the_process() {
        let recorder = Arc::new(Recorder::default());
        let registry = GameRegistry::new(recorder.clone());
        let dir = tempfile::tempdir().unwrap();
        let script = if cfg!(windows) {
            "echo %ORRERY_ENV%"
        } else {
            "echo $ORRERY_ENV"
        };
        let (program, args) = shell(script);

        registry
            .spawn(GameCommand {
                instance_id: "env",
                program: &program,
                args,
                env: vec![("ORRERY_ENV".into(), "from launcher".into())],
                working_dir: dir.path(),
                on_exit: None,
            })
            .unwrap();
        wait_until_stopped(&registry, "env").await;
        assert_eq!(
            recorder.lines.lock().unwrap().first().map(|l| l.trim()),
            Some("from launcher")
        );
    }

    #[tokio::test]
    async fn exit_is_reported_while_a_child_keeps_the_output_open() {
        let recorder = Arc::new(Recorder::default());
        let registry = GameRegistry::new(recorder.clone());
        let dir = tempfile::tempdir().unwrap();
        let script = if cfg!(windows) {
            "start /b ping -n 6 127.0.0.1 & echo started"
        } else {
            "sleep 5 & echo started"
        };
        let (program, args) = shell(script);

        registry
            .spawn(GameCommand {
                instance_id: "c",
                program: &program,
                args,
                env: Vec::new(),
                working_dir: dir.path(),
                on_exit: None,
            })
            .unwrap();
        wait_until_stopped(&registry, "c").await;
        assert!(
            recorder
                .lines
                .lock()
                .unwrap()
                .iter()
                .any(|line| line == "started")
        );
        assert!(matches!(
            recorder
                .states
                .lock()
                .unwrap()
                .last()
                .map(|event| &event.state),
            Some(GameState::Exited { code: Some(0), .. })
        ));
    }

    #[tokio::test]
    async fn kill_stops_a_long_running_process() {
        let recorder = Arc::new(Recorder::default());
        let registry = GameRegistry::new(recorder.clone());
        let dir = tempfile::tempdir().unwrap();
        let script = if cfg!(windows) {
            "ping -n 30 127.0.0.1 > NUL"
        } else {
            "sleep 30"
        };
        let (program, args) = shell(script);

        registry
            .spawn(GameCommand {
                instance_id: "b",
                program: &program,
                args: args.clone(),
                env: Vec::new(),
                working_dir: dir.path(),
                on_exit: None,
            })
            .unwrap();
        assert!(
            registry
                .spawn(GameCommand {
                    instance_id: "b",
                    program: &program,
                    args,
                    env: Vec::new(),
                    working_dir: dir.path(),
                    on_exit: None,
                })
                .is_err()
        );

        registry.kill("b").unwrap();
        wait_until_stopped(&registry, "b").await;
        assert!(registry.running().is_empty());
    }
}

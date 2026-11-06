use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio_util::sync::CancellationToken;

use crate::error::IoResultExt;
use crate::{CoreError, CoreResult};

const OUTPUT_TAIL_LINES: usize = 12;
const MAX_OUTPUT_LINE_CHARS: usize = 300;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookStage {
    PreLaunch,
    PostExit,
}

impl HookStage {
    fn label(self) -> &'static str {
        match self {
            Self::PreLaunch => "pre-launch",
            Self::PostExit => "post-exit",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct HookContext {
    values: Vec<(&'static str, String)>,
}

pub struct HookPaths<'a> {
    pub instance_dir: &'a Path,
    pub game_dir: &'a Path,
    pub java: &'a Path,
}

impl HookContext {
    pub fn new(id: &str, name: &str, paths: HookPaths<'_>, java_args: &[String]) -> Self {
        let path = |p: &Path| p.to_string_lossy().into_owned();
        Self {
            values: vec![
                ("INST_NAME", name.to_string()),
                ("INST_ID", id.to_string()),
                ("INST_DIR", path(paths.instance_dir)),
                ("INST_MC_DIR", path(paths.game_dir)),
                ("INST_JAVA", path(paths.java)),
                ("INST_JAVA_ARGS", java_args.join(" ")),
            ],
        }
    }

    pub fn env(&self) -> impl Iterator<Item = (String, String)> + '_ {
        self.values
            .iter()
            .map(|(key, value)| ((*key).to_string(), value.clone()))
    }

    pub fn expand(&self, template: &str) -> String {
        let mut out = String::with_capacity(template.len());
        let mut rest = template;
        while let Some(start) = rest.find('$') {
            out.push_str(&rest[..start]);
            let after = &rest[start + 1..];
            let (name, consumed) = match after.strip_prefix('{') {
                Some(inner) => match inner.find('}') {
                    Some(end) => (&inner[..end], end + 2),
                    None => ("", 0),
                },
                None => {
                    let end = after
                        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                        .unwrap_or(after.len());
                    (&after[..end], end)
                }
            };
            match self.values.iter().find(|(key, _)| *key == name) {
                Some((_, value)) if consumed > 0 => {
                    out.push_str(value);
                    rest = &after[consumed..];
                }
                _ => {
                    out.push('$');
                    rest = after;
                }
            }
        }
        out.push_str(rest);
        out
    }
}

pub fn split_command(command: &str) -> CoreResult<Vec<String>> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut in_word = false;
    let mut quote: Option<char> = None;
    let mut chars = command.chars().peekable();
    while let Some(c) = chars.next() {
        match quote {
            Some(q) if c == q => quote = None,
            Some('"') if c == '\\' && matches!(chars.peek(), Some('"' | '\\')) => {
                current.extend(chars.next());
            }
            Some(_) => current.push(c),
            None if c == '"' || c == '\'' => {
                quote = Some(c);
                in_word = true;
            }
            None if c.is_whitespace() => {
                if in_word {
                    words.push(std::mem::take(&mut current));
                    in_word = false;
                }
            }
            None => {
                current.push(c);
                in_word = true;
            }
        }
    }
    if quote.is_some() {
        return Err(CoreError::InvalidInput(format!(
            "unclosed quote in command {command:?}"
        )));
    }
    if in_word {
        words.push(current);
    }
    Ok(words)
}

pub fn validate_command(command: &str) -> CoreResult<()> {
    if command.contains(['\n', '\r', '\0']) {
        return Err(CoreError::InvalidInput(
            "commands must fit on one line".into(),
        ));
    }
    split_command(command).map(|_| ())
}

pub fn wrap_command(java: &Path, args: Vec<String>, prefix: Vec<String>) -> (PathBuf, Vec<String>) {
    let mut prefix = prefix.into_iter();
    match prefix.next() {
        None => (java.to_path_buf(), args),
        Some(program) => {
            let mut wrapped: Vec<String> = prefix.collect();
            wrapped.push(java.to_string_lossy().into_owned());
            wrapped.extend(args);
            (PathBuf::from(program), wrapped)
        }
    }
}

pub async fn run_hook(
    stage: HookStage,
    command: &str,
    env: &[(String, String)],
    working_dir: &Path,
    cancel: &CancellationToken,
) -> CoreResult<()> {
    let mut process = shell_command(command);
    process
        .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .current_dir(working_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    process.creation_flags(CREATE_NO_WINDOW);

    tracing::info!(stage = stage.label(), "running launch hook");
    let mut child = process.spawn().at_path(working_dir)?;
    let stdout = child
        .stdout
        .take()
        .map(|out| tokio::spawn(collect_tail(out)));
    let stderr = child
        .stderr
        .take()
        .map(|err| tokio::spawn(collect_tail(err)));
    let status = tokio::select! {
        status = child.wait() => status.at_path(working_dir)?,
        () = cancel.cancelled() => {
            let _ = child.start_kill();
            return Err(CoreError::Cancelled);
        }
    };
    let mut output = Vec::new();
    for task in [stdout, stderr].into_iter().flatten() {
        output.extend(task.await.unwrap_or_default());
    }
    for line in &output {
        tracing::info!(stage = stage.label(), "{line}");
    }
    if status.success() {
        return Ok(());
    }
    Err(CoreError::LaunchHook {
        stage: stage.label(),
        code: status.code(),
        output: output.join("\n"),
    })
}

#[cfg(windows)]
fn shell_command(command: &str) -> tokio::process::Command {
    let mut process = tokio::process::Command::new("cmd");
    process.raw_arg(format!("/D /S /C \"{command}\""));
    process
}

#[cfg(not(windows))]
fn shell_command(command: &str) -> tokio::process::Command {
    let mut process = tokio::process::Command::new("sh");
    process.arg("-c").arg(command);
    process
}

async fn collect_tail(reader: impl AsyncRead + Unpin) -> Vec<String> {
    let mut lines = BufReader::new(reader).lines();
    let mut tail = VecDeque::with_capacity(OUTPUT_TAIL_LINES);
    while let Ok(Some(line)) = lines.next_line().await {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }
        if tail.len() == OUTPUT_TAIL_LINES {
            tail.pop_front();
        }
        tail.push_back(line.chars().take(MAX_OUTPUT_LINE_CHARS).collect());
    }
    tail.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> HookContext {
        HookContext::new(
            "survival",
            "Survival Welt",
            HookPaths {
                instance_dir: Path::new("/data/instances/survival"),
                game_dir: Path::new("/data/instances/survival/minecraft"),
                java: Path::new("/java/bin/java"),
            },
            &["-Xmx4096M".into(), "-Dfoo=bar".into()],
        )
    }

    #[test]
    fn expands_known_placeholders_only() {
        let ctx = context();
        assert_eq!(
            ctx.expand("backup $INST_ID ${INST_NAME} \"$INST_MC_DIR\" $INST_JAVA_ARGS $HOME $ 5$"),
            "backup survival Survival Welt \"/data/instances/survival/minecraft\" -Xmx4096M -Dfoo=bar $HOME $ 5$"
        );
        assert_eq!(
            ctx.expand("$INST_JAVA/x ${INST_DIR"),
            "/java/bin/java/x ${INST_DIR"
        );
        assert!(ctx.env().any(|(k, v)| k == "INST_ID" && v == "survival"));
    }

    #[test]
    fn splits_commands_with_quotes() {
        assert_eq!(
            split_command(r#"prime-run  "C:\Program Files\tool.exe" --flag='a b' "say \"hi\"""#)
                .unwrap(),
            [
                "prime-run",
                r"C:\Program Files\tool.exe",
                "--flag=a b",
                r#"say "hi""#
            ]
        );
        assert_eq!(split_command("  ").unwrap(), Vec::<String>::new());
        assert_eq!(split_command("a '' b").unwrap(), ["a", "", "b"]);
        assert!(split_command("broken \"quote").is_err());
        assert!(validate_command("one\ntwo").is_err());
    }

    #[test]
    fn wraps_the_java_command() {
        let java = Path::new("/java/bin/java");
        let args = vec!["-Xmx1G".to_string(), "Main".to_string()];
        assert_eq!(
            wrap_command(java, args.clone(), Vec::new()),
            (java.to_path_buf(), args.clone())
        );
        assert_eq!(
            wrap_command(
                java,
                args,
                vec!["gamemoderun".into(), "mangohud".into(), "--dlsym".into()]
            ),
            (
                PathBuf::from("gamemoderun"),
                vec![
                    "mangohud".to_string(),
                    "--dlsym".into(),
                    "/java/bin/java".into(),
                    "-Xmx1G".into(),
                    "Main".into()
                ]
            )
        );
    }

    #[tokio::test]
    async fn failing_hook_reports_exit_code_and_output() {
        let dir = tempfile::tempdir().unwrap();
        let err = run_hook(
            HookStage::PreLaunch,
            "echo checking mods && exit 7",
            &[],
            dir.path(),
            &CancellationToken::new(),
        )
        .await
        .unwrap_err();
        assert!(
            err.to_string()
                .starts_with("pre-launch command failed with exit code 7: "),
            "{err}"
        );
        match err {
            CoreError::LaunchHook {
                stage,
                code,
                output,
            } => {
                assert_eq!(stage, "pre-launch");
                assert_eq!(code, Some(7));
                assert!(output.contains("checking mods"), "{output}");
            }
            other => panic!("unexpected error {other:?}"),
        }
    }

    #[tokio::test]
    async fn hook_sees_environment_and_working_dir() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = context();
        let mut env: Vec<(String, String)> = ctx.env().collect();
        env.push(("ORRERY_TEST".into(), "value 1".into()));
        let command = if cfg!(windows) {
            "echo %INST_ID%-%ORRERY_TEST% > marker.txt"
        } else {
            "echo \"$INST_ID-$ORRERY_TEST\" > marker.txt"
        };
        run_hook(
            HookStage::PostExit,
            command,
            &env,
            dir.path(),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
        let marker = std::fs::read_to_string(PathBuf::from(dir.path()).join("marker.txt")).unwrap();
        assert_eq!(marker.trim(), "survival-value 1");
    }
}

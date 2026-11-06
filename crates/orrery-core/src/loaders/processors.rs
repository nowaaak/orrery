use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::Deserialize;

use crate::error::IoResultExt;
use crate::minecraft::libraries::library_location;
use crate::minecraft::maven::{MavenCoordinate, safe_join};
use crate::minecraft::version::{FileDownload, Library};
use crate::net::{Checksum, Download, Http, VerifyMode, download_all, file_matches};
use crate::platform::Platform;
use crate::progress::Progress;
use crate::{CoreError, CoreResult};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
const MOJMAPS_TASK: &str = "DOWNLOAD_MOJMAPS";

#[derive(Debug, Clone, Deserialize)]
pub struct InstallProfile {
    pub version: String,
    pub minecraft: String,
    #[serde(default)]
    pub data: HashMap<String, SidedValue>,
    #[serde(default)]
    pub processors: Vec<Processor>,
    #[serde(default)]
    pub libraries: Vec<Library>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SidedValue {
    pub client: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Processor {
    #[serde(default)]
    pub sides: Option<Vec<String>>,
    pub jar: String,
    #[serde(default)]
    pub classpath: Vec<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub outputs: HashMap<String, String>,
}

impl Processor {
    pub fn runs_on_client(&self) -> bool {
        self.sides
            .as_ref()
            .is_none_or(|sides| sides.iter().any(|side| side == "client"))
    }

    fn mojmaps_output(&self) -> Option<&str> {
        let task = self
            .args
            .windows(2)
            .any(|pair| pair[0] == "--task" && pair[1] == MOJMAPS_TASK);
        if !task {
            return None;
        }
        self.args
            .windows(2)
            .find(|pair| pair[0] == "--output")
            .map(|pair| pair[1].as_str())
    }
}

pub enum DataValue<'a> {
    Artifact(&'a str),
    Literal(&'a str),
    InstallerFile(&'a str),
}

pub fn classify(raw: &str) -> DataValue<'_> {
    if let Some(inner) = raw.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
        DataValue::Artifact(inner)
    } else if let Some(inner) = raw.strip_prefix('\'').and_then(|r| r.strip_suffix('\'')) {
        DataValue::Literal(inner)
    } else {
        DataValue::InstallerFile(raw)
    }
}

pub fn artifact_path(libraries_dir: &Path, coordinate: &str) -> CoreResult<PathBuf> {
    safe_join(
        libraries_dir,
        &MavenCoordinate::parse(coordinate)?.relative_path(),
    )
}

pub fn substitute(arg: &str, values: &HashMap<String, String>) -> CoreResult<String> {
    let mut out = String::with_capacity(arg.len());
    let mut rest = arg;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let end = after.find('}').ok_or_else(|| {
            CoreError::InvalidInput(format!("unterminated processor variable in {arg:?}"))
        })?;
        let key = &after[..end];
        let value = values.get(key).ok_or_else(|| {
            CoreError::InvalidInput(format!("unknown processor variable {key:?}"))
        })?;
        out.push_str(value);
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

pub fn referenced_keys(processors: &[Processor]) -> std::collections::HashSet<&str> {
    let mut keys = std::collections::HashSet::new();
    for processor in processors.iter().filter(|p| p.runs_on_client()) {
        for arg in &processor.args {
            let mut rest = arg.as_str();
            while let Some(start) = rest.find('{') {
                let after = &rest[start + 1..];
                let Some(end) = after.find('}') else {
                    break;
                };
                keys.insert(&after[..end]);
                rest = &after[end + 1..];
            }
        }
    }
    keys
}

fn resolve_arg(
    arg: &str,
    values: &HashMap<String, String>,
    libraries_dir: &Path,
) -> CoreResult<String> {
    match classify(arg) {
        DataValue::Artifact(coordinate) => Ok(artifact_path(libraries_dir, coordinate)?
            .to_string_lossy()
            .into_owned()),
        _ => substitute(arg, values),
    }
}

fn parse_main_class(manifest: &str) -> Option<String> {
    let mut joined: Vec<String> = Vec::new();
    for line in manifest.lines() {
        match line.strip_prefix(' ') {
            Some(continued) => {
                if let Some(last) = joined.last_mut() {
                    last.push_str(continued);
                }
            }
            None => joined.push(line.to_string()),
        }
    }
    joined.iter().find_map(|line| {
        line.strip_prefix("Main-Class:")
            .map(|class| class.trim().to_string())
            .filter(|class| !class.is_empty())
    })
}

fn read_main_class(jar: &Path) -> CoreResult<String> {
    let file = std::fs::File::open(jar).at_path(jar)?;
    let archive_error = |message: String| CoreError::Archive {
        path: jar.to_path_buf(),
        message,
    };
    let mut zip = zip::ZipArchive::new(file).map_err(|e| archive_error(e.to_string()))?;
    let entry = zip
        .by_name("META-INF/MANIFEST.MF")
        .map_err(|e| archive_error(e.to_string()))?;
    let mut text = String::new();
    entry
        .take(MAX_MANIFEST_BYTES)
        .read_to_string(&mut text)
        .at_path(jar)?;
    parse_main_class(&text).ok_or_else(|| archive_error("the jar has no Main-Class".into()))
}

pub fn library_downloads(libraries: &[Library], libraries_dir: &Path) -> CoreResult<Vec<Download>> {
    let mut downloads = Vec::new();
    for library in libraries {
        let coordinate = MavenCoordinate::parse(&library.name)?;
        let (path, download) = library_location(library, &coordinate, libraries_dir)?;
        match download {
            Some(download) => downloads.push(download),
            None if path.is_file() => {}
            None => {
                return Err(CoreError::NotFound(format!(
                    "installer library {}",
                    library.name
                )));
            }
        }
    }
    Ok(downloads)
}

pub struct ProcessorRun<'a> {
    pub http: &'a Http,
    pub java: &'a Path,
    pub libraries_dir: &'a Path,
    pub work_dir: &'a Path,
    pub log: &'a Path,
    pub platform: Platform,
    pub mappings: Option<&'a FileDownload>,
    pub progress: &'a Progress,
}

impl ProcessorRun<'_> {
    pub async fn run_all(
        &self,
        processors: &[Processor],
        values: &HashMap<String, String>,
    ) -> CoreResult<()> {
        if let Some(parent) = self.log.parent() {
            tokio::fs::create_dir_all(parent).await.at_path(parent)?;
        }
        tokio::fs::write(self.log, b"").await.at_path(self.log)?;
        for processor in processors.iter().filter(|p| p.runs_on_client()) {
            self.progress.check_cancelled()?;
            if let Some(output) = processor.mojmaps_output() {
                self.download_mappings(&substitute(output, values)?).await?;
                continue;
            }
            let outputs = self.outputs(processor, values)?;
            if !outputs.is_empty() && self.outputs_match(&outputs).await? {
                continue;
            }
            self.run_one(processor, values).await?;
            if !self.outputs_match(&outputs).await? {
                return Err(CoreError::InvalidInput(format!(
                    "processor {} produced unexpected files, see {}",
                    processor.jar,
                    self.log.display()
                )));
            }
        }
        Ok(())
    }

    async fn download_mappings(&self, output: &str) -> CoreResult<()> {
        let mappings = self.mappings.ok_or_else(|| {
            CoreError::NotFound("client mappings for this Minecraft version".into())
        })?;
        let download = Download {
            url: mappings.url.clone(),
            path: PathBuf::from(output),
            checksum: Some(Checksum::Sha1(mappings.sha1.clone())),
            size: Some(mappings.size),
        };
        download_all(
            self.http,
            vec![download],
            1,
            VerifyMode::Quick,
            self.progress,
        )
        .await
    }

    fn outputs(
        &self,
        processor: &Processor,
        values: &HashMap<String, String>,
    ) -> CoreResult<Vec<(PathBuf, String)>> {
        processor
            .outputs
            .iter()
            .map(|(file, hash)| {
                let path = PathBuf::from(resolve_arg(file, values, self.libraries_dir)?);
                let hash = match classify(hash) {
                    DataValue::Literal(literal) => literal.to_string(),
                    _ => substitute(hash, values)?,
                };
                let hash = hash.trim_matches('\'').to_ascii_lowercase();
                Ok((path, hash))
            })
            .collect()
    }

    async fn outputs_match(&self, outputs: &[(PathBuf, String)]) -> CoreResult<bool> {
        for (path, hash) in outputs {
            let checksum = Checksum::Sha1(hash.clone());
            if !file_matches(path, Some(&checksum), None, VerifyMode::Full).await? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    async fn run_one(
        &self,
        processor: &Processor,
        values: &HashMap<String, String>,
    ) -> CoreResult<()> {
        let jar = artifact_path(self.libraries_dir, &processor.jar)?;
        let main_class = {
            let jar = jar.clone();
            tokio::task::spawn_blocking(move || read_main_class(&jar))
                .await
                .map_err(|e| CoreError::InvalidInput(format!("processor task failed: {e}")))??
        };
        let mut classpath = vec![jar];
        for entry in &processor.classpath {
            classpath.push(artifact_path(self.libraries_dir, entry)?);
        }
        let classpath = classpath
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(self.platform.classpath_separator());
        let args = processor
            .args
            .iter()
            .map(|arg| resolve_arg(arg, values, self.libraries_dir))
            .collect::<CoreResult<Vec<_>>>()?;

        let log = std::fs::OpenOptions::new()
            .append(true)
            .open(self.log)
            .at_path(self.log)?;
        let err_log = log.try_clone().at_path(self.log)?;
        let mut command = tokio::process::Command::new(self.java);
        command
            .arg("-cp")
            .arg(&classpath)
            .arg(&main_class)
            .args(&args)
            .current_dir(self.work_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(err_log))
            .kill_on_drop(true);
        #[cfg(windows)]
        command.creation_flags(CREATE_NO_WINDOW);
        tracing::info!(processor = %processor.jar, main = %main_class, "running loader processor");
        let status = command.status().await.at_path(self.java)?;
        if !status.success() {
            return Err(CoreError::InvalidInput(format!(
                "processor {} failed with {status}, see {}",
                processor.jar,
                self.log.display()
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values() -> HashMap<String, String> {
        HashMap::from([
            ("SIDE".to_string(), "client".to_string()),
            ("MC_SLIM".to_string(), "/libs/slim.jar".to_string()),
        ])
    }

    #[test]
    fn substitutes_variables_and_rejects_unknown_ones() {
        assert_eq!(
            substitute("--side={SIDE}", &values()).unwrap(),
            "--side=client"
        );
        assert_eq!(
            substitute("{MC_SLIM}", &values()).unwrap(),
            "/libs/slim.jar"
        );
        assert_eq!(substitute("plain", &values()).unwrap(), "plain");
        assert!(substitute("{MISSING}", &values()).is_err());
        assert!(substitute("{SIDE", &values()).is_err());
    }

    #[test]
    fn classifies_data_values() {
        assert!(matches!(
            classify("[net.neoforged:neoforge:21.1.1:client]"),
            DataValue::Artifact("net.neoforged:neoforge:21.1.1:client")
        ));
        assert!(matches!(
            classify("'1.21.1-2024'"),
            DataValue::Literal("1.21.1-2024")
        ));
        assert!(matches!(
            classify("/data/client.lzma"),
            DataValue::InstallerFile("/data/client.lzma")
        ));
    }

    #[test]
    fn artifact_arguments_point_into_the_library_folder() {
        let dir = Path::new("libs");
        let path = resolve_arg(
            "[net.neoforged:neoform:1.21.1-20240808.144430@zip]",
            &values(),
            dir,
        )
        .unwrap();
        assert_eq!(
            PathBuf::from(path),
            dir.join(
                "net/neoforged/neoform/1.21.1-20240808.144430/neoform-1.21.1-20240808.144430.zip"
            )
        );
        assert!(resolve_arg("[../evil:x:1]", &values(), dir).is_err());
    }

    #[test]
    fn reads_wrapped_main_class() {
        let manifest = "Manifest-Version: 1.0\r\nMain-Class: net.neoforged.installertools.Ins\r\n tallerTools\r\nCreated-By: test\r\n";
        assert_eq!(
            parse_main_class(manifest).as_deref(),
            Some("net.neoforged.installertools.InstallerTools")
        );
        assert_eq!(parse_main_class("Manifest-Version: 1.0\n"), None);
    }

    #[test]
    fn filters_server_processors_and_finds_the_mappings_task() {
        let processors: Vec<Processor> = serde_json::from_str(
            r#"[
                {"sides":["server"],"jar":"a:b:1","args":[]},
                {"jar":"a:b:1","args":["--task","DOWNLOAD_MOJMAPS","--version","1.21.1","--side","{SIDE}","--output","{MOJMAPS}"]},
                {"sides":["client"],"jar":"a:c:1","args":["--input","{MINECRAFT_JAR}"]}
            ]"#,
        )
        .unwrap();
        let client: Vec<bool> = processors.iter().map(Processor::runs_on_client).collect();
        assert_eq!(client, [false, true, true]);
        assert_eq!(processors[1].mojmaps_output(), Some("{MOJMAPS}"));
        assert_eq!(processors[2].mojmaps_output(), None);
    }
}

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use serde::Serialize;
use ts_rs::TS;

use crate::platform::Os;

const PROBE_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_CANDIDATES: usize = 64;
const SEARCH_DEPTH: usize = 4;
const WINDOWS_VENDORS: [&str; 14] = [
    "Java",
    "Eclipse Adoptium",
    "Eclipse Foundation",
    "AdoptOpenJDK",
    "Temurin",
    "Zulu",
    "Microsoft",
    "BellSoft",
    "Amazon Corretto",
    "Semeru",
    "Semeru Runtimes",
    "OpenJDK",
    "GraalVM",
    "RedHat",
];
const OFFICIAL_LAUNCHER_PACKAGE: &str = "Microsoft.4297127D64EC6_8wekyb3d8bbwe";

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct JavaInstallation {
    pub path: String,
    pub version: String,
    pub major: u32,
    pub arch: String,
    pub vendor: Option<String>,
    pub managed: bool,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct JavaProperties {
    version: String,
    arch: String,
    vendor: Option<String>,
    home: Option<String>,
}

pub async fn detect(runtimes_dir: &Path, os: Os) -> Vec<JavaInstallation> {
    let runtimes = runtimes_dir.to_path_buf();
    let candidates = tokio::task::spawn_blocking(move || collect_candidates(&runtimes, os))
        .await
        .unwrap_or_default();
    let probes = candidates
        .into_iter()
        .map(|(binary, managed)| async move { (probe(&binary).await, binary, managed) });
    let mut seen = HashSet::new();
    let mut found = Vec::new();
    for (properties, binary, managed) in futures::future::join_all(probes).await {
        let Some(properties) = properties else {
            continue;
        };
        let home = properties.home.as_deref().map(PathBuf::from).or_else(|| {
            binary
                .parent()
                .and_then(Path::parent)
                .map(Path::to_path_buf)
        });
        let Some(home) = home else { continue };
        let key = home.to_string_lossy().to_lowercase();
        if !seen.insert(key) {
            continue;
        }
        let launch_binary = super::java_binary(&home, os);
        let path = if launch_binary.is_file() {
            launch_binary
        } else {
            binary
        };
        let Some(major) = major_version(&properties.version) else {
            continue;
        };
        found.push(JavaInstallation {
            path: path.to_string_lossy().into_owned(),
            version: properties.version,
            major,
            arch: properties.arch,
            vendor: properties.vendor,
            managed,
        });
    }
    found.sort_by(|a, b| {
        b.major
            .cmp(&a.major)
            .then_with(|| a.managed.cmp(&b.managed))
            .then_with(|| a.path.cmp(&b.path))
    });
    found
}

fn probe_name(os: Os) -> &'static str {
    match os {
        Os::Windows => "java.exe",
        Os::MacOs | Os::Linux => "java",
    }
}

fn collect_candidates(runtimes_dir: &Path, os: Os) -> Vec<(PathBuf, bool)> {
    let binary = probe_name(os);
    let mut candidates: Vec<(PathBuf, bool)> = Vec::new();
    let push = |path: PathBuf, managed: bool, list: &mut Vec<(PathBuf, bool)>| {
        if path.is_file() && !list.iter().any(|(p, _)| p == &path) {
            list.push((path, managed));
        }
    };

    for home in find_homes(runtimes_dir, binary, SEARCH_DEPTH + 2) {
        push(home.join("bin").join(binary), true, &mut candidates);
    }
    if let Some(home) = std::env::var_os("JAVA_HOME") {
        push(
            PathBuf::from(home).join("bin").join(binary),
            false,
            &mut candidates,
        );
    }
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            push(dir.join(binary), false, &mut candidates);
        }
    }
    for root in search_roots(os) {
        for home in find_homes(&root, binary, SEARCH_DEPTH) {
            push(home.join("bin").join(binary), false, &mut candidates);
        }
    }
    candidates.truncate(MAX_CANDIDATES);
    candidates
}

fn search_roots(os: Os) -> Vec<PathBuf> {
    let env = |key: &str| std::env::var_os(key).map(PathBuf::from);
    let home = env("USERPROFILE").or_else(|| env("HOME"));
    let mut roots = Vec::new();
    if let Some(home) = &home {
        roots.push(home.join(".jdks"));
        roots.push(home.join(".sdkman").join("candidates").join("java"));
    }
    match os {
        Os::Windows => {
            for key in ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"] {
                if let Some(base) = env(key) {
                    roots.extend(WINDOWS_VENDORS.iter().map(|vendor| base.join(vendor)));
                }
            }
            if let Some(local) = env("LOCALAPPDATA") {
                roots.push(local.join("Programs").join("Eclipse Adoptium"));
                roots.push(
                    local
                        .join("Packages")
                        .join(OFFICIAL_LAUNCHER_PACKAGE)
                        .join("LocalCache")
                        .join("Local")
                        .join("runtime"),
                );
            }
            if let Some(roaming) = env("APPDATA") {
                roots.push(roaming.join(".minecraft").join("runtime"));
            }
        }
        Os::MacOs => {
            roots.push("/Library/Java/JavaVirtualMachines".into());
            if let Some(home) = &home {
                roots.push(home.join("Library/Java/JavaVirtualMachines"));
                roots.push(home.join("Library/Application Support/minecraft/runtime"));
            }
            for brew in ["/opt/homebrew/opt", "/usr/local/opt"] {
                if let Ok(entries) = std::fs::read_dir(brew) {
                    roots.extend(
                        entries
                            .flatten()
                            .filter(|e| e.file_name().to_string_lossy().starts_with("openjdk"))
                            .map(|e| e.path()),
                    );
                }
            }
        }
        Os::Linux => {
            roots.extend(
                [
                    "/usr/lib/jvm",
                    "/usr/lib64/jvm",
                    "/usr/java",
                    "/opt/java",
                    "/opt/jdk",
                ]
                .map(PathBuf::from),
            );
            if let Some(home) = &home {
                roots.push(home.join(".minecraft").join("runtime"));
            }
        }
    }
    roots
}

fn find_homes(root: &Path, binary: &str, depth: usize) -> Vec<PathBuf> {
    let mut homes = Vec::new();
    let mut stack = vec![(root.to_path_buf(), 0)];
    while let Some((dir, level)) = stack.pop() {
        if dir.join("bin").join(binary).is_file() {
            homes.push(dir);
            continue;
        }
        if level == depth {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            if entry.file_type().is_ok_and(|t| t.is_dir()) {
                stack.push((entry.path(), level + 1));
            }
        }
    }
    homes.sort();
    homes
}

async fn probe(binary: &Path) -> Option<JavaProperties> {
    let mut command = tokio::process::Command::new(binary);
    command
        .args(["-XshowSettings:properties", "-version"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    let output = tokio::time::timeout(PROBE_TIMEOUT, command.output())
        .await
        .ok()?
        .ok()?;
    let text = String::from_utf8_lossy(&output.stderr).into_owned()
        + &String::from_utf8_lossy(&output.stdout);
    let properties = parse_properties(&text);
    if properties.is_none() {
        tracing::debug!(java = %binary.display(), "java did not report its properties");
    }
    properties
}

fn parse_properties(output: &str) -> Option<JavaProperties> {
    let mut properties = JavaProperties::default();
    for line in output.lines() {
        let Some((key, value)) = line.split_once(" = ") else {
            continue;
        };
        let value = value.trim().to_string();
        match key.trim() {
            "java.version" => properties.version = value,
            "os.arch" => properties.arch = value,
            "java.vendor" => properties.vendor = Some(value),
            "java.home" => properties.home = Some(value),
            _ => {}
        }
    }
    (!properties.version.is_empty()).then_some(properties)
}

pub fn major_version(version: &str) -> Option<u32> {
    let mut parts = version.split(|c: char| !c.is_ascii_digit());
    match parts.next()?.parse::<u32>().ok()? {
        1 => parts.next()?.parse().ok(),
        major => Some(major),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_properties_dump() {
        let output = "Property settings:\n    file.encoding = UTF-8\n    java.home = C:\\Program Files\\Eclipse Adoptium\\jdk-21.0.5.11-hotspot\n    java.vendor = Eclipse Adoptium\n    java.version = 21.0.5\n    java.vendor.url = https://adoptium.net/\n    os.arch = amd64\n\nopenjdk version \"21.0.5\" 2024-10-15 LTS\n";
        assert_eq!(
            parse_properties(output),
            Some(JavaProperties {
                version: "21.0.5".into(),
                arch: "amd64".into(),
                vendor: Some("Eclipse Adoptium".into()),
                home: Some(r"C:\Program Files\Eclipse Adoptium\jdk-21.0.5.11-hotspot".into()),
            })
        );
        assert_eq!(parse_properties("Error: could not create the JVM"), None);
    }

    #[test]
    fn reads_major_versions() {
        assert_eq!(major_version("1.8.0_402"), Some(8));
        assert_eq!(major_version("21.0.5"), Some(21));
        assert_eq!(major_version("17-ea"), Some(17));
        assert_eq!(major_version("25"), Some(25));
        assert_eq!(major_version("abc"), None);
    }

    #[test]
    fn finds_homes_below_a_root() {
        let dir = tempfile::tempdir().unwrap();
        let binary = probe_name(crate::platform::Platform::current().os);
        for home in ["jdk-21/bin", "vendor/jdk-17/Contents/Home/bin", "empty/bin"] {
            std::fs::create_dir_all(dir.path().join(home)).unwrap();
        }
        std::fs::write(dir.path().join("jdk-21/bin").join(binary), "").unwrap();
        std::fs::write(
            dir.path()
                .join("vendor/jdk-17/Contents/Home/bin")
                .join(binary),
            "",
        )
        .unwrap();
        let homes = find_homes(dir.path(), binary, SEARCH_DEPTH);
        assert_eq!(
            homes,
            [
                dir.path().join("jdk-21"),
                dir.path().join("vendor/jdk-17/Contents/Home")
            ]
        );
    }
}

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::maven::{MavenCoordinate, safe_join};
use super::rules::{RuleContext, os_key, rules_allow};
use super::version::{Artifact, Library, VersionJson};
use crate::net::{Checksum, Download};
use crate::platform::Arch;
use crate::{CoreError, CoreResult};

const DEFAULT_LIBRARY_REPO: &str = "https://libraries.minecraft.net/";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeArchive {
    pub path: PathBuf,
    pub exclude: Vec<String>,
}

#[derive(Debug, Default)]
pub struct ResolvedLibraries {
    pub classpath: Vec<PathBuf>,
    pub natives: Vec<NativeArchive>,
    pub downloads: Vec<Download>,
}

impl ResolvedLibraries {
    fn add_classpath(&mut self, path: PathBuf) {
        if !self.classpath.contains(&path) {
            self.classpath.push(path);
        }
    }
}

struct NativeCandidate {
    key: String,
    arch: Option<Arch>,
    archive: NativeArchive,
}

pub fn resolve_libraries(
    version: &VersionJson,
    ctx: &RuleContext,
    libraries_dir: &Path,
) -> CoreResult<ResolvedLibraries> {
    let mut resolved = ResolvedLibraries::default();
    let mut modern_natives: Vec<NativeCandidate> = Vec::new();

    for library in &version.libraries {
        if !rules_allow(library.rules.as_deref(), ctx) {
            continue;
        }
        let coordinate = MavenCoordinate::parse(&library.name)?;

        if let Some(natives) = &library.natives {
            if let Some(template) = natives.get(os_key(ctx.platform.os)) {
                let bits = if ctx.platform.arch == Arch::X86 {
                    "32"
                } else {
                    "64"
                };
                let classifier = template.replace("${arch}", bits);
                let (path, download) =
                    legacy_native_location(library, &coordinate, &classifier, libraries_dir)?;
                resolved.downloads.extend(download);
                resolved.natives.push(NativeArchive {
                    path,
                    exclude: exclude_list(library),
                });
            }
            if let Some(artifact) = library.downloads.as_ref().and_then(|d| d.artifact.as_ref()) {
                let (path, download) = artifact_location(artifact, &coordinate, libraries_dir)?;
                resolved.downloads.extend(download);
                resolved.add_classpath(path);
            }
            continue;
        }

        let (path, download) = library_location(library, &coordinate, libraries_dir)?;
        resolved.downloads.extend(download);
        resolved.add_classpath(path.clone());

        if let Some(classifier) = coordinate.classifier.as_deref()
            && let Some(rest) = classifier.strip_prefix("natives-")
        {
            modern_natives.push(NativeCandidate {
                key: format!("{}:{}", coordinate.group, coordinate.artifact),
                arch: native_arch(rest),
                archive: NativeArchive {
                    path,
                    exclude: exclude_list(library),
                },
            });
        }
    }

    resolved
        .natives
        .extend(select_modern_natives(modern_natives, ctx.platform.arch));
    Ok(resolved)
}

fn native_arch(suffix: &str) -> Option<Arch> {
    if suffix.ends_with("-arm64") || suffix.ends_with("-aarch64") {
        Some(Arch::Aarch64)
    } else if suffix.ends_with("-x86") {
        Some(Arch::X86)
    } else {
        None
    }
}

fn select_modern_natives(candidates: Vec<NativeCandidate>, arch: Arch) -> Vec<NativeArchive> {
    let mut by_key: HashMap<String, Vec<NativeCandidate>> = HashMap::new();
    let mut order = Vec::new();
    for candidate in candidates {
        if !by_key.contains_key(&candidate.key) {
            order.push(candidate.key.clone());
        }
        by_key
            .entry(candidate.key.clone())
            .or_default()
            .push(candidate);
    }
    order
        .into_iter()
        .filter_map(|key| {
            let group = by_key.remove(&key)?;
            let exact = group.iter().position(|c| c.arch == Some(arch));
            let generic = group.iter().position(|c| c.arch.is_none());
            let index = exact.or(generic)?;
            group.into_iter().nth(index).map(|c| c.archive)
        })
        .collect()
}

fn exclude_list(library: &Library) -> Vec<String> {
    library
        .extract
        .as_ref()
        .map(|extract| extract.exclude.clone())
        .unwrap_or_default()
}

pub(crate) fn library_location(
    library: &Library,
    coordinate: &MavenCoordinate,
    libraries_dir: &Path,
) -> CoreResult<(PathBuf, Option<Download>)> {
    if let Some(artifact) = library.downloads.as_ref().and_then(|d| d.artifact.as_ref()) {
        return artifact_location(artifact, coordinate, libraries_dir);
    }
    let relative = coordinate.relative_path();
    let path = safe_join(libraries_dir, &relative)?;
    let base = library.url.as_deref().unwrap_or(DEFAULT_LIBRARY_REPO);
    let download = Download {
        url: join_url(base, &relative),
        path: path.clone(),
        checksum: library.sha1.clone().map(Checksum::Sha1),
        size: library.size,
    };
    Ok((path, Some(download)))
}

fn artifact_location(
    artifact: &Artifact,
    coordinate: &MavenCoordinate,
    libraries_dir: &Path,
) -> CoreResult<(PathBuf, Option<Download>)> {
    let relative = artifact
        .path
        .clone()
        .unwrap_or_else(|| coordinate.relative_path());
    let path = safe_join(libraries_dir, &relative)?;
    let download = (!artifact.url.is_empty()).then(|| Download {
        url: artifact.url.clone(),
        path: path.clone(),
        checksum: Some(Checksum::Sha1(artifact.sha1.clone())),
        size: (artifact.size > 0).then_some(artifact.size),
    });
    Ok((path, download))
}

fn legacy_native_location(
    library: &Library,
    coordinate: &MavenCoordinate,
    classifier: &str,
    libraries_dir: &Path,
) -> CoreResult<(PathBuf, Option<Download>)> {
    let native = coordinate.with_classifier(classifier);
    let artifact = library
        .downloads
        .as_ref()
        .and_then(|d| d.classifiers.as_ref())
        .and_then(|classifiers| classifiers.get(classifier));
    match artifact {
        Some(artifact) => artifact_location(artifact, &native, libraries_dir),
        None if library.url.is_some() || library.downloads.is_none() => {
            let relative = native.relative_path();
            let path = safe_join(libraries_dir, &relative)?;
            let base = library.url.as_deref().unwrap_or(DEFAULT_LIBRARY_REPO);
            let download = Download {
                url: join_url(base, &relative),
                path: path.clone(),
                checksum: None,
                size: None,
            };
            Ok((path, Some(download)))
        }
        None => Err(CoreError::NotFound(format!(
            "native {classifier} for library {}",
            library.name
        ))),
    }
}

pub fn join_url(base: &str, relative: &str) -> String {
    if base.ends_with('/') {
        format!("{base}{relative}")
    } else {
        format!("{base}/{relative}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{Os, Platform};

    fn ctx(os: Os, arch: Arch) -> RuleContext {
        RuleContext::new(Platform { os, arch })
    }

    fn version(libraries: serde_json::Value) -> VersionJson {
        serde_json::from_value(serde_json::json!({ "id": "test", "libraries": libraries })).unwrap()
    }

    fn artifact(path: &str) -> serde_json::Value {
        serde_json::json!({
            "path": path,
            "sha1": "abc",
            "size": 10,
            "url": format!("https://libraries.minecraft.net/{path}")
        })
    }

    #[test]
    fn legacy_natives_pick_os_classifier_and_arch() {
        let v = version(serde_json::json!([{
            "name": "tv.twitch:twitch-platform:6.5",
            "natives": { "windows": "natives-windows-${arch}", "linux": "natives-linux" },
            "extract": { "exclude": ["META-INF/"] },
            "downloads": { "classifiers": {
                "natives-windows-64": artifact("tv/twitch/twitch-platform/6.5/twitch-platform-6.5-natives-windows-64.jar"),
                "natives-windows-32": artifact("tv/twitch/twitch-platform/6.5/twitch-platform-6.5-natives-windows-32.jar"),
                "natives-linux": artifact("tv/twitch/twitch-platform/6.5/twitch-platform-6.5-natives-linux.jar")
            }}
        }]));
        let libs = Path::new("libs");

        let win = resolve_libraries(&v, &ctx(Os::Windows, Arch::X86_64), libs).unwrap();
        assert!(win.classpath.is_empty());
        assert_eq!(win.natives.len(), 1);
        assert!(
            win.natives[0]
                .path
                .ends_with("twitch-platform-6.5-natives-windows-64.jar")
        );
        assert_eq!(win.natives[0].exclude, ["META-INF/"]);

        let mac = resolve_libraries(&v, &ctx(Os::MacOs, Arch::X86_64), libs).unwrap();
        assert!(mac.natives.is_empty());
    }

    #[test]
    fn modern_natives_go_to_classpath_and_only_matching_arch_is_extracted() {
        let v = version(serde_json::json!([
            { "name": "org.lwjgl:lwjgl:3.3.1", "downloads": { "artifact": artifact("org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1.jar") } },
            { "name": "org.lwjgl:lwjgl:3.3.1:natives-windows", "downloads": { "artifact": artifact("org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-windows.jar") }, "rules": [{ "action": "allow", "os": { "name": "windows" } }] },
            { "name": "org.lwjgl:lwjgl:3.3.1:natives-windows-x86", "downloads": { "artifact": artifact("org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-windows-x86.jar") }, "rules": [{ "action": "allow", "os": { "name": "windows" } }] },
            { "name": "org.lwjgl:lwjgl:3.3.1:natives-windows-arm64", "downloads": { "artifact": artifact("org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-windows-arm64.jar") }, "rules": [{ "action": "allow", "os": { "name": "windows" } }] },
            { "name": "org.lwjgl:lwjgl:3.3.1:natives-linux", "downloads": { "artifact": artifact("org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-linux.jar") }, "rules": [{ "action": "allow", "os": { "name": "linux" } }] }
        ]));
        let libs = Path::new("libs");

        let x64 = resolve_libraries(&v, &ctx(Os::Windows, Arch::X86_64), libs).unwrap();
        assert_eq!(x64.classpath.len(), 4);
        assert_eq!(x64.natives.len(), 1);
        assert!(
            x64.natives[0]
                .path
                .ends_with("lwjgl-3.3.1-natives-windows.jar")
        );

        let arm = resolve_libraries(&v, &ctx(Os::Windows, Arch::Aarch64), libs).unwrap();
        assert!(
            arm.natives[0]
                .path
                .ends_with("lwjgl-3.3.1-natives-windows-arm64.jar")
        );
    }

    #[test]
    fn maven_style_library_builds_url_from_repository() {
        let v = version(serde_json::json!([{
            "name": "net.fabricmc:fabric-loader:0.16.0",
            "url": "https://maven.fabricmc.net/",
            "sha1": "deadbeef",
            "size": 42
        }]));
        let resolved =
            resolve_libraries(&v, &ctx(Os::Linux, Arch::X86_64), Path::new("libs")).unwrap();
        let download = &resolved.downloads[0];
        assert_eq!(
            download.url,
            "https://maven.fabricmc.net/net/fabricmc/fabric-loader/0.16.0/fabric-loader-0.16.0.jar"
        );
        assert_eq!(download.checksum, Some(Checksum::Sha1("deadbeef".into())));
        assert_eq!(download.size, Some(42));
    }
}

use std::fs::File;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;

use super::{JavaInstall, JavaRequirement, java_binary, make_executable};
use crate::error::IoResultExt;
use crate::net::{Checksum, Download, VerifyMode, download_all};
use crate::platform::{Arch, Os};
use crate::{CoreError, CoreResult};

const MARKER_FILE: &str = ".installed";

#[derive(Debug, Deserialize)]
struct Release {
    binary: Binary,
    release_name: String,
}

#[derive(Debug, Deserialize)]
struct Binary {
    package: Package,
}

#[derive(Debug, Deserialize)]
struct Package {
    checksum: String,
    link: String,
    name: String,
    size: u64,
}

fn os_name(os: Os) -> &'static str {
    match os {
        Os::Windows => "windows",
        Os::MacOs => "mac",
        Os::Linux => "linux",
    }
}

fn arch_name(arch: Arch) -> CoreResult<&'static str> {
    match arch {
        Arch::X86_64 => Ok("x64"),
        Arch::Aarch64 => Ok("aarch64"),
        Arch::X86 => Err(CoreError::Unsupported(
            "32-bit Java runtimes are not provided by the Adoptium fallback".into(),
        )),
    }
}

fn assets_url(requirement: &JavaRequirement, os: Os, arch: &str) -> String {
    format!(
        "https://api.adoptium.net/v3/assets/latest/{}/hotspot?architecture={arch}&image_type=jre&os={}&vendor=eclipse",
        requirement.major_version,
        os_name(os)
    )
}

pub(super) async fn install(
    install: &JavaInstall<'_>,
    requirement: &JavaRequirement,
) -> CoreResult<PathBuf> {
    let platform = install.platform;
    let arch = arch_name(platform.arch)?;
    let home = install
        .paths
        .runtimes_dir()
        .join(&requirement.component)
        .join(format!("adoptium-{}-{arch}", os_name(platform.os)));
    let java = adoptium_java(&home, platform.os);
    let marker = home.join(MARKER_FILE);

    if install.mode == VerifyMode::Quick
        && tokio::fs::metadata(&marker).await.is_ok()
        && tokio::fs::metadata(&java).await.is_ok()
    {
        return Ok(java);
    }

    let releases: Vec<Release> = install
        .http
        .get_json(&assets_url(requirement, platform.os, arch))
        .await?;
    let release = releases.into_iter().next().ok_or_else(|| {
        CoreError::NotFound(format!(
            "Java {} runtime for {}/{arch}",
            requirement.major_version,
            os_name(platform.os)
        ))
    })?;
    let package = release.binary.package;
    if package.name.contains('/') || package.name.contains('\\') || package.name.contains("..") {
        return Err(CoreError::InvalidInput(format!(
            "unexpected archive name {:?}",
            package.name
        )));
    }

    let archive = install.paths.cache.join("java").join(&package.name);
    download_all(
        install.http,
        vec![Download {
            url: package.link,
            path: archive.clone(),
            checksum: Some(Checksum::Sha256(package.checksum)),
            size: Some(package.size),
        }],
        1,
        VerifyMode::Full,
        install.progress,
    )
    .await?;

    let target = home.clone();
    let name = package.name.clone();
    tokio::task::spawn_blocking(move || extract(&archive, &name, &target))
        .await
        .map_err(|err| CoreError::Unsupported(format!("java extraction panicked: {err}")))??;
    make_executable(&java)?;
    tokio::fs::write(&marker, release.release_name)
        .await
        .at_path(&marker)?;
    Ok(java)
}

fn adoptium_java(home: &Path, os: Os) -> PathBuf {
    match os {
        Os::MacOs => java_binary(&home.join("Contents").join("Home"), os),
        Os::Windows | Os::Linux => java_binary(home, os),
    }
}

fn strip_first(path: &Path) -> Option<PathBuf> {
    let mut components = path.components();
    components.next()?;
    let rest: PathBuf = components.collect();
    let safe = rest.components().all(|c| matches!(c, Component::Normal(_)));
    (safe && !rest.as_os_str().is_empty()).then_some(rest)
}

fn extract(archive: &Path, name: &str, target: &Path) -> CoreResult<()> {
    if target.exists() {
        std::fs::remove_dir_all(target).at_path(target)?;
    }
    std::fs::create_dir_all(target).at_path(target)?;
    let archive_error = |message: String| CoreError::Archive {
        path: archive.to_path_buf(),
        message,
    };

    if name.ends_with(".zip") {
        let file = File::open(archive).at_path(archive)?;
        let mut zip = zip::ZipArchive::new(file).map_err(|e| archive_error(e.to_string()))?;
        for index in 0..zip.len() {
            let mut entry = zip
                .by_index(index)
                .map_err(|e| archive_error(e.to_string()))?;
            let Some(relative) = entry.enclosed_name().as_deref().and_then(strip_first) else {
                continue;
            };
            let destination = target.join(relative);
            if entry.is_dir() {
                std::fs::create_dir_all(&destination).at_path(&destination)?;
                continue;
            }
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent).at_path(parent)?;
            }
            let mut output = File::create(&destination).at_path(&destination)?;
            std::io::copy(&mut entry, &mut output).at_path(&destination)?;
        }
        Ok(())
    } else if name.ends_with(".tar.gz") {
        let file = File::open(archive).at_path(archive)?;
        let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(file));
        tar.set_preserve_permissions(true);
        for entry in tar.entries().map_err(|e| archive_error(e.to_string()))? {
            let mut entry = entry.map_err(|e| archive_error(e.to_string()))?;
            let path = entry
                .path()
                .map_err(|e| archive_error(e.to_string()))?
                .into_owned();
            let Some(relative) = strip_first(&path) else {
                continue;
            };
            let destination = target.join(relative);
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent).at_path(parent)?;
            }
            entry.unpack(&destination).at_path(&destination)?;
        }
        Ok(())
    } else {
        Err(archive_error(format!("unsupported archive format {name}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_first_removes_top_level_directory_and_rejects_traversal() {
        assert_eq!(
            strip_first(Path::new("jdk-21-jre/bin/java")),
            Some(PathBuf::from("bin/java"))
        );
        assert_eq!(strip_first(Path::new("jdk-21-jre")), None);
        assert_eq!(strip_first(Path::new("jdk/../../evil")), None);
    }

    #[test]
    fn builds_assets_url() {
        let requirement = JavaRequirement {
            component: "java-runtime-delta".into(),
            major_version: 21,
        };
        assert_eq!(
            assets_url(&requirement, Os::Linux, "aarch64"),
            "https://api.adoptium.net/v3/assets/latest/21/hotspot?architecture=aarch64&image_type=jre&os=linux&vendor=eclipse"
        );
    }
}

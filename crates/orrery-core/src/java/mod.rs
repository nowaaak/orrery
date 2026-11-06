mod adoptium;
mod mojang;
pub mod system;

use std::path::{Path, PathBuf};

use crate::net::{Http, VerifyMode};
use crate::paths::LauncherPaths;
use crate::platform::{Arch, Os, Platform};
use crate::progress::Progress;
use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaRequirement {
    pub component: String,
    pub major_version: u32,
}

impl JavaRequirement {
    pub fn legacy() -> Self {
        Self {
            component: "jre-legacy".into(),
            major_version: 8,
        }
    }
}

pub struct JavaInstall<'a> {
    pub http: &'a Http,
    pub paths: &'a LauncherPaths,
    pub platform: Platform,
    pub mode: VerifyMode,
    pub concurrency: usize,
    pub progress: &'a Progress,
}

pub async fn ensure_java(
    install: &JavaInstall<'_>,
    requirement: &JavaRequirement,
) -> CoreResult<PathBuf> {
    let component = &requirement.component;
    if !component
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(CoreError::InvalidInput(format!(
            "invalid java component {component:?}"
        )));
    }

    if let Some(platform_key) = mojang::platform_key(install.platform)
        && let Some(java) = mojang::install(install, requirement, platform_key).await?
    {
        return Ok(java);
    }

    tracing::info!(
        component = %component,
        major = requirement.major_version,
        "Mojang provides no runtime for this platform, falling back to Adoptium"
    );
    adoptium::install(install, requirement).await
}

pub fn java_binary(home: &Path, os: Os) -> PathBuf {
    match os {
        Os::Windows => home.join("bin").join("javaw.exe"),
        Os::MacOs | Os::Linux => home.join("bin").join("java"),
    }
}

pub fn x64_on_apple_silicon(platform: Platform) -> Platform {
    if platform.os == Os::MacOs && platform.arch == Arch::Aarch64 {
        Platform {
            os: Os::MacOs,
            arch: Arch::X86_64,
        }
    } else {
        platform
    }
}

#[cfg(unix)]
fn make_executable(path: &Path) -> CoreResult<()> {
    use crate::error::IoResultExt;
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).at_path(path)
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> CoreResult<()> {
    Ok(())
}

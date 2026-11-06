use std::path::{Path, PathBuf};

#[cfg(windows)]
const GPU_PREFERENCES_KEY: &str = r"HKCU\Software\Microsoft\DirectX\UserGpuPreferences";
#[cfg(windows)]
const HIGH_PERFORMANCE: &str = "GpuPreference=2;";
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn normalized_executable(executable: &Path) -> PathBuf {
    executable.components().collect()
}

#[cfg(windows)]
pub async fn prefer_dedicated_gpu(executable: &Path) -> Result<bool, String> {
    let executable = normalized_executable(executable);
    let value_name = executable
        .to_str()
        .ok_or_else(|| "java path is not valid unicode".to_string())?;
    let existing = reg(&["query", GPU_PREFERENCES_KEY, "/v", value_name]).await?;
    if existing.success() {
        return Ok(false);
    }
    let added = reg(&[
        "add",
        GPU_PREFERENCES_KEY,
        "/v",
        value_name,
        "/t",
        "REG_SZ",
        "/d",
        HIGH_PERFORMANCE,
        "/f",
    ])
    .await?;
    if added.success() {
        Ok(true)
    } else {
        Err(format!("reg add exited with {added}"))
    }
}

#[cfg(not(windows))]
pub async fn prefer_dedicated_gpu(_executable: &Path) -> Result<bool, String> {
    Ok(false)
}

#[cfg(windows)]
async fn reg(args: &[&str]) -> Result<std::process::ExitStatus, String> {
    let program = std::env::var_os("SystemRoot")
        .map(|root| PathBuf::from(root).join("System32").join("reg.exe"))
        .unwrap_or_else(|| PathBuf::from("reg.exe"));
    tokio::process::Command::new(program)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .await
        .map_err(|err| err.to_string())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn mixed_separators_are_normalized() {
        assert_eq!(
            normalized_executable(Path::new(r"C:/Users/me\data\runtimes/bin\javaw.exe")),
            PathBuf::from(r"C:\Users\me\data\runtimes\bin\javaw.exe")
        );
    }
}

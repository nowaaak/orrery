use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::Serialize;
use ts_rs::TS;

use crate::error::IoResultExt;
use crate::services::Services;
use crate::{CoreError, CoreResult};

const SCREENSHOTS_DIR: &str = "screenshots";
const MAX_SCREENSHOT_BYTES: u64 = 64 * 1024 * 1024;
const EXTENSIONS: [&str; 3] = ["png", "jpg", "jpeg"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Screenshot {
    pub file_name: String,
    pub size_bytes: u64,
    pub taken_at_ms: i64,
}

fn validate_name(name: &str) -> CoreResult<()> {
    let extension = Path::new(name)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase);
    let valid = !name.is_empty()
        && name.len() <= 255
        && !name.starts_with('.')
        && !name.contains(['/', '\\', ':', '\0'])
        && extension.is_some_and(|ext| EXTENSIONS.contains(&ext.as_str()));
    if valid {
        Ok(())
    } else {
        Err(CoreError::InvalidInput(format!(
            "invalid screenshot name {name:?}"
        )))
    }
}

fn list_dir(dir: &Path) -> CoreResult<Vec<Screenshot>> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err).at_path(dir),
    };
    let mut shots: Vec<Screenshot> = entries
        .flatten()
        .filter_map(|entry| {
            let file_name = entry.file_name().to_str()?.to_string();
            validate_name(&file_name).ok()?;
            let metadata = entry.metadata().ok()?;
            if !metadata.is_file() {
                return None;
            }
            let taken = metadata.modified().ok()?;
            Some(Screenshot {
                file_name,
                size_bytes: metadata.len(),
                taken_at_ms: taken
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or_default(),
            })
        })
        .collect();
    shots.sort_by(|a, b| {
        b.taken_at_ms
            .cmp(&a.taken_at_ms)
            .then_with(|| b.file_name.cmp(&a.file_name))
    });
    Ok(shots)
}

impl Services {
    fn screenshots_dir(&self, instance_id: &str) -> CoreResult<PathBuf> {
        Ok(self.instances.game_dir(instance_id)?.join(SCREENSHOTS_DIR))
    }

    fn screenshot_path(&self, instance_id: &str, name: &str) -> CoreResult<PathBuf> {
        validate_name(name)?;
        Ok(self.screenshots_dir(instance_id)?.join(name))
    }

    pub async fn list_screenshots(&self, instance_id: &str) -> CoreResult<Vec<Screenshot>> {
        let dir = self.screenshots_dir(instance_id)?;
        tokio::task::spawn_blocking(move || list_dir(&dir))
            .await
            .map_err(|err| CoreError::Unsupported(format!("screenshot scan panicked: {err}")))?
    }

    pub async fn read_screenshot(&self, instance_id: &str, name: &str) -> CoreResult<Vec<u8>> {
        let path = self.screenshot_path(instance_id, name)?;
        let metadata = tokio::fs::metadata(&path).await.at_path(&path)?;
        if !metadata.is_file() || metadata.len() > MAX_SCREENSHOT_BYTES {
            return Err(CoreError::InvalidInput(format!(
                "{name} is not a readable screenshot"
            )));
        }
        tokio::fs::read(&path).await.at_path(&path)
    }

    pub async fn delete_screenshot(&self, instance_id: &str, name: &str) -> CoreResult<()> {
        let path = self.screenshot_path(instance_id, name)?;
        tokio::fs::remove_file(&path).await.at_path(&path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_names() {
        for name in ["2026-11-02_16.28.33.png", "shot.JPG", "a b.jpeg"] {
            assert!(validate_name(name).is_ok(), "{name}");
        }
        for name in [
            "",
            "../x.png",
            "sub/x.png",
            r"sub\x.png",
            "C:x.png",
            ".hidden.png",
            "notes.txt",
            "x.png.exe",
        ] {
            assert!(validate_name(name).is_err(), "{name}");
        }
    }

    #[test]
    fn lists_images_newest_first() {
        let dir = tempfile::tempdir().unwrap();
        assert!(list_dir(&dir.path().join("missing")).unwrap().is_empty());
        std::fs::write(dir.path().join("2026-01-01_10.00.00.png"), b"old").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(dir.path().join("2026-01-02_10.00.00.png"), b"newer").unwrap();
        std::fs::write(dir.path().join("readme.txt"), b"skip").unwrap();
        std::fs::create_dir(dir.path().join("folder.png")).unwrap();
        let shots = list_dir(dir.path()).unwrap();
        let names: Vec<&str> = shots.iter().map(|s| s.file_name.as_str()).collect();
        assert_eq!(
            names,
            ["2026-01-02_10.00.00.png", "2026-01-01_10.00.00.png"]
        );
        assert_eq!(shots[0].size_bytes, 5);
    }
}

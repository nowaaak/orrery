use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::error::IoResultExt;
use crate::modpacks::validate_pack_path;
use crate::progress::Progress;
use crate::{CoreError, CoreResult};

const MAX_DEPTH: usize = 32;
const MAX_FILES: usize = 200_000;
const MAX_ARCHIVE_BYTES: u64 = 16 * 1024 * 1024 * 1024;
const ALWAYS_SKIPPED: [&str; 8] = [
    "logs",
    "crash-reports",
    "debug",
    ".fabric",
    ".quilt",
    "natives",
    ".cache",
    ".mixin.out",
];

#[derive(Debug, Clone, Copy)]
pub enum Selection {
    Everything,
    Only(&'static [&'static str]),
}

impl Selection {
    fn allows(self, top: &str) -> bool {
        if ALWAYS_SKIPPED
            .iter()
            .any(|skipped| top.eq_ignore_ascii_case(skipped))
        {
            return false;
        }
        match self {
            Self::Everything => true,
            Self::Only(names) => names.iter().any(|name| top.eq_ignore_ascii_case(name)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyEntry {
    pub relative: PathBuf,
    pub size: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CopyPlan {
    pub entries: Vec<CopyEntry>,
    pub skipped_links: u32,
    pub skipped_unsafe: u32,
}

impl CopyPlan {
    pub fn bytes(&self) -> u64 {
        self.entries.iter().map(|entry| entry.size).sum()
    }
}

fn is_link(metadata: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    metadata.file_type().is_symlink()
}

pub fn plan_copy(root: &Path, selection: Selection) -> CoreResult<CopyPlan> {
    let mut plan = CopyPlan::default();
    if !root.is_dir() {
        return Ok(plan);
    }
    let mut pending: Vec<(PathBuf, Vec<String>)> = vec![(root.to_path_buf(), Vec::new())];
    while let Some((dir, parts)) = pending.pop() {
        let entries = std::fs::read_dir(&dir).at_path(&dir)?;
        for entry in entries {
            let entry = entry.at_path(&dir)?;
            let path = entry.path();
            let metadata = std::fs::symlink_metadata(&path).at_path(&path)?;
            let Some(name) = entry.file_name().to_str().map(str::to_string) else {
                plan.skipped_unsafe += 1;
                continue;
            };
            if parts.is_empty() && !selection.allows(&name) {
                continue;
            }
            if is_link(&metadata) {
                plan.skipped_links += 1;
                continue;
            }
            let mut relative = parts.clone();
            relative.push(name);
            if validate_pack_path(&relative.join("/")).is_err() {
                plan.skipped_unsafe += 1;
                continue;
            }
            if metadata.is_dir() {
                if relative.len() < MAX_DEPTH {
                    pending.push((path, relative));
                } else {
                    plan.skipped_unsafe += 1;
                }
            } else if metadata.is_file() {
                plan.entries.push(CopyEntry {
                    relative: relative.iter().collect(),
                    size: metadata.len(),
                });
                if plan.entries.len() > MAX_FILES {
                    return Err(CoreError::InvalidInput(
                        "the instance has too many files to import".into(),
                    ));
                }
            }
        }
    }
    plan.entries.sort_by(|a, b| a.relative.cmp(&b.relative));
    Ok(plan)
}

pub fn execute_copy(
    source: &Path,
    plan: &CopyPlan,
    target: &Path,
    progress: &Progress,
) -> CoreResult<u64> {
    let mut copied = 0;
    for entry in &plan.entries {
        progress.check_cancelled()?;
        let from = source.join(&entry.relative);
        let to = target.join(&entry.relative);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).at_path(parent)?;
        }
        let metadata = std::fs::symlink_metadata(&from).at_path(&from)?;
        if is_link(&metadata) || !metadata.is_file() {
            continue;
        }
        std::fs::copy(&from, &to).at_path(&to)?;
        copied += 1;
        progress.add_items(1);
        progress.add_bytes(entry.size);
    }
    Ok(copied)
}

fn archive_error(path: &Path) -> impl Fn(zip::result::ZipError) -> CoreError + '_ {
    move |err| CoreError::InvalidInput(format!("cannot read {}: {err}", path.display()))
}

pub fn check_archive(archive: &Path) -> CoreResult<Vec<String>> {
    let file = File::open(archive).at_path(archive)?;
    let mut zip = zip::ZipArchive::new(file).map_err(archive_error(archive))?;
    let mut names = Vec::with_capacity(zip.len());
    let mut total = 0u64;
    for index in 0..zip.len() {
        let entry = zip.by_index_raw(index).map_err(archive_error(archive))?;
        let name = entry.name().map_err(archive_error(archive))?.to_string();
        let trimmed = name.trim_end_matches('/');
        if trimmed.is_empty() {
            continue;
        }
        if entry.is_symlink() || validate_pack_path(trimmed).is_err() {
            return Err(CoreError::InvalidInput(format!(
                "the archive contains an unsafe path: {name:?}"
            )));
        }
        total = total.saturating_add(entry.size());
        names.push(trimmed.to_string());
    }
    if total > MAX_ARCHIVE_BYTES || names.len() > MAX_FILES {
        return Err(CoreError::InvalidInput("the archive is too large".into()));
    }
    Ok(names)
}

pub fn extract_archive(archive: &Path, target: &Path) -> CoreResult<()> {
    check_archive(archive)?;
    let file = File::open(archive).at_path(archive)?;
    let mut zip = zip::ZipArchive::new(file).map_err(archive_error(archive))?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(archive_error(archive))?;
        let name = entry.name().map_err(archive_error(archive))?.to_string();
        let trimmed = name.trim_end_matches('/');
        if trimmed.is_empty() {
            continue;
        }
        let path = target.join(validate_pack_path(trimmed)?);
        if entry.is_dir() {
            std::fs::create_dir_all(&path).at_path(&path)?;
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).at_path(parent)?;
        }
        let mut out = File::create(&path).at_path(&path)?;
        std::io::copy(&mut (&mut entry).take(MAX_ARCHIVE_BYTES), &mut out).at_path(&path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    fn write(root: &Path, relative: &str, bytes: &[u8]) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }

    #[test]
    fn plans_and_copies_only_selected_content() {
        let source = tempfile::tempdir().unwrap();
        write(source.path(), "mods/sodium.jar", b"jar");
        write(source.path(), "saves/World/level.dat", b"level");
        write(source.path(), "logs/latest.log", b"log");
        write(source.path(), "options.txt", b"fov:90");
        write(source.path(), "versions/1.20.1/1.20.1.jar", b"client");

        let everything = plan_copy(source.path(), Selection::Everything).unwrap();
        let names: Vec<String> = everything
            .entries
            .iter()
            .map(|e| e.relative.to_string_lossy().replace('\\', "/"))
            .collect();
        assert!(names.contains(&"mods/sodium.jar".to_string()));
        assert!(!names.iter().any(|name| name.starts_with("logs/")));

        let only = plan_copy(source.path(), Selection::Only(&["saves", "options.txt"])).unwrap();
        assert_eq!(only.entries.len(), 2);
        assert_eq!(only.bytes(), 11);

        let target = tempfile::tempdir().unwrap();
        let copied = execute_copy(source.path(), &only, target.path(), &Progress::new()).unwrap();
        assert_eq!(copied, 2);
        assert_eq!(
            std::fs::read(target.path().join("saves/World/level.dat")).unwrap(),
            b"level"
        );
        assert!(!target.path().join("mods").exists());
    }

    #[cfg(unix)]
    #[test]
    fn skips_links() {
        let source = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        write(outside.path(), "secret.txt", b"secret");
        std::os::unix::fs::symlink(outside.path(), source.path().join("mods")).unwrap();
        let plan = plan_copy(source.path(), Selection::Everything).unwrap();
        assert!(plan.entries.is_empty());
        assert_eq!(plan.skipped_links, 1);
    }

    fn zip_with(names: &[&str]) -> tempfile::NamedTempFile {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut zip = zip::ZipWriter::new(file.reopen().unwrap());
        for name in names {
            zip.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"data").unwrap();
        }
        zip.finish().unwrap();
        file
    }

    #[test]
    fn rejects_archives_with_unsafe_paths() {
        let safe = zip_with(&["instance.cfg", "minecraft/mods/a.jar"]);
        assert_eq!(check_archive(safe.path()).unwrap().len(), 2);
        let target = tempfile::tempdir().unwrap();
        extract_archive(safe.path(), target.path()).unwrap();
        assert!(target.path().join("minecraft/mods/a.jar").is_file());

        for bad in [
            "../evil.txt",
            "minecraft/../../evil.txt",
            "/etc/passwd",
            "C:/evil.txt",
            "a\\..\\b",
        ] {
            let archive = zip_with(&["instance.cfg", bad]);
            assert!(check_archive(archive.path()).is_err(), "{bad}");
            let target = tempfile::tempdir().unwrap();
            assert!(extract_archive(archive.path(), target.path()).is_err());
            assert!(!target.path().join("instance.cfg").exists());
        }
    }
}

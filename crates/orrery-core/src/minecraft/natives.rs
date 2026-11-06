use std::fs::File;
use std::path::{Path, PathBuf};

use super::libraries::NativeArchive;
use crate::error::IoResultExt;
use crate::net::sha1_hex;
use crate::{CoreError, CoreResult};

const MARKER_FILE: &str = ".extracted";
const NATIVE_EXTENSIONS: [&str; 4] = ["dll", "so", "dylib", "jnilib"];

pub async fn extract_natives(
    archives: Vec<NativeArchive>,
    target: PathBuf,
    force: bool,
) -> CoreResult<()> {
    tokio::task::spawn_blocking(move || extract_blocking(&archives, &target, force))
        .await
        .map_err(|err| CoreError::Unsupported(format!("native extraction panicked: {err}")))?
}

fn fingerprint(archives: &[NativeArchive]) -> String {
    let joined: Vec<String> = archives
        .iter()
        .map(|archive| archive.path.to_string_lossy().into_owned())
        .collect();
    sha1_hex(joined.join("\n").as_bytes())
}

fn marker_is_current(marker: &str, expected: &str, target: &Path) -> bool {
    let mut lines = marker.lines();
    if lines.next() != Some(expected) {
        return false;
    }
    let Some(count) = lines.next().and_then(|line| line.parse::<usize>().ok()) else {
        return false;
    };
    let files: Vec<&str> = lines.collect();
    files.len() == count && files.iter().all(|name| target.join(name).is_file())
}

fn extract_blocking(archives: &[NativeArchive], target: &Path, force: bool) -> CoreResult<()> {
    let marker = target.join(MARKER_FILE);
    let expected = fingerprint(archives);
    if !force
        && std::fs::read_to_string(&marker)
            .is_ok_and(|found| marker_is_current(&found, &expected, target))
    {
        return Ok(());
    }
    std::fs::create_dir_all(target).at_path(target)?;

    let mut files = Vec::new();
    for archive in archives {
        files.extend(extract_archive(archive, target)?);
    }
    files.sort();
    files.dedup();
    let mut content = format!("{expected}\n{}", files.len());
    for name in &files {
        content.push('\n');
        content.push_str(name);
    }
    std::fs::write(&marker, content).at_path(&marker)?;
    Ok(())
}

fn extract_archive(archive: &NativeArchive, target: &Path) -> CoreResult<Vec<String>> {
    let file = File::open(&archive.path).at_path(&archive.path)?;
    let mut zip = zip::ZipArchive::new(file).map_err(|err| CoreError::Archive {
        path: archive.path.clone(),
        message: err.to_string(),
    })?;

    let mut extracted = Vec::new();
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(|err| CoreError::Archive {
            path: archive.path.clone(),
            message: err.to_string(),
        })?;
        let name = entry
            .name()
            .map_err(|err| CoreError::Archive {
                path: archive.path.clone(),
                message: err.to_string(),
            })?
            .to_string();
        if entry.is_dir()
            || archive
                .exclude
                .iter()
                .any(|prefix| name.starts_with(prefix))
        {
            continue;
        }
        let Some(file_name) = Path::new(&name).file_name() else {
            continue;
        };
        let is_native = Path::new(file_name)
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| NATIVE_EXTENSIONS.contains(&ext));
        if !is_native {
            continue;
        }
        let destination = target.join(file_name);
        let Some(name) = file_name.to_str().map(str::to_string) else {
            continue;
        };
        let mut output = match File::create(&destination) {
            Ok(output) => output,
            Err(err)
                if err.kind() == std::io::ErrorKind::PermissionDenied && destination.exists() =>
            {
                tracing::warn!(path = %destination.display(), "native file is in use, keeping existing copy");
                extracted.push(name);
                continue;
            }
            Err(err) => return Err(err).at_path(&destination),
        };
        std::io::copy(&mut entry, &mut output).at_path(&destination)?;
        extracted.push(name);
    }
    Ok(extracted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn build_jar(path: &Path, entries: &[(&str, &[u8])]) {
        let file = File::create(path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        for (name, data) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(data).unwrap();
        }
        writer.finish().unwrap();
    }

    #[tokio::test]
    async fn extracts_flattened_native_files_and_skips_excluded_entries() {
        let dir = tempfile::tempdir().unwrap();
        let jar = dir.path().join("natives.jar");
        build_jar(
            &jar,
            &[
                ("windows/x64/org/lwjgl/lwjgl.dll", b"dll"),
                ("META-INF/evil.dll", b"x"),
                ("org/lwjgl/Thing.class", b"class"),
                ("../../escape.so", b"so"),
            ],
        );
        let target = dir.path().join("out");
        let archives = vec![NativeArchive {
            path: jar,
            exclude: vec!["META-INF/".into()],
        }];

        extract_natives(archives.clone(), target.clone(), false)
            .await
            .unwrap();

        let mut names: Vec<String> = std::fs::read_dir(&target)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(names, [".extracted", "escape.so", "lwjgl.dll"]);

        std::fs::write(target.join("lwjgl.dll"), b"kept").unwrap();
        extract_natives(archives.clone(), target.clone(), false)
            .await
            .unwrap();
        assert_eq!(std::fs::read(target.join("lwjgl.dll")).unwrap(), b"kept");

        std::fs::remove_file(target.join("lwjgl.dll")).unwrap();
        extract_natives(archives.clone(), target.clone(), false)
            .await
            .unwrap();
        assert_eq!(std::fs::read(target.join("lwjgl.dll")).unwrap(), b"dll");

        std::fs::write(target.join(MARKER_FILE), fingerprint(&archives)).unwrap();
        std::fs::write(target.join("lwjgl.dll"), b"old marker").unwrap();
        extract_natives(archives.clone(), target.clone(), false)
            .await
            .unwrap();
        assert_eq!(std::fs::read(target.join("lwjgl.dll")).unwrap(), b"dll");

        extract_natives(archives, target.clone(), true)
            .await
            .unwrap();
        assert!(target.join("lwjgl.dll").exists());
    }
}

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use ts_rs::TS;

use crate::cleanup::path_size;
use crate::error::IoResultExt;
use crate::paths::LauncherPaths;
use crate::progress::{Progress, Stage};
use crate::services::Services;
use crate::storage::unix_now;
use crate::tasks::{TaskId, TaskKind, TaskSpec};
use crate::{CoreError, CoreResult};

pub const MARKER_FILE: &str = "orrery-data.json";
pub const DATA_ENTRIES: [&str; 10] = [
    "instances",
    "versions",
    "libraries",
    "assets",
    "natives",
    "runtimes",
    "skins",
    "trash",
    "meta",
    "cache",
];
const PROBE_PREFIX: &str = ".orrery-write-test";
const SPACE_RESERVE: u64 = 512 * 1024 * 1024;
const COPY_CHUNK: usize = 1024 * 1024;
const COPY_WORKERS: usize = 8;
const MARKER_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum DataDirProblem {
    Missing,
    NotADirectory,
    NotAnOrreryFolder,
    NotWritable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DataDirIssue {
    pub path: String,
    pub problem: DataDirProblem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum DataDirMode {
    Move,
    Switch,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DataDirTarget {
    pub path: String,
    pub is_default: bool,
    pub is_current: bool,
    pub exists: bool,
    pub empty: bool,
    pub is_orrery: bool,
    pub writable: bool,
    pub nested: bool,
    pub in_app_folder: bool,
    pub same_drive: bool,
    pub needed_bytes: u64,
    pub free_bytes: Option<u64>,
    pub can_move: bool,
    pub can_switch: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Marker {
    schema: u32,
    created_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveOutcome {
    Renamed,
    Copied,
}

fn lexical(path: &Path) -> String {
    let joined: PathBuf = path.components().collect();
    let text = joined.to_string_lossy().into_owned();
    if cfg!(windows) {
        text.to_lowercase()
    } else {
        text
    }
}

fn same_path(a: &Path, b: &Path) -> bool {
    lexical(a) == lexical(b)
}

fn strictly_within(child: &Path, parent: &Path) -> bool {
    let child = PathBuf::from(lexical(child));
    let parent = PathBuf::from(lexical(parent));
    child != parent && child.starts_with(&parent)
}

fn nearest_existing(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .find(|ancestor| ancestor.exists())
        .map(Path::to_path_buf)
}

fn strip_verbatim(path: &Path) -> String {
    let text = path.to_string_lossy();
    let text = text.strip_prefix(r"\\?\").unwrap_or(&text).to_string();
    if cfg!(windows) {
        text.to_lowercase()
    } else {
        text
    }
}

#[cfg(windows)]
fn volume_key(path: &Path) -> Option<String> {
    match path.components().next()? {
        Component::Prefix(prefix) => Some(prefix.as_os_str().to_string_lossy().to_lowercase()),
        _ => None,
    }
}

#[cfg(unix)]
fn volume_key(path: &Path) -> Option<String> {
    use std::os::unix::fs::MetadataExt;
    Some(
        fs::metadata(nearest_existing(path)?)
            .ok()?
            .dev()
            .to_string(),
    )
}

pub fn free_space(path: &Path) -> Option<u64> {
    let existing = nearest_existing(path)?;
    let target = strip_verbatim(&fs::canonicalize(&existing).unwrap_or(existing));
    let disks = sysinfo::Disks::new_with_refreshed_list().ok()?;
    disks
        .iter()
        .filter(|disk| target.starts_with(&strip_verbatim(disk.mount_point())))
        .max_by_key(|disk| disk.mount_point().as_os_str().len())
        .map(sysinfo::Disk::available_space)
}

pub fn probe_writable(dir: &Path) -> io::Result<()> {
    let probe = dir.join(format!("{PROBE_PREFIX}-{}", std::process::id()));
    let mut file = File::create(&probe)?;
    file.write_all(b"orrery")?;
    file.sync_all()?;
    drop(file);
    fs::remove_file(&probe)
}

pub fn write_marker(dir: &Path) -> CoreResult<()> {
    let marker = Marker {
        schema: MARKER_SCHEMA,
        created_at: unix_now(),
    };
    let path = dir.join(MARKER_FILE);
    let bytes = serde_json::to_vec_pretty(&marker).map_err(|source| CoreError::Json {
        path: path.clone(),
        source,
    })?;
    fs::write(&path, bytes).at_path(&path)
}

pub fn check(dir: &Path) -> Result<(), DataDirProblem> {
    let metadata = fs::metadata(dir).map_err(|_| DataDirProblem::Missing)?;
    if !metadata.is_dir() {
        return Err(DataDirProblem::NotADirectory);
    }
    if !dir.join(MARKER_FILE).is_file() {
        return Err(DataDirProblem::NotAnOrreryFolder);
    }
    probe_writable(dir).map_err(|_| DataDirProblem::NotWritable)
}

pub fn resolve(
    config: PathBuf,
    logs: PathBuf,
    data_dir: Option<&Path>,
) -> Result<LauncherPaths, DataDirIssue> {
    match data_dir.filter(|dir| !same_path(dir, &config)) {
        None => Ok(LauncherPaths::with_data_root(config.clone(), config, logs)),
        Some(dir) => match check(dir) {
            Ok(()) => Ok(LauncherPaths::with_data_root(
                config,
                dir.to_path_buf(),
                logs,
            )),
            Err(problem) => Err(DataDirIssue {
                path: dir.display().to_string(),
                problem,
            }),
        },
    }
}

pub fn data_size(root: &Path) -> u64 {
    DATA_ENTRIES
        .iter()
        .map(|entry| path_size(&root.join(entry)))
        .sum()
}

fn validate_target(target: &Path) -> CoreResult<()> {
    let plain = target
        .components()
        .all(|component| !matches!(component, Component::ParentDir | Component::CurDir));
    if target.is_absolute() && plain {
        Ok(())
    } else {
        Err(CoreError::InvalidInput(format!(
            "data folder must be an absolute path: {}",
            target.display()
        )))
    }
}

pub fn app_folder() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()?
        .parent()
        .map(Path::to_path_buf)
}

pub fn inspect(paths: &LauncherPaths, target: &Path) -> CoreResult<DataDirTarget> {
    inspect_with_app(paths, target, app_folder().as_deref())
}

fn inspect_with_app(
    paths: &LauncherPaths,
    target: &Path,
    app_dir: Option<&Path>,
) -> CoreResult<DataDirTarget> {
    validate_target(target)?;
    let is_default = same_path(target, &paths.config);
    let is_current = same_path(target, &paths.root);
    let nested = !is_current
        && (strictly_within(target, &paths.root)
            || strictly_within(&paths.root, target)
            || (!is_default && strictly_within(target, &paths.config))
            || (!is_default && strictly_within(&paths.config, target)));
    let exists = target.exists();
    let is_dir = target.is_dir();
    let empty = if is_default {
        !DATA_ENTRIES.iter().any(|entry| target.join(entry).exists())
    } else {
        !exists
            || (is_dir && fs::read_dir(target).is_ok_and(|mut entries| entries.next().is_none()))
    };
    let is_orrery = is_default || target.join(MARKER_FILE).is_file();
    let writable = if exists {
        is_dir && probe_writable(target).is_ok()
    } else {
        nearest_existing(target).is_some_and(|dir| probe_writable(&dir).is_ok())
    };
    let same_drive = volume_key(target).is_some() && volume_key(target) == volume_key(&paths.root);
    let needed_bytes = data_size(&paths.root);
    let free_bytes = free_space(target);
    let enough_space = same_drive
        || free_bytes.is_none_or(|free| free >= needed_bytes.saturating_add(SPACE_RESERVE));
    let in_app_folder = app_dir.is_some_and(|app| {
        same_path(target, app) || strictly_within(target, app) || strictly_within(app, target)
    });
    let usable = !is_current && !nested && !in_app_folder && writable;
    Ok(DataDirTarget {
        path: target.display().to_string(),
        is_default,
        is_current,
        exists,
        empty,
        is_orrery,
        writable,
        nested,
        in_app_folder,
        same_drive,
        needed_bytes,
        free_bytes,
        can_move: usable && empty && enough_space,
        can_switch: usable && (is_orrery || empty),
    })
}

fn is_cross_device(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::CrossesDevices || matches!(err.raw_os_error(), Some(17 | 18))
}

fn present_entries(source: &Path) -> Vec<&'static str> {
    DATA_ENTRIES
        .iter()
        .copied()
        .filter(|entry| source.join(entry).exists())
        .collect()
}

fn hash_file(path: &Path) -> CoreResult<String> {
    let mut file = File::open(path).at_path(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; COPY_CHUNK];
    loop {
        let read = file.read(&mut buffer).at_path(path)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn copy_file_verified(source: &Path, target: &Path, progress: &Progress) -> CoreResult<()> {
    let mut input = File::open(source).at_path(source)?;
    let mut output = File::create(target).at_path(target)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; COPY_CHUNK];
    loop {
        let read = input.read(&mut buffer).at_path(source)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        output.write_all(&buffer[..read]).at_path(target)?;
        progress.add_bytes(read as u64);
    }
    output.sync_all().at_path(target)?;
    drop(output);
    let permissions = fs::metadata(source).at_path(source)?.permissions();
    fs::set_permissions(target, permissions).at_path(target)?;
    let expected = hex::encode(hasher.finalize());
    let actual = hash_file(target)?;
    if expected != actual {
        return Err(CoreError::ChecksumMismatch {
            url: target.display().to_string(),
            expected,
            actual,
        });
    }
    Ok(())
}

fn plan_tree(source: &Path, target: &Path, files: &mut Vec<(PathBuf, PathBuf)>) -> CoreResult<()> {
    let kind = fs::symlink_metadata(source).at_path(source)?.file_type();
    if kind.is_dir() {
        fs::create_dir(target).at_path(target)?;
        for entry in fs::read_dir(source).at_path(source)? {
            let entry = entry.at_path(source)?;
            plan_tree(&entry.path(), &target.join(entry.file_name()), files)?;
        }
        Ok(())
    } else if kind.is_symlink() {
        copy_symlink(source, target)
    } else {
        files.push((source.to_path_buf(), target.to_path_buf()));
        Ok(())
    }
}

fn copy_files(files: &[(PathBuf, PathBuf)], progress: &Progress) -> CoreResult<()> {
    let next = AtomicUsize::new(0);
    let failure: Mutex<Option<CoreError>> = Mutex::new(None);
    let failed = || failure.lock().map_or(true, |slot| slot.is_some());
    let fail = |err: CoreError| {
        if let Ok(mut slot) = failure.lock() {
            slot.get_or_insert(err);
        }
    };
    std::thread::scope(|scope| {
        for _ in 0..COPY_WORKERS.min(files.len()) {
            scope.spawn(|| {
                while !failed() {
                    let Some((from, to)) = files.get(next.fetch_add(1, Ordering::Relaxed)) else {
                        break;
                    };
                    match catch_unwind(AssertUnwindSafe(|| copy_file_verified(from, to, progress)))
                    {
                        Ok(Ok(())) => progress.add_items(1),
                        Ok(Err(err)) => fail(err),
                        Err(_) => fail(CoreError::Unsupported(format!(
                            "copying {} panicked",
                            from.display()
                        ))),
                    }
                }
            });
        }
    });
    match failure.into_inner() {
        Ok(None) => Ok(()),
        Ok(Some(err)) => Err(err),
        Err(_) => Err(CoreError::Unsupported("a copy worker panicked".into())),
    }
}

#[cfg(unix)]
fn copy_symlink(source: &Path, target: &Path) -> CoreResult<()> {
    let link = fs::read_link(source).at_path(source)?;
    std::os::unix::fs::symlink(link, target).at_path(target)
}

#[cfg(not(unix))]
fn copy_symlink(source: &Path, _target: &Path) -> CoreResult<()> {
    Err(CoreError::Unsupported(format!(
        "cannot move the symbolic link {}",
        source.display()
    )))
}

fn remove_path(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => fs::remove_dir_all(path),
        Ok(_) => fs::remove_file(path),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

pub fn move_data(
    source: &Path,
    target: &Path,
    progress: &Progress,
    force_copy: bool,
) -> CoreResult<MoveOutcome> {
    let entries = present_entries(source);
    if let Some(conflict) = entries.iter().find(|entry| target.join(entry).exists()) {
        return Err(CoreError::InvalidInput(format!(
            "{} already exists in the new data folder",
            conflict
        )));
    }
    fs::create_dir_all(target).at_path(target)?;
    progress.set_stage(Stage::MovingData);

    if !force_copy {
        let mut renamed: Vec<&str> = Vec::new();
        let mut cross_device = false;
        for entry in &entries {
            match fs::rename(source.join(entry), target.join(entry)) {
                Ok(()) => renamed.push(entry),
                Err(err) if renamed.is_empty() && is_cross_device(&err) => {
                    cross_device = true;
                    break;
                }
                Err(err) => {
                    for done in renamed.iter().rev() {
                        if let Err(back) = fs::rename(target.join(done), source.join(done)) {
                            tracing::error!(entry = done, error = %back, "could not roll back a moved folder");
                        }
                    }
                    return Err(err).at_path(&source.join(entry));
                }
            }
        }
        if !cross_device {
            return Ok(MoveOutcome::Renamed);
        }
    }

    let total: u64 = entries
        .iter()
        .map(|entry| path_size(&source.join(entry)))
        .sum();
    progress.add_totals(0, total);
    let mut files = Vec::new();
    let result = entries
        .iter()
        .try_for_each(|entry| plan_tree(&source.join(entry), &target.join(entry), &mut files))
        .and_then(|()| {
            progress.add_totals(files.len() as u64, 0);
            copy_files(&files, progress)
        });
    if let Err(err) = result {
        for entry in &entries {
            if let Err(cleanup) = remove_path(&target.join(entry)) {
                tracing::error!(entry, error = %cleanup, "could not remove a partial copy");
            }
        }
        return Err(err);
    }
    Ok(MoveOutcome::Copied)
}

pub fn normalize(path: &Path) -> PathBuf {
    path.components().collect()
}

pub fn retire_folder(dir: &Path) {
    let _ = fs::remove_file(dir.join(MARKER_FILE));
    let _ = fs::remove_dir(dir);
}

pub fn remove_sources(source: &Path) -> Vec<PathBuf> {
    DATA_ENTRIES
        .iter()
        .map(|entry| source.join(entry))
        .filter(|path| remove_path(path).is_err())
        .collect()
}

async fn blocking<T: Send + 'static>(
    job: impl FnOnce() -> CoreResult<T> + Send + 'static,
) -> CoreResult<T> {
    tokio::task::spawn_blocking(job)
        .await
        .map_err(|err| CoreError::Unsupported(format!("data folder operation panicked: {err}")))?
}

impl Services {
    pub async fn inspect_data_dir(&self, target: PathBuf) -> CoreResult<DataDirTarget> {
        let target = normalize(&target);
        let paths = self.paths.clone();
        blocking(move || inspect(&paths, &target)).await
    }

    pub async fn change_data_dir(&self, target: PathBuf, mode: DataDirMode) -> CoreResult<TaskId> {
        let target = normalize(&target);
        self.ensure_idle()?;
        let info = self.inspect_data_dir(target.clone()).await?;
        let allowed = match mode {
            DataDirMode::Move => info.can_move,
            DataDirMode::Switch => info.can_switch,
        };
        if !allowed {
            return Err(CoreError::InvalidInput(format!(
                "{} cannot be used as the data folder",
                target.display()
            )));
        }
        let services = self.clone();
        self.tasks.spawn(
            TaskSpec {
                kind: TaskKind::MoveData,
                subject: target.display().to_string(),
                instance_id: None,
            },
            move |progress| async move {
                let source = services.paths.root.clone();
                let destination = target.clone();
                let outcome = match mode {
                    DataDirMode::Move => {
                        let task_progress = progress.clone();
                        let (from, to) = (source.clone(), destination.clone());
                        Some(blocking(move || move_data(&from, &to, &task_progress, false)).await?)
                    }
                    DataDirMode::Switch => {
                        let to = destination.clone();
                        blocking(move || fs::create_dir_all(&to).at_path(&to)).await?;
                        None
                    }
                };
                if !info.is_default && !destination.join(MARKER_FILE).is_file() {
                    let to = destination.clone();
                    blocking(move || write_marker(&to)).await?;
                }
                services
                    .settings
                    .set_data_dir((!info.is_default).then_some(destination))
                    .await?;
                let left_custom_folder = !same_path(&source, &services.paths.config);
                if outcome.is_some() {
                    let copied = outcome == Some(MoveOutcome::Copied);
                    let leftovers = blocking(move || {
                        let leftovers = if copied {
                            remove_sources(&source)
                        } else {
                            Vec::new()
                        };
                        if left_custom_folder && leftovers.is_empty() {
                            retire_folder(&source);
                        }
                        Ok(leftovers)
                    })
                    .await?;
                    for path in leftovers {
                        tracing::warn!(path = %path.display(), "could not remove the old copy");
                    }
                }
                progress.set_stage(Stage::Finished);
                Ok(())
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn seed(root: &Path) {
        write(
            &root.join("instances/survival/instance.json"),
            "{\"schema\":1}",
        );
        write(
            &root.join("instances/survival/minecraft/saves/Welt/level.dat"),
            "world",
        );
        write(&root.join("libraries/org/a/a.jar"), &"x".repeat(5000));
        write(&root.join("meta/versions/1.20.1.json"), "{}");
        write(&root.join("cache/modpacks/p.mrpack"), "pack");
    }

    fn tree(root: &Path) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for entry in DATA_ENTRIES {
            let mut files = Vec::new();
            collect(&root.join(entry), &mut files);
            for file in files {
                let relative = file
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push((relative, fs::read_to_string(&file).unwrap()));
            }
        }
        out.sort();
        out
    }

    fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            if entry.file_type().unwrap().is_dir() {
                collect(&entry.path(), out);
            } else {
                out.push(entry.path());
            }
        }
    }

    #[test]
    fn resolve_uses_the_default_or_a_checked_custom_folder() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("config");
        let logs = config.join("logs");
        let default = resolve(config.clone(), logs.clone(), None).unwrap();
        assert_eq!(default.root, config);
        assert_eq!(default.cache, config.join("cache"));
        assert_eq!(default.settings_file(), config.join("settings.json"));

        let same = resolve(config.clone(), logs.clone(), Some(&config)).unwrap();
        assert!(same.uses_default_root());

        let custom = dir.path().join("games").join("Orrery");
        let missing = resolve(config.clone(), logs.clone(), Some(&custom)).unwrap_err();
        assert_eq!(missing.problem, DataDirProblem::Missing);
        assert!(!custom.exists());

        fs::create_dir_all(&custom).unwrap();
        let foreign = resolve(config.clone(), logs.clone(), Some(&custom)).unwrap_err();
        assert_eq!(foreign.problem, DataDirProblem::NotAnOrreryFolder);

        write_marker(&custom).unwrap();
        let ok = resolve(config.clone(), logs, Some(&custom)).unwrap();
        assert_eq!(ok.root, custom);
        assert_eq!(ok.cache, custom.join("cache"));
        assert_eq!(ok.accounts_file(), config.join("accounts.json"));
        assert_eq!(fs::read_dir(&custom).unwrap().count(), 1);
    }

    #[test]
    fn inspect_rejects_nested_and_foreign_folders() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("config");
        fs::create_dir_all(&config).unwrap();
        seed(&config);
        let paths =
            LauncherPaths::with_data_root(config.clone(), config.clone(), config.join("logs"));

        let inside = inspect(&paths, &config.join("instances").join("x")).unwrap();
        assert!(inside.nested && !inside.can_move && !inside.can_switch);
        let parent = inspect(&paths, dir.path()).unwrap();
        assert!(parent.nested && !parent.can_move);
        assert!(inspect(&paths, Path::new("relative/path")).is_err());

        let fresh = inspect(&paths, &dir.path().join("new").join("Orrery")).unwrap();
        assert!(fresh.can_move && fresh.can_switch && !fresh.exists);
        assert!(fresh.needed_bytes >= 5000);

        let foreign = dir.path().join("games");
        write(&foreign.join("other.txt"), "x");
        let foreign = inspect(&paths, &foreign).unwrap();
        assert!(!foreign.empty && !foreign.can_move && !foreign.can_switch);

        let orrery = dir.path().join("old");
        fs::create_dir_all(&orrery).unwrap();
        write_marker(&orrery).unwrap();
        write(&orrery.join("instances/a/instance.json"), "{}");
        let orrery = inspect(&paths, &orrery).unwrap();
        assert!(orrery.is_orrery && orrery.can_switch && !orrery.can_move);

        let current = inspect(&paths, &config).unwrap();
        assert!(current.is_current && !current.can_move && !current.can_switch);
    }

    #[test]
    fn retired_folders_lose_their_marker_but_keep_foreign_files() {
        let dir = tempfile::tempdir().unwrap();
        let empty = dir.path().join("empty");
        fs::create_dir_all(&empty).unwrap();
        write_marker(&empty).unwrap();
        retire_folder(&empty);
        assert!(!empty.exists());

        let shared = dir.path().join("shared");
        fs::create_dir_all(&shared).unwrap();
        write_marker(&shared).unwrap();
        write(&shared.join("notes.txt"), "keep me");
        retire_folder(&shared);
        assert!(shared.join("notes.txt").is_file());
        assert!(!shared.join(MARKER_FILE).exists());
    }

    #[cfg(windows)]
    #[test]
    fn mixed_separators_are_normalized() {
        assert_eq!(
            normalize(Path::new(r"D:/Games\Orrery/")),
            PathBuf::from(r"D:\Games\Orrery")
        );
    }

    #[test]
    fn the_app_folder_is_never_a_data_folder() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("config");
        fs::create_dir_all(&config).unwrap();
        let paths =
            LauncherPaths::with_data_root(config.clone(), config.clone(), config.join("logs"));
        let app = dir.path().join("Launcher").join("Orrery");
        write(&app.join("orrery.exe"), "exe");

        let same = inspect_with_app(&paths, &app, Some(&app)).unwrap();
        assert!(same.in_app_folder && !same.can_move && !same.can_switch);
        let inside = inspect_with_app(&paths, &app.join("data"), Some(&app)).unwrap();
        assert!(inside.in_app_folder && !inside.can_move);
        let around = inspect_with_app(&paths, &dir.path().join("Launcher"), Some(&app)).unwrap();
        assert!(around.in_app_folder && !around.can_switch);
        let beside = inspect_with_app(
            &paths,
            &dir.path().join("Launcher").join("Orrery Daten"),
            Some(&app),
        )
        .unwrap();
        assert!(!beside.in_app_folder && beside.can_move && beside.can_switch);
    }

    #[test]
    fn rename_moves_everything_and_rolls_back_on_failure() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        seed(&source);
        let before = tree(&source);

        let target = dir.path().join("target");
        assert_eq!(
            move_data(&source, &target, &Progress::new(), false).unwrap(),
            MoveOutcome::Renamed
        );
        assert_eq!(tree(&target), before);
        assert!(tree(&source).is_empty());

        let blocked = dir.path().join("blocked");
        fs::create_dir_all(&blocked).unwrap();
        write(&blocked.join("libraries"), "a file where a folder must go");
        assert!(move_data(&target, &blocked, &Progress::new(), false).is_err());
        assert_eq!(tree(&target), before);
    }

    #[test]
    fn parallel_copy_handles_many_small_files() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        for index in 0..400 {
            write(
                &source.join(format!("assets/objects/{:02x}/{index:040}", index % 256)),
                &format!("asset {index}"),
            );
        }
        seed(&source);
        let before = tree(&source);

        let target = dir.path().join("target");
        let progress = Progress::new();
        assert_eq!(
            move_data(&source, &target, &progress, true).unwrap(),
            MoveOutcome::Copied
        );
        assert_eq!(tree(&target), before);
        let snapshot = progress.snapshot();
        assert_eq!(snapshot.items_done, before.len() as u64);
        assert_eq!(snapshot.items_total, before.len() as u64);
        assert_eq!(snapshot.bytes_done, snapshot.bytes_total);
    }

    #[test]
    fn copy_mode_verifies_and_cleans_up_after_failure() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        seed(&source);
        let before = tree(&source);

        let target = dir.path().join("copy");
        let progress = Progress::new();
        assert_eq!(
            move_data(&source, &target, &progress, true).unwrap(),
            MoveOutcome::Copied
        );
        assert_eq!(tree(&target), before);
        assert_eq!(tree(&source), before);
        let snapshot = progress.snapshot();
        assert_eq!(snapshot.bytes_done, snapshot.bytes_total);
        assert!(remove_sources(&source).is_empty());
        assert!(tree(&source).is_empty());

        let conflict = dir.path().join("conflict");
        write(&conflict.join("meta"), "a file where a folder must go");
        assert!(move_data(&target, &conflict, &Progress::new(), true).is_err());
        assert!(!conflict.join("instances").exists());

        let locked = target.join("meta/versions/1.20.1.json");
        let guard = lock_for_reading(&locked);
        let broken = dir.path().join("broken");
        let result = move_data(&target, &broken, &Progress::new(), true);
        drop(guard);
        assert!(result.is_err());
        for entry in DATA_ENTRIES {
            assert!(!broken.join(entry).exists(), "{entry} left behind");
        }
        assert_eq!(tree(&target), before);
    }

    #[cfg(windows)]
    fn lock_for_reading(path: &Path) -> fs::File {
        use std::os::windows::fs::OpenOptionsExt;
        fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(path)
            .unwrap()
    }

    #[cfg(unix)]
    fn lock_for_reading(path: &Path) -> impl Drop {
        use std::os::unix::fs::PermissionsExt;
        struct Restore(PathBuf);
        impl Drop for Restore {
            fn drop(&mut self) {
                let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o644));
            }
        }
        fs::set_permissions(path, fs::Permissions::from_mode(0o000)).unwrap();
        Restore(path.to_path_buf())
    }
}

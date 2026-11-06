use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::Serialize;
use ts_rs::TS;

use crate::instances::Instance;
use crate::java::JavaRequirement;
use crate::loaders::{self, installer};
use crate::minecraft::assets::AssetIndex;
use crate::minecraft::libraries::resolve_libraries;
use crate::minecraft::rules::RuleContext;
use crate::minecraft::version::VersionJson;
use crate::paths::LauncherPaths;
use crate::platform::Platform;
use crate::services::Services;
use crate::tasks::TaskStatus;
use crate::{CoreError, CoreResult};

const MAX_INHERITANCE: usize = 4;
const LEGACY_CACHE_DIRS: [&str; 2] = ["java", "modpacks"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum StorageCategory {
    Instances,
    Versions,
    Libraries,
    Assets,
    Runtimes,
    Natives,
    Cache,
    Trash,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CategoryUsage {
    pub category: StorageCategory,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct InstanceUsage {
    pub id: String,
    pub name: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CleanupSummary {
    pub unused_versions: Vec<String>,
    pub unused_runtimes: Vec<String>,
    pub library_files: u32,
    pub asset_files: u32,
    pub cache_bytes: u64,
    pub total_bytes: u64,
    pub complete: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StorageReport {
    pub categories: Vec<CategoryUsage>,
    pub instances: Vec<InstanceUsage>,
    pub cleanup: CleanupSummary,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CleanupResult {
    pub freed_bytes: u64,
}

#[derive(Debug, Default)]
struct Plan {
    targets: Vec<(PathBuf, u64)>,
    unused_versions: Vec<String>,
    unused_runtimes: Vec<String>,
    library_files: u32,
    asset_files: u32,
    cache_bytes: u64,
    complete: bool,
}

impl Plan {
    fn summary(&self) -> CleanupSummary {
        CleanupSummary {
            unused_versions: self.unused_versions.clone(),
            unused_runtimes: self.unused_runtimes.clone(),
            library_files: self.library_files,
            asset_files: self.asset_files,
            cache_bytes: self.cache_bytes,
            total_bytes: self.targets.iter().map(|(_, bytes)| bytes).sum(),
            complete: self.complete,
        }
    }
}

pub fn path_size(path: &Path) -> u64 {
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return 0;
    };
    if metadata.is_file() {
        return metadata.len();
    }
    if !metadata.is_dir() {
        return 0;
    }
    std::fs::read_dir(path)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| path_size(&entry.path()))
        .sum()
}

fn children(dir: &Path) -> Vec<(String, PathBuf)> {
    let mut out: Vec<(String, PathBuf)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| Some((entry.file_name().to_str()?.to_string(), entry.path())))
        .collect();
    out.sort();
    out
}

fn files_below(dir: &Path, out: &mut Vec<PathBuf>) {
    for (_, path) in children(dir) {
        match std::fs::symlink_metadata(&path) {
            Ok(meta) if meta.is_dir() => files_below(&path, out),
            Ok(meta) if meta.is_file() => out.push(path),
            _ => {}
        }
    }
}

fn read_version(paths: &LauncherPaths, id: &str) -> Option<VersionJson> {
    let text = std::fs::read_to_string(paths.version_json_file(id)).ok()?;
    serde_json::from_str(&text).ok()
}

fn resolve_local(paths: &LauncherPaths, id: &str) -> Option<VersionJson> {
    let mut version = read_version(paths, id)?;
    for _ in 0..MAX_INHERITANCE {
        let Some(parent_id) = version.inherits_from.clone() else {
            return Some(version);
        };
        version = version.merge_onto(read_version(paths, &parent_id)?);
    }
    None
}

pub fn used_version_ids(instances: &[Instance]) -> (HashSet<String>, HashSet<String>) {
    let mut launch_ids = HashSet::new();
    let mut base_ids = HashSet::new();
    for instance in instances {
        base_ids.insert(instance.minecraft_version.clone());
        match instance.loader.mod_loader() {
            Some((loader, loader_version)) => {
                launch_ids.insert(loaders::profile_id(
                    loader,
                    &instance.minecraft_version,
                    loader_version,
                ));
            }
            None => {
                launch_ids.insert(instance.minecraft_version.clone());
            }
        }
    }
    (launch_ids, base_ids)
}

fn plan(paths: &LauncherPaths, instances: &[Instance], platform: Platform) -> Plan {
    let (launch_ids, base_ids) = used_version_ids(instances);
    let mut plan = Plan {
        complete: true,
        ..Plan::default()
    };
    let add = |plan: &mut Plan, path: PathBuf| {
        let bytes = path_size(&path);
        plan.targets.push((path, bytes));
    };

    for (id, path) in children(&paths.versions_dir()) {
        if !base_ids.contains(&id) && !launch_ids.contains(&id) {
            plan.unused_versions.push(id);
            add(&mut plan, path);
        }
    }
    for (name, path) in children(&paths.meta_dir().join("versions")) {
        let Some(id) = name.strip_suffix(".json") else {
            continue;
        };
        if !base_ids.contains(id) && !launch_ids.contains(id) {
            if !plan.unused_versions.iter().any(|v| v == id) {
                plan.unused_versions.push(id.to_string());
            }
            add(&mut plan, path);
        }
    }
    plan.unused_versions.sort();
    for (name, path) in children(&paths.meta_dir().join("loaders")) {
        let id = name.strip_suffix(".json").unwrap_or(&name);
        if !launch_ids.contains(id) {
            add(&mut plan, path);
        }
    }
    for (id, path) in children(&paths.natives_root()) {
        if !launch_ids.contains(&id) {
            add(&mut plan, path);
        }
    }

    let mut versions = Vec::new();
    for id in &launch_ids {
        match resolve_local(paths, id) {
            Some(version) => versions.push(version),
            None => plan.complete = false,
        }
    }

    if plan.complete {
        let needed: HashSet<String> = versions
            .iter()
            .map(|v| {
                v.java_version.as_ref().map_or_else(
                    || JavaRequirement::legacy().component,
                    |j| j.component.clone(),
                )
            })
            .collect();
        for (component, path) in children(&paths.runtimes_dir()) {
            if !needed.contains(&component) {
                plan.unused_runtimes.push(component);
                add(&mut plan, path);
            }
        }

        let ctx = RuleContext::new(platform);
        let libraries_dir = paths.libraries_dir();
        let mut keep = HashSet::new();
        for id in &launch_ids {
            keep.extend(installer::generated_files(paths, id).unwrap_or_default());
        }
        let mut libraries_known = true;
        for version in &versions {
            match resolve_libraries(version, &ctx, &libraries_dir) {
                Ok(resolved) => {
                    keep.extend(resolved.classpath);
                    keep.extend(resolved.natives.into_iter().map(|n| n.path));
                    keep.extend(resolved.downloads.into_iter().map(|d| d.path));
                }
                Err(_) => libraries_known = false,
            }
        }
        if libraries_known {
            let mut files = Vec::new();
            files_below(&libraries_dir, &mut files);
            for file in files.into_iter().filter(|f| !keep.contains(f)) {
                plan.library_files += 1;
                add(&mut plan, file);
            }
        } else {
            plan.complete = false;
        }

        let assets_dir = paths.assets_dir();
        let indexes_dir = assets_dir.join("indexes");
        let used_indexes: HashSet<String> = versions
            .iter()
            .filter_map(|v| {
                v.asset_index
                    .as_ref()
                    .map(|index| index.id.clone())
                    .or_else(|| v.assets.clone())
            })
            .collect();
        let mut objects = HashSet::new();
        let mut assets_known = true;
        for id in &used_indexes {
            let parsed = std::fs::read_to_string(indexes_dir.join(format!("{id}.json")))
                .ok()
                .and_then(|text| serde_json::from_str::<AssetIndex>(&text).ok());
            match parsed {
                Some(index) => {
                    objects.extend(index.objects.into_values().map(|object| object.hash))
                }
                None => assets_known = false,
            }
        }
        if assets_known {
            for (name, path) in children(&indexes_dir) {
                let id = name.strip_suffix(".json").unwrap_or(&name);
                if !used_indexes.contains(id) {
                    add(&mut plan, path);
                }
            }
            let mut files = Vec::new();
            files_below(&assets_dir.join("objects"), &mut files);
            for file in files {
                let hash = file
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default();
                if !objects.contains(hash) {
                    plan.asset_files += 1;
                    add(&mut plan, file);
                }
            }
        } else {
            plan.complete = false;
        }
    }

    for path in cache_entries(paths) {
        let bytes = path_size(&path);
        plan.cache_bytes += bytes;
        plan.targets.push((path, bytes));
    }
    plan
}

fn cache_is_separate(paths: &LauncherPaths) -> bool {
    let cache = &paths.cache;
    let protected = [
        paths.settings_file(),
        paths.accounts_file(),
        paths.instances_dir(),
    ];
    !paths.root.starts_with(cache) && !protected.iter().any(|path| path.starts_with(cache))
}

fn cache_entries(paths: &LauncherPaths) -> Vec<PathBuf> {
    let mut entries: Vec<PathBuf> = if cache_is_separate(paths) {
        children(&paths.cache)
            .into_iter()
            .map(|(_, path)| path)
            .collect()
    } else {
        tracing::warn!(cache = %paths.cache.display(), "cache overlaps the data directory, skipping it");
        Vec::new()
    };
    for legacy in LEGACY_CACHE_DIRS {
        let path = paths.root.join(legacy);
        if path.is_dir() && !entries.contains(&path) {
            entries.push(path);
        }
    }
    entries
}

pub fn preview(
    paths: &LauncherPaths,
    instances: &[Instance],
    platform: Platform,
) -> CleanupSummary {
    plan(paths, instances, platform).summary()
}

pub fn clean(paths: &LauncherPaths, instances: &[Instance], platform: Platform) -> u64 {
    execute(paths, plan(paths, instances, platform))
}

fn remove_empty_dirs(dir: &Path) {
    for (_, path) in children(dir) {
        if std::fs::symlink_metadata(&path).is_ok_and(|m| m.is_dir()) {
            remove_empty_dirs(&path);
            let _ = std::fs::remove_dir(&path);
        }
    }
}

fn execute(paths: &LauncherPaths, plan: Plan) -> u64 {
    let mut freed = 0;
    for (path, bytes) in plan.targets {
        let result = match std::fs::symlink_metadata(&path) {
            Ok(meta) if meta.is_dir() => std::fs::remove_dir_all(&path),
            Ok(_) => std::fs::remove_file(&path),
            Err(err) => Err(err),
        };
        match result {
            Ok(()) => freed += bytes,
            Err(err) => {
                tracing::warn!(path = %path.display(), error = %err, "cleanup could not remove path")
            }
        }
    }
    remove_empty_dirs(&paths.libraries_dir());
    remove_empty_dirs(&paths.assets_dir().join("objects"));
    freed
}

fn usage(
    paths: &LauncherPaths,
    instances: &[Instance],
) -> (Vec<CategoryUsage>, Vec<InstanceUsage>) {
    let mut per_instance: Vec<InstanceUsage> = instances
        .iter()
        .map(|instance| InstanceUsage {
            id: instance.id.clone(),
            name: instance.name.clone(),
            size_bytes: path_size(&paths.instance_dir(&instance.id)),
        })
        .collect();
    per_instance.sort_by_key(|usage| std::cmp::Reverse(usage.size_bytes));
    let categories = vec![
        CategoryUsage {
            category: StorageCategory::Instances,
            size_bytes: path_size(&paths.instances_dir()),
        },
        CategoryUsage {
            category: StorageCategory::Versions,
            size_bytes: path_size(&paths.versions_dir()),
        },
        CategoryUsage {
            category: StorageCategory::Libraries,
            size_bytes: path_size(&paths.libraries_dir()),
        },
        CategoryUsage {
            category: StorageCategory::Assets,
            size_bytes: path_size(&paths.assets_dir()),
        },
        CategoryUsage {
            category: StorageCategory::Runtimes,
            size_bytes: path_size(&paths.runtimes_dir()),
        },
        CategoryUsage {
            category: StorageCategory::Natives,
            size_bytes: path_size(&paths.natives_root()),
        },
        CategoryUsage {
            category: StorageCategory::Trash,
            size_bytes: path_size(&paths.trash_dir()),
        },
        CategoryUsage {
            category: StorageCategory::Cache,
            size_bytes: cache_entries(paths)
                .iter()
                .map(|path| path_size(path))
                .sum(),
        },
    ];
    (categories, per_instance)
}

impl Services {
    pub(crate) fn ensure_idle(&self) -> CoreResult<()> {
        let busy_task = self
            .tasks
            .list()
            .iter()
            .any(|task| task.status == TaskStatus::Running);
        if !self.games.running().is_empty() || busy_task {
            return Err(CoreError::InvalidInput(
                "close running games and wait for downloads before cleaning up".into(),
            ));
        }
        Ok(())
    }

    pub async fn storage_report(&self) -> CoreResult<StorageReport> {
        let instances = self.instances.list().await?;
        let paths = self.paths.clone();
        let platform = self.platform;
        tokio::task::spawn_blocking(move || {
            let (categories, instances_usage) = usage(&paths, &instances);
            StorageReport {
                categories,
                instances: instances_usage,
                cleanup: plan(&paths, &instances, platform).summary(),
            }
        })
        .await
        .map_err(|err| CoreError::Unsupported(format!("storage report panicked: {err}")))
    }

    pub async fn clean_storage(&self) -> CoreResult<CleanupResult> {
        self.ensure_idle()?;
        let instances = self.instances.list().await?;
        let paths = self.paths.clone();
        let platform = self.platform;
        let freed_bytes = tokio::task::spawn_blocking(move || clean(&paths, &instances, platform))
            .await
            .map_err(|err| CoreError::Unsupported(format!("cleanup panicked: {err}")))?;
        tracing::info!(freed_bytes, "storage cleanup finished");
        Ok(CleanupResult { freed_bytes })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instances::{JavaSettings, LoaderSpec};
    use crate::platform::{Arch, Os};

    fn instance(id: &str, version: &str, loader: LoaderSpec) -> Instance {
        Instance {
            schema: 1,
            id: id.into(),
            name: id.into(),
            minecraft_version: version.into(),
            loader,
            java: JavaSettings::default(),
            resolution: None,
            launch: Default::default(),
            group: None,
            pinned: false,
            icon: None,
            created_at: 0,
            last_played_at: None,
            total_play_secs: 0,
            sync_packs: Vec::new(),
        }
    }

    fn write(path: &Path, content: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    fn version_json(id: &str, component: &str, index: &str, library: &str) -> String {
        serde_json::json!({
            "id": id,
            "type": "release",
            "mainClass": "net.minecraft.client.main.Main",
            "assetIndex": { "id": index, "sha1": "x", "size": 1, "url": "https://example.com/i.json" },
            "javaVersion": { "component": component, "majorVersion": 17 },
            "libraries": [{
                "name": library,
                "downloads": { "artifact": { "path": "org/keep/keep/1.0/keep-1.0.jar", "sha1": "x", "size": 1, "url": "https://example.com/k.jar" } }
            }]
        })
        .to_string()
    }

    fn setup(dir: &Path) -> LauncherPaths {
        let paths = LauncherPaths::new(dir.join("data"), dir.join("cache"), dir.join("logs"));
        write(
            &paths.version_json_file("1.20.1"),
            &version_json("1.20.1", "java-runtime-gamma", "5", "org.keep:keep:1.0"),
        );
        write(
            &paths.version_json_file("1.12.2"),
            &version_json("1.12.2", "jre-legacy", "1.12", "org.old:old:1.0"),
        );
        write(
            &paths.version_json_file("fabric-loader-0.16.0-1.20.1"),
            &serde_json::json!({ "id": "fabric-loader-0.16.0-1.20.1", "inheritsFrom": "1.20.1", "libraries": [] }).to_string(),
        );
        write(&paths.client_jar("1.20.1"), "jar");
        write(&paths.client_jar("1.12.2"), "old jar");
        write(
            &paths
                .natives_dir("fabric-loader-0.16.0-1.20.1")
                .join("a.dll"),
            "n",
        );
        write(&paths.natives_dir("1.12.2").join("b.dll"), "n");
        write(
            &paths.runtimes_dir().join("java-runtime-gamma").join("bin"),
            "java",
        );
        write(
            &paths.runtimes_dir().join("jre-legacy").join("bin"),
            "java8",
        );
        write(
            &paths.libraries_dir().join("org/keep/keep/1.0/keep-1.0.jar"),
            "keep",
        );
        write(
            &paths.libraries_dir().join("org/old/old/1.0/old-1.0.jar"),
            "old library",
        );
        write(
            &paths.assets_dir().join("indexes").join("5.json"),
            &serde_json::json!({ "objects": { "a": { "hash": "aa11", "size": 1 } } }).to_string(),
        );
        write(&paths.assets_dir().join("indexes").join("1.12.json"), "{}");
        write(
            &paths.assets_dir().join("objects").join("aa").join("aa11"),
            "keep",
        );
        write(
            &paths.assets_dir().join("objects").join("bb").join("bb22"),
            "drop",
        );
        write(
            &paths.assets_dir().join("log_configs").join("client.xml"),
            "log",
        );
        write(
            &dir.join("cache").join("modpacks").join("x.mrpack"),
            "cached",
        );
        paths
    }

    #[test]
    fn plan_keeps_everything_a_fabric_instance_needs() {
        let dir = tempfile::tempdir().unwrap();
        let paths = setup(dir.path());
        let instances = [instance(
            "fab",
            "1.20.1",
            LoaderSpec::Fabric {
                loader_version: "0.16.0".into(),
            },
        )];
        let platform = Platform {
            os: Os::Windows,
            arch: Arch::X86_64,
        };
        let plan = plan(&paths, &instances, platform);
        assert!(plan.complete);
        assert_eq!(plan.unused_versions, ["1.12.2"]);
        assert_eq!(plan.unused_runtimes, ["jre-legacy"]);
        assert_eq!(plan.library_files, 1);
        assert_eq!(plan.asset_files, 1);

        let freed = execute(&paths, plan);
        assert!(freed > 0);
        assert!(paths.client_jar("1.20.1").is_file());
        assert!(!paths.client_jar("1.12.2").exists());
        assert!(paths.natives_dir("fabric-loader-0.16.0-1.20.1").is_dir());
        assert!(!paths.natives_dir("1.12.2").exists());
        assert!(paths.runtimes_dir().join("java-runtime-gamma").exists());
        assert!(!paths.runtimes_dir().join("jre-legacy").exists());
        assert!(
            paths
                .libraries_dir()
                .join("org/keep/keep/1.0/keep-1.0.jar")
                .is_file()
        );
        assert!(!paths.libraries_dir().join("org/old").exists());
        assert!(paths.assets_dir().join("objects/aa/aa11").is_file());
        assert!(!paths.assets_dir().join("objects/bb").exists());
        assert!(paths.assets_dir().join("log_configs/client.xml").is_file());
        assert!(!paths.assets_dir().join("indexes/1.12.json").exists());
        assert!(
            paths
                .version_json_file("fabric-loader-0.16.0-1.20.1")
                .is_file()
        );
        assert!(!dir.path().join("cache").join("modpacks").exists());
    }

    #[test]
    fn plan_keeps_the_files_neoforge_processors_generated() {
        let dir = tempfile::tempdir().unwrap();
        let paths = setup(dir.path());
        let id = "neoforge-21.1.265";
        write(
            &paths.version_json_file(id),
            &serde_json::json!({ "id": id, "inheritsFrom": "1.20.1", "libraries": [] }).to_string(),
        );
        write(&paths.client_jar(id), "jar copy");
        let patched = "net/neoforged/neoforge/21.1.265/neoforge-21.1.265-client.jar";
        write(&paths.libraries_dir().join(patched), "patched");
        write(
            &paths.loader_manifest_file(id),
            &serde_json::json!({ "generated": [patched] }).to_string(),
        );
        write(
            &paths.loader_manifest_file("neoforge-21.1.1"),
            &serde_json::json!({ "generated": [] }).to_string(),
        );
        let instances = [instance(
            "neo",
            "1.20.1",
            LoaderSpec::NeoForge {
                loader_version: "21.1.265".into(),
            },
        )];
        let platform = Platform {
            os: Os::Windows,
            arch: Arch::X86_64,
        };
        let plan = plan(&paths, &instances, platform);
        assert!(plan.complete);

        execute(&paths, plan);
        assert!(paths.libraries_dir().join(patched).is_file());
        assert!(paths.client_jar(id).is_file());
        assert!(paths.loader_manifest_file(id).is_file());
        assert!(!paths.loader_manifest_file("neoforge-21.1.1").exists());
        assert!(!paths.libraries_dir().join("org/old").exists());
    }

    #[test]
    fn cache_that_is_the_data_root_never_deletes_user_data() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("data");
        let paths = LauncherPaths::new(root.clone(), root.clone(), dir.path().join("logs"));
        write(&paths.settings_file(), "{}");
        write(&paths.accounts_file(), "{}");
        write(
            &paths
                .instance_dir("survival")
                .join("minecraft")
                .join("options.txt"),
            "x",
        );
        write(&root.join("modpacks").join("pack.mrpack"), "cached pack");
        write(&root.join("java").join("jdk.zip"), "cached java");

        let platform = Platform {
            os: Os::Windows,
            arch: Arch::X86_64,
        };
        let instances = [instance("survival", "1.20.1", LoaderSpec::Vanilla)];
        let summary = preview(&paths, &instances, platform);
        assert_eq!(summary.cache_bytes, 22);
        execute(&paths, plan(&paths, &instances, platform));
        assert!(paths.settings_file().is_file());
        assert!(paths.accounts_file().is_file());
        assert!(
            paths
                .instance_dir("survival")
                .join("minecraft")
                .join("options.txt")
                .is_file()
        );
        assert!(!root.join("modpacks").exists());
        assert!(!root.join("java").exists());
    }

    #[test]
    fn missing_metadata_keeps_shared_files() {
        let dir = tempfile::tempdir().unwrap();
        let paths = setup(dir.path());
        let instances = [
            instance("new", "1.20.1", LoaderSpec::Vanilla),
            instance("never", "1.21.4", LoaderSpec::Vanilla),
        ];
        let plan = plan(
            &paths,
            &instances,
            Platform {
                os: Os::Windows,
                arch: Arch::X86_64,
            },
        );
        assert!(!plan.complete);
        assert!(plan.unused_runtimes.is_empty());
        assert_eq!(plan.library_files, 0);
        assert_eq!(plan.asset_files, 0);
        assert_eq!(
            plan.unused_versions,
            ["1.12.2", "fabric-loader-0.16.0-1.20.1"]
        );
    }
}

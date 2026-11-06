use std::path::{Path, PathBuf};

use serde::Serialize;
use ts_rs::TS;

use crate::{CoreResult, error::IoResultExt};

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LauncherPaths {
    pub config: PathBuf,
    pub root: PathBuf,
    pub cache: PathBuf,
    pub logs: PathBuf,
}

impl LauncherPaths {
    pub fn new(root: PathBuf, cache: PathBuf, logs: PathBuf) -> Self {
        Self {
            config: root.clone(),
            root,
            cache,
            logs,
        }
    }

    pub fn with_data_root(config: PathBuf, root: PathBuf, logs: PathBuf) -> Self {
        Self {
            cache: root.join("cache"),
            config,
            root,
            logs,
        }
    }

    pub fn settings_path(config: &Path) -> PathBuf {
        config.join("settings.json")
    }

    pub fn uses_default_root(&self) -> bool {
        self.root == self.config
    }

    pub async fn ensure_base_dirs(&self) -> CoreResult<()> {
        for dir in [
            self.config.clone(),
            self.root.clone(),
            self.cache.clone(),
            self.logs.clone(),
            self.meta_dir(),
            self.instances_dir(),
        ] {
            tokio::fs::create_dir_all(&dir).await.at_path(&dir)?;
        }
        Ok(())
    }

    pub fn settings_file(&self) -> PathBuf {
        Self::settings_path(&self.config)
    }

    pub fn accounts_file(&self) -> PathBuf {
        self.config.join("accounts.json")
    }

    pub fn meta_dir(&self) -> PathBuf {
        self.root.join("meta")
    }

    pub fn version_manifest_file(&self) -> PathBuf {
        self.meta_dir().join("version_manifest_v2.json")
    }

    pub fn version_json_file(&self, id: &str) -> PathBuf {
        self.meta_dir().join("versions").join(format!("{id}.json"))
    }

    pub fn java_runtime_index_file(&self) -> PathBuf {
        self.meta_dir().join("java").join("all.json")
    }

    pub fn loader_manifest_file(&self, id: &str) -> PathBuf {
        self.meta_dir().join("loaders").join(format!("{id}.json"))
    }

    pub fn client_jar(&self, id: &str) -> PathBuf {
        self.versions_dir().join(id).join(format!("{id}.jar"))
    }

    pub fn natives_dir(&self, id: &str) -> PathBuf {
        self.natives_root().join(id)
    }

    pub fn libraries_dir(&self) -> PathBuf {
        self.root.join("libraries")
    }

    pub fn assets_dir(&self) -> PathBuf {
        self.root.join("assets")
    }

    pub fn versions_dir(&self) -> PathBuf {
        self.root.join("versions")
    }

    pub fn natives_root(&self) -> PathBuf {
        self.root.join("natives")
    }

    pub fn runtimes_dir(&self) -> PathBuf {
        self.root.join("runtimes")
    }

    pub fn skins_dir(&self) -> PathBuf {
        self.root.join("skins")
    }

    pub fn trash_dir(&self) -> PathBuf {
        self.root.join("trash")
    }

    pub fn instances_dir(&self) -> PathBuf {
        self.root.join("instances")
    }

    pub fn instance_dir(&self, slug: &str) -> PathBuf {
        self.instances_dir().join(slug)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

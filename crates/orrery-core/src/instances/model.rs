use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::loaders::ModLoader;
use crate::storage::Versioned;
use crate::syncpacks::SyncSubscription;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum LoaderSpec {
    Vanilla,
    Fabric { loader_version: String },
    LegacyFabric { loader_version: String },
    Quilt { loader_version: String },
    Forge { loader_version: String },
    NeoForge { loader_version: String },
}

impl LoaderSpec {
    pub fn mod_loader(&self) -> Option<(ModLoader, &str)> {
        match self {
            Self::Vanilla => None,
            Self::Fabric { loader_version } => Some((ModLoader::Fabric, loader_version)),
            Self::LegacyFabric { loader_version } => {
                Some((ModLoader::LegacyFabric, loader_version))
            }
            Self::Quilt { loader_version } => Some((ModLoader::Quilt, loader_version)),
            Self::Forge { loader_version } => Some((ModLoader::Forge, loader_version)),
            Self::NeoForge { loader_version } => Some((ModLoader::NeoForge, loader_version)),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct JavaSettings {
    pub java_path: Option<String>,
    pub min_memory_mb: Option<u32>,
    pub max_memory_mb: Option<u32>,
    pub extra_jvm_args: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EnvVar {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export)]
pub struct LaunchSettings {
    pub account_id: Option<String>,
    pub join_server: Option<String>,
    pub extra_game_args: Vec<String>,
    pub env: Vec<EnvVar>,
    pub wrapper: Option<String>,
    pub pre_launch: Option<String>,
    pub post_exit: Option<String>,
    pub gamemode: bool,
    pub mangohud: bool,
    pub prime_offload: bool,
    pub companion_disabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ResolutionSetting {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Instance {
    pub schema: u32,
    pub id: String,
    pub name: String,
    pub minecraft_version: String,
    pub loader: LoaderSpec,
    #[serde(default)]
    pub java: JavaSettings,
    #[serde(default)]
    pub resolution: Option<ResolutionSetting>,
    #[serde(default)]
    pub launch: LaunchSettings,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub icon: Option<String>,
    pub created_at: i64,
    #[serde(default)]
    pub last_played_at: Option<i64>,
    #[serde(default)]
    pub total_play_secs: u64,
    #[serde(default)]
    pub sync_packs: Vec<SyncSubscription>,
}

impl Versioned for Instance {
    const SCHEMA: u32 = 1;
    fn schema(&self) -> u32 {
        self.schema
    }
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NewInstance {
    pub name: String,
    pub minecraft_version: String,
    pub loader: LoaderSpec,
    pub copy_settings_from: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct InstanceUpdate {
    pub name: String,
    pub minecraft_version: String,
    pub loader: LoaderSpec,
    pub java: JavaSettings,
    pub resolution: Option<ResolutionSetting>,
    pub launch: LaunchSettings,
}

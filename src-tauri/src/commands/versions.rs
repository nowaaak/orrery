use orrery_core::loaders::{self, LoaderVersion, ModLoader};
use orrery_core::minecraft::manifest::VersionSummary;
use serde::Serialize;
use tauri::State;
use ts_rs::TS;

use crate::error::CommandResult;
use crate::state::AppState;

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct VersionList {
    pub latest_release: String,
    pub versions: Vec<VersionSummary>,
}

#[tauri::command]
pub async fn list_minecraft_versions(
    state: State<'_, AppState>,
    refresh: bool,
    include_snapshots: bool,
) -> CommandResult<VersionList> {
    let manifest = state.meta.manifest(refresh).await?;
    Ok(VersionList {
        latest_release: manifest.latest.release.clone(),
        versions: manifest.supported_versions(include_snapshots),
    })
}

#[tauri::command]
pub async fn list_loader_versions(
    state: State<'_, AppState>,
    loader: ModLoader,
    game_version: String,
) -> CommandResult<Vec<LoaderVersion>> {
    Ok(loaders::loader_versions(&state.http, loader, &game_version).await?)
}

use std::path::PathBuf;

use orrery_core::curseforge::modpack::PendingDownload;
use orrery_core::instances::mods::ContentSource;
use orrery_core::modpacks::ModpackExport;
use orrery_core::modrinth::models::ModSearchPage;
use orrery_core::tasks::TaskId;
use tauri::{AppHandle, Manager, State};

use crate::error::CommandResult;
use crate::state::AppState;

#[tauri::command]
pub async fn search_modpacks(
    state: State<'_, AppState>,
    source: Option<ContentSource>,
    query: String,
    offset: u32,
) -> CommandResult<ModSearchPage> {
    Ok(state
        .search_modpacks(source.unwrap_or_default(), &query, offset)
        .await?)
}

#[tauri::command]
pub async fn is_modpack(state: State<'_, AppState>, path: PathBuf) -> CommandResult<bool> {
    Ok(state.is_modpack_file(path).await)
}

#[tauri::command]
pub async fn check_pending_downloads(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
) -> CommandResult<Vec<PendingDownload>> {
    match app.path().download_dir() {
        Ok(folder) => Ok(state
            .collect_pending_downloads(&instance_id, &folder)
            .await?),
        Err(err) => {
            tracing::warn!(error = %err, "no downloads folder");
            Ok(state.pending_downloads(&instance_id).await?)
        }
    }
}

#[tauri::command]
pub async fn provide_pending_download(
    state: State<'_, AppState>,
    instance_id: String,
    file_id: u32,
    path: PathBuf,
) -> CommandResult<Vec<PendingDownload>> {
    Ok(state
        .provide_pending_download(&instance_id, file_id, &path)
        .await?)
}

#[tauri::command]
pub async fn skip_pending_download(
    state: State<'_, AppState>,
    instance_id: String,
    file_id: u32,
) -> CommandResult<Vec<PendingDownload>> {
    Ok(state.skip_pending_download(&instance_id, file_id).await?)
}

#[tauri::command]
pub async fn install_modpack(
    state: State<'_, AppState>,
    project_id: String,
    title: String,
) -> CommandResult<TaskId> {
    Ok(state.install_modpack(&project_id, &title).await?)
}

#[tauri::command]
pub async fn import_modpack(state: State<'_, AppState>, path: PathBuf) -> CommandResult<TaskId> {
    Ok(state.import_modpack_file(path).await?)
}

#[tauri::command]
pub async fn export_modpack(
    state: State<'_, AppState>,
    instance_id: String,
    path: PathBuf,
) -> CommandResult<ModpackExport> {
    Ok(state.export_modpack(&instance_id, path).await?)
}

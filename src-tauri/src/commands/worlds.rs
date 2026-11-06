use orrery_core::trash::TrashEntry;
use orrery_core::worlds::{WorldBackup, WorldList};
use tauri::State;

use crate::error::CommandResult;
use crate::state::AppState;

#[tauri::command]
pub async fn list_worlds(
    state: State<'_, AppState>,
    instance_id: String,
) -> CommandResult<WorldList> {
    Ok(state.list_worlds(&instance_id).await?)
}

#[tauri::command]
pub async fn backup_world(
    state: State<'_, AppState>,
    instance_id: String,
    folder: String,
) -> CommandResult<WorldBackup> {
    Ok(state.backup_world(&instance_id, &folder).await?)
}

#[tauri::command]
pub async fn restore_world_backup(
    state: State<'_, AppState>,
    instance_id: String,
    folder: String,
    file_name: String,
) -> CommandResult<()> {
    Ok(state
        .restore_world_backup(&instance_id, &folder, &file_name)
        .await?)
}

#[tauri::command]
pub async fn delete_world(
    state: State<'_, AppState>,
    instance_id: String,
    folder: String,
) -> CommandResult<TrashEntry> {
    Ok(state.delete_world(&instance_id, &folder).await?)
}

#[tauri::command]
pub async fn delete_world_backup(
    state: State<'_, AppState>,
    instance_id: String,
    folder: String,
    file_name: String,
) -> CommandResult<()> {
    Ok(state
        .delete_world_backup(&instance_id, &folder, &file_name)
        .await?)
}

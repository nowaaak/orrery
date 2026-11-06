use orrery_core::trash::TrashEntry;
use tauri::State;

use crate::error::CommandResult;
use crate::state::AppState;

#[tauri::command]
pub async fn list_trash(state: State<'_, AppState>) -> CommandResult<Vec<TrashEntry>> {
    Ok(state.list_trash().await?)
}

#[tauri::command]
pub async fn restore_trash(state: State<'_, AppState>, id: String) -> CommandResult<TrashEntry> {
    Ok(state.restore_trash(&id).await?)
}

#[tauri::command]
pub async fn delete_trash_entry(state: State<'_, AppState>, id: String) -> CommandResult<()> {
    Ok(state.delete_trash_entry(&id).await?)
}

#[tauri::command]
pub async fn empty_trash(state: State<'_, AppState>) -> CommandResult<u32> {
    Ok(state.empty_trash().await?)
}

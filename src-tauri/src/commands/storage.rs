use orrery_core::cleanup::{CleanupResult, StorageReport};
use tauri::State;

use crate::error::CommandResult;
use crate::state::AppState;

#[tauri::command]
pub async fn storage_report(state: State<'_, AppState>) -> CommandResult<StorageReport> {
    Ok(state.storage_report().await?)
}

#[tauri::command]
pub async fn clean_storage(state: State<'_, AppState>) -> CommandResult<CleanupResult> {
    Ok(state.clean_storage().await?)
}

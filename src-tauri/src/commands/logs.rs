use orrery_core::logshare::SharedLog;
use tauri::State;

use crate::error::CommandResult;
use crate::state::AppState;

#[tauri::command]
pub async fn prepare_log_share(state: State<'_, AppState>, text: String) -> CommandResult<String> {
    Ok(state.prepare_log_share(&text).await)
}

#[tauri::command]
pub async fn share_log(state: State<'_, AppState>, text: String) -> CommandResult<SharedLog> {
    Ok(state.share_log(&text).await?)
}

#[tauri::command]
pub async fn read_launcher_log(state: State<'_, AppState>) -> CommandResult<Vec<String>> {
    Ok(state.launcher_log_tail().await?)
}

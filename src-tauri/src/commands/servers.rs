use orrery_core::servers::list::{SavedServer, ServerChange};
use orrery_core::servers::ping::ServerStatus;
use tauri::State;

use crate::error::CommandResult;
use crate::state::AppState;

#[tauri::command]
pub async fn list_servers(
    state: State<'_, AppState>,
    id: String,
) -> CommandResult<Vec<SavedServer>> {
    Ok(state.list_servers(&id).await?)
}

#[tauri::command]
pub async fn change_servers(
    state: State<'_, AppState>,
    id: String,
    change: ServerChange,
) -> CommandResult<Vec<SavedServer>> {
    Ok(state.change_servers(&id, change).await?)
}

#[tauri::command]
pub async fn ping_server(
    state: State<'_, AppState>,
    address: String,
) -> CommandResult<ServerStatus> {
    Ok(state.ping_server(&address).await?)
}

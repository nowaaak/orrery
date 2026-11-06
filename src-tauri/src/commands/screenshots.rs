use orrery_core::screenshots::Screenshot;
use tauri::State;
use tauri::ipc::Response;

use crate::error::CommandResult;
use crate::state::AppState;

#[tauri::command]
pub async fn list_screenshots(
    state: State<'_, AppState>,
    id: String,
) -> CommandResult<Vec<Screenshot>> {
    Ok(state.list_screenshots(&id).await?)
}

#[tauri::command]
pub async fn read_screenshot(
    state: State<'_, AppState>,
    id: String,
    name: String,
) -> CommandResult<Response> {
    Ok(Response::new(state.read_screenshot(&id, &name).await?))
}

#[tauri::command]
pub async fn delete_screenshot(
    state: State<'_, AppState>,
    id: String,
    name: String,
) -> CommandResult<()> {
    Ok(state.delete_screenshot(&id, &name).await?)
}

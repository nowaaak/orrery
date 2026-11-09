use orrery_core::auth::microsoft::{BrowserLoginInfo, DeviceCodeInfo, LoginPage};
use tauri::State;

use crate::error::CommandResult;
use crate::state::AppState;

#[tauri::command]
pub fn microsoft_login_available(state: State<'_, AppState>) -> bool {
    state.microsoft.is_configured()
}

#[tauri::command]
pub async fn begin_microsoft_login(state: State<'_, AppState>) -> CommandResult<DeviceCodeInfo> {
    Ok(state.microsoft.begin_login().await?)
}

#[tauri::command]
pub async fn begin_browser_login(
    state: State<'_, AppState>,
    page: LoginPage,
) -> CommandResult<BrowserLoginInfo> {
    Ok(state.microsoft.begin_browser_login(page).await?)
}

#[tauri::command]
pub fn cancel_microsoft_login(state: State<'_, AppState>, login_id: String) {
    state.microsoft.cancel_login(&login_id);
}

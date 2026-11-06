use orrery_core::auth::AccountsView;
use tauri::State;

use crate::error::CommandResult;
use crate::state::AppState;

#[tauri::command]
pub async fn list_accounts(state: State<'_, AppState>) -> CommandResult<AccountsView> {
    Ok(state.accounts.view().await)
}

#[tauri::command]
pub async fn remove_account(state: State<'_, AppState>, id: String) -> CommandResult<()> {
    Ok(state.remove_account(&id).await?)
}

#[tauri::command]
pub async fn set_active_account(state: State<'_, AppState>, id: String) -> CommandResult<()> {
    Ok(state.accounts.set_active(&id).await?)
}

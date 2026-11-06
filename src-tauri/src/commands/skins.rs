use std::path::PathBuf;

use orrery_core::skins::{self, LibrarySkin, ProfileView};
use tauri::State;
use tauri::ipc::Response;

use crate::error::CommandResult;
use crate::state::AppState;

#[tauri::command]
pub async fn list_skin_library(state: State<'_, AppState>) -> CommandResult<Vec<LibrarySkin>> {
    Ok(state.skin_library.list().await?)
}

#[tauri::command]
pub async fn get_library_skin(state: State<'_, AppState>, id: String) -> CommandResult<Response> {
    Ok(Response::new(state.skin_library.bytes(&id).await?))
}

#[tauri::command]
pub async fn import_skin_file(
    state: State<'_, AppState>,
    path: PathBuf,
    slim: bool,
) -> CommandResult<LibrarySkin> {
    Ok(state.import_skin_file(&path, slim).await?)
}

#[tauri::command]
pub async fn import_player_skin(
    state: State<'_, AppState>,
    player: String,
) -> CommandResult<LibrarySkin> {
    Ok(state.import_player_skin(&player).await?)
}

#[tauri::command]
pub async fn update_library_skin(
    state: State<'_, AppState>,
    id: String,
    name: String,
    slim: bool,
) -> CommandResult<LibrarySkin> {
    Ok(state.skin_library.update(&id, &name, slim).await?)
}

#[tauri::command]
pub async fn remove_library_skin(state: State<'_, AppState>, id: String) -> CommandResult<()> {
    Ok(state.skin_library.remove(&id).await?)
}

#[tauri::command]
pub async fn apply_library_skin(
    state: State<'_, AppState>,
    account_id: String,
    skin_id: String,
) -> CommandResult<()> {
    Ok(state.apply_library_skin(&account_id, &skin_id).await?)
}

#[tauri::command]
pub async fn get_account_profile(
    state: State<'_, AppState>,
    account_id: String,
) -> CommandResult<ProfileView> {
    Ok(state.account_profile(&account_id).await?)
}

#[tauri::command]
pub async fn set_account_cape(
    state: State<'_, AppState>,
    account_id: String,
    cape_id: Option<String>,
) -> CommandResult<()> {
    Ok(state
        .set_account_cape(&account_id, cape_id.as_deref())
        .await?)
}

#[tauri::command]
pub async fn fetch_texture(state: State<'_, AppState>, url: String) -> CommandResult<Response> {
    Ok(Response::new(
        skins::fetch_texture(&state.http, &url).await?,
    ))
}

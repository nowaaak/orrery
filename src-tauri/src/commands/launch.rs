use std::path::PathBuf;

use orrery_core::java::system::{self, JavaInstallation};
use orrery_core::minecraft::process::RunningGame;
use orrery_core::tasks::TaskId;
use tauri::State;

use crate::error::CommandResult;
use crate::state::AppState;

#[tauri::command]
pub async fn install_instance(state: State<'_, AppState>, id: String) -> CommandResult<TaskId> {
    Ok(state.spawn_install(&id, false).await?)
}

#[tauri::command]
pub async fn repair_instance(state: State<'_, AppState>, id: String) -> CommandResult<TaskId> {
    Ok(state.spawn_install(&id, true).await?)
}

#[tauri::command]
pub async fn launch_instance(
    state: State<'_, AppState>,
    id: String,
    account_id: Option<String>,
    world: Option<String>,
    server: Option<String>,
) -> CommandResult<TaskId> {
    Ok(state.spawn_launch(&id, account_id, world, server).await?)
}

#[tauri::command]
pub async fn list_java_installations(
    state: State<'_, AppState>,
) -> CommandResult<Vec<JavaInstallation>> {
    Ok(system::detect(&state.paths.runtimes_dir(), state.platform.os).await)
}

#[tauri::command]
pub fn kill_instance(state: State<'_, AppState>, id: String) -> CommandResult<()> {
    Ok(state.games.kill(&id)?)
}

#[tauri::command]
pub fn list_running(state: State<'_, AppState>) -> Vec<RunningGame> {
    state.games.running()
}

#[tauri::command]
pub fn send_game_toast(state: State<'_, AppState>, title: String, body: String) -> usize {
    state.bridge.toast_all(&title, &body)
}

#[tauri::command]
pub fn get_game_log(state: State<'_, AppState>, id: String) -> Vec<String> {
    state.games.log_tail(&id)
}

#[tauri::command]
pub async fn export_game_log(
    state: State<'_, AppState>,
    id: String,
    path: PathBuf,
) -> CommandResult<()> {
    Ok(state.export_game_log(&id, &path).await?)
}

use std::path::PathBuf;

use orrery_core::data_dir::{DataDirMode, DataDirTarget};
use orrery_core::paths::LauncherPaths;
use orrery_core::settings::{Settings, SettingsStore};
use orrery_core::tasks::TaskId;
use tauri::{AppHandle, Manager, State};

use crate::error::{CommandError, CommandResult};
use crate::state::AppState;
use crate::{StartupIssue, StartupState};

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> CommandResult<Settings> {
    Ok(state.settings.get().await)
}

#[tauri::command]
pub async fn update_settings(
    state: State<'_, AppState>,
    settings: Settings,
) -> CommandResult<Settings> {
    let saved = state.settings.update(settings).await?;
    if let Some(presence) = &state.presence {
        presence.set_level(saved.discord_presence);
    }
    Ok(saved)
}

#[tauri::command]
pub fn discord_available(state: State<'_, AppState>) -> bool {
    state.presence.is_some()
}

#[tauri::command]
pub fn get_paths(state: State<'_, AppState>) -> LauncherPaths {
    state.paths.clone()
}

#[tauri::command]
pub fn get_startup_issue(state: State<'_, StartupState>) -> Option<StartupIssue> {
    state.0.clone()
}

#[tauri::command]
pub async fn inspect_data_dir(
    state: State<'_, AppState>,
    path: PathBuf,
) -> CommandResult<DataDirTarget> {
    Ok(state.inspect_data_dir(path).await?)
}

#[tauri::command]
pub async fn change_data_dir(
    state: State<'_, AppState>,
    path: PathBuf,
    mode: DataDirMode,
) -> CommandResult<TaskId> {
    Ok(state.change_data_dir(path, mode).await?)
}

#[tauri::command]
pub async fn reset_data_dir(app: AppHandle) -> CommandResult<()> {
    let config = app
        .path()
        .app_local_data_dir()
        .map_err(|err| CommandError {
            kind: "io".into(),
            message: err.to_string(),
        })?;
    let store = SettingsStore::load(LauncherPaths::settings_path(&config)).await?;
    store.set_data_dir(None).await?;
    tracing::info!("data folder reset to the default location");
    app.restart();
}

#[tauri::command]
pub fn restart_app(app: AppHandle) {
    app.restart();
}

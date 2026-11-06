use std::path::PathBuf;

use orrery_core::datapacks::DataPack;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::error::{CommandError, CommandResult};
use crate::state::AppState;

#[tauri::command]
pub async fn list_datapacks(
    state: State<'_, AppState>,
    instance_id: String,
    folder: String,
) -> CommandResult<Vec<DataPack>> {
    Ok(state.list_datapacks(&instance_id, &folder).await?)
}

#[tauri::command]
pub async fn add_datapack(
    state: State<'_, AppState>,
    instance_id: String,
    folder: String,
    path: PathBuf,
) -> CommandResult<DataPack> {
    Ok(state.add_datapack(&instance_id, &folder, path).await?)
}

#[tauri::command]
pub async fn remove_datapack(
    state: State<'_, AppState>,
    instance_id: String,
    folder: String,
    name: String,
) -> CommandResult<()> {
    Ok(state.remove_datapack(&instance_id, &folder, &name).await?)
}

#[tauri::command]
pub async fn open_datapacks_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
    folder: String,
) -> CommandResult<()> {
    let dir = state.datapacks_dir(&instance_id, &folder).await?;
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|err| CommandError {
            kind: "io".into(),
            message: err.to_string(),
        })
}

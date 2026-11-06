use std::path::PathBuf;

use orrery_core::CoreError;
use orrery_core::importers::{ForeignInstance, ImportPreview, ImportTarget};
use orrery_core::optifine::{self, OptifineJar};
use orrery_core::tasks::TaskId;
use tauri::State;

use crate::error::CommandResult;
use crate::state::AppState;

#[tauri::command]
pub async fn scan_imports(state: State<'_, AppState>) -> CommandResult<Vec<ForeignInstance>> {
    Ok(state.scan_imports().await?)
}

#[tauri::command]
pub async fn inspect_import(
    state: State<'_, AppState>,
    path: PathBuf,
) -> CommandResult<Vec<ForeignInstance>> {
    Ok(state.inspect_import(path).await?)
}

#[tauri::command]
pub async fn preview_import(
    state: State<'_, AppState>,
    target: ImportTarget,
) -> CommandResult<ImportPreview> {
    Ok(state.preview_import(target).await?)
}

#[tauri::command]
pub async fn start_import(
    state: State<'_, AppState>,
    target: ImportTarget,
    name: Option<String>,
) -> CommandResult<TaskId> {
    Ok(state.start_import(target, name).await?)
}

#[tauri::command]
pub async fn inspect_optifine(path: PathBuf) -> CommandResult<Option<OptifineJar>> {
    let found = tokio::task::spawn_blocking(move || optifine::inspect(&path))
        .await
        .map_err(|e| CoreError::InvalidInput(format!("OptiFine check failed: {e}")))??;
    Ok(found)
}

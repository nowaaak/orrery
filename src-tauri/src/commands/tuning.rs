use orrery_core::tuning::{self, SettingsRecommendation};
use tauri::State;

use crate::error::CommandResult;
use crate::state::AppState;

#[tauri::command]
pub fn system_memory() -> Option<u32> {
    tuning::system_memory_mb()
}

#[tauri::command]
pub async fn recommend_settings(
    state: State<'_, AppState>,
    instance_id: String,
) -> CommandResult<SettingsRecommendation> {
    Ok(state.recommend_settings(&instance_id).await?)
}

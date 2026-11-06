use orrery_core::tasks::{TaskId, TaskView};
use tauri::State;

use crate::error::CommandResult;
use crate::state::AppState;

#[tauri::command]
pub fn list_tasks(state: State<'_, AppState>) -> Vec<TaskView> {
    state.tasks.list()
}

#[tauri::command]
pub fn cancel_task(state: State<'_, AppState>, id: TaskId) -> CommandResult<()> {
    Ok(state.tasks.cancel(id)?)
}

#[tauri::command]
pub fn dismiss_task(state: State<'_, AppState>, id: TaskId) {
    state.tasks.dismiss(id);
}

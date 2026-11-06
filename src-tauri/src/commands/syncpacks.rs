use orrery_core::instances::mods::ContentKind;
use orrery_core::modrinth::models::ModSearchPage;
use orrery_core::syncpacks::{SyncItem, SyncPack, SyncReport, SyncSubscription};
use tauri::State;

use crate::error::CommandResult;
use crate::state::AppState;

#[tauri::command]
pub async fn list_sync_packs(state: State<'_, AppState>) -> CommandResult<Vec<SyncPack>> {
    Ok(state.list_sync_packs().await?)
}

#[tauri::command]
pub async fn create_sync_pack(state: State<'_, AppState>, name: String) -> CommandResult<SyncPack> {
    Ok(state.create_sync_pack(&name).await?)
}

#[tauri::command]
pub async fn rename_sync_pack(
    state: State<'_, AppState>,
    id: String,
    name: String,
) -> CommandResult<SyncPack> {
    Ok(state.rename_sync_pack(&id, &name).await?)
}

#[tauri::command]
pub async fn delete_sync_pack(
    state: State<'_, AppState>,
    id: String,
) -> CommandResult<Vec<SyncReport>> {
    Ok(state.delete_sync_pack(&id).await?)
}

#[tauri::command]
pub async fn add_sync_item(
    state: State<'_, AppState>,
    id: String,
    item: SyncItem,
) -> CommandResult<Vec<SyncReport>> {
    Ok(state.add_sync_item(&id, item).await?)
}

#[tauri::command]
pub async fn remove_sync_item(
    state: State<'_, AppState>,
    id: String,
    project_id: String,
) -> CommandResult<Vec<SyncReport>> {
    Ok(state.remove_sync_item(&id, &project_id).await?)
}

#[tauri::command]
pub async fn set_instance_sync_packs(
    state: State<'_, AppState>,
    instance_id: String,
    subscriptions: Vec<SyncSubscription>,
) -> CommandResult<SyncReport> {
    Ok(state
        .set_instance_sync_packs(&instance_id, subscriptions)
        .await?)
}

#[tauri::command]
pub async fn sync_instance(
    state: State<'_, AppState>,
    instance_id: String,
) -> CommandResult<SyncReport> {
    Ok(state.sync_instance(&instance_id).await?)
}

#[tauri::command]
pub async fn search_sync_projects(
    state: State<'_, AppState>,
    kind: ContentKind,
    query: String,
    offset: u32,
) -> CommandResult<ModSearchPage> {
    Ok(state.search_sync_projects(kind, &query, offset).await?)
}

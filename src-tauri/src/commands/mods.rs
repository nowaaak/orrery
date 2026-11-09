use orrery_core::extras::{ModUpdate, ModUpdateRequest};
use orrery_core::instances::mods::{ContentKind, ContentSource, InstalledMod};
use orrery_core::modrinth::models::{ModSearchPage, ModVersion};
use orrery_core::packs::{ContentPack, OptifineReport, PackInstallResult};
use orrery_core::tasks::TaskId;
use std::path::PathBuf;
use tauri::State;

use crate::error::CommandResult;
use crate::state::AppState;

#[tauri::command]
pub async fn search_mods(
    state: State<'_, AppState>,
    instance_id: String,
    source: Option<ContentSource>,
    kind: ContentKind,
    query: String,
    offset: u32,
) -> CommandResult<ModSearchPage> {
    Ok(state
        .search_content(
            &instance_id,
            source.unwrap_or_default(),
            kind,
            &query,
            offset,
        )
        .await?)
}

#[tauri::command]
pub fn curseforge_available(state: State<'_, AppState>) -> bool {
    state.curseforge_available()
}

#[tauri::command]
pub async fn get_mod_versions(
    state: State<'_, AppState>,
    instance_id: String,
    kind: ContentKind,
    project_id: String,
) -> CommandResult<Vec<ModVersion>> {
    Ok(state
        .content_versions(&instance_id, kind, &project_id)
        .await?)
}

#[tauri::command]
pub async fn install_mod(
    state: State<'_, AppState>,
    instance_id: String,
    kind: ContentKind,
    project_id: String,
    version_id: Option<String>,
    title: String,
) -> CommandResult<TaskId> {
    Ok(state
        .spawn_install_content(&instance_id, kind, project_id, version_id, title)
        .await?)
}

#[tauri::command]
pub async fn list_instance_mods(
    state: State<'_, AppState>,
    instance_id: String,
) -> CommandResult<Vec<InstalledMod>> {
    Ok(state.list_mods(&instance_id).await?)
}

#[tauri::command]
pub async fn remove_mod(
    state: State<'_, AppState>,
    instance_id: String,
    project_id: String,
) -> CommandResult<()> {
    Ok(state.remove_mod(&instance_id, &project_id).await?)
}

#[tauri::command]
pub async fn set_mod_pinned(
    state: State<'_, AppState>,
    instance_id: String,
    project_id: String,
    pinned: bool,
) -> CommandResult<InstalledMod> {
    Ok(state
        .set_mod_pinned(&instance_id, &project_id, pinned)
        .await?)
}

#[tauri::command]
pub async fn set_mod_enabled(
    state: State<'_, AppState>,
    instance_id: String,
    project_id: String,
    enabled: bool,
) -> CommandResult<InstalledMod> {
    Ok(state
        .set_mod_enabled(&instance_id, &project_id, enabled)
        .await?)
}

#[tauri::command]
pub async fn check_mod_updates(
    state: State<'_, AppState>,
    instance_id: String,
) -> CommandResult<Vec<ModUpdate>> {
    Ok(state.check_mod_updates(&instance_id).await?)
}

#[tauri::command]
pub async fn update_mods(
    state: State<'_, AppState>,
    instance_id: String,
    updates: Vec<ModUpdateRequest>,
) -> CommandResult<TaskId> {
    Ok(state.spawn_update_mods(&instance_id, updates).await?)
}

#[tauri::command]
pub async fn list_content_packs(
    state: State<'_, AppState>,
    instance_id: String,
) -> CommandResult<Vec<ContentPack>> {
    Ok(state.content_packs(&instance_id).await?)
}

#[tauri::command]
pub async fn install_content_pack(
    state: State<'_, AppState>,
    instance_id: String,
    pack_id: String,
    only: Option<Vec<String>>,
) -> CommandResult<PackInstallResult> {
    Ok(state
        .install_content_pack(&instance_id, &pack_id, only)
        .await?)
}

#[tauri::command]
pub async fn remove_content_pack(
    state: State<'_, AppState>,
    instance_id: String,
    pack_id: String,
) -> CommandResult<u32> {
    Ok(state.remove_content_pack(&instance_id, &pack_id).await?)
}

#[tauri::command]
pub async fn scan_optifine_packs(
    state: State<'_, AppState>,
    instance_id: String,
) -> CommandResult<Vec<OptifineReport>> {
    Ok(state.scan_optifine_packs(&instance_id).await?)
}

#[tauri::command]
pub async fn install_orrery_menu(
    state: State<'_, AppState>,
    instance_id: String,
) -> CommandResult<TaskId> {
    Ok(state.install_orrery_menu(&instance_id).await?)
}

#[tauri::command]
pub async fn install_local_file(
    state: State<'_, AppState>,
    instance_id: String,
    path: PathBuf,
) -> CommandResult<InstalledMod> {
    Ok(state.install_local_file(&instance_id, path).await?)
}

use orrery_core::CoreError;
use orrery_core::extras::InstanceFolder;
use orrery_core::instances::{Instance, InstanceUpdate, LoaderSpec, NewInstance};
use orrery_core::loaders;
use orrery_core::trash::TrashEntry;
use std::path::PathBuf;
use tauri::ipc::Response;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::error::{CommandError, CommandResult};
use crate::state::AppState;

async fn ensure_installable(
    state: &AppState,
    minecraft_version: &str,
    loader: &LoaderSpec,
) -> CommandResult<()> {
    let manifest = state.meta.manifest(false).await?;
    if !manifest.is_supported(minecraft_version) {
        return Err(CoreError::InvalidInput(format!(
            "Minecraft {minecraft_version} is not supported"
        ))
        .into());
    }
    match loader.mod_loader() {
        Some((mod_loader, loader_version)) if mod_loader.uses_installer() => {
            let versions =
                loaders::loader_versions(&state.http, mod_loader, minecraft_version).await?;
            if !versions.iter().any(|v| v.version == loader_version) {
                return Err(CoreError::NotFound(format!(
                    "{} {loader_version} for Minecraft {minecraft_version}",
                    mod_loader.display_name()
                ))
                .into());
            }
        }
        Some((mod_loader, loader_version)) => {
            loaders::ensure_profile(
                &state.http,
                &state.meta,
                mod_loader,
                minecraft_version,
                loader_version,
            )
            .await?;
        }
        None => {}
    }
    Ok(())
}

fn ensure_not_running(state: &AppState, id: &str) -> CommandResult<()> {
    if state.games.is_running(id) {
        return Err(
            CoreError::InvalidInput("stop the game before changing this instance".into()).into(),
        );
    }
    Ok(())
}

#[tauri::command]
pub async fn list_instances(state: State<'_, AppState>) -> CommandResult<Vec<Instance>> {
    Ok(state.instances.list().await?)
}

#[tauri::command]
pub async fn get_instance(state: State<'_, AppState>, id: String) -> CommandResult<Instance> {
    Ok(state.instances.get(&id).await?)
}

#[tauri::command]
pub async fn create_instance(
    state: State<'_, AppState>,
    instance: NewInstance,
) -> CommandResult<Instance> {
    ensure_installable(&state, &instance.minecraft_version, &instance.loader).await?;
    Ok(state.instances.create(instance).await?)
}

#[tauri::command]
pub async fn update_instance(
    state: State<'_, AppState>,
    id: String,
    update: InstanceUpdate,
) -> CommandResult<Instance> {
    ensure_not_running(&state, &id)?;
    ensure_installable(&state, &update.minecraft_version, &update.loader).await?;
    Ok(state.instances.update(&id, update).await?)
}

#[tauri::command]
pub async fn duplicate_instance(
    state: State<'_, AppState>,
    id: String,
    name: String,
) -> CommandResult<Instance> {
    ensure_not_running(&state, &id)?;
    Ok(state.instances.duplicate(&id, &name).await?)
}

#[tauri::command]
pub async fn delete_instance(state: State<'_, AppState>, id: String) -> CommandResult<TrashEntry> {
    ensure_not_running(&state, &id)?;
    Ok(state.trash_instance(&id).await?)
}

#[tauri::command]
pub async fn open_instance_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    folder: Option<InstanceFolder>,
) -> CommandResult<()> {
    let dir = state
        .instance_folder(&id, folder.unwrap_or(InstanceFolder::Root))
        .await?;
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|err| CommandError {
            kind: "io".into(),
            message: err.to_string(),
        })
}

#[tauri::command]
pub async fn set_instance_group(
    state: State<'_, AppState>,
    id: String,
    group: Option<String>,
) -> CommandResult<Instance> {
    Ok(state.instances.set_group(&id, group).await?)
}

#[tauri::command]
pub async fn set_instance_pinned(
    state: State<'_, AppState>,
    id: String,
    pinned: bool,
) -> CommandResult<Instance> {
    Ok(state.instances.set_pinned(&id, pinned).await?)
}

#[tauri::command]
pub async fn set_instance_icon(
    state: State<'_, AppState>,
    id: String,
    path: Option<PathBuf>,
) -> CommandResult<Instance> {
    let image = match path {
        Some(path) => Some(
            tokio::fs::read(&path)
                .await
                .map_err(|source| CoreError::Io { path, source })?,
        ),
        None => None,
    };
    Ok(state.instances.set_icon(&id, image).await?)
}

#[tauri::command]
pub async fn get_instance_icon(state: State<'_, AppState>, id: String) -> CommandResult<Response> {
    Ok(Response::new(
        state.instances.icon(&id).await?.unwrap_or_default(),
    ))
}

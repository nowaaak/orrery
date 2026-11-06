use orrery_core::links::{self, DeepLink, ShortcutFormat};
use tauri::{AppHandle, Manager, State};

use crate::error::{CommandError, CommandResult};
use crate::links::{LinkState, scheme};
use crate::state::AppState;

#[tauri::command]
pub fn take_deep_links(links: State<'_, LinkState>) -> Vec<DeepLink> {
    links.take()
}

#[tauri::command]
pub async fn create_shortcut(
    app: AppHandle,
    state: State<'_, AppState>,
    links: State<'_, LinkState>,
    instance_id: String,
    world: Option<String>,
    server: Option<String>,
) -> CommandResult<String> {
    let instance = state.instances.get(&instance_id).await?;
    let url = links::launch_url(
        scheme(),
        &links.key,
        &instance.id,
        world.as_deref(),
        server.as_deref(),
    )?;
    let name = match (&world, &server) {
        (Some(world), _) => format!("{} - {world}", instance.name),
        (_, Some(server)) => format!("{} - {server}", instance.name),
        _ => instance.name.clone(),
    };
    let io_error = |err: &dyn std::fmt::Display| CommandError {
        kind: "io".into(),
        message: err.to_string(),
    };
    let desktop = app.path().desktop_dir().map_err(|err| io_error(&err))?;
    let icon = std::env::current_exe().map_err(|err| io_error(&err))?;
    let format = if cfg!(windows) {
        ShortcutFormat::WindowsUrl
    } else if cfg!(target_os = "macos") {
        ShortcutFormat::MacWebloc
    } else {
        ShortcutFormat::LinuxDesktop
    };
    let path = links::create_shortcut(&desktop, format, &name, &url, &icon).await?;
    Ok(path.to_string_lossy().into_owned())
}

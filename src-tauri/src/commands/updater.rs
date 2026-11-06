use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State, Url};
use tauri_plugin_updater::{Update, UpdaterExt};
use ts_rs::TS;

use crate::channel::{self, AppInfo, Channel};
use crate::error::{CommandError, CommandResult};
use crate::events;
use crate::state::AppState;

#[derive(Default)]
pub struct PendingUpdate(pub Mutex<Option<Update>>);

#[derive(Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateInfo {
    pub version: String,
    pub current_version: String,
    pub notes: Option<String>,
}

#[derive(Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
}

fn update_error(err: impl std::fmt::Display) -> CommandError {
    CommandError {
        kind: "update".into(),
        message: err.to_string(),
    }
}

fn endpoint() -> Option<Result<Url, CommandError>> {
    crate::REPOSITORY_URL.map(|repo| {
        format!(
            "{}/releases/latest/download/latest.json",
            repo.trim_end_matches('/')
        )
        .parse::<Url>()
        .map_err(update_error)
    })
}

#[tauri::command]
pub fn get_app_info(app: AppHandle) -> AppInfo {
    AppInfo {
        version: app.package_info().version.to_string(),
        channel: channel::current(),
    }
}

#[tauri::command]
pub async fn check_for_update(
    app: AppHandle,
    pending: State<'_, PendingUpdate>,
) -> CommandResult<Option<UpdateInfo>> {
    if channel::current() == Channel::Dev {
        return Ok(None);
    }
    let Some(endpoint) = endpoint() else {
        return Ok(None);
    };
    let updater = app
        .updater_builder()
        .endpoints(vec![endpoint?])
        .map_err(update_error)?
        .build()
        .map_err(update_error)?;
    let update = updater.check().await.map_err(update_error)?;
    let info = update.as_ref().map(|update| UpdateInfo {
        version: update.version.clone(),
        current_version: update.current_version.clone(),
        notes: update.body.clone(),
    });
    *pending.0.lock().unwrap_or_else(|e| e.into_inner()) = update;
    Ok(info)
}

#[tauri::command]
pub async fn install_update(
    app: AppHandle,
    pending: State<'_, PendingUpdate>,
    state: State<'_, AppState>,
) -> CommandResult<()> {
    if channel::current() == Channel::Dev {
        return Err(update_error(
            "development builds never install updates, use the installed Orrery",
        ));
    }
    if !state.games.running().is_empty() {
        return Err(update_error(
            "close Minecraft before installing the launcher update",
        ));
    }
    let update = pending
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
        .ok_or_else(|| update_error("no update is pending, check for updates first"))?;

    let mut downloaded: u64 = 0;
    let progress_app = app.clone();
    update
        .download_and_install(
            move |chunk, total| {
                downloaded += chunk as u64;
                let _ = progress_app.emit(
                    events::UPDATE_PROGRESS,
                    UpdateProgress { downloaded, total },
                );
            },
            || tracing::info!("launcher update downloaded"),
        )
        .await
        .map_err(update_error)?;
    app.restart();
}

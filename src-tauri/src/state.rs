use std::sync::Arc;

use orrery_core::auth::microsoft::{LoginEvent, LoginListener};
use orrery_core::bridge::{BridgeEvent, BridgeListener};
use orrery_core::minecraft::process::{GameListener, GameLogEvent, GameState, GameStateEvent};
use orrery_core::settings::LauncherWindow;
use orrery_core::tasks::TaskView;
use tauri::{AppHandle, Emitter, Manager};

use crate::events;

pub use orrery_core::services::Services as AppState;

const MAIN_WINDOW: &str = "main";

pub fn task_listener(app: AppHandle) -> Arc<dyn Fn(TaskView) + Send + Sync> {
    Arc::new(move |view| emit(&app, events::TASK_UPDATED, view))
}

pub fn login_listener(app: AppHandle) -> LoginListener {
    Arc::new(move |event: LoginEvent| emit(&app, events::AUTH_LOGIN, event))
}

pub fn bridge_listener(app: AppHandle) -> BridgeListener {
    Arc::new(move |event: BridgeEvent| emit(&app, events::BRIDGE_EVENT, event))
}

pub struct TauriGameListener {
    pub app: AppHandle,
}

impl GameListener for TauriGameListener {
    fn state(&self, event: GameStateEvent) {
        let started = matches!(event.state, GameState::Running { .. });
        emit(&self.app, events::GAME_STATE, event);
        let app = self.app.clone();
        tauri::async_runtime::spawn(async move { apply_window_behaviour(&app, started).await });
    }

    fn log(&self, event: GameLogEvent) {
        emit(&self.app, events::GAME_LOG, event);
    }
}

async fn apply_window_behaviour(app: &AppHandle, started: bool) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };
    let behaviour = state.settings.get().await.launcher_window;
    let result = match (behaviour, started) {
        (LauncherWindow::KeepOpen, _) => Ok(()),
        (LauncherWindow::Minimize, true) => window.minimize(),
        (LauncherWindow::Hide, true) => window.hide(),
        (_, false) if state.games.running().is_empty() => window
            .show()
            .and_then(|()| window.unminimize())
            .and_then(|()| window.set_focus()),
        (_, false) => Ok(()),
    };
    if let Err(err) = result {
        tracing::warn!(error = %err, "failed to change the launcher window");
    }
}

pub fn emit<T: serde::Serialize + Clone>(app: &AppHandle, event: &str, payload: T) {
    if let Err(err) = app.emit(event, payload) {
        tracing::warn!(event, error = %err, "failed to emit event");
    }
}

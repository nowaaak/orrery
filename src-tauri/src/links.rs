use std::path::Path;
use std::sync::Mutex;

use orrery_core::links::{self, DeepLink};
use tauri::{AppHandle, Manager};
use tauri_plugin_deep_link::DeepLinkExt;

use crate::channel::{self, Channel};
use crate::events;
use crate::state::emit;

const MAIN_WINDOW: &str = "main";

pub struct LinkState {
    pub key: String,
    pending: Mutex<Vec<DeepLink>>,
}

impl LinkState {
    pub fn take(&self) -> Vec<DeepLink> {
        std::mem::take(&mut *self.pending.lock().unwrap_or_else(|e| e.into_inner()))
    }
}

pub fn scheme() -> &'static str {
    match channel::current() {
        Channel::Dev => "orrery-dev",
        Channel::Release => "orrery",
    }
}

pub fn focus_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let result = window
            .show()
            .and_then(|()| window.unminimize())
            .and_then(|()| window.set_focus());
        if let Err(err) = result {
            tracing::warn!(error = %err, "failed to bring the launcher to the front");
        }
    }
}

fn handle_urls(app: &AppHandle, urls: Vec<String>) {
    let Some(state) = app.try_state::<LinkState>() else {
        return;
    };
    let mut received = false;
    for url in urls {
        match links::parse(&url, scheme(), &state.key) {
            Ok(link) => {
                tracing::info!(action = ?link.action, trusted = link.trusted, "received link");
                state
                    .pending
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .push(link);
                received = true;
            }
            Err(err) => tracing::warn!(error = %err, "ignored link"),
        }
    }
    if received {
        focus_main(app);
        emit(app, events::DEEP_LINK, ());
    }
}

pub fn setup(app: &AppHandle, config_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let key = tauri::async_runtime::block_on(links::load_or_create_key(config_dir))?;
    app.manage(LinkState {
        key,
        pending: Mutex::new(Vec::new()),
    });
    #[cfg(any(windows, target_os = "linux"))]
    if let Err(err) = app.deep_link().register_all() {
        tracing::warn!(error = %err, "failed to register the link scheme");
    }
    let handle = app.clone();
    app.deep_link().on_open_url(move |event| {
        handle_urls(
            &handle,
            event.urls().iter().map(ToString::to_string).collect(),
        );
    });
    match app.deep_link().get_current() {
        Ok(Some(urls)) => handle_urls(app, urls.iter().map(ToString::to_string).collect()),
        Ok(None) => {}
        Err(err) => tracing::warn!(error = %err, "failed to read the start link"),
    }
    Ok(())
}

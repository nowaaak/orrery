mod channel;
mod commands;
mod error;
mod events;
mod links;
mod state;

use std::sync::Arc;
use std::time::Duration;

use std::path::Path;

use orrery_core::data_dir::{self, DataDirIssue};
use orrery_core::paths::LauncherPaths;
use orrery_core::services::{LAUNCHER_NAME, ServiceConfig};
use orrery_core::settings::{Language, SettingsStore};
use serde::Serialize;
use tauri::Manager;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use ts_rs::TS;

use crate::state::{AppState, TauriGameListener, bridge_listener, login_listener, task_listener};

const TASK_TICK: Duration = Duration::from_millis(100);
pub(crate) const REPOSITORY_URL: Option<&str> = option_env!("LAUNCHER_REPOSITORY_URL");
const MS_CLIENT_ID: Option<&str> = option_env!("MS_CLIENT_ID");
const DISCORD_CLIENT_ID: Option<&str> = option_env!("DISCORD_CLIENT_ID");
const CURSEFORGE_API_KEY: Option<&str> = option_env!("CURSEFORGE_API_KEY");

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            links::focus_main(app)
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(commands::updater::PendingUpdate::default())
        .setup(|app| {
            let handle = app.handle().clone();
            channel::check_identity(&handle.config().identifier, channel::current())?;
            let resolver = handle.path();
            let config = resolver.app_local_data_dir()?;
            let logs = resolver.app_log_dir()?;
            app.manage(init_logging(&logs)?);
            let settings = tauri::async_runtime::block_on(async {
                let store = SettingsStore::load(LauncherPaths::settings_path(&config)).await?;
                Ok::<_, orrery_core::CoreError>(store.get().await)
            })?;
            let paths = match data_dir::resolve(config, logs, settings.data_dir.as_deref()) {
                Ok(paths) => paths,
                Err(issue) => {
                    tracing::error!(path = %issue.path, problem = ?issue.problem, "data folder is not available");
                    app.manage(StartupState(Some(StartupIssue {
                        issue,
                        language: settings.language,
                    })));
                    return Ok(());
                }
            };
            app.manage(StartupState(None));
            tracing::info!(root = %paths.root.display(), "starting launcher");
            links::setup(&handle, &paths.config)?;

            let version = handle.package_info().version.to_string();
            let config = ServiceConfig {
                user_agent: match REPOSITORY_URL {
                    Some(url) => format!("{LAUNCHER_NAME}/{version} (+{url})"),
                    None => format!("{LAUNCHER_NAME}/{version}"),
                },
                launcher_version: version,
                keyring_service: handle.config().identifier.clone(),
                microsoft_client_id: MS_CLIENT_ID.map(str::to_string),
                companion_dir: resolver
                    .resource_dir()
                    .ok()
                    .map(|dir| dir.join("resources").join("companion")),
                discord_client_id: DISCORD_CLIENT_ID
                    .filter(|id| !id.trim().is_empty())
                    .map(str::to_string),
                curseforge_api_key: CURSEFORGE_API_KEY.map(str::to_string),
            };
            let state = tauri::async_runtime::block_on(AppState::init(
                paths,
                config,
                task_listener(handle.clone()),
                Arc::new(TauriGameListener {
                    app: handle.clone(),
                }),
                login_listener(handle.clone()),
                bridge_listener(handle.clone()),
            ))?;
            tauri::async_runtime::spawn(state.tasks.clone().run_ticker(TASK_TICK));
            let purge = state.clone();
            tauri::async_runtime::spawn(async move {
                match purge.purge_trash().await {
                    Ok(0) => {}
                    Ok(removed) => tracing::info!(removed, "emptied old trash entries"),
                    Err(err) => tracing::warn!(error = %err, "could not empty old trash entries"),
                }
            });
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::settings::get_paths,
            commands::settings::discord_available,
            commands::settings::get_startup_issue,
            commands::settings::inspect_data_dir,
            commands::settings::change_data_dir,
            commands::settings::reset_data_dir,
            commands::settings::restart_app,
            commands::updater::get_app_info,
            commands::tasks::list_tasks,
            commands::tasks::cancel_task,
            commands::tasks::dismiss_task,
            commands::versions::list_minecraft_versions,
            commands::versions::list_loader_versions,
            commands::instances::list_instances,
            commands::instances::get_instance,
            commands::instances::create_instance,
            commands::instances::update_instance,
            commands::instances::duplicate_instance,
            commands::instances::delete_instance,
            commands::instances::open_instance_folder,
            commands::instances::set_instance_group,
            commands::instances::set_instance_pinned,
            commands::instances::set_instance_icon,
            commands::instances::get_instance_icon,
            commands::launch::install_instance,
            commands::launch::repair_instance,
            commands::launch::launch_instance,
            commands::launch::kill_instance,
            commands::launch::list_running,
            commands::launch::get_game_log,
            commands::launch::send_game_toast,
            commands::accounts::list_accounts,
            commands::accounts::remove_account,
            commands::accounts::set_active_account,
            commands::skins::list_skin_library,
            commands::skins::get_library_skin,
            commands::skins::import_skin_file,
            commands::skins::import_player_skin,
            commands::skins::update_library_skin,
            commands::skins::remove_library_skin,
            commands::skins::apply_library_skin,
            commands::skins::get_account_profile,
            commands::skins::set_account_cape,
            commands::skins::fetch_texture,
            commands::imports::scan_imports,
            commands::imports::inspect_import,
            commands::imports::preview_import,
            commands::imports::start_import,
            commands::imports::inspect_optifine,
            commands::auth::microsoft_login_available,
            commands::auth::begin_microsoft_login,
            commands::auth::cancel_microsoft_login,
            commands::updater::check_for_update,
            commands::updater::install_update,
            commands::mods::search_mods,
            commands::mods::curseforge_available,
            commands::mods::get_mod_versions,
            commands::mods::install_mod,
            commands::mods::list_instance_mods,
            commands::mods::remove_mod,
            commands::mods::set_mod_enabled,
            commands::mods::set_mod_pinned,
            commands::mods::check_mod_updates,
            commands::mods::update_mods,
            commands::mods::list_content_packs,
            commands::mods::install_content_pack,
            commands::mods::remove_content_pack,
            commands::mods::scan_optifine_packs,
            commands::mods::install_local_file,
            commands::mods::install_orrery_shaders,
            commands::mods::install_orrery_menu,
            commands::modpacks::search_modpacks,
            commands::modpacks::is_modpack,
            commands::modpacks::check_pending_downloads,
            commands::modpacks::provide_pending_download,
            commands::modpacks::skip_pending_download,
            commands::modpacks::install_modpack,
            commands::modpacks::import_modpack,
            commands::modpacks::export_modpack,
            commands::worlds::list_worlds,
            commands::datapacks::list_datapacks,
            commands::datapacks::add_datapack,
            commands::datapacks::remove_datapack,
            commands::datapacks::open_datapacks_folder,
            commands::screenshots::list_screenshots,
            commands::screenshots::read_screenshot,
            commands::screenshots::delete_screenshot,
            commands::servers::list_servers,
            commands::servers::change_servers,
            commands::servers::ping_server,
            commands::worlds::backup_world,
            commands::worlds::restore_world_backup,
            commands::worlds::delete_world,
            commands::worlds::delete_world_backup,
            commands::storage::storage_report,
            commands::storage::clean_storage,
            commands::syncpacks::list_sync_packs,
            commands::syncpacks::create_sync_pack,
            commands::syncpacks::rename_sync_pack,
            commands::syncpacks::delete_sync_pack,
            commands::syncpacks::add_sync_item,
            commands::syncpacks::remove_sync_item,
            commands::syncpacks::set_instance_sync_packs,
            commands::syncpacks::sync_instance,
            commands::syncpacks::search_sync_projects,
            commands::trash::list_trash,
            commands::trash::restore_trash,
            commands::trash::delete_trash_entry,
            commands::trash::empty_trash,
            commands::launch::export_game_log,
            commands::launch::list_java_installations,
            commands::links::take_deep_links,
            commands::links::create_shortcut,
            commands::logs::prepare_log_share,
            commands::logs::share_log,
            commands::logs::read_launcher_log,
            commands::tuning::system_memory,
            commands::tuning::recommend_settings,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|err| {
            eprintln!("failed to run launcher: {err}");
            std::process::exit(1);
        });
}

#[derive(Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StartupIssue {
    pub issue: DataDirIssue,
    pub language: Language,
}

pub(crate) struct StartupState(pub Option<StartupIssue>);

fn init_logging(logs: &Path) -> Result<WorkerGuard, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(logs)?;
    let file_appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("launcher")
        .filename_suffix("log")
        .max_log_files(7)
        .build(logs)?;
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,orrery_lib=debug,orrery_core=debug"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stdout))
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(file_writer),
        )
        .try_init()?;
    Ok(guard)
}

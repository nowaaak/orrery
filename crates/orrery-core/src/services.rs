use std::path::PathBuf;
use std::sync::Arc;

use crate::auth::microsoft::{Endpoints, LoginListener, MicrosoftAuth};
use crate::auth::secrets::KeyringStore;
use crate::auth::{Account, AccountStore};
use crate::bridge::{BridgeEvent, BridgeHub, BridgeListener, companion_jar, java_path};
use crate::curseforge::CurseForgeClient;
use crate::discord::{GameLocation, GamePresence, Presence};
use crate::instances::{Instance, InstanceStore};
use crate::loaders::{self, ModLoader, installer::LoaderInstall};
use crate::minecraft::arguments::{LaunchCredentials, LaunchSpec, Resolution, build_arguments};
use crate::minecraft::crash;
use crate::minecraft::hooks::{
    HookContext, HookPaths, HookStage, run_hook, split_command, wrap_command,
};
use crate::minecraft::install::{InstalledVersion, Installer};
use crate::minecraft::manifest::MetaStore;
use crate::minecraft::process::{ExitHook, ExitInfo, GameCommand, GameListener, GameRegistry};
use crate::modrinth::ModrinthClient;
use crate::net::{Http, VerifyMode};
use crate::paths::LauncherPaths;
use crate::platform::{Os, Platform};
use crate::progress::{Progress, Stage};
use crate::redact::redact_args;
use crate::servers::ServerAddress;
use crate::servers::playtime::ServerPlaytime;
use crate::settings::SettingsStore;
use crate::skins::SkinLibrary;
use crate::syncpacks::{SYNC_FILE, SyncPackStore};
use crate::tasks::{TaskId, TaskKind, TaskListener, TaskRegistry, TaskSpec};
use crate::trash::Trash;
use crate::{CoreError, CoreResult};
use crate::{game_options, gpu, worlds};

pub const LAUNCHER_NAME: &str = "orrery";
const PRIME_OFFLOAD_ENV: [(&str, &str); 3] = [
    ("DRI_PRIME", "1"),
    ("__NV_PRIME_RENDER_OFFLOAD", "1"),
    ("__GLX_VENDOR_LIBRARY_NAME", "nvidia"),
];

pub struct ServiceConfig {
    pub launcher_version: String,
    pub user_agent: String,
    pub keyring_service: String,
    pub microsoft_client_id: Option<String>,
    pub discord_client_id: Option<String>,
    pub curseforge_api_key: Option<String>,
    pub companion_dir: Option<PathBuf>,
}

#[derive(Clone)]
pub struct Services {
    pub paths: LauncherPaths,
    pub http: Http,
    pub platform: Platform,
    pub launcher_version: String,
    pub settings: Arc<SettingsStore>,
    pub meta: Arc<MetaStore>,
    pub instances: Arc<InstanceStore>,
    pub accounts: Arc<AccountStore>,
    pub microsoft: Arc<MicrosoftAuth>,
    pub games: Arc<GameRegistry>,
    pub tasks: Arc<TaskRegistry>,
    pub modrinth: ModrinthClient,
    pub curseforge: CurseForgeClient,
    pub mods_lock: Arc<tokio::sync::Mutex<()>>,
    pub presence: Option<Presence>,
    pub skin_library: Arc<SkinLibrary>,
    pub bridge: Arc<BridgeHub>,
    pub server_playtime: Arc<ServerPlaytime>,
    pub companion_dir: Option<PathBuf>,
    pub trash: Arc<Trash>,
    pub sync_packs: Arc<SyncPackStore>,
}

impl Services {
    pub async fn init(
        paths: LauncherPaths,
        config: ServiceConfig,
        task_listener: TaskListener,
        game_listener: Arc<dyn GameListener>,
        login_listener: LoginListener,
        bridge_listener: BridgeListener,
    ) -> CoreResult<Self> {
        paths.ensure_base_dirs().await?;
        let http = Http::new(&config.user_agent)?;
        let settings = SettingsStore::load(paths.settings_file()).await?;
        let accounts = Arc::new(AccountStore::load(paths.accounts_file()).await?);
        let microsoft = MicrosoftAuth::new(
            http.clone(),
            Endpoints::default(),
            config.microsoft_client_id,
            Arc::new(KeyringStore::new(config.keyring_service)),
            accounts.clone(),
            login_listener,
        );
        microsoft.discard_stored_sessions().await;
        let instances = Arc::new(InstanceStore::new(paths.instances_dir()));
        let presence = config.discord_client_id.map(Presence::start);
        let server_playtime = Arc::new(ServerPlaytime::new(instances.clone()));
        let bridge = Arc::new(BridgeHub::new(track_bridge(
            bridge_listener,
            presence.clone(),
            server_playtime.clone(),
        )));
        Ok(Self {
            meta: Arc::new(MetaStore::new(http.clone(), paths.clone())),
            instances,
            settings: Arc::new(settings),
            accounts,
            microsoft,
            games: GameRegistry::new(game_listener),
            tasks: TaskRegistry::new(task_listener),
            modrinth: ModrinthClient::new(http.clone()),
            curseforge: CurseForgeClient::new(http.clone(), config.curseforge_api_key),
            mods_lock: Arc::new(tokio::sync::Mutex::new(())),
            presence,
            skin_library: Arc::new(SkinLibrary::new(paths.skins_dir().join("library"))),
            bridge,
            server_playtime,
            companion_dir: config.companion_dir,
            trash: Arc::new(Trash::new(paths.trash_dir())),
            sync_packs: Arc::new(SyncPackStore::new(paths.meta_dir().join(SYNC_FILE))),
            platform: Platform::current(),
            launcher_version: config.launcher_version,
            http,
            paths,
        })
    }

    pub async fn spawn_install(&self, instance_id: &str, repair: bool) -> CoreResult<TaskId> {
        let instance = self.instances.get(instance_id).await?;
        let services = self.clone();
        let (kind, subject, mode) = if repair {
            (
                TaskKind::RepairInstance,
                instance.name.clone(),
                VerifyMode::Full,
            )
        } else {
            (
                TaskKind::InstallInstance,
                instance.name.clone(),
                VerifyMode::Quick,
            )
        };
        self.tasks.spawn(
            TaskSpec {
                kind,
                subject,
                instance_id: Some(instance.id.clone()),
            },
            move |progress| async move {
                services.install(&instance, mode, &progress).await?;
                progress.set_stage(Stage::Finished);
                Ok(())
            },
        )
    }

    pub async fn spawn_launch(
        &self,
        instance_id: &str,
        account_id: Option<String>,
        world: Option<String>,
        server: Option<String>,
    ) -> CoreResult<TaskId> {
        let instance = self.instances.get(instance_id).await?;
        if self.games.is_running(&instance.id) {
            return Err(CoreError::InvalidInput(format!(
                "{} is already running",
                instance.name
            )));
        }
        if world.is_some() && server.is_some() {
            return Err(CoreError::InvalidInput(
                "a launch can open a world or join a server, not both".into(),
            ));
        }
        if let Some(world) = &world {
            worlds::ensure_world(&self.instances.game_dir(&instance.id)?, world).await?;
        }
        let server = match (server, &world) {
            (Some(server), _) => Some(ServerAddress::parse(&server)?),
            (None, None) => instance
                .launch
                .join_server
                .as_deref()
                .map(ServerAddress::parse)
                .transpose()?,
            (None, Some(_)) => None,
        };
        let services = self.clone();
        self.tasks.spawn(
            TaskSpec {
                kind: TaskKind::LaunchInstance,
                subject: instance.name.clone(),
                instance_id: Some(instance.id.clone()),
            },
            move |progress| async move {
                services
                    .launch(
                        &instance,
                        account_id.as_deref(),
                        world.as_deref(),
                        server.as_ref(),
                        &progress,
                    )
                    .await
            },
        )
    }

    async fn install(
        &self,
        instance: &Instance,
        mode: VerifyMode,
        progress: &Progress,
    ) -> CoreResult<InstalledVersion> {
        let settings = self.settings.get().await;
        let concurrency = settings.max_concurrent_downloads as usize;
        let mod_loader = instance.loader.mod_loader();
        let installer = Installer {
            http: &self.http,
            paths: &self.paths,
            meta: &self.meta,
            concurrency,
            platform: self.platform,
            profile_client_jar: mod_loader.is_some_and(|(loader, _)| loader.uses_installer()),
        };
        let java_override = instance.java.java_path.as_ref().map(PathBuf::from);
        let version_id = match mod_loader {
            None => instance.minecraft_version.clone(),
            Some((loader, loader_version)) if loader.uses_installer() => {
                let vanilla = installer
                    .install(
                        &instance.minecraft_version,
                        java_override.clone(),
                        mode,
                        progress,
                    )
                    .await?;
                LoaderInstall {
                    http: &self.http,
                    paths: &self.paths,
                    meta: &self.meta,
                    java: &vanilla.java,
                    platform: self.platform,
                    concurrency,
                    mode,
                    progress,
                }
                .ensure(loader, &instance.minecraft_version, loader_version)
                .await?
            }
            Some((loader, loader_version)) => {
                progress.set_stage(Stage::FetchingMetadata);
                loaders::ensure_profile(
                    &self.http,
                    &self.meta,
                    loader,
                    &instance.minecraft_version,
                    loader_version,
                )
                .await?
            }
        };
        installer
            .install(&version_id, java_override, mode, progress)
            .await
    }

    async fn launch(
        &self,
        instance: &Instance,
        account_id: Option<&str>,
        world: Option<&str>,
        server: Option<&ServerAddress>,
        progress: &Progress,
    ) -> CoreResult<()> {
        progress.set_stage(Stage::Authenticating);
        let account = self
            .accounts
            .launch_account(instance.launch.account_id.as_deref().or(account_id))
            .await?;
        let credentials = self.credentials(&account).await?;
        progress.check_cancelled()?;
        if let Err(err) = self.apply_sync(instance, progress).await {
            tracing::warn!(instance = %instance.id, error = %err, "could not update sync packs before launch");
        }

        let installed = self.install(instance, VerifyMode::Quick, progress).await?;
        progress.set_stage(Stage::Launching);

        let settings = self.settings.get().await;
        let game_dir = self.instances.game_dir(&instance.id)?;
        tokio::fs::create_dir_all(&game_dir)
            .await
            .map_err(|source| CoreError::Io {
                path: game_dir.clone(),
                source,
            })?;
        if settings.optimize_game_options {
            let client_jar = self.paths.client_jar(&instance.minecraft_version);
            match game_options::write_defaults_if_missing(
                &game_dir,
                &instance.minecraft_version,
                client_jar,
            )
            .await
            {
                Ok(true) => tracing::info!(instance = %instance.id, "wrote default game options"),
                Ok(false) => {}
                Err(err) => tracing::warn!(error = %err, "failed to write default game options"),
            }
        }
        match self.refresh_orrery_menu(instance).await {
            Ok(true) => tracing::info!(instance = %instance.id, "rebuilt orrery menu pack"),
            Ok(false) => {}
            Err(err) => tracing::warn!(error = %err, "failed to rebuild orrery menu pack"),
        }
        if settings.prefer_dedicated_gpu {
            match gpu::prefer_dedicated_gpu(&installed.java).await {
                Ok(true) => {
                    tracing::info!(java = %installed.java.display(), "set high performance gpu preference")
                }
                Ok(false) => {}
                Err(err) => tracing::warn!(error = %err, "failed to set gpu preference"),
            }
        }
        let assets_dir = self.paths.assets_dir();
        let libraries_dir = self.paths.libraries_dir();
        let min_memory_mb = instance
            .java
            .min_memory_mb
            .unwrap_or(settings.default_min_memory_mb);
        let max_memory_mb = instance
            .java
            .max_memory_mb
            .unwrap_or(settings.default_max_memory_mb);
        let companion = self.companion_args(instance).await;
        let mut jvm_args = instance.java.extra_jvm_args.clone();
        let bridge_token = match &companion {
            Some((extra, token)) => {
                jvm_args.extend(extra.iter().cloned());
                Some(token.clone())
            }
            None => None,
        };
        let spec = LaunchSpec {
            version: &installed.version,
            platform: self.platform,
            classpath: &installed.classpath,
            natives_dir: &installed.natives_dir,
            game_dir: &game_dir,
            assets_dir: &assets_dir,
            libraries_dir: &libraries_dir,
            credentials: &credentials,
            min_memory_mb,
            max_memory_mb,
            extra_jvm_args: &jvm_args,
            resolution: instance.resolution.map(|r| Resolution {
                width: r.width,
                height: r.height,
            }),
            logging_argument: installed.logging_argument.clone(),
            launcher_name: LAUNCHER_NAME,
            launcher_version: &self.launcher_version,
            quick_play_world: world,
            server,
            extra_game_args: &instance.launch.extra_game_args,
        };
        let args = build_arguments(&spec)?;
        if world.is_some() && !args.iter().any(|arg| arg == "--quickPlaySingleplayer") {
            return Err(CoreError::Unsupported(
                "starting a world directly needs Minecraft 1.20 or newer".into(),
            ));
        }
        let instance_dir = self.instances.instance_dir(&instance.id)?;
        let mut java_args = vec![
            format!("-Xms{min_memory_mb}M"),
            format!("-Xmx{max_memory_mb}M"),
        ];
        java_args.extend(instance.java.extra_jvm_args.iter().cloned());
        let hooks = HookContext::new(
            &instance.id,
            &instance.name,
            HookPaths {
                instance_dir: &instance_dir,
                game_dir: &game_dir,
                java: &installed.java,
            },
            &java_args,
        );
        let launch = &instance.launch;
        let mut env: Vec<(String, String)> = hooks.env().collect();
        env.extend(
            launch
                .env
                .iter()
                .map(|var| (var.key.clone(), var.value.clone())),
        );
        let mut prefix = Vec::new();
        if self.platform.os == Os::Linux {
            if launch.prime_offload {
                env.extend(PRIME_OFFLOAD_ENV.map(|(k, v)| (k.to_string(), v.to_string())));
            }
            if launch.gamemode {
                prefix.push("gamemoderun".to_string());
            }
            if launch.mangohud {
                prefix.push("mangohud".to_string());
            }
        }
        if let Some(wrapper) = non_empty(&launch.wrapper) {
            prefix.extend(split_command(&hooks.expand(wrapper))?);
        }
        let (program, args) = wrap_command(&installed.java, args, prefix);
        tracing::info!(
            instance = %instance.id,
            program = %program.display(),
            env = ?launch.env.iter().map(|var| &var.key).collect::<Vec<_>>(),
            args = ?redact_args(&redact_args(&args, &credentials.access_token), bridge_token.as_deref().unwrap_or_default()),
            "launching game"
        );
        progress.check_cancelled()?;
        if let Some(command) = non_empty(&launch.pre_launch) {
            run_hook(
                HookStage::PreLaunch,
                &hooks.expand(command),
                &env,
                &game_dir,
                progress.cancel_token(),
            )
            .await?;
            progress.check_cancelled()?;
        }
        let post_exit = non_empty(&launch.post_exit).map(|command| hooks.expand(command));
        let post_exit_env = env.clone();
        let instances = self.instances.clone();
        let exit_instance = instance.id.clone();
        let exit_presence = self.presence.clone();
        let exit_bridge = self.bridge.clone();
        let exit_playtime = self.server_playtime.clone();
        let exit_dir = game_dir.clone();
        let started = std::time::SystemTime::now();
        let on_exit: ExitHook = Box::new(move |exit: ExitInfo| {
            Box::pin(async move {
                if let Err(err) = instances
                    .add_playtime(&exit_instance, exit.played_secs)
                    .await
                {
                    tracing::warn!(error = %err, "failed to store playtime");
                }
                if let Some(presence) = &exit_presence {
                    presence.game_stopped(&exit_instance);
                }
                exit_bridge.close(&exit_instance);
                exit_playtime.finish(&exit_instance).await;
                if let Some(command) = post_exit
                    && let Err(err) = run_hook(
                        HookStage::PostExit,
                        &command,
                        &post_exit_env,
                        &exit_dir,
                        &tokio_util::sync::CancellationToken::new(),
                    )
                    .await
                {
                    tracing::warn!(error = %err, "post-exit command failed");
                }
                if exit.stopped_by_user || exit.code == Some(0) {
                    return None;
                }
                let started = started - std::time::Duration::from_secs(5);
                tokio::task::spawn_blocking(move || {
                    crash::analyze(&exit_dir, started, &exit.log_tail)
                })
                .await
                .ok()
            })
        });
        self.games.spawn(GameCommand {
            instance_id: &instance.id,
            program: &program,
            args,
            env,
            working_dir: &game_dir,
            on_exit: Some(on_exit),
        })?;
        if let Some(presence) = &self.presence {
            presence.game_started(
                &instance.id,
                GamePresence {
                    instance_name: instance.name.clone(),
                    minecraft_version: instance.minecraft_version.clone(),
                    loader: instance
                        .loader
                        .mod_loader()
                        .map(|(loader, _)| loader.display_name().to_string()),
                    world: world.map(str::to_string),
                    server: server.map(ToString::to_string),
                    dimension: None,
                    started_at: crate::storage::unix_now(),
                },
                settings.discord_presence,
            );
        }
        progress.set_stage(Stage::Finished);
        self.instances.mark_played(&instance.id).await
    }

    async fn companion_args(&self, instance: &Instance) -> Option<(Vec<String>, String)> {
        let dir = self.companion_dir.as_ref()?;
        let runs_fabric_mods = instance
            .loader
            .mod_loader()
            .is_some_and(|(loader, _)| loader.runs_fabric_mods());
        if instance.launch.companion_disabled || !runs_fabric_mods {
            return None;
        }
        let jar = match companion_jar(dir, &instance.minecraft_version).await {
            Ok(Some(jar)) => jar,
            Ok(None) => return None,
            Err(err) => {
                tracing::warn!(error = %err, "the bundled companion mod is not usable");
                return None;
            }
        };
        let ticket = match self.bridge.open(&instance.id).await {
            Ok(ticket) => ticket,
            Err(err) => {
                tracing::warn!(error = %err, "could not open the companion bridge");
                return None;
            }
        };
        let property = match instance.loader.mod_loader() {
            Some((ModLoader::Quilt, _)) => "loader.addMods",
            _ => "fabric.addMods",
        };
        let mut args = vec![format!("-D{property}={}", java_path(&jar))];
        args.extend(ticket.jvm_args());
        Some((args, ticket.token))
    }

    pub async fn remove_account(&self, id: &str) -> CoreResult<()> {
        let removed = self.accounts.remove(id).await?;
        if let Err(err) = self.microsoft.forget(&removed.id) {
            tracing::warn!(error = %err, "failed to delete stored Microsoft login");
        }
        Ok(())
    }

    async fn credentials(&self, account: &Account) -> CoreResult<LaunchCredentials> {
        self.microsoft.launch_credentials(account).await
    }
}

fn non_empty(value: &Option<String>) -> Option<&str> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn track_bridge(
    listener: BridgeListener,
    presence: Option<Presence>,
    server_playtime: Arc<ServerPlaytime>,
) -> BridgeListener {
    Arc::new(move |event: BridgeEvent| {
        match &event {
            BridgeEvent::Status {
                instance_id,
                status,
            } => {
                if let Some(presence) = &presence {
                    presence.game_moved(
                        instance_id,
                        GameLocation {
                            world: status.world.clone(),
                            server: status.server.clone(),
                            dimension: status.dimension.clone(),
                        },
                    );
                }
                server_playtime.track(instance_id, status.server.as_deref());
            }
            BridgeEvent::Disconnected { instance_id } => {
                server_playtime.track(instance_id, None);
            }
            BridgeEvent::Connected { .. } | BridgeEvent::Screenshot { .. } => {}
        }
        listener(event);
    })
}

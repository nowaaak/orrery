use std::path::{Path, PathBuf};
use std::time::Instant;

use orrery_core::cleanup;
use orrery_core::game_options::{self, HardwareTier};
use orrery_core::gpu;
use orrery_core::instances::Instance;
use orrery_core::instances::mods::{ContentKind, ModsDir};
use orrery_core::loaders::installer::{self, LoaderInstall};
use orrery_core::loaders::{self, ModLoader};
use orrery_core::menupack;
use orrery_core::minecraft::install::Installer;
use orrery_core::minecraft::manifest::MetaStore;
use orrery_core::modpacks;
use orrery_core::modrinth::ModrinthClient;
use orrery_core::modrinth::install::{ModTarget, install};
use orrery_core::net::{Http, VerifyMode};
use orrery_core::optifine;
use orrery_core::paths::LauncherPaths;
use orrery_core::platform::Platform;
use orrery_core::progress::Progress;
use orrery_core::shaderpack::{self, Activation};
use orrery_core::worlds;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(
        args.next()
            .ok_or("usage: smoke <data-dir> <version> [--prepare]")?,
    );
    let game_version = args.next().ok_or("missing version")?;
    let flags: Vec<String> = args.collect();
    let prepare = flags.iter().any(|f| f == "--prepare");
    let mod_loader = if flags.iter().any(|f| f == "--neoforge") {
        Some(ModLoader::NeoForge)
    } else if flags.iter().any(|f| f == "--forge") {
        Some(ModLoader::Forge)
    } else if flags.iter().any(|f| f == "--legacy-fabric") {
        Some(ModLoader::LegacyFabric)
    } else if flags.iter().any(|f| f == "--quilt") {
        Some(ModLoader::Quilt)
    } else if flags.iter().any(|f| f == "--fabric") {
        Some(ModLoader::Fabric)
    } else {
        None
    };

    let paths = LauncherPaths::new(root.join("data"), root.join("cache"), root.join("logs"));
    paths.ensure_base_dirs().await?;
    let http = Http::new("orrery-smoke/0.1.0")?;
    let meta = MetaStore::new(http.clone(), paths.clone());
    let platform = Platform::current();
    let installer = Installer {
        http: &http,
        paths: &paths,
        meta: &meta,
        concurrency: 16,
        platform,
        profile_client_jar: mod_loader.is_some_and(ModLoader::uses_installer),
    };

    let version = match mod_loader {
        Some(loader) => {
            let versions = loaders::loader_versions(&http, loader, &game_version).await?;
            let pinned = flags
                .iter()
                .find_map(|f| f.strip_prefix("--loader-version="));
            let chosen = versions
                .iter()
                .find(|l| pinned.map_or(l.stable, |pinned| l.version == pinned))
                .ok_or("loader version not found")?;
            println!("using {} loader {}", loader.display_name(), chosen.version);
            if loader.uses_installer() {
                let started = Instant::now();
                let progress = Progress::new();
                let vanilla = installer
                    .install(&game_version, None, VerifyMode::Quick, &progress)
                    .await?;
                let id = LoaderInstall {
                    http: &http,
                    paths: &paths,
                    meta: &meta,
                    java: &vanilla.java,
                    platform,
                    concurrency: 16,
                    mode: VerifyMode::Quick,
                    progress: &progress,
                }
                .ensure(loader, &game_version, &chosen.version)
                .await?;
                println!(
                    "installed {id} in {:.1}s, generated {:?}",
                    started.elapsed().as_secs_f32(),
                    installer::generated_files(&paths, &id)
                );
                id
            } else {
                loaders::ensure_profile(&http, &meta, loader, &game_version, &chosen.version)
                    .await?
            }
        }
        None => game_version.clone(),
    };

    let started = Instant::now();
    let progress = Progress::new();
    let installed = installer
        .install(&version, None, VerifyMode::Quick, &progress)
        .await?;
    let snapshot = progress.snapshot();
    println!(
        "installed {version} in {:.1}s, java {}, {} classpath entries, last stage {:?} ({} files)",
        started.elapsed().as_secs_f32(),
        installed.java.display(),
        installed.classpath.len(),
        snapshot.stage,
        snapshot.items_total
    );

    let java_check = std::process::Command::new(&installed.java)
        .arg("-version")
        .output()?;
    println!(
        "java -version exit {:?}: {}",
        java_check.status.code(),
        String::from_utf8_lossy(&java_check.stderr)
            .lines()
            .next()
            .unwrap_or_default()
    );

    if let Some(folder) = flags.iter().find_map(|f| f.strip_prefix("--backup-world=")) {
        let game_dir = root.join("game").join(&version);
        let backups = root.join("backups");
        for world in worlds::list_worlds(&game_dir, &backups) {
            println!(
                "world {:?} name {:?} mode {:?} hardcore {} version {:?} last played {:?} size {} icon {} backups {}",
                world.folder,
                world.name,
                world.game_mode,
                world.hardcore,
                world.version,
                world.last_played_ms,
                world.size_bytes,
                world.icon.as_ref().map_or(0, String::len),
                world.backups.len()
            );
        }
        let world_dir = game_dir.join("saves").join(folder);
        let backup = worlds::backup_world(&world_dir, &backups.join(folder))?;
        println!(
            "backup {} with {} bytes",
            backup.file_name, backup.size_bytes
        );
        std::fs::remove_dir_all(world_dir.join("region"))?;
        worlds::restore_backup(
            &game_dir.join("saves"),
            folder,
            &backups.join(folder).join(&backup.file_name),
        )?;
        println!(
            "restored, region files: {}",
            std::fs::read_dir(world_dir.join("region"))?.count()
        );
    }
    if flags.iter().any(|f| f == "--storage-preview") {
        let instance: Instance = serde_json::from_value(serde_json::json!({
            "schema": 1,
            "id": "keep",
            "name": "Keep",
            "minecraftVersion": "1.20.1",
            "loader": { "type": "fabric", "loaderVersion": "0.20.1" },
            "createdAt": 0,
        }))?;
        let summary = cleanup::preview(&paths, std::slice::from_ref(&instance), platform);
        println!(
            "cleanup preview: complete {} versions {:?} runtimes {:?} library files {} asset files {} cache {} total {} MB",
            summary.complete,
            summary.unused_versions,
            summary.unused_runtimes,
            summary.library_files,
            summary.asset_files,
            summary.cache_bytes,
            summary.total_bytes / 1024 / 1024
        );
        if flags.iter().any(|f| f == "--storage-clean") {
            let freed = cleanup::clean(&paths, std::slice::from_ref(&instance), platform);
            println!("cleanup freed {} MB", freed / 1024 / 1024);
        }
        println!(
            "data sizes: libraries {} MB, assets {} MB, runtimes {} MB, versions {} MB",
            cleanup::path_size(&paths.libraries_dir()) / 1024 / 1024,
            cleanup::path_size(&paths.assets_dir()) / 1024 / 1024,
            cleanup::path_size(&paths.runtimes_dir()) / 1024 / 1024,
            cleanup::path_size(&paths.versions_dir()) / 1024 / 1024
        );
    }
    if !prepare {
        return Ok(());
    }

    let game_dir = root.join("game").join(&version);
    std::fs::create_dir_all(&game_dir)?;
    if let Some(jar) = flags.iter().find_map(|f| f.strip_prefix("--optifine=")) {
        let jar = Path::new(jar);
        let found = optifine::inspect(jar)?.ok_or("not an OptiFine jar")?;
        println!("optifine {found:?}");
        let mods = game_dir.join("mods");
        std::fs::create_dir_all(&mods)?;
        std::fs::copy(jar, mods.join(jar.file_name().ok_or("no file name")?))?;
    }
    let tier = flags.iter().find_map(|f| match f.as_str() {
        "--tune=low" => Some(HardwareTier::Low),
        "--tune=medium" => Some(HardwareTier::Medium),
        _ => None,
    });
    if let Some(tier) = tier {
        let data_version = game_options::read_data_version(&paths.client_jar(&game_version));
        if let Some(text) = game_options::default_options(&game_version, data_version, tier) {
            std::fs::write(game_dir.join("options.txt"), &text)?;
            println!(
                "wrote options.txt:
{text}"
            );
        }
        println!(
            "gpu preference set: {:?}",
            gpu::prefer_dedicated_gpu(&installed.java).await
        );
    }
    if let Some(slug) = flags.iter().find_map(|f| f.strip_prefix("--mrpack=")) {
        let client = ModrinthClient::new(http.clone());
        let versions = client
            .modpack_versions(slug, &modpacks::MODPACK_LOADERS)
            .await?;
        let version = modpacks::pick_modpack_version(versions, Some(&game_version))
            .ok_or("no modpack version for this Minecraft version")?;
        let progress = Progress::new();
        let archive =
            modpacks::download_pack(&http, &version, &root.join("cache"), &progress).await?;
        let plan = modpacks::plan(modpacks::read_index(&archive)?)?;
        println!(
            "modpack {} {} for {} {:?}, {} files",
            plan.name,
            version.version_number,
            plan.minecraft_version,
            plan.loader,
            plan.files.len()
        );
        modpacks::install_files(&http, &plan, &game_dir, &progress).await?;
        let overrides = modpacks::extract_overrides(&archive, &game_dir)?;
        let mods = ModsDir::new(root.join("game"), &game_dir);
        let indexed = modpacks::index_content(&client, &mods, &game_dir).await?;
        println!("modpack installed, {overrides} override files, {indexed} indexed entries");
    }
    if let Some(target) = flags.iter().find_map(|f| f.strip_prefix("--export=")) {
        let instance: Instance = serde_json::from_value(serde_json::json!({
            "schema": 1,
            "id": "smoke",
            "name": "Smoke Export",
            "minecraftVersion": game_version,
            "loader": match mod_loader {
                Some(ModLoader::Fabric) => serde_json::json!({ "type": "fabric", "loaderVersion": "0.20.1" }),
                Some(ModLoader::Quilt) => serde_json::json!({ "type": "quilt", "loaderVersion": "0.24.0" }),
                Some(ModLoader::NeoForge) => serde_json::json!({ "type": "neo_forge", "loaderVersion": "21.1.265" }),
                Some(ModLoader::Forge) => serde_json::json!({ "type": "forge", "loaderVersion": "47.4.28" }),
                Some(ModLoader::LegacyFabric) => serde_json::json!({ "type": "legacy_fabric", "loaderVersion": "0.20.1" }),
                None => serde_json::json!({ "type": "vanilla" }),
            },
            "createdAt": 0,
        }))?;
        let client = ModrinthClient::new(http.clone());
        let report = modpacks::export(&client, &instance, &game_dir, Path::new(target)).await?;
        println!(
            "exported {} linked files, {} override files, skipped {:?}",
            report.linked, report.override_files, report.skipped
        );
        let reread = modpacks::plan(modpacks::read_index(Path::new(target))?)?;
        println!(
            "re-read export: {} for {} {:?} with {} files",
            reread.name,
            reread.minecraft_version,
            reread.loader,
            reread.files.len()
        );
    }
    let requests = flags.iter().filter_map(|f| {
        [
            ("--mod=", ContentKind::Mod),
            ("--pack=", ContentKind::ResourcePack),
            ("--shader=", ContentKind::Shader),
        ]
        .into_iter()
        .find_map(|(prefix, kind)| f.strip_prefix(prefix).map(|slug| (kind, slug)))
    });
    for (kind, slug) in requests {
        let mods = ModsDir::new(root.join("game"), &game_dir);
        let target = ModTarget {
            game_version: installed.version.id.rsplit('-').next().unwrap_or_default(),
            loaders: kind
                .modrinth_loaders(mod_loader)
                .ok_or("content needs a mod loader")?,
            kind,
        };
        let client = ModrinthClient::new(http.clone());
        let added = install(&http, &client, &mods, &target, slug, None, &Progress::new()).await?;
        for entry in added {
            println!(
                "installed {:?} {} {} ({}, dependency: {})",
                entry.kind, entry.title, entry.version_number, entry.file_name, entry.dependency
            );
        }
    }
    if flags.iter().any(|f| f == "--orrery-shaders") {
        let mods = ModsDir::new(root.join("game"), &game_dir);
        let entry =
            shaderpack::install_bundled(&mods, Activation::Select, HardwareTier::Medium).await?;
        println!("installed {} ({})", entry.title, entry.file_name);
    }
    if flags.iter().any(|f| f == "--orrery-menu") {
        let mods = ModsDir::new(root.join("game"), &game_dir);
        let target = menupack::GameTarget {
            minecraft_version: &game_version,
            client_jar: paths.client_jar(&game_version),
            optimize_options: false,
        };
        let entry = menupack::install_bundled(&mods, &target, Activation::Select).await?;
        println!("installed {} ({})", entry.title, entry.file_name);
    }
    println!("game folder prepared in {}", game_dir.display());
    Ok(())
}

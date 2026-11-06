# Architecture

## Overview

The launcher is a Cargo workspace with two crates and a React frontend.

| Part     | Path                 | Responsibility                                                                                                                  |
| -------- | -------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Core     | `crates/orrery-core` | All domain logic: metadata, downloads, Java, launching, accounts, mod loaders, Modrinth, instances. It has no Tauri dependency. |
| Shell    | `src-tauri`          | Thin Tauri layer: commands, event bridge, plugins, logging setup.                                                               |
| Frontend | `src`                | React 19, TypeScript, Vite, Tailwind, shadcn/ui, TanStack Query, Zustand.                                                       |

Keeping the core free of Tauri makes it testable without a window. Paths, the HTTP client and event listeners are injected.

## Core modules

| Module                                | Purpose                                                                                                                                                                                          |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `services`                            | Wires all stores together and contains the install and launch flow. Tauri state is `Services`.                                                                                                   |
| `net`                                 | HTTP client with retries and the parallel downloader. Downloads stream into a temporary sibling file, hash while writing, check the size and rename atomically.                                  |
| `minecraft::manifest`                 | Version manifest with a 15 minute cache and offline fallback, version JSON download with SHA-1 check, `inheritsFrom` resolution.                                                                 |
| `minecraft::version`                  | Version JSON model for both argument formats and the merge of child versions such as Fabric profiles.                                                                                            |
| `minecraft::rules`                    | OS, architecture and feature rules.                                                                                                                                                              |
| `minecraft::libraries`                | Classpath, legacy natives with `${arch}` classifiers and modern per-architecture natives.                                                                                                        |
| `minecraft::assets`                   | Asset index and objects.                                                                                                                                                                         |
| `minecraft::natives`                  | Extracts native libraries into a flat directory, guarded by a fingerprint marker.                                                                                                                |
| `minecraft::arguments`                | Builds JVM and game arguments, substitutes placeholders and adds the server join (`--quickPlayMultiplayer` from 1.20, `--server` and `--port` before).                                           |
| `minecraft::hooks`                    | Pre-launch and post-exit commands through the system shell, the wrapper command and the `$INST_*` placeholders, which are also set as environment variables.                                     |
| `minecraft::process`                  | Starts the game without a console window, converts Log4j XML output into plain lines, keeps a log tail and supports kill.                                                                        |
| `java`                                | Mojang runtime manifest first, Adoptium as fallback. On Apple Silicon, versions without arm64 natives use an x86_64 runtime through Rosetta.                                                     |
| `java::system`                        | Finds Java installations in `JAVA_HOME`, `PATH`, vendor folders and the managed runtimes and reads version and architecture from `-XshowSettings:properties`.                                    |
| `redact`                              | Removes access tokens, IP addresses, the home folder and account and OS user names from logs before they are logged or shared.                                                                   |
| `logshare`                            | Uploads a redacted log to mclo.gs after the user confirmed the preview, and reads the latest launcher log.                                                                                       |
| `servers`                             | Parses and validates `host[:port]` server addresses.                                                                                                                                             |
| `auth`                                | Account store, keyring access and the Microsoft login chain. Every account is a Microsoft account.                                                                                               |
| `loaders`                             | Fabric, Legacy Fabric and Quilt meta APIs. Profiles are stored as version JSONs with `inheritsFrom`. Missing library hashes are read from the Maven `.sha1` files.                               |
| `loaders::forge`, `loaders::neoforge` | Version lists from the official Maven repositories and the installer download URLs.                                                                                                              |
| `loaders::installer`                  | Shared Forge and NeoForge install: installer checks, bundled libraries, processors, generated files per profile in `meta/loaders`. Details in [loaders.md](loaders.md).                          |
| `optifine`                            | Recognises a downloaded OptiFine jar and reads its Minecraft and OptiFine version for the guided import.                                                                                         |
| `trash`                               | Moves deleted instances and worlds into `trash/<id>/content` with an `entry.json`, restores them, and removes entries after the configured number of days.                                       |
| `syncpacks`                           | Sync packs in `meta/sync-packs.json`, subscriptions in `instance.json`, and the sync that installs, retags or removes content marked `sync:<pack>`. Runs after every change and before a launch. |
| `loaders::processors`                 | Runs the client processors of a Forge style installer with the managed Java, without a console window, and checks their outputs.                                                                 |
| `modrinth`                            | Search, version selection, dependency resolution and verified installation. Downloads are only accepted from `cdn.modrinth.com`.                                                                 |
| `curseforge`                          | CurseForge client with the key from the build, fingerprints, installs, modpacks, blocked files via Modrinth or Downloads, export, updates.                                                       |
| `instances`                           | Instance CRUD, immutable slugs, duplication and the per-instance `mods.json` index.                                                                                                              |
| `tuning`                              | Memory and JVM flag recommendation from system RAM (sysinfo), version, loader, mod count and shaders.                                                                                            |
| `minecraft::crash`                    | Finds new crash reports or JVM error files after an unexpected exit and classifies the likely cause.                                                                                             |
| `shaderpack`                          | Embeds `shaderpack/` at build time, builds a deterministic zip, installs it with Iris, selects it in `iris.properties` and writes profile options per hardware tier.                             |
| `menupack`                            | Embeds `menupack/`, writes `pack.mcmeta` per instance version, selects the pack in `options.txt` and rebuilds it before launch.                                                                  |
| `extras`                              | Instance folders, log export, update checks through Modrinth hashes and CurseForge file indexes, and batch updates.                                                                              |
| `nbt`                                 | Reader and writer for uncompressed NBT with Java's modified UTF-8. Compounds keep their order, so files round-trip byte for byte.                                                                |
| `servers`                             | `servers.dat` editing that keeps unknown fields and hidden entries, writes a backup first and only runs while the game is stopped, plus an own Server List Ping.                                 |
| `screenshots`                         | Lists, reads and deletes screenshots with validated file names.                                                                                                                                  |
| `datapacks`                           | Data packs per world, their description from `pack.mcmeta` and their state from `level.dat`.                                                                                                     |
| `local_content`                       | Adds a dropped `.jar` or `.zip` after detecting its type from the archive and looks it up on Modrinth by hash.                                                                                   |
| `discord`                             | Discord IPC over the local named pipe or Unix socket (including Flatpak and Snap paths), run as one task that reconnects quietly and never blocks a launch.                                      |
| `packs`                               | Content packs from the bundled `packs.json`, resolved per Minecraft version, tagged per installed mod so that a pack can be removed as a whole, and the OptiFine feature scan of resource packs. |
| `skins`                               | Shared skin library, skin lookup by player name through the Mojang API and texture downloads restricted to `textures.minecraft.net`. Uploads and capes go through `auth::microsoft`.             |
| `bridge`                              | Local TCP bridge to Orrery Companion with a one-time token per game start, and the SHA-512 check of the bundled companion jars. See [companion.md](companion.md).                                |
| `links`                               | `orrery://` links and desktop shortcuts. Shortcuts carry a local key from `link.key`, links without it need a confirmation in the launcher.                                                      |
| `tasks` and `progress`                | Background task registry with atomic progress counters and cancellation.                                                                                                                         |

## Progress reporting

Long running work (install, launch, repair, mod install) runs as a task in the `TaskRegistry`. Each task owns a `Progress` value with atomic counters and a cancellation token. A ticker emits changed tasks as `task:updated` at most every 100 ms. Completion and failure are emitted immediately. The frontend fills a Zustand store from `list_tasks` and then follows the events, so progress survives navigation and reloads.

| Event             | Payload                        |
| ----------------- | ------------------------------ |
| `task:updated`    | `TaskView`                     |
| `game:state`      | `GameStateEvent`               |
| `game:log`        | `GameLogEvent` (batched lines) |
| `auth:login`      | `LoginEvent`                   |
| `update:progress` | `UpdateProgress`               |

TypeScript types for all payloads and command results are generated from Rust with `ts-rs` into `src/bindings`.

## Data layout

Configuration always lives in Tauri's `app_local_data_dir` (local, not roaming): `settings.json`, `accounts.json` and the launcher logs. The data root defaults to the same folder and can be moved to any folder in the settings (`dataDir` in `settings.json`). `LauncherPaths::with_data_root` in `crates/orrery-core/src/paths.rs` and `data_dir::resolve` are the only places that turn these into concrete paths.

```
<config>/                 app_local_data_dir
  settings.json           including dataDir
  accounts.json           accounts without any tokens
  logs/                   launcher logs, rotated daily

<root>/                   data root, the config folder unless dataDir is set
  orrery-data.json        marker, only in a custom data folder
  meta/                   manifests and version JSONs, needed for offline starts
  libraries/              shared Maven layout
  assets/                 indexes, objects, log configs
  versions/<id>/<id>.jar  client jars
  natives/<version>/
  runtimes/<component>/<platform>/
  skins/<account>.png
  cache/                  Java archives and modpack downloads, safe to delete
  instances/<slug>/
    instance.json
    mods.json
    minecraft/            game directory
```

Every JSON file carries a `schema` field and is written atomically.

### Changing the data folder

- At startup a custom data folder must exist, contain the marker and be writable. Otherwise Orrery starts without services and shows a screen to retry or to go back to the default folder. It never creates a missing custom folder.
- Moving runs as a `move_data` task while nothing else may run. On the same drive the top-level folders are renamed and renamed back if one fails. Across drives every file is copied by 8 parallel workers, synced and checked with SHA-256, and the copy is removed again on any error. Only after a complete copy is the marker written, `settings.json` switched and the old copy deleted. The app then restarts so that every store picks up the new paths.

## Microsoft login

1. Device code flow against `login.microsoftonline.com/consumers` with the scope `XboxLive.signin offline_access`.
2. Xbox Live user token, then XSTS for `rp://api.minecraftservices.com/`.
3. `login_with_xbox`, then `entitlements/license` must list `product_minecraft` or `game_minecraft`, otherwise the sign-in stops with `auth_no_minecraft_profile`.
4. The Minecraft profile.

The refresh token is stored in the system keyring (Windows Credential Manager, macOS Keychain, Secret Service on Linux). Long values are split into chunks. Microsoft rotates refresh tokens, so every refresh stores the new one. The Minecraft access token is kept in memory only and never written to disk or the keyring. Every game start runs the whole chain again: refresh token, Xbox Live, XSTS, `login_with_xbox`, the entitlement check and the profile. The profile id must match the stored account, and the player name and UUID passed to the game come from that freshly verified profile, never from `accounts.json`. When any step fails, for example without internet or because the account no longer owns the game, the game does not start and the in-memory token is dropped. Skin upload and cape selection reuse the in-memory token while it is valid for more than five minutes. Older versions stored the Minecraft token in the keyring, and Orrery deletes those entries at startup. A per-account lock prevents concurrent refreshes.

New Azure app registrations must be approved by Mojang before `api.minecraftservices.com` accepts them. Until then the login ends with `auth_app_not_approved`.

Orrery has no offline accounts, no username-only profiles, no demo mode and no alternative authentication servers. There is no build flag or development switch that skips the sign-in or the ownership check.

## Security notes

- The frontend has no file system or shell plugin. Every file access goes through validated commands.
- Instance ids, version ids, Maven paths, asset hashes and mod file names are validated against path traversal.
- Access tokens are redacted from logged launch arguments. Environment variables are logged by name only.
- Logs are only shared after the user saw the redacted preview and confirmed it. The core redacts the text again before the upload.
- `orrery://launch` links from outside Orrery never start a game without a confirmation. Development builds use the `orrery-dev` scheme.
- Custom launch commands run with the user's rights, are always visible in the instance settings and are never taken from imported modpacks.
- Auto updates are signed and `requireSignedVersion` blocks downgrades to older signed releases.

# Roadmap

| Milestone | Scope                                                                                              | Status                             |
| --------- | -------------------------------------------------------------------------------------------------- | ---------------------------------- |
| M0        | Workspace, error handling, paths, storage, task system, CI                                         | Done                               |
| M1        | Vanilla install and launch for releases and snapshots from 1.8, Java runtimes                      | Done                               |
| M2        | Account switcher, local skins                                                                      | Done, no offline accounts          |
| M3        | Microsoft login in the browser or with a code, keyring storage, automatic refresh                  | Done, needs Mojang approval        |
| M4        | Fabric through the Fabric Meta API                                                                 | Done                               |
| M5        | Instance management (create, edit, duplicate, delete, open folder, repair)                         | Done                               |
| M6        | Modrinth search and per-instance installation with dependencies                                    | Done                               |
| M7        | Signed auto updates and GitHub release pipeline                                                    | Done                               |
| M8        | Logo, space theme, memory recommendation, crash detection, playtime, mod updates, folder shortcuts | Done                               |
| M9        | Quilt, resource packs, shaders, performance pack, hardware based options, dedicated GPU on Windows | Done                               |
| M10       | Modpacks (.mrpack), world manager, storage cleanup, command palette, English translation           | Done                               |
| M11       | Launch options, join server on launch, game log search and sharing                                 | Done                               |
| M12       | Instance groups, servers tab with ping, screenshots, data packs, shortcuts, drag and drop          | Done                               |
| M13       | Discord Rich Presence                                                                              | Done, needs a Discord app id       |
| M14       | OptiFine replacement pack, OptiFine resource pack detection, more smart packs                      | Done                               |
| M15       | 3D skin viewer, skin library, skin upload and cape selection                                       | Done, upload needs Mojang approval |
| M16       | Companion mod foundation with launcher bridge and server policy channel                            | Done                               |
| M17       | In-game HUD editor, HUD modules and quality of life features                                       | Done                               |
| M18       | Cosmetics without a backend                                                                        | Removed, not planned               |
| M19       | Import from other launchers                                                                        | Done, real export only from Prism  |
| M20       | CurseForge                                                                                         | Done, tested with RLCraft          |
| M21       | Forge, NeoForge, Legacy Fabric and guided OptiFine import                                          | Done                               |
| M22       | Crash assistant with fixes and mod bisect                                                          | Planned, phase D                   |
| M23       | Play with friends                                                                                  | Planned, phase D                   |
| M24       | Polish, accessibility, CLI and automation                                                          | Planned, phase D                   |
| M25       | Orrery Shaders, an own shader pack for Iris built into the launcher                                | Removed                            |
| M26       | Orrery Menu, an own main menu design built into the launcher                                       | Stages 1 and 2 done, prioritized   |
| M27       | First run, data safety and licenses                                                                | Done                               |
| M28       | Sync packs                                                                                         | Stage 1 done, untested in the app  |
| M29       | Browser login and distribution                                                                     | Partly done, code signing open     |

## Verified manually

- Real installs and launches on Windows for 1.8.9, 1.12.2, 1.20.1 and the latest release with Fabric
- Modrinth installation of Sodium and Mod Menu including Fabric API and Text Placeholder API, game loaded with all mods
- Performance pack on 1.20.1 Fabric, Fabric API added from the jar metadata, game loaded with all mods
- Hardware based `options.txt` kept by 1.8.9, 1.20.1 and 26.3
- Fabulously Optimized 5.2.8 installed from Modrinth (52 files, overrides, content index), game loaded all mods, export read back
- RLCraft installed from CurseForge with a real API key: 185 files registered with their CurseForge ids, two files with blocked downloads listed for the manual download with working links to their pages, Forge 1.12.2 loaded all 183 mods up to the RLCraft main menu. The update check found 48 newer files, the `.zip` export linked 186 files by fingerprint and its manifest is valid, a wrong file for a blocked entry was rejected. Both blocked files were then downloaded in the browser, picked up from the downloads folder and registered, and Forge loaded all 185 mods
- Waystones installed from CurseForge into Forge 1.20.1 with Balm as a dependency, the game loaded both up to the main menu
- Quick Play into a world on 1.20.1, world list with icon and size, backup and restore of a real world
- Storage cleanup on real data freed 955 MB, 1.20.1 with Fabric started afterwards without downloading anything
- Whole UI in German and English, checked page by page in both languages
- Keyring roundtrip with a 4500 character secret in the Windows Credential Manager
- Join a server on launch: 1.12.2 through `--server` and 1.21.1 through Quick Play both connected to `localhost:25565`, pre-launch and post-exit commands saw the `$INST_*` values and an own environment variable, a failing pre-launch command stopped the launch
- Launcher minimizes or hides while the game runs and comes back after exit
- A server list written by Orrery was loaded and saved again by Minecraft 1.21.1, including a server name with an emoji, and Orrery read the result back
- Server List Ping against a local status server shows MOTD, players, version and favicon
- A desktop shortcut opened through Windows started 1.12.2 and joined its server, a link without the shortcut key asked for confirmation
- A mod jar added from a file was recognized on Modrinth by its hash, a Forge jar and a data pack were rejected with a hint
- OptiFine replacement pack on 1.20.1 Fabric: 11 mods installed with their dependencies, the game loaded all of them up to the main menu, removing the pack kept the shared dependencies
- A resource pack with OptiFine connected textures was detected and the suggested Continuity install cleared the hint
- Skin page: skins imported by player name, legacy and modern skins render with the second layer, slim arms render three pixels wide
- Orrery Companion loaded and connected to the launcher on Fabric 1.20.1, 1.21.11 and 26.3 and on Quilt 1.20.1, also in instances without Fabric API
- HUD modules on 26.3 in a world: FPS, coordinates with biome and facing, clock, memory at their stored positions and scales. 1.21.11 and 1.20.1 start with the HUD code, without a world to look at
- Orrery title screen on Fabric 1.20.1, 1.21.11 and 26.3 and on Quilt 1.20.1, checked with window screenshots: continue button, Orrery button, account name, skin model from 1.21.11 on, switched off leaves only the Orrery button. A Quick Play start into a world on 26.3 stored that world as the last one
- Bridge features on 26.3 in a world: a launcher toast showed up in the game, a screenshot reached the gallery right away without polling, the instance page showed world and dimension after a teleport to the Nether, and the servers tab showed the stored playtime of a server
- Import of a Prism Launcher instance by drag and drop: Fabric 1.21.1 with Lithium, a world, memory settings, the Java path and a pre-launch command shown as not taken over, the game started with Lithium loaded. An archive with `../` was rejected, an ATLauncher Forge instance showed the Forge note
- NeoForge 21.1.265 on 1.21.1 and 26.3.0.101 on 26.3 installed through their official installers and started on Windows with a local development build up to the title screen, without errors from the loader
- Forge 47.4.28 on 1.20.1 and 66.1.5 on 26.3 installed and started the same way up to the title screen, Forge mod loading reported the right Minecraft version
- Forge 14.23.5.2864 on 1.12.2 and 11.15.1.2318 on 1.8.9 installed and started with Java 8 up to the title screen, Forge Mod Loader loaded all of its mods
- OptiFine 1.20.1 HD U I6 from optifine.net was recognised, also under another file name, preselected Forge 47.2.18 from its own changelog and loaded in that Forge instance up to the title screen
- Legacy Fabric 0.20.1 on 1.8.9 installed from its meta API with checked natives and started up to the title screen without errors
- Discord IPC handshake against the Discord app on Windows: a refused application id is handled quietly and the game starts normally
- Share preview of a real game log and of the launcher log contains no Windows user name, home path or account name

## Open items

- macOS and Linux have only been verified through CI builds, not by starting the game
- Microsoft login has been tested on Windows against the real services up to Xbox Live and XSTS. The Minecraft login step stays blocked until Mojang approves the Azure app
- Import from the Modrinth App, CurseForge, ATLauncher, GDLauncher and the official launcher is only tested with fixtures built from their source code, not with real exports. Forge, NeoForge and Legacy Fabric instances from other launchers are only covered by these fixtures
- Log sharing has only been tested against a mock of the mclo.gs API, no real upload yet
- Feral GameMode, MangoHud and PRIME offload need a check on a Linux machine
- Rich Presence has only been tested against a fake Discord and a refused handshake. It needs a registered Discord application with the Orrery logo uploaded as art asset `orrery`, its id goes into `.cargo/config.toml` as `DISCORD_CLIENT_ID`
- Connected textures of a real OptiFine resource pack still need a look in game, on 1.20.1 and on the latest release
- Skin upload and cape selection have only been tested against a mock of the Minecraft profile API. The viewer shows capes but no elytra yet
- The `orrery:policy` channel has no test server yet, and the HUD editor has not been used by hand yet
- The server ping does not resolve SRV records yet, so servers that only publish one show as not reachable although the game can join them
- Dragging files from the file manager onto the window and shortcuts on macOS and Linux have not been tried by hand
- Optional: a Windows-only button that opens Minecraft Bedrock through the `minecraft:` protocol

## Planned milestones

The order follows value per effort and dependencies. Phase A brings launcher parity with Prism Launcher and the Modrinth App. Phase B gives Orrery its own in-game identity through a companion mod. Phase C widens the content reach. Phase D adds what the other launchers lack.

Every milestone keeps the existing rules: verified downloads, atomic writes, validated paths, German and English UI, no running costs and no new dependency without approval.

### Phase A: Launcher parity

#### M11 Launch options and game log

**Goal:** Give every instance the launch controls that power users know from Prism Launcher.

**Scope**

- Per-instance environment variables, wrapper command, pre-launch and post-exit commands with `$INST_NAME`, `$INST_ID`, `$INST_DIR`, `$INST_MC_DIR`, `$INST_JAVA` and `$INST_JAVA_ARGS`
- Extra game arguments and a fixed account per instance
- Window behaviour while the game runs: keep open, minimize, or close and reopen after exit
- Join a server on launch: `--quickPlayMultiplayer` from 1.20, `--server` and `--port` before 1.20
- Pick Java from detected system installations in addition to the managed runtimes
- Linux: Feral GameMode, MangoHud and dedicated GPU toggles
- Game log with search, level filter, follow toggle and a page for the launcher log
- Share a log through mclo.gs only after confirmation. Access tokens, IP addresses and user paths are redacted first

**Done when:** a failing pre-launch command stops the launch with a clear message, wrapper and environment variables reach the game process (covered by a test), a 1.12.2 and a 1.21 instance join a server on launch, and a shared log contains no token, IP address or user name.

**Depends on:** nothing.

**Risks:** user commands run with the user's rights. Modpack imports never set commands, and commands are always visible in the instance settings.

#### M12 Instances and content

**Goal:** Keep large instance collections manageable and cover every kind of content a player touches.

**Scope**

- Instance groups, pinning, custom icons, sort orders and a "Continue playing" row
- Servers tab: read and write `servers.dat` (a new NBT writer next to the reader in `worlds.rs`), an own Server List Ping for MOTD, player count, version and icon, one-click join through M11
- Screenshot gallery per instance with preview, copy, delete and open folder
- Data packs per world
- Drag and drop of mod jars, zips and `.mrpack` files onto the window
- Marks for content that does not match the Minecraft version of the instance
- Copy `options.txt`, key binds and the server list from another instance when creating one (done, key binds and language are converted for 1.12 and older)
- Desktop shortcuts and Quick Play shortcuts for a world or server, `orrery://` deep links

**Done when:** a server added in Orrery shows up in the in-game multiplayer list and the other way round, the ping result matches the in-game list for a public server, groups and icons survive a restart and a data folder move, and a desktop shortcut starts its instance.

**Depends on:** M11 for joining, approval of the deep link and single instance plugins (see Decisions needed).

**Risks:** the game rewrites `servers.dat` while it runs. Orrery only writes it while the instance is stopped, atomically and with a backup.

#### M13 Discord Rich Presence

**Goal:** Show in Discord what the player is doing, without a library and without a backend.

**Scope**

- Discord IPC implemented in the core: named pipe on Windows, Unix socket on macOS and Linux, including Flatpak and Snap paths
- Discord application id as a compile-time variable like `MS_CLIENT_ID`. Without it the feature is off
- Setting with three levels: off, version and loader only (default), with server or world name
- Elapsed time, Orrery logo and instance name
- Richer details from the companion mod in M17

**Done when:** the presence appears a few seconds after launch, clears on exit, and Orrery keeps working when Discord is closed or restarted during play.

**Depends on:** a registered Discord application (free).

**Risks:** the IPC protocol is not formally versioned. The client tolerates unknown replies and reconnects quietly.

#### M14 OptiFine replacement and smart packs

**Goal:** Cover everything players install OptiFine for, legally and on Fabric and Quilt.

**Scope**

- "OptiFine replacement" pack following the `PERFORMANCE_PACK` pattern in `extras.rs`: Iris, Continuity, Entity Model Features, Entity Texture Features, a CIT mod, a zoom mod and dynamic lights. The exact projects are checked per Minecraft version during implementation
- Resource pack scan for OptiFine features (`optifine/ctm`, `optifine/cem`, `optifine/cit`, emissive textures, custom sky) with a suggestion of the mods that render them
- More packs: shader ready, low-end PC and quality of life
- Every pack shows the mods it adds and can be removed as a whole
- Pack lists live in one data file so that updates touch nothing else

**Done when:** a resource pack with connected textures renders correctly after accepting the suggestion on 1.20.1 and on the latest release, and a pack never installs a mod that does not exist for the instance version.

**Depends on:** nothing.

**Risks:** replacement mods change between Minecraft versions, so every pack is resolved per version and skips missing projects with a note.

### Phase B: In-game identity

#### M15 Skins and capes

**Goal:** A skin manager on the level of Prism Launcher and the Modrinth App.

**Scope**

- 3D skin viewer on WebGL2 without a new dependency: classic and slim arms, second layer, cape and elytra, drag to rotate, respects reduced motion
- Skin library per account with import from a file or a player name
- Upload for Microsoft accounts (`POST /minecraft/profile/skins`, classic or slim) and choice of owned capes (`PUT /minecraft/profile/capes/active`, `DELETE` to hide)

**Done when:** an uploaded skin appears in game, a cape change shows on the official profile, and the viewer renders the second layer and slim arms correctly.

**Depends on:** Mojang approval of the Azure app (M3) for real uploads.

**Risks:** the profile endpoints are not officially documented. Errors and rate limits are shown to the user and never retried in a loop.

#### M16 Companion mod foundation

**Goal:** An own Fabric mod that the launcher installs, configures and talks to.

**Scope**

- Gradle project `companion/` in this repository with Fabric Loom and Mojang mappings
- First version matrix: the latest 26.x release and 1.21.11. More versions on demand
- CI builds the jars, the launcher ships them as Tauri resources and installs them with a SHA-512 check. No separate hosting
- On by default for Fabric and Quilt instances, switchable per instance, never written into exported modpacks
- Local bridge to the launcher, in-game Orrery menu on a configurable key, settings per instance, German and English
- `orrery:policy` plugin channel through which servers switch modules off
- Orrery title screen (M26 stage 2, done): a continue button for the last world or server, a button for the Orrery menu, the active account with a rotatable skin model from 1.21.11 on (1.20.1 has no skin widget yet, so it shows the name only). The vanilla buttons keep their place, so buttons that other mods add (Mod Menu) and the panorama of the Orrery Menu pack stay. The mod records the last world or server itself, so it works without the launcher. Switchable in the Orrery menu, and nothing changes when another mod replaces the title screen
- `docs/companion.md` with the bridge protocol, the policy format and the fair play rules

**Done when:** a new Fabric instance on each supported version starts with the mod, the launcher shows the bridge as connected, a switched-off instance starts without the mod, and a test server switches a module off.

**Depends on:** a new CI job with Gradle and Java.

**Risks:** a second language and build system, and work for every Minecraft release. A small version matrix and a thin version-specific layer keep this manageable.

#### M17 In-game HUD and quality of life

**Goal:** Quality-of-life features inside the game, without a backend.

**Scope**

- HUD editor with drag and drop, snapping and scaling
- HUD modules: FPS, CPS, coordinates with biome and facing, keystrokes, armor and held item durability, status effects, clock, server address and ping, memory
- Zoom (smooth and scroll), crosshair editor
- Camera tweaks: no view bobbing, no damage tilt, low fire, hand position
- Chat: timestamps, stacking of repeated lines, copy, longer history
- Waypoints and death point
- Bridge features: Discord presence with server, world and dimension, live status in the launcher, playtime per server, screenshots in the launcher gallery right away, launcher notifications as in-game toasts

**Stages:** 1 (done) module system, policy per module, HUD renderer and editor, FPS, coordinates, clock, memory, server and ping. 2 (done) keystrokes, CPS, durability, status effects. 3a (done) zoom. Freelook, fullbright and auto sprint were built and then removed, because they go beyond display and comfort. Toggle sprint and sneak, view bobbing and damage tilt are vanilla options. 3b (done) crosshair editor, low fire, hand position. 4a (done) chat timestamps, stacking, longer history, chat history with copy. 4b (done) waypoints, death point. Auto reconnect was built and then removed, because it rejoined after kicks too. 5 (done) bridge features.

**Done when:** every module can be placed, saved and restored, the policy channel switches each module off, and FPS stays within measurement noise with all HUD modules on (fixed test scene).

**Depends on:** M16, M13 for presence details.

**Risks:** servers may still forbid single modules. Every module obeys the policy channel.

#### M18 Cosmetics without a backend

**Status:** removed. Hats, wings and a custom cape image were built, with shared items stored in unused skin pixels. Mojang's rules for launchers do not allow unofficial cape enablement, and the shared items changed the player's uploaded skin. Orrery therefore has no cosmetics and only offers the capes a Minecraft profile owns. Emotes, visible to the player only, stay in the companion.

**Risks:** only works for Microsoft accounts on online-mode servers. Every change needs a skin upload. Models need own artwork.

### Phase C: Reach

M21 came before M20 because CurseForge modpacks need Forge and NeoForge. M20 is built and tested against a mock of the API until a CurseForge API key is approved.

#### M19 Import from other launchers

**Goal:** Switching to Orrery costs one click.

**Scope**

- Sources: Prism Launcher and MultiMC, Modrinth App, CurseForge app, ATLauncher, GDLauncher and the official launcher (worlds, resource packs, options)
- Detects default locations and accepts any folder or archive by drag and drop
- Preview step that lists what will be copied
- Commands, hooks and Java paths are never taken over silently
- Every path is validated like `.mrpack` overrides. Prism Launcher 11.1.1 had to fix a path traversal in CurseForge overrides

**Done when:** an instance from each source starts with the same mods and worlds, and an archive with `../` paths is rejected.

**Depends on:** M21 for Forge and NeoForge instances. Until then those are imported with a clear note.

**Risks:** other launchers change their formats without notice. Each importer has fixture tests built from the launcher's source code, and real exports are added where available.

#### M21 Forge, NeoForge and Legacy Fabric

**Goal:** Every major loader, including the 1.8.9 PvP setups.

**Scope**

- NeoForge from 1.20.2
- Forge for 1.8.9, 1.12.2 and 1.20.1, including the installer processors
- Downloads only from the official Maven repositories with hash checks
- Legacy Fabric for 1.8.9 through the existing meta API pattern in `loaders`
- Guided OptiFine import: the player downloads the jar from optifine.net, drops it onto Orrery and gets a ready instance
- Forge and NeoForge in the rest of the launcher: instances imported through M19 keep their loader, `.mrpack` files with a `forge` or `neoforge` dependency install, Modrinth search and dropped mod jars accept these loaders

**Stages:** 1 (done) NeoForge from 1.20.2: versions from the official Maven, installer profile, processors with the managed Java and without a console window, hash checks. 2 (done) Forge for Minecraft 1.17 and newer on the same processor pipeline, with the installer's bundled libraries. 3 (done) Forge 1.12.2 and 1.8.9 with their older installer formats. 4 (done) Legacy Fabric for 1.8.9 and other versions up to 1.13.2. 5 (done) Forge and NeoForge in import, modpacks, Modrinth search and drag and drop. 6 (done) Guided OptiFine import as a Forge mod. Details in [loaders.md](loaders.md).

**Done when:** each loader starts its listed versions with a mod installed, and the processors run without a console window.

**Depends on:** nothing.

**Risks:** installer processors are Java programs from the loader projects. They run with the managed Java and only from verified files. Forge asks launchers not to automate its installation because its download page is funded by ads. Orrery automates it like Prism Launcher and the Modrinth App and shows the request with links to the Forge website and Patreon. Until Mojang approves the Azure app, public builds cannot start the game, so in-game checks run on a local development build.

#### M20 CurseForge

**Goal:** CurseForge as a second content source next to Modrinth.

**Scope**

- API key as a compile-time variable from a CI secret. Without it CurseForge is hidden, which keeps development builds working
- Search and installation of mods, modpacks, resource packs and shaders with dependencies
- CurseForge zip import and export
- Download host allowlist for the CurseForge CDN
- Projects that block third-party downloads: look the file up on Modrinth by hash, otherwise a guided manual download that picks the file up from the downloads folder
- Update checks across both sources

**Stages:** 1 (done) API client, fingerprint, CDN allowlist and the key from the build. 2 (done) search and installation of mods, resource packs and shaders with dependencies, source switch on the content tab. 3 (done) modpacks from the search, from a file or dropped, blocked files through Modrinth or the downloads folder, export as `.zip`. 4 (done) update checks and updates for CurseForge content. 5 (done) real check with a key: RLCraft installs and starts. Details in [curseforge.md](curseforge.md).

**Done when:** a popular CurseForge modpack installs and starts, a blocked file is resolved through Modrinth or the manual flow, and the key never appears in the repository or in logs.

**Depends on:** an approved CurseForge API key, M21 for Forge packs.

**Risks:** the API terms can change and authors can block third-party access at any time.

### Phase D: Differentiation

#### M22 Crash assistant

**Goal:** Fix common crashes in one click, locally. Neither Prism Launcher nor the Modrinth App does this.

**Scope**

- Local rules in `minecraft/crash.rs` with fixes that can be undone: missing dependency, wrong mod version, wrong Java, out of memory, mixin conflict
- Mod bisect: disables half of the mods per start until the culprit is found, then restores the original state
- Debug bundle with logs, crash report, mod list and system info, redacted like in M11

**Done when:** each rule has a test with a real crash report, and bisect finds a planted broken mod among 40 mods in at most 6 starts.

**Depends on:** M11 for redaction.

**Risks:** a wrong fix makes things worse. Every fix shows what it changes and can be undone.

#### M23 Play with friends

**Goal:** Play together without a server of our own.

**Scope**

- Step 1: "Open to friends" through the optional e4mc mod, labelled clearly as a third-party relay
- Step 2: a local Fabric or vanilla server run from the launcher with start, stop, console and backups, plus a guide for Tailscale or playit.gg
- Step 3 as a spike: own peer-to-peer invite codes with UPnP or NAT-PMP, public STUN servers and encryption, as Horizon does it

**Done when:** a friend joins a singleplayer world through step 1, and a local server keeps its world and backups across restarts.

**Depends on:** M12 for the server list.

**Risks:** e4mc depends on a third party. Port mapping fails on many networks, so step 3 stays optional.

#### M24 Polish and automation

**Goal:** Finish the experience and make Orrery scriptable.

**Scope**

- Accent colors and backgrounds
- A banner or background image per instance on the instance page
- OS notifications, for example when the game crashed while Orrery was minimized
- HTTP proxy
- Accessibility pass: screen reader labels, contrast and full keyboard use
- CLI with `--json` output for launch, list and import, optionally an MCP server for AI agents that asks before destructive actions
- Content deduplication through hard links
- Windows-only Bedrock button
- Real game starts on macOS and Linux

**Done when:** every page passes a keyboard-only walkthrough, the CLI starts an instance by id, and the game has been started on macOS and Linux.

**Depends on:** approval of the notification plugin.

**Risks:** hard links do not work across drives, so deduplication only applies within the data folder.

### Ideas from the NoRisk Client comparison

In November 2026 the NoRisk Client launcher (v0.6.29) was compared with Orrery through its public repository, website and a local install. These milestones take over what fits Orrery's rules: no backend, no telemetry, no ads. Analytics, rewarded ads, referrals, the shop, friends, chat and the clip recorder stay out.

#### M27 First run, data safety and licenses

**Goal:** A friendly first start that stays within Mojang's rules, and no lost data by a wrong click.

**Scope**

- Welcome screen (done) on every start without an account: sign in with Microsoft or look around first until Orrery is closed. Without an account the player can create instances and install content, but not start the game
- Pressing play without an account opens the sign-in and continues the launch afterwards (done)
- Trash (done): deleted instances and worlds are moved to a trash folder in the data folder, can be restored from settings and are removed after a configurable time (default 14 days) or on request
- Pin a mod (done): excluded from update checks and from updating all, shown with a pin mark
- Network outages: dropped. Every game start signs in again online and checks game ownership, so the game does not start without internet
- Licenses page in About (done): all third-party Rust crates and npm packages with their license texts, collected at build time by an own script from `cargo metadata` and `node_modules`, without a new dependency

**Done when:** every start without an account shows the welcome screen, a launch without an account resumes after signing in, a deleted instance comes back with all its files, a pinned mod is skipped by updating all, and the licenses page lists every shipped dependency.

**Depends on:** nothing.

**Risks:** the trash costs disk space until it is emptied. The storage page counts it and empties it on request.

#### M28 Sync packs

**Goal:** One list of mods, resource packs and settings that several instances share.

**Scope**

- A sync pack holds Modrinth projects (stage 1, done), later also local files and selected config files (stage 2)
- Instances subscribe to sync packs. Each project is resolved per Minecraft version and loader of the instance, and missing versions are skipped with a note
- Per instance a single project of a pack can be excluded
- Changing a pack updates every subscribed instance the next time it is not running
- Builds on the content packs from M14, which already install and remove projects as a group

**Done when:** a pack with ten mods keeps a 1.20.1 and a 1.21.11 Fabric instance in sync, an excluded mod stays away in one instance, and removing the pack removes only what it added.

**Depends on:** nothing.

**Stages:** 1 (done) packs of Modrinth mods, resource packs and shaders on an own page, subscriptions and exclusions per instance on the content tab, sync after every change and before launch. 2 local files and config files such as `options.txt`.

**Risks:** two packs can bring the same project. The first pack in the order of the subscriptions owns it, the second does not install it again. Content the player installed by hand is never removed by a pack.

#### M29 Browser login and distribution

**Goal:** A smoother sign-in and easier installation on every system.

**Scope**

- Microsoft sign-in in the system browser with PKCE and a loopback redirect, next to the device code (done and tested on Windows, tests on macOS and Linux are still open)
- Download page on the website that detects the operating system and lists all files (done, the list is written into the site from the latest release when it is published)
- Free code signing for open source projects on Windows through the SignPath Foundation, if Orrery meets its conditions (prepared: policy page and integration plan in [code-signing.md](code-signing.md), the application is up to the owner)

**Done when:** a sign-in through the browser works on all three systems, the website offers the matching download first, and Windows shows a verified publisher.

**Depends on:** Mojang approval of the Azure app and a loopback redirect URI in it for the browser login. Acceptance by the SignPath Foundation.

**Risks:** the redirect needs a free local port. Device code stays as the fallback.

### Prioritized

#### M25 Orrery Shaders

**Status:** removed in 0.5.2. An own shader pack for Iris was built and shipped inside the launcher. It is not part of Orrery any more. Instances that already have it keep the file as an ordinary shader pack, and Orrery no longer updates it.

#### M26 Orrery Menu

**Goal:** The Minecraft main menu in the Orrery look, on every supported version and without mods.

**Scope**

- Stage 1 (done): resource pack in `menupack/` with a generated space panorama, buttons, sliders, menu backgrounds, separators and own splash texts. Embedded at build time, one click in the resource packs tab, selected in `options.txt`, `pack.mcmeta` written per instance and rebuilt before launch when the version changes, update detection
- Stage 2 (done): the Orrery title screen in the companion mod (see M16 and [companion.md](companion.md))
- Details in [menupack.md](menupack.md)

**Done when:** the pack loads without a compatibility warning on 1.8.9, 1.20.1, 1.21.1 and the latest release, and the title screen, option screens, hover and disabled buttons look right in game.

**Depends on:** nothing for stage 1, M16 for stage 2.

**Risks:** a resource pack cannot move or add buttons. Before 1.20.2 buttons stay vanilla because they share `widgets.png` with the hotbar.

## Companion mod principles

- **Independent:** the launcher works completely without the mod, and the mod works without the launcher with default settings.
- **Bridge:** the launcher listens on 127.0.0.1 on a random port and passes the port and a one-time token as JVM properties (`-Dorrery.bridge`). The token is redacted from logs like access tokens. Messages are JSON lines with a protocol version.
- **Network:** the mod makes no network connections of its own apart from the local bridge.
- **Fair play:** only display and comfort features. Nothing automates movement or combat, changes brightness or the camera beyond vanilla, or reveals hidden information. Servers can switch every module off through `orrery:policy`.
- **Versions:** Mojang mappings everywhere. From 26.1 on Minecraft is no longer obfuscated and Loom does not remap it. 1.21.11 is the last obfuscated release. A thin version-specific layer keeps most code shared, and the launcher hides the mod for versions it does not support.
- **Delivery:** built in CI, shipped inside the launcher, verified with SHA-512 and updated together with the launcher.
- **License:** only own code. The NoRisk launcher is GPL-3 and its client is closed source, so nothing is copied from either.

## Feature comparison

State in November 2026, based on public sources.

| Feature                         | Orrery today   | Orrery target  | Prism Launcher | Modrinth App | NoRisk      |
| ------------------------------- | -------------- | -------------- | -------------- | ------------ | ----------- |
| Hardware based tuning           | Yes            | Yes            | No             | Memory only  | Not found   |
| World backups and Quick Play    | Yes            | Yes            | Quick Play     | Quick Play   | Partly      |
| Launch hooks, wrapper, env vars | Yes            | Yes            | Yes            | Yes          | Not found   |
| Server list with ping           | Yes            | Yes            | Yes            | Yes          | Yes         |
| Discord Rich Presence           | Yes            | Yes            | No             | Yes          | Yes         |
| 3D skin viewer and capes        | Yes            | Yes            | Yes            | Yes          | Yes         |
| Own in-game mod                 | Yes            | Yes            | No             | No           | Closed      |
| Import from other launchers     | Yes            | Yes            | Partly         | Yes          | Yes         |
| CurseForge                      | Yes, needs key | Yes, needs key | Yes            | No           | Yes         |
| Forge and NeoForge              | Yes            | Yes            | Yes            | Yes          | Yes         |
| Crash analysis with fixes       | Hints only     | M22            | No             | No           | Server side |
| Play with friends               | No             | M23            | No             | Not released | Yes         |
| Friends list and chat           | No             | Not planned    | No             | Yes          | Yes         |

## Not planned

- Friends list, chat, online status and voice chat: they need a backend, and Orrery does not run one.
- Cosmetics shop and paid features: Orrery stays free and has no payment backend.
- Telemetry and ads.
- Automatic OptiFine download: the OptiFine license forbids redistribution and its downloads go through an ad page. M14 and the guided import in M21 cover the need.
- A hosted relay for friends: it would cost bandwidth every month. M23 uses e4mc or the player's own network.
- Clip recorder: a large native subsystem per operating system. OBS and the GPU vendor tools already cover it.
- Using the Microsoft client id of the official launcher: Orrery uses its own Azure app.
- A Flathub listing: since 2026 Flathub requires manifests and submissions without generative AI and reviews disclosed AI-written code case by case, with a permanent ban on violations. Orrery is developed with AI help, so it stays with AppImage, deb and rpm. Running in a Flatpak would also need the Secret Service portal for the keyring and deep links through the desktop file

## Decisions needed

- A DNS resolver for SRV records in the server ping, for example `hickory-resolver` (M12)
- Register a Discord application (M13)
- Apply for a CurseForge API key (M20)
- Whether the companion mod is also published on Modrinth

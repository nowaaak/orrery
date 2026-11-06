# Changelog

## 0.4.0

- Cosmetics are removed: hats, wings, the own cape image and the cosmetics stored in skins. Orrery only offers the capes your Minecraft profile owns. Emotes stay
- Freelook, fullbright, auto sprint and auto reconnect are removed from the companion
- The continue button for the last server in the companion's title screens is hidden when the account may not play multiplayer, for example because of parental controls or a multiplayer ban
- The installer, the app bundle, the welcome screen and the release notes state that Orrery is independent and not an official Minecraft product
- CurseForge as a second source in builds with a CurseForge API key: search and install mods, resource packs and shaders with their dependencies, install CurseForge modpacks from the modpack dialog, from a file or by dropping the zip, export an instance as a CurseForge zip, and get updates for CurseForge content
- Modpack exports also contain the `scripts`, `resources` and `structures` folders, so packs with CraftTweaker scripts or custom resources stay complete
- Files whose authors block downloads through other apps are taken from Modrinth when the same file is there. Otherwise the content tab lists them with a link to the download page and picks them up from the downloads folder as soon as they are downloaded

- The website offers the matching download for your system first and lists every installer of the latest release
- Sync packs: one list of mods, resource packs and shaders that several instances subscribe to. Orrery installs the matching version in each instance, skips what does not exist for it, removes what the pack no longer contains and leaves content you installed yourself alone. Single projects can be switched off per instance
- Native libraries that went missing, for example removed by a virus scanner, are unpacked again on the next start instead of crashing the game
- Licenses in About Orrery: every library inside the launcher with its license text, searchable
- Trash: deleted instances and worlds go to the trash, a toast offers to undo right away, and settings list them for restoring. Orrery empties entries after 14 days, configurable or never
- Pin a mod, resource pack or shader with the pin button in its row: Orrery stops offering updates for it until you unpin it
- Every sign-in and session refresh checks with Mojang's entitlement service that the account owns Minecraft: Java Edition. Accounts without the game cannot be added
- Welcome screen on the first start: sign in with Microsoft or look around first. Playing always needs the sign-in, so pressing play without an account opens it and starts the game right after it
- Every game start signs in again online and checks with Mojang that the account owns Minecraft: Java Edition. The player name and UUID come from that verified profile. The Minecraft access token is kept in memory only, and tokens that older versions stored in the keyring are deleted. Without internet the game does not start
- Guided OptiFine import: download OptiFine from optifine.net, drop the jar onto Orrery and pick a new or an existing Forge instance. The Forge version that OptiFine names in its changelog is preselected
- Forge, NeoForge and Legacy Fabric in the rest of the launcher: imports from other launchers keep these loaders, and mod jars dropped onto an instance are checked against its loader
- Legacy Fabric for Minecraft 1.13.2 and older, for example 1.8.9 PvP setups, with mods from Modrinth
- Forge for Minecraft 1.8.9, 1.12.2 and 1.17 and newer, installed the same way as NeoForge. The version picker passes on Forge's request to support the project, with links to its website and Patreon
- Modrinth modpacks for Forge and NeoForge can be installed
- NeoForge for Minecraft 1.20.2 and newer: pick it when creating an instance, Orrery downloads the official installer, checks every file and sets the loader up on the first start without a console window
- Import instances from Prism Launcher, MultiMC, Modrinth App, CurseForge, ATLauncher, GDLauncher and the official launcher, found automatically or chosen as folder or archive. A preview lists what is copied, and Java paths and commands are never taken over

## 0.3.0

- Orrery is now licensed under the GNU General Public License v3.0
- Offline accounts are removed. Every game start needs a Microsoft account that owns Minecraft: Java Edition, as Mojang requires for launchers that use its services. Offline accounts saved earlier disappear from the list
- About Orrery shows a link to the privacy page and the notice that Orrery is not an official Minecraft product
- Emotes in the companion: wave, cheer, clap, dance and point on the N key, visible to you only
- Cosmetics on the skin page: top hat, crown, beanie, halo, backpack, feather and bat wings with an own color, plus an own cape image. The 3D preview shows them right away, and Microsoft accounts can store head and back item invisibly in the skin so that other Orrery players see them. The companion draws them in the game for every player, and the own cape replaces the cape of your own player
- Orrery Companion: Orrery's own mod for Fabric and Quilt on 1.20.1, 1.21.11 and 26.3. It connects the game to the launcher, opens the Orrery menu on right Shift and respects servers that switch its modules off. It can be switched off per instance and never ends up in exported modpacks
- The companion feeds the launcher while you play: Discord shows world or server together with the dimension, the instance page shows where you are, the servers tab counts playtime per server, new screenshots appear in the gallery right away and launcher tasks that finish while you play show up as toasts in the game
- Waypoints with a compass bar, automatic death point and auto reconnect after a disconnect
- Chat timestamps, stacking of repeated messages, a longer history and a chat history screen that copies messages
- Crosshair editor with live preview, low fire and hand position as companion modules
- Zoom, freelook, fullbright and auto sprint as companion modules
- More HUD modules: keystrokes with clicks per second, CPS, armor and item durability, status effects
- Orrery modules in the companion: FPS, coordinates with biome and facing, clock, memory, server and ping as HUD boxes. Every module is off by default, can be switched off by servers and is placed with a HUD editor (drag and drop, snapping, mouse wheel scaling)
- New Orrery title screen in the companion, replacing the vanilla one: own background with slow motion, glass tiles with icons, Inter as font, greeting, continue card and a More popover for Realms, language and accessibility. The classic menu stays one click away
- Orrery title screen extras for the classic menu: continue with the last world or server, a button for the Orrery menu and the active account, from 1.21.11 with a rotatable skin model. The continue button and the account can be switched off in the Orrery menu
- Skin page per account with a 3D viewer for classic and slim arms, the second layer and capes. A shared skin library imports skins from files or player names
- Microsoft accounts upload skins to Minecraft and choose one of their capes (needs Mojang's approval of the app)
- Content packs: performance, OptiFine replacement, shader ready, low-end PC and quality of life. Each pack can be removed as a whole
- Resource packs that use OptiFine features such as connected textures, custom entity models or custom skies are detected, and Orrery offers the mods that render them
- Discord Rich Presence with three levels: off, version and mod loader, or with world or server. Orrery talks only to the local Discord app and keeps working when Discord is closed
- Groups, pinned instances, own icons and four sort orders on the instance overview
- Servers tab: edit the in-game server list, see MOTD, players, version and ping, and join with one click
- Screenshot gallery per instance with a viewer, copy to clipboard and delete with undo
- Data packs per world: add, remove and see whether Minecraft enabled them
- Drag mods, resource packs, shader packs and modpacks onto the window to add them
- Content built for another Minecraft version or mod loader is marked in the content tab
- Desktop shortcuts for instances, worlds and servers. Links from outside Orrery ask before they start a game
- Launch options per instance: a fixed account, joining a server on launch, extra game arguments, environment variables, a wrapper command and commands before launch and after exit with `$INST_*` placeholders. On Linux also Feral GameMode, MangoHud and the dedicated graphics card
- Pick an installed Java version: Orrery finds Java in common folders, `JAVA_HOME` and `PATH`
- The launcher can minimize or hide itself while you play and comes back when the game closes
- Game log with search, filter for warnings and errors and a follow switch. Share a log through mclo.gs after a preview without credentials, IP addresses or names
- Launcher log viewer in Settings and in the command palette
- Orrery Menu: the Minecraft main menu in the Orrery look with a space panorama, own buttons, backgrounds and splash texts. One click in the resource packs tab, works on every version from 1.8 and without mods
- Orrery Shaders: Orrery's own shader pack with soft shadows, volumetric clouds, light rays, reflective water with caustics, a starry night sky with nebula and aurora, waving plants and temporal anti-aliasing. One click in the Shaders tab installs Iris if needed and activates the pack with settings that fit your PC

## 0.2.1

- Branded Windows installer with the Orrery logo, its own welcome and header images, in English and German
- The data folder can no longer be Orrery's program folder or lie inside it, so uninstalling never touches your data
- "What's new?" link in Settings, About Orrery

## 0.2.0

- Choose where Orrery keeps its data, for example on another drive. Moving copies every file with verification, rolls back on errors and shows a clear message if the folder is missing at start
- Moving data across drives is about three times faster
- Settings show the installed version and offer a manual update check; Orrery also checks every 6 hours
- Development builds run as Orrery Dev with their own data and never update

## 0.1.0

First release.

- Install and launch every Minecraft: Java Edition release and snapshot from 1.8 onwards, with verified downloads and automatic Java runtimes
- Vanilla, Fabric and Quilt instances with their own memory, Java and window settings
- Offline accounts with local skins and Microsoft accounts via device code login (Microsoft login needs Mojang's approval of the app). Offline accounts were removed in 0.3.0 and the releases that contained them were withdrawn
- Modrinth search and installation of mods, resource packs and shaders, mod updates and a one-click performance pack
- Modrinth modpacks: install from Modrinth, import `.mrpack` files and export instances
- World manager with backups, restore and Quick Play straight into a world on 1.20 and newer
- Memory recommendation, hardware based game options and a dedicated GPU preference on Windows
- Crash detection with likely causes, playtime tracking and quick access to instance folders
- Storage overview with cleanup of unused game files
- Command palette on Ctrl+K and a user interface in German and English
- Signed auto updates through GitHub Releases

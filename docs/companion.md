# Orrery Companion

Orrery Companion is Orrery's own client mod in `companion/`. The launcher ships it and loads it into Fabric and Quilt instances. The launcher works without the mod, and the mod works without the launcher.

## Supported versions

| Minecraft | Loader          | Mappings                       | Java |
| --------- | --------------- | ------------------------------ | ---- |
| 26.3      | Fabric          | none, the game is unobfuscated | 25   |
| 1.21.11   | Fabric          | Mojang                         | 21   |
| 1.20.1    | Fabric or Quilt | Mojang                         | 17   |

One source tree builds every version with Stonecutter. The source is written for 26.3, and `//? if` comments switch code for older versions. `loom-back-compat` applies `net.fabricmc.fabric-loom` for 26.x and `net.fabricmc.fabric-loom-remap` for older versions.

## Building

```sh
npm run companion
```

This runs `collectJar` in `companion/` and copies one jar per version into `src-tauri/resources/companion/`, together with `companion.json`, which lists the SHA-512 of every jar. The jars are build output and are not committed. CI builds them in the `companion` job and the release workflow bundles them into the app.

The Gradle wrapper needs Java 17 or newer to start. Gradle downloads Java 25 for its daemon from Adoptium (`gradle/gradle-daemon-jvm.properties`) and the Java versions for compiling through the foojay toolchain resolver, so nothing has to be installed by hand.

The mod bundles the Fabric API modules it needs as jar-in-jar (`deps.fabric_api_modules` in `stonecutter.properties.toml`), so instances without Fabric API still start. When an instance has a newer Fabric API, the loader uses that one.

## Loading

The launcher never copies the mod into the instance. It checks the jar against `companion.json` and passes it with `-Dfabric.addMods=<path>` (Fabric) or `-Dloader.addMods=<path>` (Quilt). The mod therefore never ends up in an exported modpack. Instances can switch it off in their launch settings.

## Bridge

For every game start the launcher listens on `127.0.0.1` on a random port and passes the address and a one-time token:

```
-Dorrery.bridge=127.0.0.1:<port>
-Dorrery.bridge.token=<token>
```

The token is redacted from the launcher log. Messages are JSON objects, one per line, UTF-8, at most 64 KiB per line.

| Direction       | Message      | Fields                                                                 |
| --------------- | ------------ | ---------------------------------------------------------------------- |
| mod to launcher | `hello`      | `version` (protocol, currently 1), `token`, `mod_version`              |
| launcher to mod | `welcome`    | `version`, `instanceId`                                                |
| mod to launcher | `status`     | `screen` (`menu`, `world` or `server`), `world`, `server`, `dimension` |
| mod to launcher | `screenshot` | `file` (file name in `screenshots/`)                                   |
| launcher to mod | `toast`      | `title`, `body`                                                        |

The launcher closes the connection when the first line is not a `hello` with the right token within 10 seconds. Unknown message types are ignored on both sides. The mod reconnects quietly when the connection drops.

The mod sends `status` once a second when something changed. The launcher uses it for:

- Discord Rich Presence: on the detailed level the presence shows the world or server and the dimension, for example `Playing in New World · The Nether`
- The badge on the instance page, which shows world or server and dimension while the game runs
- Playtime per server: time on a server is added up per instance in `server-playtime.json` in the instance folder and shown in the servers tab. Addresses count as the same server regardless of case and the default port

`screenshot` follows the vanilla chat message after a screenshot was saved. The launcher accepts plain `.png` file names only and refreshes the screenshot gallery. While a companion is connected the gallery no longer polls the folder.

Launcher tasks that finish or fail while a game with the companion runs, for example a mod install, arrive in the game as `toast`. Starting the game and moving the data folder do not.

## Server policy

Servers can switch Orrery modules off with a plugin message on the channel `orrery:policy`. The payload is one UTF-8 string with a JSON object:

```json
{ "disabled": ["zoom", "waypoints"] }
```

Module ids match `[a-z0-9_]{1,32}`, at most 64 are read, and the payload may be at most 4096 bytes. The policy applies until the player leaves the server. The Orrery menu (default key right Shift, because 26.3 binds O to the friends list) shows which modules the server switched off.

## Modules and HUD

Every feature of the mod is a module with an id. The Orrery menu lists them under Modules, and every module is off until the player switches it on. A module that the server switched off through `orrery:policy` is greyed out.

| Id              | Module                                                                              |
| --------------- | ----------------------------------------------------------------------------------- |
| `fps`           | Frames per second                                                                   |
| `coordinates`   | Block coordinates, biome and facing. Hidden when the server sets `reducedDebugInfo` |
| `clock`         | Local time                                                                          |
| `memory`        | Java heap use                                                                       |
| `server`        | Server address and ping, only on servers                                            |
| `keystrokes`    | Movement keys, both mouse buttons with clicks per second and the jump key           |
| `cps`           | Clicks per second for both mouse buttons, counted from the raw mouse input          |
| `durability`    | Armor and the items in both hands with their remaining durability                   |
| `effects`       | Active status effects with level and remaining time in the effect color             |
| `zoom`          | Smooth zoom while the zoom key (C) is held, mouse wheel changes the zoom level      |
| `crosshair`     | Own crosshair: cross, dot, cross with dot, circle or T, with length, gap, color     |
| `low_fire`      | Moves the burning overlay down so it covers less of the screen                      |
| `hand_position` | Moves the held item sideways, up or down and closer or further away                 |

| `chat` | Timestamps, stacking of repeated messages (`×3`) and a longer history (up to 1000) |

| `waypoints` | Waypoints per world or server and dimension, a compass bar with name and distance |
| `auto_reconnect` | Countdown and reconnect on the disconnect screen, at most five tries in a row |

Waypoints are set with B or in the waypoint screen behind the settings button of the module, and the death point replaces the previous one on every death. They live in `waypoints.json`, keyed by `world:<folder>` or `server:<address>`. Auto reconnect only knows the last server the player joined in this session, and the tries reset after a minute on the server.

The chat history in the Orrery menu lists the stored messages. A click copies a message to the clipboard. Vanilla has no way to find the chat line under the mouse, so copying lives in this screen instead of the chat itself.

Modules with options have a settings button next to their switch. The settings live in `modules.json` under `options`, and the crosshair settings show a live preview.

Zoom lowers the mouse sensitivity together with the field of view. Toggle sneak, toggle sprint, view bobbing and damage tilt are vanilla options, so the mod does not duplicate them.

HUD modules draw boxes on the game screen. The HUD editor (Orrery menu, Edit HUD) moves them by drag and drop and scales them with the mouse wheel from 0.5 to 2.5. Boxes snap to the screen edges, the screen centre and the edges of other boxes. In the editor every module shows a preview, also when it has nothing to show in the game right now. Positions are stored relative to the screen size, so the layout survives a different window size or GUI scale. The HUD is hidden together with the vanilla HUD (F1) and while the debug screen (F3) is open.

## Emotes

The emote key (default N, because 26.3 binds G to quick actions) opens a small menu with wave, cheer, clap, dance and point. Emotes only change the own player model on the own screen, nothing is sent to the server or to other players. In first person the camera switches to the front view while the emote plays and comes back afterwards. Walking away cancels the emote.

The companion has no cosmetics and no capes. It never changes how the player or other players look to anyone else, and the only capes in the game are the official ones from the player's Minecraft profile.

## Title screen

The mod replaces the vanilla title screen with the Orrery title screen. A mixin swaps the screen when the game opens a plain `TitleScreen`, so a mod that replaces the title screen with its own class wins, and demo mode keeps the vanilla screen.

- Greeting with the player name, continue card for the last world or server, tiles for Singleplayer and Multiplayer, buttons for Options, the Orrery menu and Quit
- Realms, language, accessibility, Mod Menu (when installed) and the classic menu sit in a popover behind More
- Background: a fixed image (`textures/gui/title/background.png`) with a slow pan and zoom and two drifting glows. Resource packs can replace the image under the same path
- The skin model of the active account on the right from 1.21.11 on. 1.20.1 has no skin widget
- Inter Medium and SemiBold (SIL Open Font License 1.1, `LICENSE-Inter.txt` in the jar) as TrueType fonts, anti-aliased rounded shapes drawn in physical pixels, hover animations and a short fade in
- A compact layout below 360 GUI pixels of height, which is the usual case with automatic GUI scale

The Orrery menu switches the Orrery title screen off. The classic menu then gets the extras: a continue button above Singleplayer, the Orrery button and the account. That part has its own switch, and the Orrery button always stays.

The background, the glow and the icons come from `cargo run --release -p orrery-core --example menu_assets`, which also builds the Orrery Menu pack.

## Files

The mod writes to `config/orrery-companion/` in the instance:

| File               | Content                                                                  |
| ------------------ | ------------------------------------------------------------------------ |
| `settings.json`    | `orreryTitleScreen` and `titleScreen` (classic menu extras), booleans    |
| `last-played.json` | `kind` (`world` or `server`), `target` (world folder or address), `name` |
| `modules.json`     | One object per module id with `enabled`, `x`, `y` (0 to 1) and `scale`   |

All files are written atomically. An unreadable file is ignored and logged.

## Fair play

- Only display and comfort features. Nothing automates movement or combat, changes brightness or the camera beyond vanilla, or reveals hidden information.
- Every module obeys `orrery:policy`.
- The mod makes no network connections of its own apart from the local bridge.

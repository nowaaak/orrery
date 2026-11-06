# Orrery

An independent, open-source launcher for Minecraft: Java Edition, built with Tauri 2, a Rust backend and a React + TypeScript frontend. It runs on Windows, macOS and Linux.

Website: [nowaaak.github.io/orrery](https://nowaaak.github.io/orrery/) · [Download](https://github.com/nowaaak/orrery/releases/latest) · [Privacy](https://nowaaak.github.io/orrery/privacy.html)

NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.

## Microsoft accounts

Playing requires a Microsoft account that owns Minecraft: Java Edition.

- Sign-in uses Microsoft's device code flow: Orrery shows a code, and you enter it on Microsoft's own page at `microsoft.com/link`. Orrery never sees your password or verification codes.
- Orrery requests only the `XboxLive.signin` and `offline_access` scopes. The Microsoft refresh token is kept in the system keychain. The Minecraft access token is kept in memory only.
- Every game start signs in again online: Xbox Live, XSTS, Minecraft services and Mojang's entitlement check, which must confirm that the account owns the game. The player name and UUID come from the verified profile. Without internet, or when the check fails, the game does not start.
- Orrery has no offline accounts, no username-only profiles, no demo mode and no alternative authentication services. There is no development or testing switch that skips the sign-in.
- Orrery uses its own Azure app registration, has no server of its own and sends account data only to Microsoft, Xbox Live and Minecraft services.

Mojang has not yet approved Orrery's Azure app registration for Minecraft services. Until it is approved, the sign-in stops after the Microsoft step with a message that says so, and no game can be started.

## Features

- Installs and launches every release and snapshot from Minecraft 1.8 onwards
- Verifies every download (SHA-1, SHA-256 or SHA-512) and writes files atomically
- Provides the matching Java runtime automatically (Mojang runtimes, Adoptium as fallback)
- Microsoft accounts via device code login, with skin upload and a choice between the capes your Minecraft profile owns. Orrery offers no other capes
- Microsoft refresh tokens live in the system keyring, never in JSON files
- Fabric, Legacy Fabric and Quilt as mod loaders through their meta APIs, Forge and NeoForge through their official installers
- Modrinth search and installation per instance for mods, resource packs and shaders, including required dependencies
- CurseForge search, installation, modpacks and updates in builds with a CurseForge API key. Files whose authors block third-party downloads are opened on CurseForge's website for you to download yourself
- Orrery Shaders, a built-in shader pack for Iris with soft shadows, a starry sky and clear water
- Orrery Menu, a built-in resource pack that gives the main menu a space panorama, own buttons and splash texts
- One-click performance pack, hardware based game options and a dedicated GPU preference on Windows
- Modrinth modpacks (`.mrpack`): install from Modrinth, import from a file and export an instance
- World manager with backups, restore and Quick Play straight into a world (1.20 and newer)
- Storage overview and cleanup of game files no instance needs anymore
- Command palette on Ctrl+K (⌘K on macOS)
- Movable data folder, for example to another drive, with verified copying across drives
- User interface in German and English
- Multiple instances with their own version, loader, mods, memory and window settings
- Live game log, task progress and cancellation
- Memory recommendation per instance based on system RAM, version, mods and shaders
- Crash detection with a likely cause, crash report access and log export
- Playtime tracking, mod updates through Modrinth and quick access to instance folders
- Signed auto updates through GitHub Releases
- Orrery Companion, an optional client mod for Fabric and Quilt that can be switched off per instance, see below

## Requirements

- Node.js 24 and npm
- Rust (stable, 1.99 or newer)
- Platform dependencies for Tauri 2, see the [Tauri prerequisites](https://tauri.app/start/prerequisites/)
    - Windows: Visual Studio Build Tools with the C++ workload and WebView2
    - Linux: `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`, `patchelf`, `libssl-dev`

## Development

```sh
npm install
npm run app
```

`npm run app` starts the development build as **Orrery Dev** with its own identifier (`io.github.nowaaak.orrery.dev`). It keeps its own data folder, accounts, keyring entries and WebView storage, so it never touches an installed Orrery. Development builds never look for or install launcher updates. A development build started with the release identifier (plain `npm run tauri dev`) refuses to start.

To use Orrery day to day, install the latest release from the [releases page](https://github.com/nowaaak/orrery/releases/latest). The installed version updates itself.

Useful scripts:

| Command                                                 | Purpose                                                |
| ------------------------------------------------------- | ------------------------------------------------------ |
| `npm run typecheck`                                     | TypeScript check                                       |
| `npm run lint`                                          | ESLint                                                 |
| `npm test`                                              | Frontend unit tests (Vitest)                           |
| `npm run bindings`                                      | Regenerate TypeScript types from Rust (`src/bindings`) |
| `cargo test --workspace`                                | Rust unit and integration tests                        |
| `cargo clippy --workspace --all-targets -- -D warnings` | Rust lints                                             |

The core crate ships a smoke test that installs and starts a real game outside the app:

```sh
cargo run -p orrery-core --example smoke -- <data-dir> 1.20.1 --launch
cargo run -p orrery-core --example smoke -- <data-dir> 1.21.1 --fabric --mod=sodium --launch
```

## Build configuration

These environment variables are read at compile time:

| Variable                  | Purpose                                                                                                                                                        |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `MS_CLIENT_ID`            | Azure application (client) id for the Microsoft login. Set in `.cargo/config.toml`, it is public and not a secret. Without it the Microsoft login is disabled. |
| `LAUNCHER_REPOSITORY_URL` | GitHub repository URL. Enables the auto updater and is added to the HTTP user agent.                                                                           |
| `DISCORD_CLIENT_ID`       | Application id of a Discord application for Rich Presence. It is public, set it in `.cargo/config.toml`. Without it Rich Presence is hidden.                   |

## Companion mod

Orrery Companion is Orrery's own client mod. The launcher loads it into Fabric and Quilt instances (1.20.1, 1.21.11 and 26.3) unless it is switched off in the instance settings. It never ends up in exported modpacks. It adds:

- The Orrery title screen in place of the vanilla one, with the classic menu one click away. The Minecraft logo and Mojang's textures are not changed or shipped
- The Orrery menu (right Shift) with display and comfort modules, all off by default: FPS, coordinates, clock, memory, server and ping, keystrokes, CPS, durability, status effects, zoom, crosshair editor, low fire, hand position, chat timestamps and history, and waypoints
- Emotes that only you see
- A local connection to the launcher for Discord Rich Presence, playtime per server, screenshots and task toasts

Servers can switch any module off through the `orrery:policy` plugin channel. The companion has no cosmetics, no capes and no features that automate movement or combat, change brightness or reveal hidden information.

`npm run companion` builds Orrery Companion for every supported Minecraft version and bundles it into the app. Details in [docs/companion.md](docs/companion.md).

## License

Orrery, including Orrery Companion, Orrery Shaders and Orrery Menu, is licensed under the [GNU General Public License v3.0 only](LICENSE). Bundled third-party fonts keep their own licenses, for example `companion/src/main/resources/LICENSE-Inter.txt`.

## Contact

Questions, bug reports and requests go to [GitHub issues](https://github.com/nowaaak/orrery/issues) or by email to [nxwaaak@gmail.com](mailto:nxwaaak@gmail.com).

## Documentation

- [Architecture](docs/architecture.md)
- [Releasing](docs/RELEASING.md)
- [Roadmap](docs/ROADMAP.md)

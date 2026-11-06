# Releasing

## One-time setup

1. Create the GitHub repository and push the code.
2. Add these repository secrets (Settings, Secrets and variables, Actions):
    - `TAURI_SIGNING_PRIVATE_KEY`: the content of the private updater key
    - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: its password (empty if none was set)

The Azure application id for the Microsoft login is committed in `.cargo/config.toml`. It is public by design. The login only works after Mojang has approved the app.

The updater public key is committed in `src-tauri/tauri.conf.json`. The matching private key was generated locally with `tauri signer generate`. Keep it outside the repository and keep a backup. Losing it means installed launchers can no longer verify updates.

To create a new key pair, run `npm run tauri signer generate -- -w <path>` and replace the `pubkey` in `tauri.conf.json`. Existing installations will then reject updates signed with the new key.

## Release steps

1. Update the version in `package.json` and in `[workspace.package]` of `Cargo.toml`. `tauri.conf.json` reads the version from `package.json`.
2. Add an entry to `CHANGELOG.md`.
3. Commit, then tag and push: `git tag v0.2.0 && git push origin v0.2.0`.
4. The `Release` workflow builds Windows (NSIS), macOS (Apple Silicon and Intel) and Linux (AppImage, deb) and creates a draft release with `latest.json`.
5. Check the draft and publish it. Publishing starts the `Pages` workflow, which writes the installers of the new release into `site/downloads.json` for the download section of the website. The updater reads `releases/latest/download/latest.json`, so only published releases are offered.

On Linux the auto updater only works with the AppImage.

Every release build runs `npm run licenses` before `npm run build` (`beforeBuildCommand` in `tauri.conf.json`). `scripts/licenses.mjs` collects the licenses of all Rust crates reachable from the workspace without dev dependencies (`cargo metadata --locked`) and of all npm packages that are not marked `dev` in `package-lock.json`, with the license files from their folders, and writes `public/licenses.json`. The file is not committed. About Orrery shows it, development builds show a hint until `npm run licenses` ran once.

The Windows installer images live in `src-tauri/icons/installer/`. `sidebar.bmp` (164 × 314) and `header.bmp` (150 × 57) are 24-bit BMPs rendered from the SVG files next to them. Change the SVG, render it at exactly that size and save it as a 24-bit BMP without alpha.

Only release builds check for and install updates. Development builds (`npm run app`) use the identifier `io.github.nowaaak.orrery.dev` and never update, so they can run next to an installed Orrery without touching it.

## Code signing for operating systems

The builds are not signed for Windows SmartScreen or macOS Gatekeeper, so users see a warning on first start. To add OS signing later, add the following secrets and pass them as environment variables to `tauri-action` in `.github/workflows/release.yml`:

- macOS: `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`
- Windows: configure `bundle.windows.signCommand` in `tauri.conf.json`, for example with Azure Trusted Signing

See the Tauri guides for [macOS signing](https://tauri.app/distribute/sign/macos/) and [Windows signing](https://tauri.app/distribute/sign/windows/).

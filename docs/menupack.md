# Orrery Menu

Orrery ships its own resource pack for the Minecraft main menu. The source lives in `menupack/` and is embedded into the launcher at build time, like the Orrery Shaders. It changes how the menu looks, not its layout. The companion mod adds a continue button, an Orrery button and the active account to the title screen, see [companion.md](companion.md).

## Content

| File in `menupack/assets/minecraft/`                                    | Used by          |
| ----------------------------------------------------------------------- | ---------------- |
| `textures/gui/title/background/panorama_0..5.png`: space panorama       | 1.8 and newer    |
| `texts/splashes.txt`: own splash texts                                  | 1.8 and newer    |
| `textures/gui/options_background.png`: tile behind option screens       | 1.8 to 1.20.4    |
| `textures/gui/sprites/widget/button*.png`, `slider*.png` with `.mcmeta` | 1.20.2 and newer |
| `textures/gui/menu_background.png`, `menu_list_background.png`          | 1.20.5 and newer |
| `textures/gui/header_separator.png`, `footer_separator.png`             | 1.20.5 and newer |

`menupack/pack.png` is the pack icon. A version ignores paths it does not know. Before 1.20.2 the buttons stay vanilla, because they live in `widgets.png` together with the hotbar and Orrery does not ship Mojang's pixels. The Minecraft logo is not changed.

## Assets

`cargo run --release -p orrery-core --example menu_assets` renders every PNG and writes it to `menupack/`. The output is deterministic, and the generated files are committed. The splash texts are written by hand.

- The panorama is a cube map rendered from a direction per pixel: star field, nebula in the theme colors, and a small orrery with a sun, two orbits and two planets. The face order matches vanilla: faces 0 to 3 turn right, face 4 is the sky and joins the top edge of face 0, face 5 is the ground and joins its bottom edge. This was checked against the edges of the vanilla panorama.
- Buttons copy the vanilla structure (1 px outline, light top and left edge, two shadow rows) in Orrery colors. The nine-slice borders in the `.mcmeta` files match the sprites.
- Since 1.20.5 the game blurs the panorama by default, so the look has to work blurred as well.

## Pack format

`pack.mcmeta` is not part of `menupack/`. The launcher writes it for each instance, because every Minecraft version expects its own format:

| Source                                                                                  | Written as                    |
| --------------------------------------------------------------------------------------- | ----------------------------- |
| `version.json` in the client jar, `pack_version` is a number                            | `pack_format`                 |
| `pack_version.resource` (1.20 to 1.21.8)                                                | `pack_format`                 |
| `pack_version.resource_major` (1.21.9 and newer)                                        | `min_format` and `max_format` |
| No `version.json` (before 1.14): 1.8 = 1, 1.9 and 1.10 = 2, 1.11 and 1.12 = 3, 1.13 = 4 | `pack_format`                 |

If the client jar is missing, for example before the first start of an instance, the pack gets format 15. Before every launch the launcher compares the stored `pack.mcmeta` with the one the version needs and rebuilds the zip when they differ. This also covers a changed Minecraft version of the instance.

## How the launcher installs it

`orrery_core::menupack` builds a deterministic zip and writes it to `resourcepacks/Orrery-Menu-<version>.zip`. The entry in `mods.json` uses the project id `orrery-menu`. Its `version_id` is `<version>+<hash of the embedded files>`, so any change to the assets is offered as an update.

Installing selects the pack in `options.txt`:

- 1.13 and newer use `file/Orrery-Menu-<version>.zip`, older versions the plain file name.
- The pack is placed directly after `vanilla` and `fabric`, so packs the player selected keep priority.
- Without `options.txt` the launcher writes one with the hardware based defaults (when that setting is on) and the pack list.
- An update replaces the old file name in place and never re-selects a pack the player turned off. An unreadable pack list is left untouched.

## Development

1. Change the generator or `splashes.txt`, then run the generator.
2. Raise `MENUPACK_VERSION` in `crates/orrery-core/src/menupack.rs` when a release changes the pack, so the file name changes with it.
3. `cargo test -p orrery-core --lib menupack` checks the assets, the format logic and the `options.txt` handling.
4. `cargo run --release -p orrery-core --example smoke -- <dir> <version> --orrery-menu --launch --wait=40` installs the pack into a test instance and starts the game. `logs/latest.log` must list the pack in `Reloading ResourceManager`.

`build.rs` only accepts `assets/` and `pack.png` in `menupack/` and stops the build for anything else, including a `pack.mcmeta`.

## License

Only own code and own generated images. Vanilla textures are used as a reference for sizes and structure, never copied.

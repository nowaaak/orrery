# Mod loaders

Fabric, Legacy Fabric and Quilt profiles come ready from their meta APIs (`loaders/mod.rs`). Forge and NeoForge only ship installers, so Orrery runs the installer steps itself (`loaders/installer.rs`, `loaders/processors.rs`). The version lists live in `loaders/forge.rs` and `loaders/neoforge.rs`.

## Legacy Fabric

Legacy Fabric brings the Fabric loader to Minecraft 1.13.2 and older, for example 1.8.9. Its profiles come from `meta.legacyfabric.net/v2` in the same format as Fabric's, so they are stored the same way.

- The libraries only name a repository. Their SHA-1 is read from the `.sha1` file next to each jar, as for Fabric
- Legacy Fabric replaces LWJGL 2 with its own build, whose natives have no main jar. For a natives library Orrery reads the SHA-1 of every native jar instead, so the natives are checked as well
- Modrinth lists these mods under the loader `legacy-fabric`
- A `.mrpack` with `fabric-loader` for Minecraft 1.13.2 or older becomes a Legacy Fabric instance, because Fabric itself starts at 1.14. Exports write `fabric-loader` as Modrinth does
- The Orrery Companion has no build for these versions, so the switch is hidden

## Versions

- NeoForge: `maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge`. `20.2.x` belongs to Minecraft 1.20.2, `21.0.x` to 1.21, `21.1.x` to 1.21.1, and from 26 on the first two or three parts are the Minecraft version (`26.1.2.126` is 26.1.2, `26.3.0.101` is 26.3). Builds for snapshots (`+snapshot`) and April fools versions are hidden. Versions with a suffix such as `-beta` are marked as beta
- Forge: `maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml`, entries look like `1.20.1-47.4.28`, for 1.8.9 often `1.8.9-11.15.1.2318-1.8.9`. The repeated Minecraft version is dropped from the shown version and looked up in the list again before the download. Minecraft 1.8.9, 1.12.2 and 1.17 and newer are offered
- Creating an instance only checks that the version exists. The install runs on the first start or repair

## Forge's request

Every Forge installer says: "Please do not automate the download and installation of Forge. Our efforts are supported by ads from the download page." Like Prism Launcher and the Modrinth App, Orrery downloads Forge straight from the official Maven anyway. When Forge is selected, the version picker shows this request together with links to the Forge website and Forge's Patreon.

## Install

1. The vanilla version is installed first, including its Java runtime
2. The installer jar is downloaded from the official Maven and checked against its `.sha1` file
3. `install_profile.json` and `version.json` are read from the jar. Both must name the expected loader and Minecraft version
4. Libraries the installer ships below `maven/` are extracted and checked against their SHA-1 when the profile names one
5. Every other library must come over HTTPS from the loader's Maven (`maven.minecraftforge.net` or `maven.neoforged.net`) or from `libraries.minecraft.net` and is checked by SHA-1
6. Data files of the installer (`data/client.lzma`) are extracted into `cache/loaders/<id>` and removed afterwards
7. The client processors run with the managed Java of the Minecraft version, without a console window. Their output goes to `logs/loaders/<id>.log`. Server-only processors are skipped, as are processors whose declared outputs already match
8. `DOWNLOAD_MOJMAPS` is not run as a processor: Orrery downloads the client mappings from Mojang's version file itself and checks their SHA-1, so no processor needs network access
9. Declared processor outputs are checked by SHA-1. After the last processor every generated file and every library without a download URL must exist
10. The profile goes to `meta/versions/<id>.json` and the list of generated files to `meta/loaders/<id>.json`

Installs run one after another, so two instances with the same version never write the same files at once. The next start only checks that the profile and the generated files are there. A repair runs the processors again.

Profile ids follow the installers: `neoforge-21.1.265` and `1.20.1-forge-47.4.28`.

## Legacy Forge installers

Forge for 1.8.9 and older 1.12.2 builds ships a different `install_profile.json` with an `install` and a `versionInfo` part and no processors. Newer 1.12.2 builds already use the format above, just without processors.

1. `install.filePath` names the universal jar at the root of the installer. It is extracted to the library path of `install.path`
2. `versionInfo` becomes the profile. Libraries marked `"clientreq": false` are server only and skipped
3. The other libraries only name a repository. Orrery reads their SHA-1 from the `.sha1` file next to the jar and, when the profile lists `checksums`, accepts the file only if its SHA-1 is one of them. `http://files.minecraftforge.net/maven/` is replaced by `https://maven.minecraftforge.net/`, libraries without a repository come from `libraries.minecraft.net`
4. The profile id is changed to `1.8.9-forge-11.15.1.2318`, and the universal jar is listed in `meta/loaders/<id>.json`

## Start

The official launcher puts a copy of the vanilla client jar into `versions/<id>/<id>.jar`, and Forge and NeoForge exclude that file name from their module layer (`-DignoreList`). Orrery does the same for both. Fabric and Quilt keep using the shared vanilla jar.

Forge profiles contain an empty `logging` block. The merge keeps the log configuration of the vanilla version in that case.

## Storage cleanup

Processor outputs are not listed in any version file. The cleanup keeps every file named in `meta/loaders/<id>.json` of a used profile and removes the lists of unused ones.

## Mods, modpacks and imports

- Modrinth search, mod updates and the smart packs ask Modrinth for versions of the instance's loader (`fabric`, `legacy-fabric`, `quilt` plus `fabric`, `forge`, `neoforge`)
- Dropped mod jars are sorted by their metadata: `fabric.mod.json` is a Fabric mod (also fine for Legacy Fabric and Quilt), `quilt.mod.json` a Quilt mod, `META-INF/neoforge.mods.toml` a NeoForge mod, `META-INF/mods.toml` a Forge mod unless it depends on `neoforge` (NeoForge 20.2 to 20.4 still used that file), `mcmod.info` an old Forge mod. A mod for another loader is rejected with a message
- `.mrpack` files with a `forge` or `neoforge` dependency create Forge and NeoForge instances
- Imports from other launchers keep Forge, NeoForge and Legacy Fabric, see [import.md](import.md)

## OptiFine

OptiFine may not be redistributed and its downloads go through an ad page, so the player downloads the jar from optifine.net. Orrery then sets it up as a Forge mod, which is how OptiFine is meant to run with Forge.

- An OptiFine jar is recognised by its `optifine/` or `net/optifine/` package. The version comes from the constant `OptiFine_1.20.1_HD_U_I6` that Java stores as one string in `Config.class`, so a renamed file still works. Only if that string is missing, the file name is used
- Dropping the jar onto the window outside an instance, or choosing it in the import dialog, opens a dialog: a new Forge instance for the same Minecraft version (Forge version selectable, newest first) or an existing Forge instance with that version
- Dropped onto an instance, OptiFine is only accepted in Forge instances with the same Minecraft version. Fabric and Vanilla instances get a pointer to the OptiFine replacement pack from M14
- The first section of `changelog.txt` in the jar names the Forge version OptiFine was tested with ("compatible with Forge 47.2.18"). The dialog preselects it when Orrery can install it, otherwise the newest Forge version, and links the download page
- The version classes sit below `notch/net/optifine/` and `srg/net/optifine/` in current jars, so every `Config.class` in an `optifine` package is searched

## Not yet supported

- The Orrery Companion only runs on Fabric and Quilt, so the switch is hidden for Forge and NeoForge instances
- The smart packs and the OptiFine replacement are put together for Fabric. On Forge and NeoForge they only install the projects that have a matching version
- Forge for Minecraft 1.7.10 and for 1.9 to 1.16.5

## Tested

- NeoForge 21.1.265 for 1.21.1 (ten processors, six of them for the client) and 26.3.0.101 for 26.3 (two processors, one for the client) install on Windows through `cargo run -p orrery-core --example smoke -- <dir> <version> --neoforge --prepare`. Both started with a local development build up to the title screen, the log shows NeoForge mod loading for the right Minecraft version and no loader errors
- Forge 47.4.28 for 1.20.1, 52.1.21 for 1.21.1 and 66.1.5 for 26.3 install on Windows the same way with `--forge`. 1.20.1 and 26.3 started up to the title screen without loader errors
- Forge 14.23.5.2864 for 1.12.2 and 11.15.1.2318 for 1.8.9 install and start with Java 8 up to the title screen. 1.12.2 logs that FML has no signature data, because Forge no longer signs these builds
- OptiFine 1.20.1 HD U I6 from optifine.net is recognised, also renamed, and loads in Forge 47.2.18 up to the title screen
- Legacy Fabric 0.20.1 for 1.8.9 installs with `--legacy-fabric` and starts with Java 8 up to the title screen without errors
- Not tested yet: a mod from Modrinth in game, macOS and Linux, Forge between 1.17 and 1.19, NeoForge between 1.20.2 and 1.21

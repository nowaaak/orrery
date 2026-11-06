# Import from other launchers

The Import button on the instance page and the drop zone copy instances from other launchers into new Orrery instances. The source is never changed.

## Sources

| Launcher           | Found at                                                                                                                            | Metadata                                                                                     |
| ------------------ | ----------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| Prism Launcher     | `PrismLauncher` in the user data folder and Flatpak, `PRISMLAUNCHER_DATA_DIR`, `InstanceDir` from `prismlauncher.cfg`               | `instance.cfg` (Qt INI), `mmc-pack.json`                                                     |
| MultiMC            | Portable, choose the folder. `~/.local/share/multimc` on Linux                                                                      | `instance.cfg` (old flat INI), `mmc-pack.json`                                               |
| Modrinth App       | `ModrinthApp/app.db` or `THESEUS_CONFIG_DIR`, both the schema before 0.15 and the one from 0.15, and the old Theseus `profile.json` | SQLite, read only                                                                            |
| CurseForge         | `curseforge/minecraft/Instances` in the user and the documents folder                                                               | `minecraftinstance.json`                                                                     |
| ATLauncher         | `ATLauncher/instances` and `atlauncher/instances`                                                                                   | `instance.json`                                                                              |
| GDLauncher         | Legacy `gdlauncher_next/instances`, Carbon `gdlauncher_carbon/data/instances`, both also when moved elsewhere                       | `config.json` or `instance.json`                                                             |
| Minecraft Launcher | `.minecraft` (on macOS `minecraft`), Flatpak                                                                                        | `launcher_profiles.json` and `launcher_profiles_microsoft_store.json`, one entry per profile |

Any other folder or `.zip` archive can be chosen or dropped. Orrery looks for an instance in the folder itself, in `instances`, `Instances` or `profiles` below it, and in the single top folder of an archive.

## What is copied

- The whole game folder, except logs, crash reports, debug output, loader caches and natives
- From the official launcher only worlds, resource packs, shader packs, screenshots, `options.txt` and `servers.dat`, plus `mods` and `config` for profiles with a mod loader. The shared `versions`, `libraries` and `assets` stay where they are
- Maximum memory when it lies between 512 MB and 64 GB, minimum memory when it lies between 256 MB and that maximum

Java paths, JVM arguments and commands before launch, after exit or as wrapper are never taken over. The preview lists them, so nothing is lost silently. It also warns when the Minecraft version cannot be determined.

Mods are identified on Modrinth by their hash afterwards, like after a modpack import. Without network access the import still finishes.

## Safety

- Every copied path is checked like the overrides of a `.mrpack`: no `..`, no absolute paths, no drive letters, no reserved Windows names
- Symbolic links and Windows junctions are skipped and counted in the preview. Unsafe paths and folders nested deeper than 32 levels in a folder import are skipped and counted, too
- A folder with more than 200,000 files is refused. An archive is rejected above 16 GB uncompressed or 200,000 entries
- An archive is rejected as a whole as soon as one entry has an unsafe path. Nothing is extracted then
- Archives are extracted below `cache/imports` and removed after the import. Leftovers older than a day are removed before the next archive is read

## Limits

- Fabric, Quilt, Forge and NeoForge are taken over with their version. Fabric on Minecraft 1.13.2 and older becomes Legacy Fabric. Forge versions are read in every spelling the launchers use (`47.2.0`, `1.20.1-47.2.0`, `forge-47.2.0`, `1.8.9-forge1.8.9-11.15.1.2318-1.8.9`)
- LiteLoader, Rift, NeoForge for 1.20.1 and Forge on Minecraft versions Orrery does not install (1.7.10, 1.9 to 1.16.5) become Vanilla instances with the same Minecraft version. Worlds, resource packs and settings are kept, and the preview says so
- Profiles of the official launcher that follow the latest release or snapshot get the current version from Mojang's manifest
- The formats were taken from the launchers' source code. Real exports of every launcher have not been tested yet, only the Prism format end to end

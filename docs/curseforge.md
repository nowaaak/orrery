# CurseForge

CurseForge is a second content source next to Modrinth. It needs an API key from the [CurseForge developer console](https://console.curseforge.com/), so builds without a key hide everything that belongs to it.

## API key

- `CURSEFORGE_API_KEY` is read with `option_env!` when `src-tauri` is compiled and passed to the core through `ServiceConfig`. The release workflow takes it from the repository secret of the same name
- A missing, empty or blank key, or one with characters that are not allowed in an HTTP header, counts as no key. `curseforge_available` then returns `false` and the source switch, the modpack tab and the `.zip` export stay hidden
- The key only travels in the `x-api-key` header, marked as sensitive, so it never appears in URLs, error messages or logs. A test checks that a failed request does not contain it
- Downloads from the CurseForge CDN use a second HTTP client that sends the key as CurseForge asks for since its announcement of key checks on `edge.forgecdn.net`. That client follows redirects only to `edge.forgecdn.net` and `mediafilez.forgecdn.net` over HTTPS and stops at any other host, and files found on Modrinth are downloaded with the normal client, so the key never reaches another server
- A key compiled into a program that is handed out can be read from the program. CurseForge expects this for launchers and lets the key be replaced in the console. It is never part of the repository

## Endpoints

| Use                       | Endpoint                                                                                 |
| ------------------------- | ---------------------------------------------------------------------------------------- |
| Search                    | `GET /v1/mods/search` with `gameId=432`, `classId`, `gameVersion` and `modLoaderType`    |
| Project and projects      | `GET /v1/mods/{id}`, `POST /v1/mods`                                                     |
| Files of a project        | `GET /v1/mods/{id}/files`, 50 per page, at most 20 pages                                 |
| Files by id               | `POST /v1/mods/files`                                                                    |
| Identify files for export | `POST /v1/fingerprints`                                                                  |
| Update check              | `latestFilesIndexes` from `POST /v1/mods`, then `POST /v1/mods/files` for the names only |

The documentation does not list the class ids or the fingerprint. Orrery uses the same values as Prism Launcher: mods 6, resource packs 12, shaders 6552 and modpacks 4471. The fingerprint is MurmurHash2 with seed 1 over the file without the bytes 9, 10, 13 and 32. A test checks the hash against the SMHasher verification value.

Search sorts by popularity and stops at 10,000 results, the limit of the API. Fabric and Quilt instances search Fabric mods, because most mods that run on Quilt are published for Fabric. File lists are filtered by the loader names in `gameVersions`, so Quilt accepts both Quilt and Fabric files.

## Installed content

Content from CurseForge has the project id `curseforge-<id>` in `mods.json` and the file id as version id. Install, update, removal and undo recognise the source by this prefix, so only the search needs to be told which source to use. Modrinth update checks skip these entries.

Required dependencies are installed with the project. A dependency is skipped when it is already installed from CurseForge or when content of the same type with the same title is installed from Modrinth, so Fabric API or JEI does not end up twice.

## Downloads

Files are only downloaded from `edge.forgecdn.net` and `mediafilez.forgecdn.net` over HTTPS and checked against the SHA-1 from CurseForge. When an author blocks downloads through other apps, the API returns no download URL. Orrery then asks Modrinth for a file with the same SHA-1 and takes it from `cdn.modrinth.com` with the SHA-512 from Modrinth. A single mod whose file is on neither platform stops with a message that it can only be downloaded on the CurseForge website.

## Modpacks

A CurseForge modpack is a `.zip` with `manifest.json` (`manifestType` `minecraftModpack`, `manifestVersion` 1). It is installed from the modpack dialog, imported from a file or dropped onto the window.

- The loader comes from the primary entry of `minecraft.modLoaders`, read the same way as in the CurseForge importer. Packs for a loader or Minecraft version that Orrery cannot install are rejected before an instance is created
- Only files with `required` are installed. The project class decides the folder. Files of other classes, for example worlds, are skipped with a log entry
- The overrides folder named in the manifest is extracted with the same path checks and size limit as `.mrpack` overrides
- Content in the overrides is looked up on Modrinth by hash afterwards. Files that are already registered from CurseForge are not added a second time
- If anything fails, the new instance is removed again

From the search Orrery takes the newest release that is not a server pack, otherwise the newest beta. The pack itself must be downloadable through the API.

### Blocked files

Files that are neither available through the API nor on Modrinth go into `pending-downloads.json` in the instance folder. The rest of the pack is installed, so the instance exists and the list survives a restart.

The content tab shows a card for these files with a link to each download page on `www.curseforge.com`, which is the only CurseForge address the opener may open. While the card is visible, Orrery checks the downloads folder every four seconds and whenever the window gets focus. A file counts as found when its size and SHA-1 match, so a browser renaming it to `name (1).jar` does not matter. Without a size from CurseForge the file name has to match as well. Found files are copied, not moved, and registered like installed content. Each entry can also be picked by hand, which uses the same check, or skipped. The list is not touched while the game runs.

## Export

The instance menu exports a `.zip` when the save dialog gets that extension. Every file in `mods`, `resourcepacks` and `shaderpacks` is identified by its fingerprint and written as `projectID` and `fileID`. Files that CurseForge does not know are left out and named in the result, like the `.mrpack` export does with files that are not on Modrinth. `config`, `defaultconfigs`, `kubejs`, `scripts`, `resources` and `structures` go into `overrides`, the same folders as in the `.mrpack` export. Instances with Legacy Fabric cannot be exported, because CurseForge has no such loader.

## Not covered yet

- Sync packs only search Modrinth
- A dropped single jar is only identified on Modrinth

## Verified

RLCraft (Forge 1.12.2) installed with a real key and loaded all 183 mods up to its main menu. Two of its files block third-party downloads and are on neither platform. They were listed for the manual download, downloaded in the browser, picked up from the downloads folder and loaded with the rest, 185 mods in total. Waystones with Balm installed into Forge 1.20.1 and loaded. Update check, export and the rejection of a wrong file for a blocked entry were checked on the same instances. Search for mods, modpacks, resource packs and shaders returned results through the app with the same key.

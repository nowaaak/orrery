import type { OptifineFeature } from "@/bindings/OptifineFeature";
import type { ContentKind } from "@/bindings/ContentKind";

type KindText = { tab: string; empty: string; none: string; placeholder: string; scope: string };

export const de = {
    kinds: {
        mod: {
            tab: "Mods",
            empty: "Noch keine Mods installiert.",
            none: "Keine passenden Mods gefunden.",
            placeholder: "z. B. Sodium, Iris, Mod Menu",
            scope: "Mods",
        },
        resource_pack: {
            tab: "Ressourcenpakete",
            empty: "Noch keine Ressourcenpakete installiert.",
            none: "Keine passenden Ressourcenpakete gefunden.",
            placeholder: "z. B. Faithful, Fresh Animations",
            scope: "Ressourcenpakete",
        },
        shader: {
            tab: "Shader",
            empty: "Noch keine Shader installiert.",
            none: "Keine passenden Shader gefunden.",
            placeholder: "z. B. Complementary, BSL",
            scope: "Iris-Shader",
        },
    } satisfies Record<ContentKind, KindText>,
    needsLoader:
        "Mods und Shader brauchen Fabric oder Quilt. Stelle den Modloader in den Einstellungen um.",
    irisHintBefore: "Shader brauchen die Mod ",
    irisHintAfter: ". Ohne sie werden Shader im Spiel nicht angezeigt.",
    installIris: "Iris installieren",
    installIrisFailed: "Iris konnte nicht installiert werden",
    orreryShaders: {
        description:
            "Der eigene Shader von Orrery: Sternenhimmel, weiche Schatten und klares Wasser. Iris wird bei Bedarf mitinstalliert.",
        install: "Installieren und aktivieren",
        failed: "Orrery Shaders konnten nicht installiert werden",
    },
    orreryMenu: {
        description:
            "Das eigene Hauptmenü von Orrery: Weltraum-Panorama, Buttons und Hintergründe im Orrery-Look. Läuft auch ohne Mods.",
        install: "Installieren und aktivieren",
        failed: "Orrery Menu konnte nicht installiert werden",
    },
    updatesFailed: "Updates fehlgeschlagen",
    installed: "Installiert",
    closeGameToChange: "Beende das Spiel, um Mods zu ändern.",
    disabledHint: "Deaktivierte Einträge bleiben gespeichert, werden aber nicht geladen.",
    checkingUpdates: "Suche Updates …",
    updateCount: (count: number) => `${count} aktualisieren`,
    allUpToDate: "Alles aktuell",
    removed: (title: string) => `${title} entfernt`,
    undo: "Rückgängig",
    removeFailed: "Entfernen fehlgeschlagen",
    dependency: "Abhängigkeit",
    mismatch: {
        version: "Andere Minecraft-Version",
        loader: "Anderer Mod-Loader",
    },
    mismatchHint: (versions: string) => `Gebaut für ${versions}`,
    localFile: "Lokale Datei",
    enableItem: (title: string) => `${title} aktivieren`,
    disableItem: (title: string) => `${title} deaktivieren`,
    toggleFailed: "Umschalten fehlgeschlagen",
    pinItem: (title: string) => `Updates für ${title} sperren`,
    unpinItem: (title: string) => `Updates für ${title} erlauben`,
    pinHint: "Updates sperren: Orrery bietet für diesen Inhalt keine Updates mehr an",
    pinnedHint: "Updates gesperrt. Klicken, um wieder Updates zu erlauben",
    pinFailed: "Sperre konnte nicht gespeichert werden",
    removeItem: (title: string) => `${title} entfernen`,
    remove: "Entfernen",
    searchSource: (source: string) => `${source} durchsuchen`,
    source: "Quelle",
    fromCurseForge: "CurseForge",
    searchScope: (scope: string, version: string) => `${scope} für Minecraft ${version}.`,
    searchMods: "Mods suchen",
    installFailed: "Installation fehlgeschlagen",
    packs: {
        title: "Pakete",
        description:
            "Mehrere Mods auf einmal. Fehlt eine Mod für diese Minecraft-Version, wird sie übersprungen.",
        items: {
            performance: {
                name: "Performance-Paket",
                description: "Mehr FPS und weniger Arbeitsspeicher.",
            },
            optifine: {
                name: "OptiFine-Ersatz",
                description:
                    "Shader, verbundene Texturen, eigene Modelle, Zoom und dynamisches Licht ohne OptiFine.",
            },
            shader_ready: {
                name: "Shader-bereit",
                description: "Alles, was Shaderpakete brauchen.",
            },
            low_end: {
                name: "Schwacher PC",
                description: "Für ältere Laptops und integrierte Grafik.",
            },
            quality_of_life: {
                name: "Komfort",
                description: "Modliste, Hungeranzeige, Block-Infos und bessere Inventare.",
            },
        } as Record<string, { name: string; description: string }>,
        complete: "Ergänzen",
        removePack: (name: string) => `${name} entfernen`,
        removed: (name: string, count: number) =>
            count === 1 ? `${name}: 1 Mod entfernt` : `${name}: ${count} Mods entfernt`,
        removeFailed: "Paket konnte nicht entfernt werden",
        alreadyInstalled: (names: string) => `Bereits installiert: ${names}.`,
        unavailable: (names: string) => `Für diese Version nicht verfügbar: ${names}.`,
        installing: (name: string, count: number) =>
            count === 1
                ? `${name}: 1 Mod wird installiert`
                : `${name}: ${count} Mods werden installiert`,
        nothingToInstall: "Nichts zu installieren",
        failed: "Paket konnte nicht installiert werden",
    },
    optifine: {
        title: (name: string) => `${name} nutzt OptiFine-Funktionen`,
        features: (names: string) => `Enthält: ${names}.`,
        needs: (mods: string) => `Damit sie angezeigt werden, fehlen: ${mods}.`,
        needsLoader:
            "Diese Funktionen gehen nur in Fabric- oder Quilt-Instanzen mit den passenden Mods.",
        names: {
            connected_textures: "verbundene Texturen",
            entity_models: "eigene Kreaturenmodelle",
            random_entities: "zufällige Kreaturentexturen",
            emissive: "leuchtende Texturen",
            item_textures: "eigene Gegenstandstexturen",
            sky: "eigener Himmel",
            animations: "Animationen",
        } satisfies Record<OptifineFeature, string>,
    },
    modpacks: {
        button: "Modpack",
        progressHint: "Den Fortschritt siehst du unten links.",
        installing: (title: string) => `${title} wird installiert`,
        installFailed: "Modpack konnte nicht installiert werden",
        fileFilter: "Modpack",
        importing: "Modpack wird importiert",
        importFailed: "Import fehlgeschlagen",
        title: "Modpack installieren",
        description:
            "Fabric- und Quilt-Modpacks von Modrinth. Jedes Modpack wird eine eigene Instanz.",
        descriptionCurseForge:
            "Modpacks von CurseForge mit Forge, NeoForge, Fabric oder Quilt. Jedes Modpack wird eine eigene Instanz.",
        placeholder: "z. B. Fabulously Optimized, Cobblemon",
        search: "Modpacks suchen",
        none: "Keine passenden Modpacks gefunden.",
        starting: "Startet …",
        importFromFile: "Aus Datei importieren …",
    },
    pending: {
        title: (count: number) =>
            count === 1 ? "1 Datei fehlt noch" : `${count} Dateien fehlen noch`,
        description:
            "Die Autoren dieser Projekte erlauben keine Downloads über andere Apps. Lade die Dateien auf CurseForge herunter. Orrery findet sie danach im Downloads-Ordner und übernimmt sie.",
        check: "Jetzt prüfen",
        open: "Download-Seite",
        choose: "Datei wählen …",
        skip: "Überspringen",
        added: (title: string) => `${title} übernommen`,
        wrongFile: "Datei passt nicht",
        skipped: (title: string) => `${title} übersprungen`,
        skipFailed: "Konnte nicht übersprungen werden",
    },
};

export const en: typeof de = {
    kinds: {
        mod: {
            tab: "Mods",
            empty: "No mods installed yet.",
            none: "No matching mods found.",
            placeholder: "e.g. Sodium, Iris, Mod Menu",
            scope: "Mods",
        },
        resource_pack: {
            tab: "Resource packs",
            empty: "No resource packs installed yet.",
            none: "No matching resource packs found.",
            placeholder: "e.g. Faithful, Fresh Animations",
            scope: "Resource packs",
        },
        shader: {
            tab: "Shaders",
            empty: "No shaders installed yet.",
            none: "No matching shaders found.",
            placeholder: "e.g. Complementary, BSL",
            scope: "Iris shaders",
        },
    },
    needsLoader: "Mods and shaders need Fabric or Quilt. Switch the mod loader in the settings.",
    irisHintBefore: "Shaders need the ",
    irisHintAfter: " mod. Without it, shaders are not shown in the game.",
    installIris: "Install Iris",
    installIrisFailed: "Could not install Iris",
    orreryShaders: {
        description:
            "Orrery's own shader: starry skies, soft shadows and clear water. Iris is installed along with it if needed.",
        install: "Install and activate",
        failed: "Could not install Orrery Shaders",
    },
    orreryMenu: {
        description:
            "Orrery's own main menu: a space panorama, buttons and backgrounds in the Orrery look. Works without mods too.",
        install: "Install and activate",
        failed: "Could not install Orrery Menu",
    },
    updatesFailed: "Could not install updates",
    installed: "Installed",
    closeGameToChange: "Close the game to change mods.",
    disabledHint: "Disabled entries stay installed but are not loaded.",
    checkingUpdates: "Checking for updates …",
    updateCount: (count) => `Update ${count}`,
    allUpToDate: "All up to date",
    removed: (title) => `${title} removed`,
    undo: "Undo",
    removeFailed: "Could not remove",
    dependency: "Dependency",
    mismatch: {
        version: "Other Minecraft version",
        loader: "Other mod loader",
    },
    mismatchHint: (versions) => `Built for ${versions}`,
    localFile: "Local file",
    enableItem: (title) => `Enable ${title}`,
    disableItem: (title) => `Disable ${title}`,
    toggleFailed: "Could not enable or disable",
    pinItem: (title) => `Pin ${title} and skip its updates`,
    unpinItem: (title) => `Unpin ${title} and allow updates`,
    pinHint: "Pin: Orrery no longer offers updates for this content",
    pinnedHint: "Pinned, updates are skipped. Click to allow updates again",
    pinFailed: "Could not save the pin",
    removeItem: (title) => `Remove ${title}`,
    remove: "Remove",
    searchSource: (source) => `Search ${source}`,
    source: "Source",
    fromCurseForge: "CurseForge",
    searchScope: (scope, version) => `${scope} for Minecraft ${version}.`,
    searchMods: "Search mods",
    installFailed: "Installation failed",
    packs: {
        title: "Packs",
        description:
            "Several mods at once. Mods that do not exist for this Minecraft version are skipped.",
        items: {
            performance: {
                name: "Performance pack",
                description: "More FPS and less memory.",
            },
            optifine: {
                name: "OptiFine replacement",
                description:
                    "Shaders, connected textures, custom models, zoom and dynamic lights without OptiFine.",
            },
            shader_ready: {
                name: "Shader ready",
                description: "Everything shader packs need.",
            },
            low_end: {
                name: "Low-end PC",
                description: "For older laptops and integrated graphics.",
            },
            quality_of_life: {
                name: "Quality of life",
                description: "Mod list, hunger display, block info and better inventories.",
            },
        },
        complete: "Complete",
        removePack: (name) => `Remove ${name}`,
        removed: (name, count) =>
            count === 1 ? `${name}: removed 1 mod` : `${name}: removed ${count} mods`,
        removeFailed: "Could not remove the pack",
        alreadyInstalled: (names) => `Already installed: ${names}.`,
        unavailable: (names) => `Not available for this version: ${names}.`,
        installing: (name, count) =>
            count === 1 ? `${name}: installing 1 mod` : `${name}: installing ${count} mods`,
        nothingToInstall: "Nothing to install",
        failed: "Could not install the pack",
    },
    optifine: {
        title: (name) => `${name} uses OptiFine features`,
        features: (names) => `Contains: ${names}.`,
        needs: (mods) => `To show them, these are missing: ${mods}.`,
        needsLoader: "These features only work in Fabric or Quilt instances with the right mods.",
        names: {
            connected_textures: "connected textures",
            entity_models: "custom entity models",
            random_entities: "random entity textures",
            emissive: "emissive textures",
            item_textures: "custom item textures",
            sky: "custom sky",
            animations: "animations",
        },
    },
    modpacks: {
        button: "Modpack",
        progressHint: "You can follow the progress in the bottom left.",
        installing: (title) => `Installing ${title}`,
        installFailed: "Could not install the modpack",
        fileFilter: "Modpack",
        importing: "Importing modpack",
        importFailed: "Import failed",
        title: "Install modpack",
        description:
            "Fabric and Quilt modpacks from Modrinth. Each modpack becomes its own instance.",
        descriptionCurseForge:
            "Modpacks from CurseForge with Forge, NeoForge, Fabric or Quilt. Each modpack becomes its own instance.",
        placeholder: "e.g. Fabulously Optimized, Cobblemon",
        search: "Search modpacks",
        none: "No matching modpacks found.",
        starting: "Starting …",
        importFromFile: "Import from file …",
    },
    pending: {
        title: (count) =>
            count === 1 ? "1 file is still missing" : `${count} files are still missing`,
        description:
            "The authors of these projects do not allow downloads through other apps. Download the files on CurseForge. Orrery then finds them in your Downloads folder and adds them.",
        check: "Check now",
        open: "Download page",
        choose: "Choose file …",
        skip: "Skip",
        added: (title) => `Added ${title}`,
        wrongFile: "This file does not match",
        skipped: (title) => `Skipped ${title}`,
        skipFailed: "Could not skip the file",
    },
};

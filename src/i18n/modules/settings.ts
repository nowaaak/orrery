import type { DataDirProblem } from "@/bindings/DataDirProblem";
import type { Language } from "@/bindings/Language";
import type { LauncherWindow } from "@/bindings/LauncherWindow";
import type { PresenceLevel } from "@/bindings/PresenceLevel";
import type { StorageCategory } from "@/bindings/StorageCategory";
import type { Theme } from "@/bindings/Theme";

export const de = {
    title: "Einstellungen",
    saved: "Einstellungen gespeichert",
    saveFailed: "Speichern fehlgeschlagen",
    general: "Allgemein",
    generalDescription: "Standardwerte für neue Instanzen und Downloads.",
    theme: "Design",
    themes: {
        system: "System",
        light: "Hell",
        dark: "Dunkel",
    } satisfies Record<Theme, string>,
    language: "Sprache",
    languageLocked: "Erst speichern oder verwerfen",
    languages: {
        system: "Systemsprache",
        de: "Deutsch",
        en: "English",
    } satisfies Record<Language, string>,
    minMemory: "Minimaler RAM",
    maxMemory: "Maximaler RAM",
    systemMemory: (gb: number) => `Dein PC hat ${gb} GB RAM. `,
    memoryHint:
        "Für Vanilla reichen meist 2 bis 4 GB, mit Mods 4 bis 8 GB. Jede Instanz zeigt in ihren Einstellungen eine passende Empfehlung.",
    memoryRecommended: "empfohlen",
    memoryTight: "knapp",
    downloads: "Parallele Downloads",
    snapshots: "Snapshots anzeigen",
    snapshotsDescription: "Entwicklungsversionen in der Versionsauswahl einblenden.",
    gameOptions: "Optimierte Spieleinstellungen",
    gameOptionsDescription:
        "Beim ersten Start einer Instanz Sichtweite und Effekte passend zu deinem PC einstellen. Vorhandene Einstellungen bleiben unberührt.",
    dedicatedGpu: "Leistungsstarke Grafikkarte",
    dedicatedGpuDescription:
        "Java in Windows auf die schnelle Grafikkarte stellen. Wichtig für Laptops mit zwei GPUs. Eigene Windows-Einstellungen werden nicht überschrieben.",
    discordPresence: "Discord-Status",
    discordPresenceDescription:
        "Zeigt in Discord, dass du Minecraft spielst. Orrery spricht nur mit der Discord-App auf diesem PC.",
    discordPresences: {
        off: "Aus",
        basic: "Version und Mod-Loader",
        detailed: "Mit Welt oder Server",
    } satisfies Record<PresenceLevel, string>,
    launcherWindow: "Launcher während des Spielens",
    launcherWindows: {
        keep_open: "Geöffnet lassen",
        minimize: "Minimieren",
        hide: "Ausblenden und nach dem Spiel wieder zeigen",
    } satisfies Record<LauncherWindow, string>,
    paths: {
        title: "Datenordner",
        description:
            "Hier liegen Instanzen, Welten, Spielversionen und Java. Einstellungen und Accounts bleiben im Benutzerordner.",
        data: "Daten",
        cache: "Cache",
        logs: "Launcher-Logs",
        showLog: "Log anzeigen",
        used: (size: string) => `Belegt: ${size}`,
        defaultFolder: "Standardordner",
        change: "Ändern …",
        useDefault: "Standardordner verwenden",
        dialogTitle: "Datenordner ändern",
        dialogDescription: "Wähle, ob Orrery die vorhandenen Daten mitnimmt.",
        target: "Neuer Ordner",
        subfolder: (name: string) =>
            `Der gewählte Ordner enthält schon andere Dateien. Orrery legt deshalb den Unterordner „${name}“ an.`,
        needed: (size: string) => `Zu verschiebende Daten: ${size}`,
        free: (size: string) => `Frei auf dem Ziel: ${size}`,
        otherDrive:
            "Anderes Laufwerk: Die Daten werden kopiert, geprüft und erst danach am alten Ort gelöscht.",
        sameDrive: "Gleiches Laufwerk: Die Ordner werden nur umbenannt, das geht schnell.",
        existingData: "In diesem Ordner liegen schon Orrery-Daten. Du kannst ihn direkt verwenden.",
        move: "Daten verschieben",
        switchOnly: "Nur umstellen",
        switchHint:
            "„Nur umstellen“ lässt die bisherigen Daten liegen. Orrery startet danach mit dem, was im neuen Ordner liegt.",
        problems: {
            current: "Das ist bereits der aktuelle Datenordner.",
            nested: "Der Ordner liegt im aktuellen Datenordner oder enthält ihn.",
            appFolder:
                "Das ist der Programmordner von Orrery oder er liegt darin. Eine Deinstallation könnte deine Daten dort löschen.",
            notWritable: "In diesen Ordner kann Orrery nicht schreiben.",
            notEmpty: "Zum Verschieben muss der Ordner leer sein.",
            notUsable: "Der Ordner ist nicht leer und enthält keine Orrery-Daten.",
            noSpace: "Auf dem Ziel ist nicht genug Platz frei.",
        },
        moving: "Datenordner wird geändert …",
        movingHint:
            "Bitte warte, bis alles fertig ist. Orrery startet danach automatisch neu. Spiele und Downloads sind so lange gesperrt.",
        restarting: "Fertig. Orrery startet neu …",
        failed: "Datenordner konnte nicht geändert werden",
    },
    startup: {
        title: "Datenordner nicht erreichbar",
        description: (path: string) =>
            `Orrery findet seinen Datenordner nicht: ${path}. Es wurde nichts angelegt oder verändert.`,
        problems: {
            missing: "Der Ordner existiert nicht. Ist das Laufwerk angeschlossen?",
            not_a_directory: "An dieser Stelle liegt eine Datei statt eines Ordners.",
            not_an_orrery_folder:
                "Der Ordner enthält keine Orrery-Daten. Vielleicht ist ein anderes Laufwerk mit demselben Buchstaben angeschlossen.",
            not_writable: "Orrery hat keine Schreibrechte in diesem Ordner.",
        } satisfies Record<DataDirProblem, string>,
        retry: "Erneut versuchen",
        reset: "Standardordner verwenden",
        resetHint:
            "Mit dem Standardordner startet Orrery wieder im Benutzerordner. Die Daten im gewählten Ordner bleiben unangetastet.",
        failed: "Zurücksetzen fehlgeschlagen",
    },
    storage: {
        title: "Speicher",
        usage: (size: string) =>
            `Orrery belegt ${size}. Spielversionen, Java und Downloads lassen sich jederzeit neu laden.`,
        calculating: "Speicherbelegung wird berechnet …",
        categories: {
            instances: "Instanzen mit Welten und Mods",
            versions: "Spielversionen",
            libraries: "Bibliotheken",
            assets: "Texturen und Sounds",
            runtimes: "Java-Laufzeiten",
            natives: "Natives",
            cache: "Cache",
            trash: "Papierkorb",
        } satisfies Record<StorageCategory, string>,
        largestInstances: "Größte Instanzen",
        cleanupFrees: (size: string) => `Aufräumen gibt ${size} frei`,
        nothingToClean: "Nichts aufzuräumen",
        keepsInstanceData: "Welten, Mods und Einstellungen deiner Instanzen bleiben unberührt.",
        cleanUp: "Aufräumen",
        cleaning: "Räumt auf …",
        incomplete:
            "Mindestens eine Instanz wurde noch nie gestartet. Bibliotheken, Texturen und Java bleiben deshalb vorerst unangetastet.",
        confirmTitle: "Speicher aufräumen?",
        confirmDescription:
            "Orrery löscht Dateien, die keine deiner Instanzen mehr braucht. Wenn du später eine entfernte Version wieder spielst, lädt Orrery sie neu herunter.",
        freed: (size: string) => `${size} freigegeben`,
        failed: "Aufräumen fehlgeschlagen",
        unusedVersions: (list: string) => `Nicht mehr genutzte Versionen: ${list}`,
        unusedRuntimes: (list: string) => `Nicht mehr genutzte Java-Laufzeiten: ${list}`,
        libraryFiles: (count: number) => `${count} Bibliotheksdateien`,
        assetFiles: (count: number) => `${count} Textur- und Sounddateien`,
        cache: (size: string) => `Cache (${size})`,
    },
};

export const en: typeof de = {
    title: "Settings",
    saved: "Settings saved",
    saveFailed: "Could not save settings",
    general: "General",
    generalDescription: "Defaults for new instances and downloads.",
    theme: "Theme",
    themes: {
        system: "System",
        light: "Light",
        dark: "Dark",
    },
    language: "Language",
    languageLocked: "Save or discard your changes first",
    languages: {
        system: "System language",
        de: "Deutsch",
        en: "English",
    },
    minMemory: "Minimum RAM",
    maxMemory: "Maximum RAM",
    systemMemory: (gb) => `Your PC has ${gb} GB of RAM. `,
    memoryHint:
        "Vanilla usually runs fine with 2 to 4 GB, modded games with 4 to 8 GB. Each instance shows a matching recommendation in its settings.",
    memoryRecommended: "recommended",
    memoryTight: "tight",
    downloads: "Parallel downloads",
    snapshots: "Show snapshots",
    snapshotsDescription: "Include development versions in the version picker.",
    gameOptions: "Optimized game settings",
    gameOptionsDescription:
        "Set render distance and effects to suit your PC when an instance starts for the first time. Existing settings stay untouched.",
    dedicatedGpu: "High-performance graphics card",
    dedicatedGpuDescription:
        "Tell Windows to run Java on the faster graphics card. Important for laptops with two GPUs. Your own Windows settings are not overwritten.",
    discordPresence: "Discord status",
    discordPresenceDescription:
        "Shows in Discord that you are playing Minecraft. Orrery only talks to the Discord app on this PC.",
    discordPresences: {
        off: "Off",
        basic: "Version and mod loader",
        detailed: "With world or server",
    },
    launcherWindow: "Launcher while playing",
    launcherWindows: {
        keep_open: "Keep open",
        minimize: "Minimize",
        hide: "Hide and show again after the game",
    },
    paths: {
        title: "Data folder",
        description:
            "Instances, worlds, game versions and Java live here. Settings and accounts stay in your user folder.",
        data: "Data",
        cache: "Cache",
        logs: "Launcher logs",
        showLog: "Show log",
        used: (size) => `Used: ${size}`,
        defaultFolder: "Default folder",
        change: "Change …",
        useDefault: "Use default folder",
        dialogTitle: "Change data folder",
        dialogDescription: "Choose whether Orrery takes your existing data along.",
        target: "New folder",
        subfolder: (name) =>
            `The chosen folder already contains other files, so Orrery creates the subfolder “${name}”.`,
        needed: (size) => `Data to move: ${size}`,
        free: (size) => `Free on the target: ${size}`,
        otherDrive:
            "Different drive: the data is copied, verified and only then deleted from the old location.",
        sameDrive: "Same drive: the folders are only renamed, which is quick.",
        existingData: "This folder already contains Orrery data. You can use it directly.",
        move: "Move data",
        switchOnly: "Switch only",
        switchHint:
            "“Switch only” leaves your current data where it is. Orrery then starts with whatever is in the new folder.",
        problems: {
            current: "This is already the current data folder.",
            nested: "The folder is inside the current data folder or contains it.",
            appFolder:
                "This is Orrery's program folder or lies inside it. Uninstalling could delete your data there.",
            notWritable: "Orrery cannot write to this folder.",
            notEmpty: "To move your data, the folder has to be empty.",
            notUsable: "The folder is not empty and contains no Orrery data.",
            noSpace: "There is not enough free space on the target.",
        },
        moving: "Changing the data folder …",
        movingHint:
            "Please wait until everything is done. Orrery restarts automatically afterwards. Games and downloads are blocked until then.",
        restarting: "Done. Restarting Orrery …",
        failed: "Could not change the data folder",
    },
    startup: {
        title: "Data folder not available",
        description: (path) =>
            `Orrery cannot find its data folder: ${path}. Nothing was created or changed.`,
        problems: {
            missing: "The folder does not exist. Is the drive connected?",
            not_a_directory: "There is a file at this location instead of a folder.",
            not_an_orrery_folder:
                "The folder contains no Orrery data. Maybe a different drive with the same letter is connected.",
            not_writable: "Orrery is not allowed to write to this folder.",
        },
        retry: "Try again",
        reset: "Use default folder",
        resetHint:
            "With the default folder, Orrery starts in your user folder again. The data in the chosen folder stays untouched.",
        failed: "Could not reset the data folder",
    },
    storage: {
        title: "Storage",
        usage: (size) =>
            `Orrery uses ${size}. Game versions, Java and downloads can be downloaded again at any time.`,
        calculating: "Calculating storage usage …",
        categories: {
            instances: "Instances with worlds and mods",
            versions: "Game versions",
            libraries: "Libraries",
            assets: "Textures and sounds",
            runtimes: "Java runtimes",
            natives: "Natives",
            cache: "Cache",
            trash: "Trash",
        },
        largestInstances: "Largest instances",
        cleanupFrees: (size) => `Cleaning up frees ${size}`,
        nothingToClean: "Nothing to clean up",
        keepsInstanceData: "Worlds, mods and settings of your instances stay untouched.",
        cleanUp: "Clean up",
        cleaning: "Cleaning up …",
        incomplete:
            "At least one instance has never been started. Libraries, textures and Java are left alone for now.",
        confirmTitle: "Clean up storage?",
        confirmDescription:
            "Orrery deletes files that none of your instances need anymore. If you play a removed version again later, Orrery downloads it again.",
        freed: (size) => `${size} freed`,
        failed: "Could not clean up storage",
        unusedVersions: (list) => `Unused versions: ${list}`,
        unusedRuntimes: (list) => `Unused Java runtimes: ${list}`,
        libraryFiles: (count) => (count === 1 ? "1 library file" : `${count} library files`),
        assetFiles: (count) =>
            count === 1 ? "1 texture or sound file" : `${count} texture and sound files`,
        cache: (size) => `Cache (${size})`,
    },
};

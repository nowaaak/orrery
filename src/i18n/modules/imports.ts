import type { ImportSource } from "@/bindings/ImportSource";

export const de = {
    button: "Importieren",
    title: "Aus anderem Launcher importieren",
    description:
        "Orrery findet Instanzen aus Prism Launcher, MultiMC, Modrinth App, CurseForge, ATLauncher, GDLauncher und dem offiziellen Launcher.",
    scanning: "Suche nach Instanzen …",
    scanFailed: "Suche fehlgeschlagen",
    empty: "Keine anderen Launcher an den üblichen Orten gefunden. Wähle einen Ordner oder ein Archiv.",
    chooseFolder: "Ordner wählen",
    chooseArchive: "Archiv wählen",
    archiveFilter: "Instanz-Archiv",
    inspecting: "Wird geprüft …",
    inspectFailed: "Konnte nicht gelesen werden",
    noneFound: "In diesem Ordner wurde keine Instanz eines bekannten Launchers gefunden.",
    sources: {
        prism: "Prism Launcher",
        multi_mc: "MultiMC",
        modrinth_app: "Modrinth App",
        curse_forge: "CurseForge",
        at_launcher: "ATLauncher",
        gd_launcher: "GDLauncher",
        official: "Minecraft Launcher",
    } satisfies Record<ImportSource, string>,
    unknownVersion: "Version unbekannt",
    latest: (kind: string): string =>
        kind === "snapshot" ? "Neuester Snapshot" : "Neueste Version",
    back: "Zurück",
    previewLoading: "Vorschau wird erstellt …",
    previewFailed: "Vorschau fehlgeschlagen",
    name: "Name der neuen Instanz",
    copies: (files: number, size: string) =>
        files === 1 ? `1 Datei, ${size}` : `${files} Dateien, ${size}`,
    rootFiles: "Einzelne Dateien",
    nothingToCopy: "Keine Dateien zum Kopieren gefunden.",
    import: "Importieren",
    importing: "Wird gestartet …",
    started: (name: string) => `${name} wird importiert`,
    startFailed: "Import konnte nicht gestartet werden",
    progressHint: "Den Fortschritt siehst du in den Aufgaben unten links.",
    warnings: {
        loaderNotSupported: (loader: string) =>
            `${loader} kann Orrery für diese Minecraft-Version nicht einrichten. Die Instanz startet ohne Mod-Loader, Welten und Einstellungen bleiben erhalten.`,
        unknownVersion:
            "Die Minecraft-Version ist unbekannt. Ohne Version kann Orrery die Instanz nicht anlegen.",
        javaNotTaken: (path: string) =>
            `Eigener Java-Pfad wird nicht übernommen: ${path}. Orrery wählt Java selbst.`,
        jvmArgsNotTaken: (args: string) => `JVM-Argumente werden nicht übernommen: ${args}`,
        commandsNotTaken: (commands: string) =>
            `Befehle werden nicht übernommen und nicht ausgeführt: ${commands}`,
        skippedLinks: (count: number) =>
            `${count} verknüpfte Ordner oder Dateien werden übersprungen.`,
        skippedUnsafe: (count: number) =>
            `${count} Dateien mit unsicheren Namen werden übersprungen.`,
    },
};

export const en: typeof de = {
    button: "Import",
    title: "Import from another launcher",
    description:
        "Orrery finds instances from Prism Launcher, MultiMC, Modrinth App, CurseForge, ATLauncher, GDLauncher and the official launcher.",
    scanning: "Looking for instances …",
    scanFailed: "Search failed",
    empty: "No other launchers found in the usual places. Choose a folder or an archive.",
    chooseFolder: "Choose folder",
    chooseArchive: "Choose archive",
    archiveFilter: "Instance archive",
    inspecting: "Checking …",
    inspectFailed: "Could not be read",
    noneFound: "No instance from a known launcher was found in this folder.",
    sources: {
        prism: "Prism Launcher",
        multi_mc: "MultiMC",
        modrinth_app: "Modrinth App",
        curse_forge: "CurseForge",
        at_launcher: "ATLauncher",
        gd_launcher: "GDLauncher",
        official: "Minecraft Launcher",
    },
    unknownVersion: "Unknown version",
    latest: (kind) => (kind === "snapshot" ? "Latest snapshot" : "Latest release"),
    back: "Back",
    previewLoading: "Preparing the preview …",
    previewFailed: "Preview failed",
    name: "Name of the new instance",
    copies: (files, size) => (files === 1 ? `1 file, ${size}` : `${files} files, ${size}`),
    rootFiles: "Single files",
    nothingToCopy: "No files to copy were found.",
    import: "Import",
    importing: "Starting …",
    started: (name) => `Importing ${name}`,
    startFailed: "Could not start the import",
    progressHint: "You can follow the progress in the tasks at the bottom left.",
    warnings: {
        loaderNotSupported: (loader) =>
            `Orrery cannot set up ${loader} for this Minecraft version. The instance starts without a mod loader, worlds and settings are kept.`,
        unknownVersion:
            "The Minecraft version is unknown. Orrery cannot create the instance without it.",
        javaNotTaken: (path) =>
            `The own Java path is not taken over: ${path}. Orrery picks Java itself.`,
        jvmArgsNotTaken: (args) => `JVM arguments are not taken over: ${args}`,
        commandsNotTaken: (commands) => `Commands are not taken over and not run: ${commands}`,
        skippedLinks: (count) => `${count} linked folders or files are skipped.`,
        skippedUnsafe: (count) => `${count} files with unsafe names are skipped.`,
    },
};

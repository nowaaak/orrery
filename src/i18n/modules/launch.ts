import type { CrashHint } from "@/bindings/CrashHint";

type HintText = { title: string; advice: string };

export const de = {
    progress: (stage: string, value: number | null) =>
        value === null ? stage : `${stage} ${value} %`,
    emptyLog: "Noch keine Ausgabe. Starte das Spiel, um das Log hier zu sehen.",
    crashed: "Minecraft ist abgestürzt",
    instanceCrashed: (name: string) => `${name} ist abgestürzt`,
    unexpectedExit: "Minecraft wurde unerwartet beendet",
    exitCode: (code: number) => `Exit-Code ${code}. Details stehen im Log der Instanz.`,
    details: "Details",
    stateLoadFailed: "Spielstatus konnte nicht geladen werden",
    textFile: "Textdatei",
    logExported: "Log exportiert",
    exportFailed: "Export fehlgeschlagen",
    crashReports: "Crash-Reports",
    exportLog: "Log exportieren",
    toSettings: "Zu den Einstellungen",
    log: {
        search: "Log durchsuchen",
        searchPlaceholder: "Suchen …",
        searchShortcut: "Strg F",
        level: "Filter",
        levels: {
            all: "Alle Zeilen",
            warn: "Warnungen und Fehler",
            error: "Nur Fehler",
        },
        follow: "Mitlaufen",
        count: (shown: number, total: number) =>
            shown === total ? `${total} Zeilen` : `${shown} von ${total} Zeilen`,
        noMatches: "Keine Zeile passt zum Filter.",
        share: "Teilen",
    },
    share: {
        title: "Log teilen",
        description:
            "Das Log wird auf mclo.gs hochgeladen. Jeder mit dem Link kann es 90 Tage lang lesen. Orrery entfernt vorher Zugangsdaten, IP-Adressen, deinen Benutzerordner und Accountnamen.",
        preparing: "Log wird vorbereitet …",
        preview: "Vorschau des Uploads",
        lines: (count: number) => (count === 1 ? "1 Zeile" : `${count} Zeilen`),
        upload: "Hochladen",
        uploading: "Wird hochgeladen …",
        uploaded: "Log geteilt",
        failed: "Teilen fehlgeschlagen",
        link: "Link",
        copy: "Link kopieren",
        copied: "Link kopiert",
        open: "Im Browser öffnen",
    },
    launcherLog: {
        title: "Launcher-Log",
        description:
            "Die letzten Einträge des Launchers. Hilfreich, wenn ein Download oder Start fehlschlägt.",
        empty: "Noch keine Einträge.",
        refresh: "Aktualisieren",
        loadFailed: "Launcher-Log konnte nicht gelesen werden",
    },
    summaryFallback: {
        crashReport: "Minecraft ist abgestürzt.",
        nativeCrash: "Die Java-Laufzeit ist abgestürzt.",
        exited: "Minecraft wurde unerwartet beendet.",
    },
    hints: {
        out_of_memory: {
            title: "Zu wenig Arbeitsspeicher",
            advice: "Minecraft hatte nicht genug RAM. Erhöhe den Arbeitsspeicher in den Einstellungen der Instanz. Die RAM-Empfehlung dort zeigt einen passenden Wert.",
        },
        heap_too_large: {
            title: "Arbeitsspeicher zu hoch eingestellt",
            advice: "Java konnte den eingestellten Arbeitsspeicher nicht reservieren. Senke den RAM in den Einstellungen der Instanz oder übernimm die Empfehlung.",
        },
        mod_conflict: {
            title: "Mod-Konflikt oder fehlende Abhängigkeit",
            advice: "Eine Mod passt nicht zu den anderen oder zu dieser Minecraft-Version. Deaktiviere zuletzt installierte Mods oder prüfe im Mods-Tab auf Updates.",
        },
        java_version: {
            title: "Falsche Java-Version",
            advice: "Das Spiel oder eine Mod braucht eine andere Java-Version. Entferne einen eigenen Java-Pfad in den Einstellungen der Instanz, dann wählt Orrery automatisch die passende Version.",
        },
        graphics: {
            title: "Grafikfehler",
            advice: "Der Grafiktreiber hat einen Fehler gemeldet. Aktualisiere deinen Grafiktreiber. Auch Shader-Mods können das auslösen.",
        },
        native_crash: {
            title: "Java-Laufzeit abgestürzt",
            advice: "Die Java-Laufzeit selbst ist abgestürzt. Häufige Ursachen sind Grafiktreiber oder Mods mit nativen Bibliotheken.",
        },
        unknown: {
            title: "Ursache unbekannt",
            advice: "Die Ursache ließ sich nicht automatisch bestimmen. Crash-Report und Log enthalten die Details.",
        },
    } satisfies Record<CrashHint, HintText>,
};

export const en: typeof de = {
    progress: (stage, value) => (value === null ? stage : `${stage} ${value}%`),
    emptyLog: "No output yet. Start the game to see its log here.",
    crashed: "Minecraft crashed",
    instanceCrashed: (name) => `${name} crashed`,
    unexpectedExit: "Minecraft quit unexpectedly",
    exitCode: (code) => `Exit code ${code}. See the instance log for details.`,
    details: "Details",
    stateLoadFailed: "Could not load the game status",
    textFile: "Text file",
    logExported: "Log exported",
    exportFailed: "Could not export the log",
    crashReports: "Crash reports",
    exportLog: "Export log",
    toSettings: "Open settings",
    log: {
        search: "Search the log",
        searchPlaceholder: "Search …",
        searchShortcut: "Ctrl F",
        level: "Filter",
        levels: {
            all: "All lines",
            warn: "Warnings and errors",
            error: "Errors only",
        },
        follow: "Follow",
        count: (shown, total) =>
            shown === total ? `${total} lines` : `${shown} of ${total} lines`,
        noMatches: "No line matches the filter.",
        share: "Share",
    },
    share: {
        title: "Share log",
        description:
            "The log is uploaded to mclo.gs. Anyone with the link can read it for 90 days. Orrery removes credentials, IP addresses, your user folder and account names first.",
        preparing: "Preparing the log …",
        preview: "Upload preview",
        lines: (count) => (count === 1 ? "1 line" : `${count} lines`),
        upload: "Upload",
        uploading: "Uploading …",
        uploaded: "Log shared",
        failed: "Could not share the log",
        link: "Link",
        copy: "Copy link",
        copied: "Link copied",
        open: "Open in browser",
    },
    launcherLog: {
        title: "Launcher log",
        description:
            "The latest entries from the launcher. Useful when a download or launch fails.",
        empty: "No entries yet.",
        refresh: "Refresh",
        loadFailed: "Could not read the launcher log",
    },
    summaryFallback: {
        crashReport: "Minecraft crashed.",
        nativeCrash: "The Java runtime crashed.",
        exited: "Minecraft quit unexpectedly.",
    },
    hints: {
        out_of_memory: {
            title: "Not enough memory",
            advice: "Minecraft ran out of RAM. Increase the memory in the instance settings. The RAM recommendation there suggests a suitable value.",
        },
        heap_too_large: {
            title: "Memory set too high",
            advice: "Java could not reserve the configured memory. Lower the RAM in the instance settings or apply the recommendation.",
        },
        mod_conflict: {
            title: "Mod conflict or missing dependency",
            advice: "A mod is incompatible with your other mods or with this Minecraft version. Disable recently installed mods or check for updates in the Content tab.",
        },
        java_version: {
            title: "Wrong Java version",
            advice: "The game or a mod needs a different Java version. Remove the custom Java path in the instance settings and Orrery will pick the right version automatically.",
        },
        graphics: {
            title: "Graphics error",
            advice: "The graphics driver reported an error. Update your graphics driver. Shader mods can also cause this.",
        },
        native_crash: {
            title: "Java runtime crashed",
            advice: "The Java runtime itself crashed. Common causes are graphics drivers or mods with native libraries.",
        },
        unknown: {
            title: "Unknown cause",
            advice: "The cause could not be determined automatically. The crash report and log contain the details.",
        },
    },
};

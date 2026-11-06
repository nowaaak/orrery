export const de = {
    title: "OptiFine einrichten",
    description: (label: string, minecraft: string) =>
        `OptiFine ${label} für Minecraft ${minecraft}. OptiFine läuft als Mod in einer Forge-Instanz.`,
    chooseJar: "OptiFine-Jar wählen",
    jarFilter: "OptiFine",
    notOptifine: "Das ist kein OptiFine-Jar.",
    target: "Ziel",
    newInstance: "Neue Forge-Instanz",
    name: "Name der neuen Instanz",
    defaultName: (minecraft: string) => `OptiFine ${minecraft}`,
    forgeVersion: "Forge-Version",
    noForge: "Für diese Minecraft-Version gibt es kein Forge, das Orrery einrichten kann.",
    compatibility:
        "OptiFine nennt auf seiner Download-Seite, mit welcher Forge-Version es getestet ist. Startet das Spiel nicht, wähle in den Instanz-Einstellungen eine ältere Forge-Version.",
    compatibilityKnown: (forge: string) =>
        `OptiFine ist laut seinem Changelog mit Forge ${forge} getestet, deshalb ist diese Version vorausgewählt.`,
    downloadPage: "OptiFine-Downloads",
    install: "Einrichten",
    installing: "Wird eingerichtet …",
    done: (name: string) => `OptiFine wurde zu ${name} hinzugefügt`,
    failed: "OptiFine konnte nicht eingerichtet werden",
};

export const en: typeof de = {
    title: "Set up OptiFine",
    description: (label, minecraft) =>
        `OptiFine ${label} for Minecraft ${minecraft}. OptiFine runs as a mod in a Forge instance.`,
    chooseJar: "Choose OptiFine jar",
    jarFilter: "OptiFine",
    notOptifine: "This is not an OptiFine jar.",
    target: "Target",
    newInstance: "New Forge instance",
    name: "Name of the new instance",
    defaultName: (minecraft) => `OptiFine ${minecraft}`,
    forgeVersion: "Forge version",
    noForge: "There is no Forge for this Minecraft version that Orrery can set up.",
    compatibility:
        "OptiFine names the Forge version it was tested with on its download page. If the game does not start, pick an older Forge version in the instance settings.",
    compatibilityKnown: (forge) =>
        `According to its changelog OptiFine was tested with Forge ${forge}, so that version is preselected.`,
    downloadPage: "OptiFine downloads",
    install: "Set up",
    installing: "Setting up …",
    done: (name) => `OptiFine was added to ${name}`,
    failed: "OptiFine could not be set up",
};

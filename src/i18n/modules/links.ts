export const de = {
    createShortcut: "Desktop-Verknüpfung erstellen",
    shortcutCreated: "Verknüpfung auf dem Desktop erstellt",
    shortcutFailed: "Verknüpfung konnte nicht erstellt werden",
    confirmTitle: "Spiel über einen Link starten?",
    confirmDescription:
        "Ein Link außerhalb von Orrery möchte ein Spiel starten. Starte nur, wenn du den Link selbst geöffnet hast.",
    confirmInstance: (name: string) => `Instanz: ${name}`,
    confirmWorld: (world: string) => `Welt: ${world}`,
    confirmServer: (server: string) => `Server: ${server}`,
    confirmStart: "Starten",
    unknownInstance: "Die Instanz aus dem Link gibt es nicht",
};

export const en: typeof de = {
    createShortcut: "Create desktop shortcut",
    shortcutCreated: "Shortcut created on the desktop",
    shortcutFailed: "Could not create the shortcut",
    confirmTitle: "Start the game from a link?",
    confirmDescription:
        "A link from outside Orrery wants to start a game. Only continue if you opened the link yourself.",
    confirmInstance: (name) => `Instance: ${name}`,
    confirmWorld: (world) => `World: ${world}`,
    confirmServer: (server) => `Server: ${server}`,
    confirmStart: "Start",
    unknownInstance: "The instance from the link does not exist",
};

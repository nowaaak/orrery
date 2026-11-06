export const de = {
    title: "Papierkorb",
    description: (days: number) =>
        days === 0
            ? "Gelöschte Instanzen und Welten bleiben hier, bis du sie entfernst."
            : `Gelöschte Instanzen und Welten bleiben hier ${days} Tage. Danach entfernt Orrery sie.`,
    keep: "Aufbewahren",
    keepDays: (days: number) => (days === 0 ? "Bis zum Leeren" : `${days} Tage`),
    empty: "Der Papierkorb ist leer.",
    loadFailed: "Papierkorb konnte nicht gelesen werden",
    kinds: { instance: "Instanz", world: "Welt" },
    worldFrom: (instance: string) => `Welt aus ${instance}`,
    deletedAgo: (ago: string) => `gelöscht ${ago}`,
    restore: "Wiederherstellen",
    restoreItem: (name: string) => `${name} wiederherstellen`,
    restored: (name: string) => `${name} ist wieder da`,
    restoreFailed: "Wiederherstellen fehlgeschlagen",
    deleteItem: (name: string) => `${name} endgültig löschen`,
    deleteTitle: (name: string) => `${name} endgültig löschen?`,
    deleteDescription: "Das lässt sich nicht rückgängig machen.",
    deleteConfirm: "Endgültig löschen",
    deleteFailed: "Löschen fehlgeschlagen",
    emptyAll: "Papierkorb leeren",
    emptyTitle: "Papierkorb leeren?",
    emptyDescription: (count: number) =>
        count === 1
            ? "Ein Eintrag wird endgültig gelöscht. Das lässt sich nicht rückgängig machen."
            : `${count} Einträge werden endgültig gelöscht. Das lässt sich nicht rückgängig machen.`,
    emptied: "Papierkorb geleert",
    emptyFailed: "Leeren fehlgeschlagen",
    moved: (name: string) => `${name} liegt im Papierkorb`,
    undo: "Rückgängig",
};

export const en: typeof de = {
    title: "Trash",
    description: (days) =>
        days === 0
            ? "Deleted instances and worlds stay here until you remove them."
            : `Deleted instances and worlds stay here for ${days} days. After that Orrery removes them.`,
    keep: "Keep",
    keepDays: (days) => (days === 0 ? "Until emptied" : `${days} days`),
    empty: "The trash is empty.",
    loadFailed: "Could not read the trash",
    kinds: { instance: "Instance", world: "World" },
    worldFrom: (instance) => `World from ${instance}`,
    deletedAgo: (ago) => `deleted ${ago}`,
    restore: "Restore",
    restoreItem: (name) => `Restore ${name}`,
    restored: (name) => `${name} is back`,
    restoreFailed: "Could not restore",
    deleteItem: (name) => `Delete ${name} permanently`,
    deleteTitle: (name) => `Delete ${name} permanently?`,
    deleteDescription: "This cannot be undone.",
    deleteConfirm: "Delete permanently",
    deleteFailed: "Could not delete",
    emptyAll: "Empty trash",
    emptyTitle: "Empty the trash?",
    emptyDescription: (count) =>
        count === 1
            ? "One entry will be deleted permanently. This cannot be undone."
            : `${count} entries will be deleted permanently. This cannot be undone.`,
    emptied: "Trash emptied",
    emptyFailed: "Could not empty the trash",
    moved: (name) => `${name} is in the trash`,
    undo: "Undo",
};

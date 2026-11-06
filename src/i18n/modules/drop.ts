import type { ContentKind } from "@/bindings/ContentKind";

export const de = {
    overlayInstance: (name: string) => `Zu ${name} hinzufügen`,
    overlayGlobal: "Modpack oder Instanz importieren",
    overlayHint: "Mods (.jar), Ressourcen- und Shaderpakete (.zip) oder Modpacks (.mrpack)",
    overlayGlobalHint:
        "Lass .mrpack-Dateien, Instanzordner oder Archive anderer Launcher los. Für Mods und Pakete öffne vorher eine Instanz.",
    added: (title: string, kind: ContentKind) =>
        `${title} als ${{ mod: "Mod", resource_pack: "Ressourcenpaket", shader: "Shaderpaket" }[kind]} hinzugefügt`,
    addFailed: (name: string) => `${name} konnte nicht hinzugefügt werden`,
    needsInstance: "Öffne eine Instanz, um Mods und Pakete per Drag and Drop hinzuzufügen.",
    unsupported: (name: string) => `${name} wird nicht unterstützt`,
};

export const en: typeof de = {
    overlayInstance: (name) => `Add to ${name}`,
    overlayGlobal: "Import modpack or instance",
    overlayHint: "Mods (.jar), resource and shader packs (.zip) or modpacks (.mrpack)",
    overlayGlobalHint:
        "Drop .mrpack files, instance folders or archives from other launchers. Open an instance first to add mods and packs.",
    added: (title, kind) =>
        `${title} added as ${{ mod: "mod", resource_pack: "resource pack", shader: "shader pack" }[kind]}`,
    addFailed: (name) => `Could not add ${name}`,
    needsInstance: "Open an instance to add mods and packs by drag and drop.",
    unsupported: (name) => `${name} is not supported`,
};

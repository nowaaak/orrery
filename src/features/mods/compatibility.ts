import type { Instance } from "@/bindings/Instance";
import type { InstalledMod } from "@/bindings/InstalledMod";

export type Mismatch = "version" | "loader";

const MODRINTH_LOADERS: Record<Exclude<Instance["loader"]["type"], "vanilla">, string[]> = {
    fabric: ["fabric"],
    legacy_fabric: ["legacy-fabric"],
    quilt: ["quilt", "fabric"],
    forge: ["forge"],
    neo_forge: ["neoforge"],
};

export function contentMismatch(
    mod: Pick<InstalledMod, "kind" | "gameVersions" | "loaders">,
    instance: Pick<Instance, "minecraftVersion" | "loader">,
): Mismatch | null {
    if (mod.gameVersions.length > 0 && !mod.gameVersions.includes(instance.minecraftVersion)) {
        return "version";
    }
    const loader = instance.loader.type;
    if (mod.kind !== "mod" || mod.loaders.length === 0 || loader === "vanilla") {
        return null;
    }
    const accepted = MODRINTH_LOADERS[loader];
    return mod.loaders.some((value) => accepted.includes(value)) ? null : "loader";
}

export function isLocalContent(projectId: string): boolean {
    return projectId.startsWith("local-");
}

export function isCurseForgeContent(projectId: string): boolean {
    return projectId.startsWith("curseforge-");
}

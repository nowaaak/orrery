import { describe, expect, it } from "vitest";
import { contentMismatch, isLocalContent } from "./compatibility";

const fabric = {
    minecraftVersion: "1.21.1",
    loader: { type: "fabric", loaderVersion: "0.16" },
} as const;
const quilt = {
    minecraftVersion: "1.21.1",
    loader: { type: "quilt", loaderVersion: "0.27" },
} as const;
const forge = {
    minecraftVersion: "1.21.1",
    loader: { type: "forge", loaderVersion: "52.1.21" },
} as const;
const neoForge = {
    minecraftVersion: "1.21.1",
    loader: { type: "neo_forge", loaderVersion: "21.1.265" },
} as const;

describe("contentMismatch", () => {
    it("flags other Minecraft versions", () => {
        const mod = { kind: "mod", gameVersions: ["1.20.1"], loaders: ["fabric"] } as const;
        expect(
            contentMismatch(
                { ...mod, gameVersions: [...mod.gameVersions], loaders: [...mod.loaders] },
                fabric,
            ),
        ).toBe("version");
    });

    it("flags mods for other loaders but lets Quilt run Fabric mods", () => {
        const forgeOnly = { kind: "mod" as const, gameVersions: ["1.21.1"], loaders: ["neoforge"] };
        const fabricMod = { kind: "mod" as const, gameVersions: ["1.21.1"], loaders: ["fabric"] };
        expect(contentMismatch(forgeOnly, fabric)).toBe("loader");
        expect(contentMismatch(fabricMod, quilt)).toBeNull();
        expect(contentMismatch(fabricMod, fabric)).toBeNull();
        expect(contentMismatch(forgeOnly, neoForge)).toBeNull();
        expect(contentMismatch(fabricMod, neoForge)).toBe("loader");
        expect(contentMismatch(forgeOnly, forge)).toBe("loader");
    });

    it("ignores unknown data and non-mod loaders", () => {
        expect(contentMismatch({ kind: "mod", gameVersions: [], loaders: [] }, fabric)).toBeNull();
        expect(
            contentMismatch(
                { kind: "shader", gameVersions: ["1.21.1"], loaders: ["iris"] },
                fabric,
            ),
        ).toBeNull();
    });
});

describe("isLocalContent", () => {
    it("recognizes dropped files without a Modrinth project", () => {
        expect(isLocalContent("local-0123456789abcdef")).toBe(true);
        expect(isLocalContent("AANobbMI")).toBe(false);
    });
});

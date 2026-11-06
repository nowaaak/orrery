import { describe, expect, it } from "vitest";
import type { Instance } from "@/bindings/Instance";
import { groupNames, instanceSections, sortInstances } from "./instanceOrder";

function instance(name: string, extra: Partial<Instance> = {}): Instance {
    return {
        schema: 1,
        id: name.toLowerCase(),
        name,
        minecraftVersion: "1.21.1",
        loader: { type: "vanilla" },
        java: { javaPath: null, minMemoryMb: null, maxMemoryMb: null, extraJvmArgs: [] },
        resolution: null,
        launch: {
            accountId: null,
            joinServer: null,
            extraGameArgs: [],
            env: [],
            wrapper: null,
            preLaunch: null,
            postExit: null,
            gamemode: false,
            mangohud: false,
            primeOffload: false,
            companionDisabled: false,
        },
        group: null,
        pinned: false,
        icon: null,
        createdAt: 0,
        lastPlayedAt: null,
        totalPlaySecs: 0,
        syncPacks: [],
        ...extra,
    };
}

const LIST = [
    instance("beta 10", { createdAt: 3, totalPlaySecs: 50, lastPlayedAt: 100 }),
    instance("Alpha", { createdAt: 1, totalPlaySecs: 500, group: "PvP" }),
    instance("beta 2", { createdAt: 2, lastPlayedAt: 200, pinned: true, group: "PvP" }),
    instance("Gamma", { createdAt: 4, group: "Modded" }),
];

describe("sortInstances", () => {
    it("sorts by each order", () => {
        const names = (sort: Parameters<typeof sortInstances>[1]) =>
            sortInstances(LIST, sort).map((i) => i.name);
        expect(names("name")).toEqual(["Alpha", "beta 2", "beta 10", "Gamma"]);
        expect(names("created")).toEqual(["Gamma", "beta 10", "beta 2", "Alpha"]);
        expect(names("playtime")).toEqual(["Alpha", "beta 10", "beta 2", "Gamma"]);
        expect(names("last_played")).toEqual(["beta 2", "beta 10", "Alpha", "Gamma"]);
    });
});

describe("instanceSections", () => {
    it("puts pinned first, then groups by name, then the rest", () => {
        const sections = instanceSections(LIST, "name").map((section) => [
            section.kind === "group" ? section.name : section.kind,
            section.instances.map((i) => i.name),
        ]);
        expect(sections).toEqual([
            ["pinned", ["beta 2"]],
            ["Modded", ["Gamma"]],
            ["PvP", ["Alpha"]],
            ["ungrouped", ["beta 10"]],
        ]);
        expect(groupNames(LIST)).toEqual(["Modded", "PvP"]);
    });
});

import { describe, expect, it } from "vitest";
import { loaderLabel, warningText } from "./ImportDialog";

describe("import labels", () => {
    it("names loaders", () => {
        expect(loaderLabel({ type: "vanilla" })).toBe("Vanilla");
        expect(loaderLabel({ type: "fabric", version: "0.16.9" })).toBe("Fabric 0.16.9");
        expect(loaderLabel({ type: "neo_forge", version: "21.1.77" })).toBe("NeoForge 21.1.77");
        expect(loaderLabel({ type: "other", name: "liteloader" })).toBe("liteloader");
    });

    it("explains every warning", () => {
        expect(warningText({ type: "java_not_taken", path: "C:/Java/javaw.exe" })).toContain(
            "C:/Java/javaw.exe",
        );
        expect(warningText({ type: "commands_not_taken", commands: ["pre", "post"] })).toContain(
            "pre · post",
        );
        expect(
            warningText({
                type: "loader_not_supported",
                loader: { type: "forge", version: "47.2.0" },
            }),
        ).toContain("Forge 47.2.0");
        expect(warningText({ type: "skipped_links", count: 2 })).toContain("2");
    });
});

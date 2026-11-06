import { describe, expect, it } from "vitest";
import { optifineLabel } from "./OptifineDialog";

describe("optifineLabel", () => {
    it("names release and preview builds", () => {
        const jar = {
            minecraftVersion: "1.20.1",
            edition: "HD_U",
            release: "I6",
            preview: false,
            forgeVersion: "47.2.18",
        };
        expect(optifineLabel(jar)).toBe("HD U I6");
        expect(optifineLabel({ ...jar, release: "J3", preview: true })).toBe("HD U J3 preview");
    });
});

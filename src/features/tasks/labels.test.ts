import { beforeEach, describe, expect, it } from "vitest";
import { setLocale } from "@/i18n";
import { isContentTask, isModpackTask, taskTitle } from "./labels";

beforeEach(() => setLocale("de"));

describe("task labels", () => {
    it("builds titles in the active language", () => {
        expect(taskTitle({ kind: "update_mods", subject: "3" })).toBe("3 Updates installieren");
        expect(taskTitle({ kind: "install_pack", subject: "performance" })).toBe(
            "Performance-Paket installieren",
        );
        expect(taskTitle({ kind: "install_pack", subject: "optifine" })).toBe(
            "OptiFine-Ersatz installieren",
        );
        setLocale("en");
        expect(taskTitle({ kind: "launch_instance", subject: "Survival" })).toBe("Start Survival");
        expect(taskTitle({ kind: "import_modpack", subject: "FO" })).toBe("Import FO");
    });

    it("groups every task that changes installed content", () => {
        expect(isContentTask("install_mod")).toBe(true);
        expect(isContentTask("update_mods")).toBe(true);
        expect(isContentTask("install_pack")).toBe(true);
        expect(isContentTask("install_orrery_shaders")).toBe(true);
        expect(isContentTask("install_orrery_menu")).toBe(true);
        expect(isContentTask("launch_instance")).toBe(false);
        expect(isModpackTask("import_modpack")).toBe(true);
        expect(isModpackTask("install_mod")).toBe(false);
    });
});

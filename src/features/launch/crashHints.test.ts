import { beforeEach } from "vitest";
import type { CrashReport } from "@/bindings/CrashReport";
import { setLocale } from "@/i18n";
import { crashHint, crashSummary } from "./crashHints";

beforeEach(() => setLocale("de"));

function report(overrides: Partial<CrashReport>): CrashReport {
    return { hint: "unknown", summary: null, reportFile: null, ...overrides };
}

describe("crashSummary", () => {
    it("prefers the summary from the backend", () => {
        expect(crashSummary(report({ summary: "java.lang.OutOfMemoryError" }))).toBe(
            "java.lang.OutOfMemoryError",
        );
    });

    it("falls back by situation", () => {
        expect(crashSummary(report({ hint: "native_crash", reportFile: "crash-2026.txt" }))).toBe(
            "Minecraft ist abgestürzt.",
        );
        expect(crashSummary(report({ hint: "native_crash", reportFile: "hs_err_pid1.log" }))).toBe(
            "Die Java-Laufzeit ist abgestürzt.",
        );
        expect(crashSummary(report({}))).toBe("Minecraft wurde unerwartet beendet.");
    });

    it("follows the current locale", () => {
        setLocale("en");
        expect(crashSummary(report({ hint: "native_crash" }))).toBe("The Java runtime crashed.");
        expect(crashHint("out_of_memory").title).toBe("Not enough memory");
    });
});

describe("crashHint", () => {
    it("marks memory and Java hints as settings related", () => {
        expect(crashHint("out_of_memory").opensSettings).toBe(true);
        expect(crashHint("java_version").opensSettings).toBe(true);
        expect(crashHint("mod_conflict").opensSettings).toBe(false);
    });
});

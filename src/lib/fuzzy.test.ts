import { describe, expect, it } from "vitest";
import { fuzzyScore, normalize } from "./fuzzy";

describe("fuzzyScore", () => {
    it("matches everything for an empty query", () => {
        expect(fuzzyScore("  ", "Einstellungen")).toBe(0);
    });

    it("ignores case and accents", () => {
        expect(normalize("Überleben Café")).toBe("uberleben cafe");
        expect(fuzzyScore("uberleben", "Überleben")).not.toBeNull();
    });

    it("rejects targets that miss a character", () => {
        expect(fuzzyScore("xyz", "Einstellungen")).toBeNull();
    });

    it("requires every word of the query", () => {
        expect(fuzzyScore("survival spielen", "Survival spielen")).not.toBeNull();
        expect(fuzzyScore("survival löschen", "Survival spielen")).toBeNull();
    });

    it("prefers word starts and contiguous matches", () => {
        const start = fuzzyScore("set", "Einstellungen Settings")!;
        const scattered = fuzzyScore("set", "Spiel starten")!;
        expect(start).toBeGreaterThan(scattered);
        const abbreviation = fuzzyScore("ni", "Neue Instanz")!;
        const inside = fuzzyScore("ni", "Benutzerprofil")!;
        expect(abbreviation).toBeGreaterThan(inside);
    });

    it("ranks a direct substring above a subsequence", () => {
        expect(fuzzyScore("mod", "Modpack installieren")!).toBeGreaterThan(
            fuzzyScore("mod", "Meine Offline Daten")!,
        );
    });
});

import { describe, expect, it } from "vitest";
import { filterLog, lineLevel, splitMatches } from "./logFilter";

const LINES = [
    "[12:00:00] [Render thread/INFO]: Setting user: Steve",
    "[12:00:01] [Render thread/WARN]: Missing sound for event",
    "[12:00:02] [Render thread/ERROR]: Failed to load texture",
    "java.lang.IllegalStateException: boom",
    "\tat net.minecraft.Main.main(Main.java:1)",
    "2026-11-02 12:00:03 [WARN] launcher warning",
];

describe("lineLevel", () => {
    it("classifies vanilla, stack trace and launcher lines", () => {
        expect(LINES.map(lineLevel)).toEqual(["info", "warn", "error", "error", "error", "warn"]);
    });
});

describe("filterLog", () => {
    it("keeps original indices and filters by level", () => {
        expect(filterLog(LINES, "", "all")).toHaveLength(6);
        expect(filterLog(LINES, "", "warn").map((entry) => entry.index)).toEqual([1, 2, 3, 4, 5]);
        expect(filterLog(LINES, "", "error").map((entry) => entry.index)).toEqual([2, 3, 4]);
    });

    it("searches case-insensitively within the level", () => {
        expect(filterLog(LINES, "  TEXTURE ", "all").map((entry) => entry.index)).toEqual([2]);
        expect(filterLog(LINES, "steve", "warn")).toEqual([]);
    });
});

describe("splitMatches", () => {
    it("marks every match", () => {
        expect(splitMatches("Steve and steve", "STEVE")).toEqual([
            { text: "Steve", match: true },
            { text: " and ", match: false },
            { text: "steve", match: true },
        ]);
        expect(splitMatches("plain", " ")).toEqual([{ text: "plain", match: false }]);
    });
});

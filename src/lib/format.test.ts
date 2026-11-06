import { beforeEach } from "vitest";
import { setLocale } from "@/i18n";
import { formatBytes, formatPlaytime, percent } from "./format";

beforeEach(() => setLocale("de"));

describe("formatBytes", () => {
    it("formats with German decimal separator", () => {
        expect(formatBytes(0)).toBe("0 B");
        expect(formatBytes(512)).toBe("512 B");
        expect(formatBytes(1536)).toBe("1,5 KB");
        expect(formatBytes(250 * 1024 * 1024)).toBe("250 MB");
    });
});

describe("percent", () => {
    it("returns null for unknown totals and clamps at 100", () => {
        expect(percent(5, 0)).toBeNull();
        expect(percent(1, 3)).toBe(33);
        expect(percent(7, 5)).toBe(100);
    });
});

describe("formatPlaytime", () => {
    it("formats minutes and hours", () => {
        expect(formatPlaytime(0)).toBe("0 Min.");
        expect(formatPlaytime(30)).toBe("unter 1 Min.");
        expect(formatPlaytime(45 * 60)).toBe("45 Min.");
        expect(formatPlaytime(3 * 3600)).toBe("3 Std.");
        expect(formatPlaytime(3 * 3600 + 12 * 60)).toBe("3 Std. 12 Min.");
    });
});

describe("English formatting", () => {
    it("uses English separators and units", () => {
        setLocale("en");
        expect(formatBytes(1536)).toBe("1.5 KB");
        expect(formatPlaytime(3 * 3600 + 12 * 60)).toBe("3 h 12 min");
    });
});

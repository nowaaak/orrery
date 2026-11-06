import { beforeEach } from "vitest";
import { setLocale } from "@/i18n";
import { formatGb, isTight, memoryOptions } from "./memory";

beforeEach(() => setLocale("de"));

describe("memoryOptions", () => {
    it("uses half steps up to 4 GB and whole GB above", () => {
        const options = memoryOptions(8192);
        expect(options.slice(0, 7)).toEqual([1024, 1536, 2048, 2560, 3072, 3584, 4096]);
        expect(options.slice(7)).toEqual([5120, 6144, 7168, 8192]);
    });

    it("never offers more than the system memory but always up to 4 GB", () => {
        expect(Math.max(...memoryOptions(32768))).toBe(32768);
        expect(Math.max(...memoryOptions(2048))).toBe(4096);
    });

    it("starts at the given floor for minimum memory", () => {
        expect(memoryOptions(8192, 512)[0]).toBe(512);
    });
});

describe("formatting", () => {
    it("formats gigabytes in German", () => {
        expect(formatGb(1536)).toBe("1,5 GB");
        expect(formatGb(4096)).toBe("4 GB");
    });

    it("formats gigabytes in English", () => {
        setLocale("en");
        expect(formatGb(1536)).toBe("1.5 GB");
    });

    it("flags values that leave less than 3 GB for the system", () => {
        expect(isTight(6144, 8192)).toBe(true);
        expect(isTight(4096, 8192)).toBe(false);
        expect(isTight(4096, null)).toBe(false);
    });
});

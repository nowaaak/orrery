import { describe, expect, it } from "vitest";
import { normalizePixels } from "./normalizeSkin";

function alphaAt(data: Uint8ClampedArray, width: number, x: number, y: number) {
    return data[(y * width + x) * 4 + 3];
}

describe("normalizePixels", () => {
    it("clears a fully opaque hat on legacy skins and makes the base opaque", () => {
        const data = new Uint8ClampedArray(64 * 32 * 4).fill(255);
        data[(20 * 64 + 20) * 4 + 3] = 0;
        normalizePixels(data, 64, 32);
        expect(alphaAt(data, 64, 40, 8)).toBe(0);
        expect(alphaAt(data, 64, 20, 20)).toBe(255);
    });

    it("keeps a hat that already has transparency", () => {
        const data = new Uint8ClampedArray(64 * 32 * 4).fill(255);
        data[(2 * 64 + 50) * 4 + 3] = 0;
        normalizePixels(data, 64, 32);
        expect(alphaAt(data, 64, 40, 8)).toBe(255);
        expect(alphaAt(data, 64, 50, 2)).toBe(0);
    });

    it("leaves the overlay layers of modern skins alone", () => {
        const data = new Uint8ClampedArray(64 * 64 * 4);
        normalizePixels(data, 64, 64);
        expect(alphaAt(data, 64, 8, 8)).toBe(255);
        expect(alphaAt(data, 64, 40, 8)).toBe(0);
        expect(alphaAt(data, 64, 20, 36)).toBe(0);
        expect(alphaAt(data, 64, 20, 52)).toBe(255);
    });
});

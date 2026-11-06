import { describe, expect, it } from "vitest";
import { FLOATS_PER_VERTEX, buildMesh, playerBoxes } from "./skinModel";

const SIZES = { skin: [64, 64], cape: [64, 32] } as Record<"skin" | "cape", [number, number]>;

describe("playerBoxes", () => {
    it("adds a second layer to every part of modern skins", () => {
        const boxes = playerBoxes(false, false, false);
        expect(boxes).toHaveLength(12);
        expect(boxes.filter((box) => box.inflate > 0)).toHaveLength(6);
    });

    it("uses three pixel arms for slim skins", () => {
        const arms = playerBoxes(true, false, false).filter(
            (box) => box.from[1] === 12 && box.size[1] === 12 && box.from[0] !== -4,
        );
        expect(arms.every((box) => box.size[0] === 3)).toBe(true);
        expect(arms.map((box) => box.from[0])).toEqual([-7, -7, 4, 4]);
    });

    it("mirrors the right limbs and keeps only the hat for legacy skins", () => {
        const boxes = playerBoxes(false, true, true);
        expect(boxes.filter((box) => box.inflate > 0)).toHaveLength(1);
        expect(boxes.filter((box) => box.uv[0] === 40 && box.uv[1] === 16)).toHaveLength(2);
        expect(boxes.at(-1)?.texture).toBe("cape");
    });
});

describe("buildMesh", () => {
    it("emits two triangles per face with skin UVs", () => {
        const mesh = buildMesh(playerBoxes(false, false, true), SIZES);
        expect(mesh.skin.length).toBe(12 * 6 * 6 * FLOATS_PER_VERTEX);
        expect(mesh.cape.length).toBe(6 * 6 * FLOATS_PER_VERTEX);

        const headFront = mesh.skin.slice(0, 6 * FLOATS_PER_VERTEX);
        const firstUv = [headFront[6], headFront[7]];
        expect(firstUv).toEqual([8 / 64, 8 / 64]);
        expect([headFront[3], headFront[4], headFront[5]]).toEqual([0, 0, 1]);
    });
});

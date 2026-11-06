export type Vec3 = [number, number, number];
export type TextureKind = "skin" | "cape";

export type Box = {
    from: Vec3;
    size: Vec3;
    uv: [number, number];
    inflate: number;
    texture: TextureKind;
    capeOutside?: boolean;
};

export const FLOATS_PER_VERTEX = 8;

type Part = { from: Vec3; size: Vec3; base: [number, number]; overlay: [number, number] | null };

export function playerBoxes(slim: boolean, legacy: boolean, cape: boolean): Box[] {
    const arm = slim ? 3 : 4;
    const parts: Part[] = [
        { from: [-4, 24, -4], size: [8, 8, 8], base: [0, 0], overlay: [32, 0] },
        { from: [-4, 12, -2], size: [8, 12, 4], base: [16, 16], overlay: legacy ? null : [16, 32] },
        {
            from: [-4 - arm, 12, -2],
            size: [arm, 12, 4],
            base: [40, 16],
            overlay: legacy ? null : [40, 32],
        },
        {
            from: [4, 12, -2],
            size: [arm, 12, 4],
            base: legacy ? [40, 16] : [32, 48],
            overlay: legacy ? null : [48, 48],
        },
        { from: [-4, 0, -2], size: [4, 12, 4], base: [0, 16], overlay: legacy ? null : [0, 32] },
        {
            from: [0, 0, -2],
            size: [4, 12, 4],
            base: legacy ? [0, 16] : [16, 48],
            overlay: legacy ? null : [0, 48],
        },
    ];
    const boxes: Box[] = [];
    parts.forEach((part, index) => {
        boxes.push({
            from: part.from,
            size: part.size,
            uv: part.base,
            inflate: 0,
            texture: "skin",
        });
        if (part.overlay) {
            boxes.push({
                from: part.from,
                size: part.size,
                uv: part.overlay,
                inflate: index === 0 ? 0.5 : 0.25,
                texture: "skin",
            });
        }
    });
    if (cape) {
        boxes.push({
            from: [-5, 8, -3.1],
            size: [10, 16, 1],
            uv: [0, 0],
            inflate: 0,
            texture: "cape",
            capeOutside: true,
        });
    }
    return boxes;
}

type Face = {
    normal: Vec3;
    corners: [Vec3, Vec3, Vec3, Vec3];
    rect: [number, number, number, number];
};

function boxFaces(box: Box): Face[] {
    const [w, h, d] = box.size;
    const [u, v] = box.uv;
    const g = box.inflate;
    const x0 = box.from[0] - g;
    const y0 = box.from[1] - g;
    const z0 = box.from[2] - g;
    const x1 = box.from[0] + w + g;
    const y1 = box.from[1] + h + g;
    const z1 = box.from[2] + d + g;
    const front: [number, number, number, number] = [u + d, v + d, w, h];
    const back: [number, number, number, number] = [u + d + w + d, v + d, w, h];
    const plusZ = box.capeOutside ? back : front;
    const minusZ = box.capeOutside ? front : back;
    return [
        {
            normal: [0, 0, 1],
            corners: [
                [x0, y1, z1],
                [x1, y1, z1],
                [x1, y0, z1],
                [x0, y0, z1],
            ],
            rect: plusZ,
        },
        {
            normal: [0, 0, -1],
            corners: [
                [x1, y1, z0],
                [x0, y1, z0],
                [x0, y0, z0],
                [x1, y0, z0],
            ],
            rect: minusZ,
        },
        {
            normal: [-1, 0, 0],
            corners: [
                [x0, y1, z0],
                [x0, y1, z1],
                [x0, y0, z1],
                [x0, y0, z0],
            ],
            rect: [u, v + d, d, h],
        },
        {
            normal: [1, 0, 0],
            corners: [
                [x1, y1, z1],
                [x1, y1, z0],
                [x1, y0, z0],
                [x1, y0, z1],
            ],
            rect: [u + d + w, v + d, d, h],
        },
        {
            normal: [0, 1, 0],
            corners: [
                [x0, y1, z0],
                [x1, y1, z0],
                [x1, y1, z1],
                [x0, y1, z1],
            ],
            rect: [u + d, v, w, d],
        },
        {
            normal: [0, -1, 0],
            corners: [
                [x0, y0, z1],
                [x1, y0, z1],
                [x1, y0, z0],
                [x0, y0, z0],
            ],
            rect: [u + d + w, v, w, d],
        },
    ];
}

export function buildMesh(
    boxes: Box[],
    textureSize: Record<TextureKind, [number, number]>,
): Record<TextureKind, Float32Array> {
    const data: Record<TextureKind, number[]> = { skin: [], cape: [] };
    for (const box of boxes) {
        const [tw, th] = textureSize[box.texture];
        for (const face of boxFaces(box)) {
            const [ru, rv, rw, rh] = face.rect;
            const uvs: [number, number][] = [
                [ru / tw, rv / th],
                [(ru + rw) / tw, rv / th],
                [(ru + rw) / tw, (rv + rh) / th],
                [ru / tw, (rv + rh) / th],
            ];
            for (const index of [0, 1, 2, 0, 2, 3]) {
                const corner = face.corners[index]!;
                const uv = uvs[index]!;
                data[box.texture].push(...corner, ...face.normal, uv[0], uv[1]);
            }
        }
    }
    return { skin: new Float32Array(data.skin), cape: new Float32Array(data.cape) };
}

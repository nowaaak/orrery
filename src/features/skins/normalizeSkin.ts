type Region = [number, number, number, number];

const OPAQUE_MODERN: Region[] = [
    [0, 0, 32, 16],
    [0, 16, 64, 32],
    [16, 48, 48, 64],
];
const OPAQUE_LEGACY: Region[] = [
    [0, 0, 32, 16],
    [0, 16, 64, 32],
];
const HAT: Region = [32, 0, 64, 16];

export function normalizePixels(data: Uint8ClampedArray, width: number, height: number) {
    const scale = width / 64;
    const legacy = height === width / 2;
    const each = ([x0, y0, x1, y1]: Region, visit: (index: number) => void) => {
        for (let y = y0 * scale; y < y1 * scale; y++) {
            for (let x = x0 * scale; x < x1 * scale; x++) {
                visit((y * width + x) * 4 + 3);
            }
        }
    };
    for (const region of legacy ? OPAQUE_LEGACY : OPAQUE_MODERN) {
        each(region, (alpha) => {
            data[alpha] = 255;
        });
    }
    if (legacy) {
        let transparent = false;
        each(HAT, (alpha) => {
            if (data[alpha]! < 128) transparent = true;
        });
        if (!transparent) {
            each(HAT, (alpha) => {
                data[alpha] = 0;
            });
        }
    }
}

export async function normalizeSkin(blob: Blob): Promise<ImageBitmap> {
    const image = await createImageBitmap(blob);
    if (image.width < 64 || image.width % 64 !== 0) {
        return image;
    }
    const canvas = document.createElement("canvas");
    canvas.width = image.width;
    canvas.height = image.height;
    const context = canvas.getContext("2d", { willReadFrequently: true });
    if (!context) {
        return image;
    }
    context.drawImage(image, 0, 0);
    const pixels = context.getImageData(0, 0, image.width, image.height);
    normalizePixels(pixels.data, image.width, image.height);
    context.putImageData(pixels, 0, 0);
    image.close();
    return createImageBitmap(canvas);
}

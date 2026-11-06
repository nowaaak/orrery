import { useEffect, useRef } from "react";
import { cn } from "@/lib/utils";
import { normalizeSkin } from "./normalizeSkin";

type Rect = [number, number, number, number, number, number];

function frontParts(slim: boolean, legacy: boolean): Rect[] {
    const arm = slim ? 3 : 4;
    const leftArm: [number, number] = legacy ? [44, 20] : [36, 52];
    const leftLeg: [number, number] = legacy ? [4, 20] : [20, 52];
    const base: Rect[] = [
        [8, 8, 8, 8, 4, 0],
        [20, 20, 8, 12, 4, 8],
        [44, 20, arm, 12, 4 - arm, 8],
        [leftArm[0], leftArm[1], arm, 12, 12, 8],
        [4, 20, 4, 12, 4, 20],
        [leftLeg[0], leftLeg[1], 4, 12, 8, 20],
    ];
    const overlay: Rect[] = legacy
        ? [[40, 8, 8, 8, 4, 0]]
        : [
              [40, 8, 8, 8, 4, 0],
              [20, 36, 8, 12, 4, 8],
              [44, 36, arm, 12, 4 - arm, 8],
              [52, 52, arm, 12, 12, 8],
              [4, 36, 4, 12, 4, 20],
              [4, 52, 4, 12, 8, 20],
          ];
    return [...base, ...overlay];
}

export function SkinFront({
    skin,
    slim,
    className,
}: {
    skin: Blob | null | undefined;
    slim: boolean;
    className?: string;
}) {
    const ref = useRef<HTMLCanvasElement>(null);

    useEffect(() => {
        const canvas = ref.current;
        const context = canvas?.getContext("2d");
        if (!canvas || !context || !skin) return;
        let cancelled = false;
        void normalizeSkin(skin).then((image) => {
            if (cancelled) return;
            context.imageSmoothingEnabled = false;
            context.clearRect(0, 0, canvas.width, canvas.height);
            const legacy = image.height === image.width / 2;
            const scale = image.width / 64;
            for (const [sx, sy, w, h, dx, dy] of frontParts(slim, legacy)) {
                context.drawImage(
                    image,
                    sx * scale,
                    sy * scale,
                    w * scale,
                    h * scale,
                    dx,
                    dy,
                    w,
                    h,
                );
            }
        });
        return () => {
            cancelled = true;
        };
    }, [skin, slim]);

    return (
        <canvas
            ref={ref}
            width={16}
            height={32}
            aria-hidden
            className={cn("[image-rendering:pixelated]", className)}
        />
    );
}

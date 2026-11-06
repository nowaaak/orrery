import { useEffect, useRef, useState } from "react";
import type { Account } from "@/bindings/Account";
import { cn } from "@/lib/utils";

type SkinHeadProps = {
    account: Account;
    size?: number;
    className?: string;
};

export function SkinHead({ account, size = 32, className }: SkinHeadProps) {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const [drawnKey, setDrawnKey] = useState<string | null>(null);
    const remoteUrl = account.skin?.url ?? null;
    const drawn = remoteUrl !== null && drawnKey === remoteUrl;

    useEffect(() => {
        const canvas = canvasRef.current;
        const context = canvas?.getContext("2d");
        if (!canvas || !context || remoteUrl === null) {
            return;
        }
        const image = new Image();
        image.crossOrigin = "anonymous";
        image.onload = () => {
            context.imageSmoothingEnabled = false;
            context.clearRect(0, 0, 8, 8);
            context.drawImage(image, 8, 8, 8, 8, 0, 0, 8, 8);
            context.drawImage(image, 40, 8, 8, 8, 0, 0, 8, 8);
            setDrawnKey(remoteUrl);
        };
        image.src = remoteUrl;
    }, [remoteUrl]);

    return (
        <span
            className={cn(
                "relative inline-flex shrink-0 items-center justify-center overflow-hidden rounded-md bg-muted text-xs font-semibold text-muted-foreground uppercase",
                className,
            )}
            style={{ width: size, height: size }}
            aria-hidden
        >
            {!drawn && account.username.slice(0, 1)}
            <canvas
                ref={canvasRef}
                width={8}
                height={8}
                className={cn("absolute inset-0 size-full", !drawn && "invisible")}
                style={{ imageRendering: "pixelated" }}
            />
        </span>
    );
}

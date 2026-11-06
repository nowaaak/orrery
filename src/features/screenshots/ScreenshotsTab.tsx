import { useQuery, useQueryClient, type QueryClient } from "@tanstack/react-query";
import {
    ChevronLeftIcon,
    ChevronRightIcon,
    CopyIcon,
    FolderOpenIcon,
    ImageIcon,
    Trash2Icon,
} from "lucide-react";
import { useEffect, useRef, useState, useSyncExternalStore, type KeyboardEvent } from "react";
import { toast } from "sonner";
import { openInstanceFolder } from "@/api/instances";
import { errorMessage } from "@/api/invoke";
import { deleteScreenshot, listScreenshots, readScreenshot } from "@/api/screenshots";
import type { Instance } from "@/bindings/Instance";
import type { Screenshot } from "@/bindings/Screenshot";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { Skeleton } from "@/components/ui/skeleton";
import { useGameStore } from "@/features/launch/gameStore";
import { getT, useT } from "@/i18n";
import { formatBytes } from "@/lib/format";
import { cn } from "@/lib/utils";

const PAGE_SIZE = 24;
const UNDO_MS = 5000;
export const screenshotsKey = (instanceId: string) => ["screenshots", instanceId] as const;

const pendingDeletes = new Map<string, ReturnType<typeof setTimeout>>();
const pendingListeners = new Set<() => void>();
let pendingSnapshot: ReadonlySet<string> = new Set();

function notifyPending() {
    pendingSnapshot = new Set(pendingDeletes.keys());
    pendingListeners.forEach((listener) => listener());
}

function usePendingDeletes() {
    return useSyncExternalStore(
        (listener) => {
            pendingListeners.add(listener);
            return () => pendingListeners.delete(listener);
        },
        () => pendingSnapshot,
    );
}

function scheduleDelete(queryClient: QueryClient, instanceId: string, name: string) {
    const key = `${instanceId}/${name}`;
    const timer = setTimeout(() => {
        pendingDeletes.delete(key);
        deleteScreenshot(instanceId, name)
            .catch((error: unknown) =>
                toast.error(getT().screenshots.deleteFailed, { description: errorMessage(error) }),
            )
            .finally(() => {
                notifyPending();
                void queryClient.invalidateQueries({ queryKey: screenshotsKey(instanceId) });
            });
    }, UNDO_MS);
    pendingDeletes.set(key, timer);
    notifyPending();
    toast.success(getT().screenshots.deleted, {
        duration: UNDO_MS,
        action: {
            label: getT().screenshots.undo,
            onClick: () => {
                clearTimeout(pendingDeletes.get(key));
                pendingDeletes.delete(key);
                notifyPending();
            },
        },
    });
}

function useScreenshotBlob(instanceId: string, name: string, enabled: boolean) {
    return useQuery({
        queryKey: ["screenshot", instanceId, name],
        queryFn: async () => {
            const bytes = await readScreenshot(instanceId, name);
            return new Blob([bytes], {
                type: name.toLowerCase().endsWith(".png") ? "image/png" : "image/jpeg",
            });
        },
        enabled,
        staleTime: Infinity,
        gcTime: 60_000,
    });
}

function BlobImage({ blob, className }: { blob: Blob | undefined; className?: string }) {
    const ref = useRef<HTMLImageElement>(null);
    useEffect(() => {
        const image = ref.current;
        if (!blob || !image) {
            return;
        }
        const url = URL.createObjectURL(blob);
        image.src = url;
        return () => {
            image.removeAttribute("src");
            URL.revokeObjectURL(url);
        };
    }, [blob]);
    return <img ref={ref} alt="" decoding="async" className={className} />;
}

export function ScreenshotsTab({ instance }: { instance: Instance }) {
    const t = useT();
    const queryClient = useQueryClient();
    const running = useGameStore((state) => instance.id in state.running);
    const companion = useGameStore((state) => instance.id in state.companion);
    const pending = usePendingDeletes();
    const [shown, setShown] = useState(PAGE_SIZE);
    const [selected, setSelected] = useState<string | null>(null);
    const screenshots = useQuery({
        queryKey: screenshotsKey(instance.id),
        queryFn: () => listScreenshots(instance.id),
        refetchInterval: running && !companion ? 5000 : false,
    });

    useEffect(() => {
        if (!running) {
            void queryClient.invalidateQueries({ queryKey: screenshotsKey(instance.id) });
        }
    }, [running, instance.id, queryClient]);

    const visible = (screenshots.data ?? []).filter(
        (shot) => !pending.has(`${instance.id}/${shot.fileName}`),
    );
    const selectedIndex = visible.findIndex((shot) => shot.fileName === selected);

    return (
        <div className="flex flex-col gap-4">
            <div className="flex flex-wrap items-center justify-between gap-3">
                <p className="text-sm text-muted-foreground tabular-nums">
                    {screenshots.data ? t.screenshots.count(visible.length) : t.screenshots.loading}
                </p>
                <Button
                    variant="outline"
                    size="sm"
                    onClick={() =>
                        openInstanceFolder(instance.id, "screenshots").catch((error: unknown) =>
                            toast.error(getT().common.openFolderFailed, {
                                description: errorMessage(error),
                            }),
                        )
                    }
                >
                    <FolderOpenIcon data-icon="inline-start" />
                    {t.screenshots.openFolder}
                </Button>
            </div>
            {screenshots.error ? (
                <p className="text-sm text-destructive select-text">
                    {errorMessage(screenshots.error)}
                </p>
            ) : !screenshots.data ? (
                <Grid>
                    {[0, 1, 2, 3].map((key) => (
                        <Skeleton key={key} className="aspect-video rounded-xl" />
                    ))}
                </Grid>
            ) : visible.length === 0 ? (
                <div className="flex flex-col items-center gap-2 rounded-xl border border-dashed p-8 text-center">
                    <ImageIcon className="size-8 text-muted-foreground" aria-hidden />
                    <p className="font-medium">{t.screenshots.emptyTitle}</p>
                    <p className="text-sm text-muted-foreground">{t.screenshots.emptyHint}</p>
                </div>
            ) : (
                <>
                    <Grid>
                        {visible.slice(0, shown).map((shot) => (
                            <Thumbnail
                                key={shot.fileName}
                                instanceId={instance.id}
                                shot={shot}
                                onOpen={() => setSelected(shot.fileName)}
                            />
                        ))}
                    </Grid>
                    {visible.length > shown && (
                        <Button
                            variant="outline"
                            className="self-center"
                            onClick={() => setShown((count) => count + PAGE_SIZE)}
                        >
                            {t.screenshots.showMore(Math.min(PAGE_SIZE, visible.length - shown))}
                        </Button>
                    )}
                </>
            )}
            <Viewer
                instanceId={instance.id}
                shots={visible}
                index={selectedIndex}
                onSelect={(index) => setSelected(visible[index]?.fileName ?? null)}
                onClose={() => setSelected(null)}
                onDelete={(shot) => {
                    setSelected(null);
                    scheduleDelete(queryClient, instance.id, shot.fileName);
                }}
            />
        </div>
    );
}

function Grid({ children }: { children: React.ReactNode }) {
    return (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(14rem,1fr))] gap-3">{children}</div>
    );
}

function Thumbnail({
    instanceId,
    shot,
    onOpen,
}: {
    instanceId: string;
    shot: Screenshot;
    onOpen: () => void;
}) {
    const t = useT();
    const ref = useRef<HTMLButtonElement>(null);
    const [inView, setInView] = useState(false);
    const blob = useScreenshotBlob(instanceId, shot.fileName, inView);

    useEffect(() => {
        const element = ref.current;
        if (!element || inView) {
            return;
        }
        const observer = new IntersectionObserver(
            (entries) => {
                if (entries.some((entry) => entry.isIntersecting)) {
                    setInView(true);
                    observer.disconnect();
                }
            },
            { rootMargin: "200px" },
        );
        observer.observe(element);
        return () => observer.disconnect();
    }, [inView]);

    return (
        <button
            ref={ref}
            type="button"
            aria-label={t.screenshots.open(shot.fileName)}
            onClick={onOpen}
            className="group relative aspect-video overflow-hidden rounded-xl border bg-muted outline-none focus-visible:ring-2 focus-visible:ring-ring"
        >
            {blob.data ? (
                <BlobImage
                    blob={blob.data}
                    className="size-full object-cover transition-transform duration-200 group-hover:scale-[1.03] motion-reduce:transition-none"
                />
            ) : (
                <Skeleton className="size-full rounded-none" />
            )}
            <span className="absolute inset-x-0 bottom-0 truncate bg-gradient-to-t from-black/70 to-transparent px-2.5 pt-6 pb-1.5 text-left text-xs text-white">
                {shot.fileName}
            </span>
        </button>
    );
}

function Viewer({
    instanceId,
    shots,
    index,
    onSelect,
    onClose,
    onDelete,
}: {
    instanceId: string;
    shots: Screenshot[];
    index: number;
    onSelect: (index: number) => void;
    onClose: () => void;
    onDelete: (shot: Screenshot) => void;
}) {
    const t = useT();
    const shot = shots[index];
    const blob = useScreenshotBlob(instanceId, shot?.fileName ?? "", shot !== undefined);

    const copy = async () => {
        if (!blob.data) return;
        try {
            const png = blob.data.type === "image/png" ? blob.data : await toPng(blob.data);
            await navigator.clipboard.write([new ClipboardItem({ "image/png": png })]);
            toast.success(getT().screenshots.copied);
        } catch (error) {
            toast.error(getT().screenshots.copyFailed, { description: errorMessage(error) });
        }
    };

    const onKeyDown = (event: KeyboardEvent) => {
        if (event.key === "ArrowLeft" && index > 0) {
            event.preventDefault();
            onSelect(index - 1);
        } else if (event.key === "ArrowRight" && index < shots.length - 1) {
            event.preventDefault();
            onSelect(index + 1);
        }
    };

    return (
        <Dialog open={shot !== undefined} onOpenChange={(open) => !open && onClose()}>
            <DialogContent className="gap-3 sm:max-w-5xl" onKeyDown={onKeyDown}>
                {shot && (
                    <>
                        <DialogTitle className="truncate pr-8">{shot.fileName}</DialogTitle>
                        <DialogDescription className="tabular-nums">
                            {t.screenshots.position(index + 1, shots.length)} ·{" "}
                            {formatBytes(shot.sizeBytes)} ·{" "}
                            {new Date(shot.takenAtMs).toLocaleString()}
                        </DialogDescription>
                        <div className="relative flex aspect-video items-center justify-center overflow-hidden rounded-lg bg-black">
                            {blob.data ? (
                                <BlobImage
                                    blob={blob.data}
                                    className="max-h-full max-w-full object-contain"
                                />
                            ) : blob.error ? (
                                <p className="text-sm text-white/80">{t.screenshots.loadFailed}</p>
                            ) : (
                                <Skeleton className="size-full rounded-none" />
                            )}
                            <NavButton
                                side="left"
                                label={t.screenshots.previous}
                                disabled={index === 0}
                                onClick={() => onSelect(index - 1)}
                            />
                            <NavButton
                                side="right"
                                label={t.screenshots.next}
                                disabled={index === shots.length - 1}
                                onClick={() => onSelect(index + 1)}
                            />
                        </div>
                        <div className="flex flex-wrap justify-end gap-2">
                            <Button
                                variant="outline"
                                disabled={!blob.data}
                                onClick={() => void copy()}
                            >
                                <CopyIcon data-icon="inline-start" />
                                {t.screenshots.copy}
                            </Button>
                            <Button variant="outline" onClick={() => onDelete(shot)}>
                                <Trash2Icon data-icon="inline-start" />
                                {t.screenshots.delete}
                            </Button>
                        </div>
                    </>
                )}
            </DialogContent>
        </Dialog>
    );
}

function NavButton({
    side,
    label,
    disabled,
    onClick,
}: {
    side: "left" | "right";
    label: string;
    disabled: boolean;
    onClick: () => void;
}) {
    const Icon = side === "left" ? ChevronLeftIcon : ChevronRightIcon;
    return (
        <Button
            variant="secondary"
            size="icon"
            aria-label={label}
            disabled={disabled}
            onClick={onClick}
            className={cn(
                "absolute top-1/2 -translate-y-1/2 rounded-full opacity-80 hover:opacity-100 disabled:opacity-0",
                side === "left" ? "left-3" : "right-3",
            )}
        >
            <Icon />
        </Button>
    );
}

async function toPng(blob: Blob): Promise<Blob> {
    const bitmap = await createImageBitmap(blob);
    const canvas = document.createElement("canvas");
    canvas.width = bitmap.width;
    canvas.height = bitmap.height;
    canvas.getContext("2d")?.drawImage(bitmap, 0, 0);
    return new Promise((resolve, reject) =>
        canvas.toBlob(
            (png) => (png ? resolve(png) : reject(new Error("PNG conversion failed"))),
            "image/png",
        ),
    );
}

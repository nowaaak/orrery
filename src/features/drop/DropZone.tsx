import { useQueryClient } from "@tanstack/react-query";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { FileDownIcon } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { matchPath, useLocation } from "react-router";
import { toast } from "sonner";
import { inspectImport } from "@/api/imports";
import { inspectOptifine } from "@/api/optifine";
import { errorMessage } from "@/api/invoke";
import { importModpack, isModpack } from "@/api/modpacks";
import { installLocalFile } from "@/api/mods";
import { openImport } from "@/features/imports/ImportDialog";
import { openOptifine } from "@/features/optifine/OptifineDialog";
import { useInstances } from "@/features/instances/useInstances";
import { installedModsKey } from "@/features/mods/useMods";
import { getT, useT } from "@/i18n";

function fileName(path: string): string {
    return path.split(/[\\/]/).pop() ?? path;
}

function extension(path: string): string {
    return fileName(path).split(".").pop()?.toLowerCase() ?? "";
}

export function DropZone() {
    const t = useT();
    const queryClient = useQueryClient();
    const { pathname } = useLocation();
    const instances = useInstances();
    const instanceId = matchPath("/instances/:id", pathname)?.params.id ?? null;
    const instance = instances.data?.find((item) => item.id === instanceId) ?? null;
    const [dragging, setDragging] = useState(false);
    const target = useRef(instance);

    useEffect(() => {
        target.current = instance;
    }, [instance]);

    useEffect(() => {
        const handle = async (paths: string[]) => {
            const current = target.current;
            for (const path of paths) {
                const name = fileName(path);
                const ext = extension(path);
                try {
                    if (ext === "mrpack" || (ext === "zip" && (await isModpack(path)))) {
                        await importModpack(path);
                        toast.success(getT().content.modpacks.importing, {
                            description: getT().content.modpacks.progressHint,
                        });
                    } else if (ext === "jar" || (ext === "zip" && current)) {
                        if (!current) {
                            const optifine = ext === "jar" ? await inspectOptifine(path) : null;
                            if (optifine) {
                                openOptifine(path, optifine);
                            } else {
                                toast.info(getT().drop.needsInstance);
                            }
                            continue;
                        }
                        const added = await installLocalFile(current.id, path);
                        toast.success(getT().drop.added(added.title, added.kind));
                        await queryClient.invalidateQueries({
                            queryKey: installedModsKey(current.id),
                        });
                    } else {
                        const found = await inspectImport(path);
                        if (found.length === 0) {
                            toast.error(getT().drop.unsupported(name));
                        } else {
                            openImport(found);
                        }
                    }
                } catch (error) {
                    toast.error(getT().drop.addFailed(name), { description: errorMessage(error) });
                }
            }
        };

        let disposed = false;
        let unlisten: (() => void) | null = null;
        getCurrentWebview()
            .onDragDropEvent((event) => {
                if (event.payload.type === "enter" || event.payload.type === "over") {
                    setDragging(true);
                } else if (event.payload.type === "leave") {
                    setDragging(false);
                } else {
                    setDragging(false);
                    void handle(event.payload.paths);
                }
            })
            .then((stop) => {
                if (disposed) {
                    stop();
                } else {
                    unlisten = stop;
                }
            })
            .catch((error: unknown) => console.error("drag and drop is unavailable", error));
        return () => {
            disposed = true;
            unlisten?.();
        };
    }, [queryClient]);

    if (!dragging) {
        return null;
    }
    return (
        <div
            aria-hidden
            className="pointer-events-none fixed inset-0 z-[60] flex items-center justify-center bg-background/80 p-8 backdrop-blur-sm"
        >
            <div className="flex max-w-md flex-col items-center gap-3 rounded-2xl border-2 border-dashed border-primary/60 bg-card px-10 py-8 text-center">
                <FileDownIcon className="size-10 text-primary" />
                <p className="text-lg font-semibold">
                    {instance ? t.drop.overlayInstance(instance.name) : t.drop.overlayGlobal}
                </p>
                <p className="text-sm text-muted-foreground">
                    {instance ? t.drop.overlayHint : t.drop.overlayGlobalHint}
                </p>
            </div>
        </div>
    );
}

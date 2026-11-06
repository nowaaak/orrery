import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { open } from "@tauri-apps/plugin-dialog";
import { FolderOpenIcon, PackageIcon, PlusIcon, Trash2Icon } from "lucide-react";
import { toast } from "sonner";
import { errorMessage } from "@/api/invoke";
import { addDatapack, listDatapacks, openDatapacksFolder, removeDatapack } from "@/api/worlds";
import type { DataPack } from "@/bindings/DataPack";
import type { World } from "@/bindings/World";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogHeader,
    DialogTitle,
} from "@/components/ui/dialog";
import { Skeleton } from "@/components/ui/skeleton";
import { getT, useT } from "@/i18n";

export const datapacksKey = (instanceId: string, folder: string) =>
    ["datapacks", instanceId, folder] as const;

function reportError(title: string) {
    return (error: unknown) => toast.error(title, { description: errorMessage(error) });
}

export function DatapacksDialog({
    instanceId,
    world,
    open: isOpen,
    locked,
    onOpenChange,
}: {
    instanceId: string;
    world: World;
    open: boolean;
    locked: boolean;
    onOpenChange: (open: boolean) => void;
}) {
    return (
        <Dialog open={isOpen} onOpenChange={onOpenChange}>
            <DialogContent className="sm:max-w-xl">
                {isOpen && (
                    <DatapacksContent instanceId={instanceId} world={world} locked={locked} />
                )}
            </DialogContent>
        </Dialog>
    );
}

function DatapacksContent({
    instanceId,
    world,
    locked,
}: {
    instanceId: string;
    world: World;
    locked: boolean;
}) {
    const t = useT();
    const text = t.worlds.datapacks;
    const queryClient = useQueryClient();
    const key = datapacksKey(instanceId, world.folder);
    const packs = useQuery({
        queryKey: key,
        queryFn: () => listDatapacks(instanceId, world.folder),
    });
    const refresh = () => queryClient.invalidateQueries({ queryKey: key });
    const add = useMutation({
        mutationFn: (path: string) => addDatapack(instanceId, world.folder, path),
        onSuccess: (pack) => {
            toast.success(getT().worlds.datapacks.added(pack.name));
            return refresh();
        },
        onError: reportError(text.addFailed),
    });
    const remove = useMutation({
        mutationFn: (pack: DataPack) => removeDatapack(instanceId, world.folder, pack.name),
        onSuccess: (_, pack) => {
            toast.success(getT().worlds.datapacks.removed(pack.name));
            return refresh();
        },
        onError: reportError(text.removeFailed),
    });

    const onAdd = async () => {
        const path = await open({
            multiple: false,
            directory: false,
            filters: [{ name: text.filter, extensions: ["zip"] }],
        });
        if (typeof path === "string") {
            add.mutate(path);
        }
    };

    return (
        <>
            <DialogHeader>
                <DialogTitle>{text.title(world.name)}</DialogTitle>
                <DialogDescription className="text-pretty">{text.description}</DialogDescription>
            </DialogHeader>
            {locked && <p className="text-sm text-muted-foreground">{text.locked}</p>}
            {packs.error ? (
                <p className="text-sm text-destructive select-text">{errorMessage(packs.error)}</p>
            ) : !packs.data ? (
                <Skeleton className="h-24 w-full" />
            ) : packs.data.length === 0 ? (
                <div className="flex flex-col items-center gap-2 rounded-xl border border-dashed p-6 text-center">
                    <PackageIcon className="size-7 text-muted-foreground" aria-hidden />
                    <p className="text-sm text-muted-foreground">{text.empty}</p>
                </div>
            ) : (
                <ul className="flex max-h-80 flex-col gap-2 overflow-y-auto">
                    {packs.data.map((pack) => (
                        <li
                            key={pack.name}
                            className="flex items-center gap-3 rounded-lg border px-3 py-2"
                        >
                            <div className="min-w-0 flex-1">
                                <div className="flex min-w-0 items-center gap-2">
                                    <p className="truncate text-sm font-medium">{pack.name}</p>
                                    <Badge
                                        variant={pack.state === "enabled" ? "secondary" : "outline"}
                                        className="shrink-0"
                                    >
                                        {text.states[pack.state]}
                                    </Badge>
                                </div>
                                {pack.description && (
                                    <p className="line-clamp-2 text-xs text-muted-foreground">
                                        {pack.description}
                                    </p>
                                )}
                            </div>
                            <Button
                                variant="ghost"
                                size="icon"
                                aria-label={text.remove(pack.name)}
                                disabled={locked || remove.isPending}
                                onClick={() => remove.mutate(pack)}
                            >
                                <Trash2Icon />
                            </Button>
                        </li>
                    ))}
                </ul>
            )}
            <div className="flex flex-wrap justify-between gap-2">
                <Button
                    variant="ghost"
                    onClick={() =>
                        openDatapacksFolder(instanceId, world.folder).catch(
                            reportError(getT().common.openFolderFailed),
                        )
                    }
                >
                    <FolderOpenIcon data-icon="inline-start" />
                    {text.openFolder}
                </Button>
                <Button disabled={locked || add.isPending} onClick={() => void onAdd()}>
                    <PlusIcon data-icon="inline-start" />
                    {add.isPending ? text.adding : text.add}
                </Button>
            </div>
        </>
    );
}

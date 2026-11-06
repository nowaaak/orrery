import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { BoxIcon, GlobeIcon, RotateCcwIcon, Trash2Icon } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { errorMessage } from "@/api/invoke";
import { deleteTrashEntry, emptyTrash, listTrash, restoreTrash } from "@/api/trash";
import type { TrashEntry } from "@/bindings/TrashEntry";
import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { relativeTime } from "@/features/instances/labels";
import { instancesKey, useInstances } from "@/features/instances/useInstances";
import { useSettings, useUpdateSettings } from "@/features/settings/useSettings";
import { getT, useT } from "@/i18n";
import { formatBytes } from "@/lib/format";
import { trashKey } from "./undo";

const KEEP_DAYS = [0, 7, 14, 30, 90];

type Pending = { type: "entry"; entry: TrashEntry } | { type: "all" } | null;

export function TrashCard() {
    const t = useT();
    const text = t.trash;
    const queryClient = useQueryClient();
    const trash = useQuery({ queryKey: trashKey, queryFn: listTrash });
    const instances = useInstances();
    const settings = useSettings();
    const update = useUpdateSettings();
    const [pending, setPending] = useState<Pending>(null);
    const days = settings.data?.trashDays ?? 14;

    const refresh = () =>
        Promise.all([
            queryClient.invalidateQueries({ queryKey: trashKey }),
            queryClient.invalidateQueries({ queryKey: instancesKey }),
            queryClient.invalidateQueries({ queryKey: ["worlds"] }),
            queryClient.invalidateQueries({ queryKey: ["storage"] }),
        ]);

    const restore = useMutation({
        mutationFn: (entry: TrashEntry) => restoreTrash(entry.id),
        onSuccess: (restored) => toast.success(getT().trash.restored(restored.name)),
        onError: (error) =>
            toast.error(getT().trash.restoreFailed, { description: errorMessage(error) }),
        onSettled: refresh,
    });
    const remove = useMutation({
        mutationFn: (entry: TrashEntry) => deleteTrashEntry(entry.id),
        onError: (error) =>
            toast.error(getT().trash.deleteFailed, { description: errorMessage(error) }),
        onSettled: refresh,
    });
    const clear = useMutation({
        mutationFn: emptyTrash,
        onSuccess: () => toast.success(getT().trash.emptied),
        onError: (error) =>
            toast.error(getT().trash.emptyFailed, { description: errorMessage(error) }),
        onSettled: refresh,
    });

    const instanceName = (id: string) =>
        instances.data?.find((instance) => instance.id === id)?.name ?? id;
    const entries = trash.data ?? [];
    const busy = restore.isPending || remove.isPending || clear.isPending;

    const confirm = () => {
        if (pending?.type === "entry") {
            remove.mutate(pending.entry);
        } else if (pending?.type === "all") {
            clear.mutate();
        }
        setPending(null);
    };

    return (
        <Card className="max-w-3xl">
            <CardHeader>
                <CardTitle>{text.title}</CardTitle>
                <CardDescription>{text.description(days)}</CardDescription>
            </CardHeader>
            <CardContent className="flex flex-col gap-4">
                <div className="flex flex-wrap items-end justify-between gap-3">
                    <div className="grid gap-2">
                        <Label htmlFor="trash-days">{text.keep}</Label>
                        <Select
                            value={String(days)}
                            disabled={!settings.data || update.isPending}
                            onValueChange={(value) =>
                                settings.data &&
                                update.mutate({ ...settings.data, trashDays: Number(value) })
                            }
                        >
                            <SelectTrigger id="trash-days" className="w-48">
                                <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                                {KEEP_DAYS.map((value) => (
                                    <SelectItem key={value} value={String(value)}>
                                        {text.keepDays(value)}
                                    </SelectItem>
                                ))}
                            </SelectContent>
                        </Select>
                    </div>
                    <Button
                        variant="outline"
                        disabled={busy || entries.length === 0}
                        onClick={() => setPending({ type: "all" })}
                    >
                        <Trash2Icon data-icon="inline-start" />
                        {text.emptyAll}
                    </Button>
                </div>

                {trash.isPending ? (
                    <div className="flex flex-col gap-2">
                        <Skeleton className="h-12 w-full" />
                        <Skeleton className="h-12 w-full" />
                    </div>
                ) : trash.error ? (
                    <p className="text-sm text-destructive select-text">
                        {text.loadFailed}: {errorMessage(trash.error)}
                    </p>
                ) : entries.length === 0 ? (
                    <p className="rounded-xl border border-dashed p-5 text-center text-sm text-muted-foreground">
                        {text.empty}
                    </p>
                ) : (
                    <ul className="flex flex-col divide-y">
                        {entries.map((entry) => {
                            const Icon = entry.kind === "instance" ? BoxIcon : GlobeIcon;
                            const origin =
                                entry.kind === "world"
                                    ? text.worldFrom(instanceName(entry.instanceId))
                                    : text.kinds.instance;
                            return (
                                <li key={entry.id} className="flex items-center gap-3 py-2.5">
                                    <Icon
                                        className="size-4 shrink-0 text-muted-foreground"
                                        aria-hidden
                                    />
                                    <div className="min-w-0 flex-1">
                                        <p className="truncate text-sm font-medium">{entry.name}</p>
                                        <p className="truncate text-xs text-muted-foreground tabular-nums">
                                            {origin} ·{" "}
                                            {text.deletedAgo(relativeTime(entry.deletedAt))} ·{" "}
                                            {formatBytes(entry.sizeBytes)}
                                        </p>
                                    </div>
                                    <Button
                                        variant="outline"
                                        size="sm"
                                        disabled={busy}
                                        aria-label={text.restoreItem(entry.name)}
                                        onClick={() => restore.mutate(entry)}
                                    >
                                        <RotateCcwIcon data-icon="inline-start" />
                                        {text.restore}
                                    </Button>
                                    <Button
                                        variant="ghost"
                                        size="icon"
                                        disabled={busy}
                                        aria-label={text.deleteItem(entry.name)}
                                        title={text.deleteConfirm}
                                        onClick={() => setPending({ type: "entry", entry })}
                                    >
                                        <Trash2Icon />
                                    </Button>
                                </li>
                            );
                        })}
                    </ul>
                )}
            </CardContent>

            <AlertDialog open={pending !== null} onOpenChange={(open) => !open && setPending(null)}>
                <AlertDialogContent>
                    <AlertDialogHeader>
                        <AlertDialogTitle>
                            {pending?.type === "entry"
                                ? text.deleteTitle(pending.entry.name)
                                : text.emptyTitle}
                        </AlertDialogTitle>
                        <AlertDialogDescription>
                            {pending?.type === "entry"
                                ? text.deleteDescription
                                : text.emptyDescription(entries.length)}
                        </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel>{t.common.cancel}</AlertDialogCancel>
                        <AlertDialogAction variant="destructive" onClick={confirm}>
                            {text.deleteConfirm}
                        </AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
        </Card>
    );
}

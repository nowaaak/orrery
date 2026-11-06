import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
    ArchiveIcon,
    ArchiveRestoreIcon,
    EarthIcon,
    EllipsisVerticalIcon,
    FolderOpenIcon,
    HistoryIcon,
    MonitorUpIcon,
    PackageIcon,
    PlayIcon,
    Trash2Icon,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import { openInstanceFolder } from "@/api/instances";
import { errorMessage } from "@/api/invoke";
import {
    backupWorld,
    deleteWorld,
    deleteWorldBackup,
    listWorlds,
    restoreWorldBackup,
} from "@/api/worlds";
import type { Instance } from "@/bindings/Instance";
import type { World } from "@/bindings/World";
import type { WorldBackup } from "@/bindings/WorldBackup";
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
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogHeader,
    DialogTitle,
} from "@/components/ui/dialog";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuSeparator,
    DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Skeleton } from "@/components/ui/skeleton";
import { lastPlayedLabel } from "@/features/instances/labels";
import { useGameStore } from "@/features/launch/gameStore";
import { useLaunchInstance } from "@/features/launch/useLaunchInstance";
import { useTaskStore } from "@/features/tasks/taskStore";
import { formatBytes } from "@/lib/format";
import { getT, useT } from "@/i18n";
import { showTrashUndo } from "@/features/trash/undo";
import { createShortcutWithToast } from "@/features/links/shortcuts";
import { DatapacksDialog } from "./DatapacksDialog";

const worldsKey = (instanceId: string) => ["worlds", instanceId] as const;

function reportError(title: string) {
    return (error: unknown) => toast.error(title, { description: errorMessage(error) });
}

export function WorldsTab({ instance }: { instance: Instance }) {
    const t = useT();
    const queryClient = useQueryClient();
    const running = useGameStore((state) => instance.id in state.running);
    const worlds = useQuery({
        queryKey: worldsKey(instance.id),
        queryFn: () => listWorlds(instance.id),
    });

    useEffect(() => {
        if (!running) {
            void queryClient.invalidateQueries({ queryKey: worldsKey(instance.id) });
        }
    }, [running, instance.id, queryClient]);

    const count = worlds.data?.worlds.filter((world) => world.exists).length ?? 0;

    return (
        <div className="flex flex-col gap-4">
            <div className="flex flex-wrap items-center justify-between gap-3">
                <p className="text-sm text-muted-foreground">
                    {worlds.data ? t.worlds.count(count) : t.worlds.loading}
                    {running && ` · ${t.worlds.lockedWhileRunning}`}
                </p>
                <Button
                    variant="outline"
                    size="sm"
                    onClick={() =>
                        openInstanceFolder(instance.id, "saves").catch(
                            reportError(getT().common.openFolderFailed),
                        )
                    }
                >
                    <FolderOpenIcon data-icon="inline-start" />
                    {t.worlds.openFolder}
                </Button>
            </div>
            {worlds.data && !worlds.data.quickPlay && (
                <p className="text-sm text-muted-foreground">{t.worlds.quickPlayUnsupported}</p>
            )}
            {worlds.error ? (
                <p className="text-sm text-destructive select-text">{errorMessage(worlds.error)}</p>
            ) : !worlds.data ? (
                <div className="flex flex-col gap-2">
                    {[0, 1, 2].map((key) => (
                        <Skeleton key={key} className="h-[4.5rem] w-full rounded-xl" />
                    ))}
                </div>
            ) : worlds.data.worlds.length === 0 ? (
                <div className="flex flex-col items-center gap-2 rounded-xl border border-dashed p-8 text-center">
                    <EarthIcon className="size-8 text-muted-foreground" aria-hidden />
                    <p className="font-medium">{t.worlds.emptyTitle}</p>
                    <p className="text-sm text-muted-foreground">{t.worlds.emptyHint}</p>
                </div>
            ) : (
                <ul className="flex flex-col gap-2">
                    {worlds.data.worlds.map((world) => (
                        <WorldRow
                            key={world.folder}
                            instanceId={instance.id}
                            world={world}
                            quickPlay={worlds.data.quickPlay}
                            running={running}
                        />
                    ))}
                </ul>
            )}
        </div>
    );
}

function WorldIcon({ icon }: { icon: string | null }) {
    return icon ? (
        <img
            src={icon}
            alt=""
            className="size-12 shrink-0 rounded-lg bg-muted object-cover [image-rendering:pixelated]"
        />
    ) : (
        <span className="flex size-12 shrink-0 items-center justify-center rounded-lg bg-muted">
            <EarthIcon className="size-6 text-muted-foreground" aria-hidden />
        </span>
    );
}

function WorldRow({
    instanceId,
    world,
    quickPlay,
    running,
}: {
    instanceId: string;
    world: World;
    quickPlay: boolean;
    running: boolean;
}) {
    const t = useT();
    const queryClient = useQueryClient();
    const launch = useLaunchInstance();
    const busyTask = useTaskStore((state) =>
        Object.values(state.tasks).some(
            (task) => task.instanceId === instanceId && task.status.type === "running",
        ),
    );
    const [pending, setPending] = useState(false);
    const [showBackups, setShowBackups] = useState(false);
    const [showDatapacks, setShowDatapacks] = useState(false);
    const refresh = () => queryClient.invalidateQueries({ queryKey: worldsKey(instanceId) });

    const onPlay = async () => {
        setPending(true);
        try {
            await launch(instanceId, world.folder);
        } finally {
            setPending(false);
        }
    };

    const onBackup = async () => {
        setPending(true);
        const toastId = toast.loading(getT().worlds.backupCreating(world.name));
        try {
            const backup = await backupWorld(instanceId, world.folder);
            toast.success(getT().worlds.backupCreated, {
                id: toastId,
                description: `${world.name}, ${formatBytes(backup.sizeBytes)}`,
            });
            await refresh();
        } catch (error) {
            toast.error(getT().worlds.backupFailed, {
                id: toastId,
                description: errorMessage(error),
            });
        } finally {
            setPending(false);
        }
    };

    const onDelete = async () => {
        setPending(true);
        try {
            const entry = await deleteWorld(instanceId, world.folder);
            await refresh();
            showTrashUndo({ ...entry, name: world.name }, () => void refresh());
        } catch (error) {
            reportError(getT().worlds.deleteFailed)(error);
        } finally {
            setPending(false);
        }
    };

    const meta = world.exists
        ? [
              world.gameMode && t.worlds.gameModes[world.gameMode],
              world.version,
              lastPlayedLabel(world.lastPlayedMs === null ? null : world.lastPlayedMs / 1000),
              formatBytes(world.sizeBytes),
          ]
        : [t.worlds.backupCount(world.backups.length), t.worlds.worldGone];
    const locked = running || pending;

    return (
        <li className="flex items-center gap-3 rounded-xl border bg-card px-3 py-3">
            <WorldIcon icon={world.icon} />
            <div className="min-w-0 flex-1">
                <div className="flex min-w-0 items-center gap-2">
                    <p className="min-w-0 truncate font-medium">{world.name}</p>
                    {world.hardcore && (
                        <Badge variant="destructive" className="shrink-0">
                            {t.worlds.hardcore}
                        </Badge>
                    )}
                    {!world.exists && (
                        <Badge variant="secondary" className="shrink-0">
                            {t.worlds.backupsOnly}
                        </Badge>
                    )}
                </div>
                <p className="truncate text-sm text-muted-foreground">
                    {meta.filter(Boolean).join(" · ")}
                </p>
            </div>
            {world.exists && quickPlay && (
                <Button
                    size="sm"
                    disabled={locked || busyTask}
                    onClick={() => void onPlay()}
                    aria-label={t.worlds.playWorld(world.name)}
                >
                    <PlayIcon data-icon="inline-start" />
                    {t.common.play}
                </Button>
            )}
            {world.exists && (
                <Button
                    size="sm"
                    variant="outline"
                    disabled={locked}
                    onClick={() => void onBackup()}
                    aria-label={t.worlds.createBackup(world.name)}
                >
                    <ArchiveIcon data-icon="inline-start" />
                    {t.worlds.backup}
                </Button>
            )}
            <DropdownMenu>
                <DropdownMenuTrigger asChild>
                    <Button
                        variant="ghost"
                        size="icon"
                        aria-label={t.worlds.actionsFor(world.name)}
                    >
                        <EllipsisVerticalIcon />
                    </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end" className="w-56">
                    <DropdownMenuItem
                        disabled={world.backups.length === 0}
                        onSelect={() => setShowBackups(true)}
                    >
                        <HistoryIcon />
                        {t.worlds.backupsMenu(world.backups.length)}
                    </DropdownMenuItem>
                    {world.exists && quickPlay && (
                        <DropdownMenuItem
                            onSelect={() => void createShortcutWithToast(instanceId, world.folder)}
                        >
                            <MonitorUpIcon />
                            {t.links.createShortcut}
                        </DropdownMenuItem>
                    )}
                    {world.exists && (
                        <DropdownMenuItem onSelect={() => setShowDatapacks(true)}>
                            <PackageIcon />
                            {t.worlds.datapacksMenu}
                        </DropdownMenuItem>
                    )}
                    {world.exists && (
                        <>
                            <DropdownMenuSeparator />
                            <DropdownMenuItem
                                variant="destructive"
                                disabled={locked}
                                onSelect={() => void onDelete()}
                            >
                                <Trash2Icon />
                                {t.worlds.deleteWorld}
                            </DropdownMenuItem>
                        </>
                    )}
                </DropdownMenuContent>
            </DropdownMenu>
            <BackupsDialog
                instanceId={instanceId}
                world={world}
                open={showBackups}
                locked={running}
                onOpenChange={setShowBackups}
            />
            <DatapacksDialog
                instanceId={instanceId}
                world={world}
                open={showDatapacks}
                locked={running}
                onOpenChange={setShowDatapacks}
            />
        </li>
    );
}

function BackupsDialog({
    instanceId,
    world,
    open,
    locked,
    onOpenChange,
}: {
    instanceId: string;
    world: World;
    open: boolean;
    locked: boolean;
    onOpenChange: (open: boolean) => void;
}) {
    const t = useT();
    const backupDate = useMemo(
        () => new Intl.DateTimeFormat(t.common.locale, { dateStyle: "medium", timeStyle: "short" }),
        [t.common.locale],
    );
    const queryClient = useQueryClient();
    const [pending, setPending] = useState(false);
    const [restoreTarget, setRestoreTarget] = useState<WorldBackup | null>(null);
    const refresh = () => queryClient.invalidateQueries({ queryKey: worldsKey(instanceId) });

    const onRestore = async (backup: WorldBackup) => {
        setPending(true);
        const toastId = toast.loading(getT().worlds.restoring);
        try {
            await restoreWorldBackup(instanceId, world.folder, backup.fileName);
            toast.success(getT().worlds.restored(world.name), {
                id: toastId,
                description: getT().worlds.restoredFrom(backupDate.format(backup.createdAtMs)),
            });
            await refresh();
            onOpenChange(false);
        } catch (error) {
            toast.error(getT().worlds.restoreFailed, {
                id: toastId,
                description: errorMessage(error),
            });
        } finally {
            setPending(false);
        }
    };

    const onDelete = async (backup: WorldBackup) => {
        setPending(true);
        try {
            await deleteWorldBackup(instanceId, world.folder, backup.fileName);
            await refresh();
            if (world.backups.length <= 1) {
                onOpenChange(false);
            }
        } catch (error) {
            reportError(getT().worlds.backupDeleteFailed)(error);
        } finally {
            setPending(false);
        }
    };

    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent className="sm:max-w-lg">
                <DialogHeader>
                    <DialogTitle>{t.worlds.backupsTitle(world.name)}</DialogTitle>
                    <DialogDescription>{t.worlds.backupsDescription}</DialogDescription>
                </DialogHeader>
                <ul className="flex max-h-80 flex-col divide-y overflow-y-auto">
                    {world.backups.map((backup) => (
                        <li key={backup.fileName} className="flex items-center gap-3 py-2.5">
                            <div className="min-w-0 flex-1">
                                <p className="text-sm font-medium tabular-nums">
                                    {backupDate.format(backup.createdAtMs)}
                                </p>
                                <p className="text-xs text-muted-foreground tabular-nums">
                                    {formatBytes(backup.sizeBytes)}
                                </p>
                            </div>
                            <Button
                                size="sm"
                                variant="outline"
                                disabled={pending || locked}
                                onClick={() => setRestoreTarget(backup)}
                            >
                                <ArchiveRestoreIcon data-icon="inline-start" />
                                {t.worlds.restore}
                            </Button>
                            <Button
                                size="icon"
                                variant="ghost"
                                disabled={pending}
                                aria-label={t.worlds.deleteBackup(
                                    backupDate.format(backup.createdAtMs),
                                )}
                                onClick={() => void onDelete(backup)}
                            >
                                <Trash2Icon />
                            </Button>
                        </li>
                    ))}
                </ul>
            </DialogContent>
            <AlertDialog
                open={restoreTarget !== null}
                onOpenChange={(next) => !next && setRestoreTarget(null)}
            >
                <AlertDialogContent>
                    <AlertDialogHeader>
                        <AlertDialogTitle>{t.worlds.restoreConfirmTitle}</AlertDialogTitle>
                        <AlertDialogDescription>
                            {world.exists
                                ? t.worlds.restoreReplaces(world.name)
                                : t.worlds.restoreRecreates(world.name)}
                        </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel>{t.common.cancel}</AlertDialogCancel>
                        <AlertDialogAction
                            onClick={() => restoreTarget && void onRestore(restoreTarget)}
                        >
                            {t.worlds.restore}
                        </AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
        </Dialog>
    );
}

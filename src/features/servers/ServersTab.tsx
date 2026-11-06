import { useMutation, useQuery, useQueryClient, type UseQueryResult } from "@tanstack/react-query";
import {
    ArrowDownIcon,
    ArrowUpIcon,
    EllipsisVerticalIcon,
    MonitorUpIcon,
    PencilIcon,
    PlayIcon,
    PlusIcon,
    RefreshCwIcon,
    ServerIcon,
    SignalIcon,
    TimerIcon,
    Trash2Icon,
    UsersIcon,
} from "lucide-react";
import { useEffect, useState, type FormEvent } from "react";
import { toast } from "sonner";
import { errorMessage } from "@/api/invoke";
import { changeServers, listServers, pingServer } from "@/api/servers";
import type { Instance } from "@/bindings/Instance";
import type { SavedServer } from "@/bindings/SavedServer";
import type { ServerChange } from "@/bindings/ServerChange";
import type { ServerStatus } from "@/bindings/ServerStatus";
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
    DialogFooter,
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
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { isOnServer } from "@/features/launch/companionStatus";
import { useGameStore } from "@/features/launch/gameStore";
import { useLaunchInstance } from "@/features/launch/useLaunchInstance";
import { createShortcutWithToast } from "@/features/links/shortcuts";
import { getT, useT } from "@/i18n";
import { formatPlaytime } from "@/lib/format";

export const serversKey = (instanceId: string) => ["servers", instanceId] as const;
const PING_STALE_MS = 60_000;

type Editing = { mode: "add" } | { mode: "edit"; server: SavedServer } | null;

export function ServersTab({ instance }: { instance: Instance }) {
    const t = useT();
    const queryClient = useQueryClient();
    const running = useGameStore((state) => instance.id in state.running);
    const [editing, setEditing] = useState<Editing>(null);
    const [removing, setRemoving] = useState<SavedServer | null>(null);
    const servers = useQuery({
        queryKey: serversKey(instance.id),
        queryFn: () => listServers(instance.id),
    });
    const change = useMutation({
        mutationFn: (next: ServerChange) => changeServers(instance.id, next),
        onSuccess: (list) => queryClient.setQueryData(serversKey(instance.id), list),
        onError: (error) =>
            toast.error(getT().servers.saveFailed, { description: errorMessage(error) }),
    });

    useEffect(() => {
        if (!running) {
            void queryClient.invalidateQueries({ queryKey: serversKey(instance.id) });
        }
    }, [running, instance.id, queryClient]);

    const list = servers.data ?? [];

    return (
        <div className="flex flex-col gap-4">
            <div className="flex flex-wrap items-center justify-between gap-3">
                <p className="text-sm text-muted-foreground">
                    {servers.data ? t.servers.count(list.length) : t.servers.loading}
                    {running && ` · ${t.servers.lockedWhileRunning}`}
                </p>
                <div className="flex gap-2">
                    <Button
                        variant="ghost"
                        size="sm"
                        disabled={list.length === 0}
                        onClick={() => void queryClient.invalidateQueries({ queryKey: ["ping"] })}
                    >
                        <RefreshCwIcon data-icon="inline-start" />
                        {t.servers.refresh}
                    </Button>
                    <Button
                        variant="outline"
                        size="sm"
                        disabled={running || !servers.data}
                        onClick={() => setEditing({ mode: "add" })}
                    >
                        <PlusIcon data-icon="inline-start" />
                        {t.servers.add}
                    </Button>
                </div>
            </div>
            {servers.error ? (
                <p className="text-sm text-destructive select-text">
                    {errorMessage(servers.error)}
                </p>
            ) : !servers.data ? (
                <div className="flex flex-col gap-2">
                    {[0, 1, 2].map((key) => (
                        <Skeleton key={key} className="h-[4.5rem] w-full rounded-xl" />
                    ))}
                </div>
            ) : list.length === 0 ? (
                <div className="flex flex-col items-center gap-2 rounded-xl border border-dashed p-8 text-center">
                    <ServerIcon className="size-8 text-muted-foreground" aria-hidden />
                    <p className="font-medium">{t.servers.emptyTitle}</p>
                    <p className="text-sm text-muted-foreground">{t.servers.emptyHint}</p>
                </div>
            ) : (
                <ul className="flex flex-col gap-2">
                    {list.map((server, position) => (
                        <ServerRow
                            key={`${server.index}:${server.address}`}
                            instanceId={instance.id}
                            server={server}
                            locked={running || change.isPending}
                            previousIndex={list[position - 1]?.index ?? null}
                            nextIndex={list[position + 1]?.index ?? null}
                            onEdit={() => setEditing({ mode: "edit", server })}
                            onRemove={() => setRemoving(server)}
                            onMove={(to) =>
                                change.mutate({
                                    type: "move",
                                    index: server.index,
                                    expectedAddress: server.address,
                                    to,
                                })
                            }
                        />
                    ))}
                </ul>
            )}
            <ServerDialog
                editing={editing}
                pending={change.isPending}
                onClose={() => setEditing(null)}
                onSubmit={(server) => {
                    const next: ServerChange =
                        editing?.mode === "edit"
                            ? {
                                  type: "edit",
                                  index: editing.server.index,
                                  expectedAddress: editing.server.address,
                                  server,
                              }
                            : { type: "add", server };
                    change.mutate(next, { onSuccess: () => setEditing(null) });
                }}
            />
            <AlertDialog
                open={removing !== null}
                onOpenChange={(open) => !open && setRemoving(null)}
            >
                <AlertDialogContent>
                    <AlertDialogHeader>
                        <AlertDialogTitle>
                            {removing && t.servers.removeTitle(removing.name)}
                        </AlertDialogTitle>
                        <AlertDialogDescription>
                            {t.servers.removeDescription}
                        </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel>{t.common.cancel}</AlertDialogCancel>
                        <AlertDialogAction
                            variant="destructive"
                            onClick={() => {
                                if (!removing) return;
                                const name = removing.name;
                                change.mutate(
                                    {
                                        type: "remove",
                                        index: removing.index,
                                        expectedAddress: removing.address,
                                    },
                                    {
                                        onSuccess: () =>
                                            toast.success(getT().servers.removed(name)),
                                    },
                                );
                            }}
                        >
                            {t.servers.remove}
                        </AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
        </div>
    );
}

function ServerRow({
    instanceId,
    server,
    locked,
    previousIndex,
    nextIndex,
    onEdit,
    onRemove,
    onMove,
}: {
    instanceId: string;
    server: SavedServer;
    locked: boolean;
    previousIndex: number | null;
    nextIndex: number | null;
    onEdit: () => void;
    onRemove: () => void;
    onMove: (to: number) => void;
}) {
    const t = useT();
    const launch = useLaunchInstance();
    const running = useGameStore((state) => instanceId in state.running);
    const playingHere = useGameStore((state) =>
        isOnServer(state.companion[instanceId]?.status, server.address),
    );
    const status = useQuery({
        queryKey: ["ping", server.address],
        queryFn: () => pingServer(server.address),
        staleTime: PING_STALE_MS,
        retry: false,
    });
    const icon = status.data?.favicon ?? server.icon;

    return (
        <li className="flex items-center gap-4 rounded-xl border bg-card p-3">
            {icon ? (
                <img
                    src={icon}
                    alt=""
                    className="size-12 shrink-0 rounded-lg bg-muted object-cover [image-rendering:pixelated]"
                />
            ) : (
                <span className="flex size-12 shrink-0 items-center justify-center rounded-lg bg-muted">
                    <ServerIcon className="size-6 text-muted-foreground" aria-hidden />
                </span>
            )}
            <div className="flex min-w-0 flex-1 flex-col gap-0.5">
                <div className="flex min-w-0 items-baseline gap-2">
                    <p className="truncate font-medium">{server.name}</p>
                    <p className="truncate font-mono text-xs text-muted-foreground select-text">
                        {server.address}
                    </p>
                    {playingHere && (
                        <Badge variant="secondary" className="shrink-0">
                            {t.servers.playingNow}
                        </Badge>
                    )}
                </div>
                <ServerStatusLine status={status} />
                {server.playSecs >= 60 && (
                    <p className="flex items-center gap-1.5 text-xs text-muted-foreground tabular-nums">
                        <TimerIcon className="size-3.5" aria-hidden />
                        {t.servers.playedFor(formatPlaytime(server.playSecs))}
                    </p>
                )}
            </div>
            <Button
                size="sm"
                disabled={running}
                aria-label={t.servers.joinServer(server.name)}
                onClick={() => void launch(instanceId, null, server.address)}
            >
                <PlayIcon data-icon="inline-start" />
                {t.servers.join}
            </Button>
            <DropdownMenu>
                <DropdownMenuTrigger asChild>
                    <Button
                        variant="ghost"
                        size="icon"
                        aria-label={t.servers.actionsFor(server.name)}
                    >
                        <EllipsisVerticalIcon />
                    </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end" className="w-48">
                    <DropdownMenuItem disabled={locked} onSelect={onEdit}>
                        <PencilIcon />
                        {t.servers.edit}
                    </DropdownMenuItem>
                    <DropdownMenuItem
                        disabled={locked || previousIndex === null}
                        onSelect={() => previousIndex !== null && onMove(previousIndex)}
                    >
                        <ArrowUpIcon />
                        {t.servers.moveUp}
                    </DropdownMenuItem>
                    <DropdownMenuItem
                        disabled={locked || nextIndex === null}
                        onSelect={() => nextIndex !== null && onMove(nextIndex)}
                    >
                        <ArrowDownIcon />
                        {t.servers.moveDown}
                    </DropdownMenuItem>
                    <DropdownMenuItem
                        onSelect={() =>
                            void createShortcutWithToast(instanceId, null, server.address)
                        }
                    >
                        <MonitorUpIcon />
                        {t.links.createShortcut}
                    </DropdownMenuItem>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem variant="destructive" disabled={locked} onSelect={onRemove}>
                        <Trash2Icon />
                        {t.servers.remove}
                    </DropdownMenuItem>
                </DropdownMenuContent>
            </DropdownMenu>
        </li>
    );
}

function ServerStatusLine({ status }: { status: UseQueryResult<ServerStatus> }) {
    const t = useT();
    if (status.isPending) {
        return <p className="text-sm text-muted-foreground">{t.servers.pinging}</p>;
    }
    if (!status.data) {
        return <p className="text-sm text-muted-foreground">{t.servers.offline}</p>;
    }
    const data = status.data;
    return (
        <div className="flex min-w-0 flex-col gap-0.5 text-sm">
            {data.motd && (
                <p className="line-clamp-2 whitespace-pre-line text-muted-foreground">
                    {data.motd}
                </p>
            )}
            <p className="flex flex-wrap items-center gap-x-3 gap-y-0.5 text-xs text-muted-foreground tabular-nums">
                {data.playersOnline !== null && (
                    <span className="inline-flex items-center gap-1">
                        <UsersIcon className="size-3.5" aria-hidden />
                        {t.servers.players(data.playersOnline, data.playersMax)}
                    </span>
                )}
                {data.version && <span>{data.version}</span>}
                <span className="inline-flex items-center gap-1">
                    <SignalIcon className="size-3.5" aria-hidden />
                    {t.servers.latency(data.latencyMs)}
                </span>
            </p>
        </div>
    );
}

function ServerDialog({
    editing,
    pending,
    onClose,
    onSubmit,
}: {
    editing: Editing;
    pending: boolean;
    onClose: () => void;
    onSubmit: (server: { name: string; address: string }) => void;
}) {
    return (
        <Dialog open={editing !== null} onOpenChange={(open) => !open && onClose()}>
            <DialogContent>
                {editing && (
                    <ServerForm
                        key={editing.mode === "edit" ? editing.server.index : "add"}
                        initial={editing.mode === "edit" ? editing.server : null}
                        pending={pending}
                        onSubmit={onSubmit}
                    />
                )}
            </DialogContent>
        </Dialog>
    );
}

function ServerForm({
    initial,
    pending,
    onSubmit,
}: {
    initial: SavedServer | null;
    pending: boolean;
    onSubmit: (server: { name: string; address: string }) => void;
}) {
    const t = useT();
    const text = t.servers.dialog;
    const [name, setName] = useState(initial?.name ?? "");
    const [address, setAddress] = useState(initial?.address ?? "");

    const submit = (event: FormEvent) => {
        event.preventDefault();
        onSubmit({
            name: name.trim() === "" ? text.namePlaceholder : name.trim(),
            address: address.trim(),
        });
    };

    return (
        <form onSubmit={submit} className="grid gap-4">
            <DialogHeader>
                <DialogTitle>{initial ? text.editTitle : text.addTitle}</DialogTitle>
                <DialogDescription>{text.description}</DialogDescription>
            </DialogHeader>
            <div className="grid gap-2">
                <Label htmlFor="server-name">{text.name}</Label>
                <Input
                    id="server-name"
                    value={name}
                    maxLength={128}
                    placeholder={text.namePlaceholder}
                    autoComplete="off"
                    onChange={(event) => setName(event.target.value)}
                />
            </div>
            <div className="grid gap-2">
                <Label htmlFor="server-address">{text.address}</Label>
                <Input
                    id="server-address"
                    value={address}
                    placeholder={text.addressPlaceholder}
                    spellCheck={false}
                    autoComplete="off"
                    required
                    className="font-mono"
                    onChange={(event) => setAddress(event.target.value)}
                />
            </div>
            <DialogFooter>
                <Button type="submit" disabled={pending || address.trim() === ""}>
                    {pending ? t.common.saving : text.save}
                </Button>
            </DialogFooter>
        </form>
    );
}

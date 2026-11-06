import { useInfiniteQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
    CheckIcon,
    EllipsisIcon,
    PencilIcon,
    PlusIcon,
    SearchIcon,
    Trash2Icon,
    XIcon,
} from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { errorMessage } from "@/api/invoke";
import {
    addSyncItem,
    createSyncPack,
    deleteSyncPack,
    removeSyncItem,
    renameSyncPack,
    searchSyncProjects,
} from "@/api/syncpacks";
import { PageHeader } from "@/app/PageHeader";
import type { ContentKind } from "@/bindings/ContentKind";
import type { SyncPack } from "@/bindings/SyncPack";
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
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
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
    DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useInstances } from "@/features/instances/useInstances";
import { ModIcon } from "@/features/mods/ModIcon";
import { getT, useT } from "@/i18n";
import { compactNumber } from "@/lib/format";
import { useDebouncedValue } from "@/lib/useDebouncedValue";
import { announceSync, syncPacksKey, useSetSubscriptions, useSyncPacks } from "./useSyncPacks";

const KINDS: ContentKind[] = ["mod", "resource_pack", "shader"];

export function SyncPacksPage() {
    const t = useT();
    const text = t.syncpacks;
    const packs = useSyncPacks();
    const [creating, setCreating] = useState(false);

    return (
        <div className="flex flex-col gap-6">
            <PageHeader
                title={text.title}
                actions={
                    <Button onClick={() => setCreating(true)}>
                        <PlusIcon data-icon="inline-start" />
                        {text.create}
                    </Button>
                }
            />
            <p className="max-w-3xl text-sm text-pretty text-muted-foreground">{text.intro}</p>
            {packs.isPending ? (
                <Skeleton className="h-48 w-full max-w-3xl" />
            ) : packs.error ? (
                <p className="text-sm text-destructive select-text">
                    {text.loadFailed}: {errorMessage(packs.error)}
                </p>
            ) : packs.data.length === 0 ? (
                <p className="max-w-3xl rounded-xl border border-dashed p-6 text-center text-sm text-muted-foreground">
                    {text.empty}
                </p>
            ) : (
                packs.data.map((pack) => <PackCard key={pack.id} pack={pack} />)
            )}
            <NameDialog open={creating} onOpenChange={setCreating} />
        </div>
    );
}

function NameDialog({
    open,
    onOpenChange,
    pack,
}: {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    pack?: SyncPack;
}) {
    const t = useT();
    const text = t.syncpacks;
    const queryClient = useQueryClient();
    const [name, setName] = useState(pack?.name ?? "");
    const save = useMutation({
        mutationFn: () => (pack ? renameSyncPack(pack.id, name) : createSyncPack(name)),
        onSuccess: () => {
            onOpenChange(false);
            setName(pack?.name ?? "");
        },
        onError: (error) =>
            toast.error(pack ? getT().syncpacks.renameFailed : getT().syncpacks.createFailed, {
                description: errorMessage(error),
            }),
        onSettled: () => queryClient.invalidateQueries({ queryKey: syncPacksKey }),
    });
    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent>
                <form
                    className="flex flex-col gap-4"
                    onSubmit={(event) => {
                        event.preventDefault();
                        save.mutate();
                    }}
                >
                    <DialogHeader>
                        <DialogTitle>{pack ? text.rename : text.createTitle}</DialogTitle>
                    </DialogHeader>
                    <div className="grid gap-2">
                        <Label htmlFor="sync-pack-name">{text.name}</Label>
                        <Input
                            id="sync-pack-name"
                            value={name}
                            maxLength={48}
                            placeholder={text.namePlaceholder}
                            onChange={(event) => setName(event.target.value)}
                        />
                    </div>
                    <DialogFooter>
                        <Button type="button" variant="ghost" onClick={() => onOpenChange(false)}>
                            {t.common.cancel}
                        </Button>
                        <Button type="submit" disabled={name.trim() === "" || save.isPending}>
                            {pack ? t.common.save : text.createConfirm}
                        </Button>
                    </DialogFooter>
                </form>
            </DialogContent>
        </Dialog>
    );
}

function PackCard({ pack }: { pack: SyncPack }) {
    const t = useT();
    const text = t.syncpacks;
    const queryClient = useQueryClient();
    const instances = useInstances();
    const subscribe = useSetSubscriptions();
    const [adding, setAdding] = useState(false);
    const [renaming, setRenaming] = useState(false);
    const [deleting, setDeleting] = useState(false);
    const refresh = () => queryClient.invalidateQueries({ queryKey: syncPacksKey });

    const removeItem = useMutation({
        mutationFn: (projectId: string) => removeSyncItem(pack.id, projectId),
        onSuccess: announceSync,
        onError: (error) =>
            toast.error(getT().syncpacks.removeFailed, { description: errorMessage(error) }),
        onSettled: refresh,
    });
    const remove = useMutation({
        mutationFn: () => deleteSyncPack(pack.id),
        onSuccess: announceSync,
        onError: (error) =>
            toast.error(getT().syncpacks.deleteFailed, { description: errorMessage(error) }),
        onSettled: () =>
            Promise.all([refresh(), queryClient.invalidateQueries({ queryKey: ["instances"] })]),
    });

    return (
        <Card className="max-w-3xl">
            <CardHeader className="flex flex-row items-start justify-between gap-4">
                <div className="flex flex-col gap-1.5">
                    <CardTitle>{pack.name}</CardTitle>
                    <CardDescription>{text.items(pack.items.length)}</CardDescription>
                </div>
                <div className="flex items-center gap-2">
                    <Button variant="outline" onClick={() => setAdding(true)}>
                        <PlusIcon data-icon="inline-start" />
                        {text.addProject}
                    </Button>
                    <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                            <Button
                                variant="ghost"
                                size="icon"
                                aria-label={text.actions(pack.name)}
                            >
                                <EllipsisIcon />
                            </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                            <DropdownMenuItem onSelect={() => setRenaming(true)}>
                                <PencilIcon />
                                {text.rename}
                            </DropdownMenuItem>
                            <DropdownMenuItem
                                variant="destructive"
                                onSelect={() => setDeleting(true)}
                            >
                                <Trash2Icon />
                                {text.remove}
                            </DropdownMenuItem>
                        </DropdownMenuContent>
                    </DropdownMenu>
                </div>
            </CardHeader>
            <CardContent className="grid gap-6 md:grid-cols-[minmax(0,1fr)_16rem]">
                <div className="min-w-0">
                    {pack.items.length === 0 ? (
                        <p className="text-sm text-muted-foreground">{text.noItems}</p>
                    ) : (
                        <ul className="flex flex-col divide-y">
                            {pack.items.map((item) => (
                                <li key={item.projectId} className="flex items-center gap-3 py-2">
                                    <ModIcon url={item.iconUrl} />
                                    <div className="min-w-0 flex-1">
                                        <p className="truncate text-sm font-medium">{item.title}</p>
                                        <p className="text-xs text-muted-foreground">
                                            {text.kindLabel[item.kind]}
                                        </p>
                                    </div>
                                    <Button
                                        variant="ghost"
                                        size="icon"
                                        disabled={removeItem.isPending}
                                        aria-label={text.removeItem(item.title)}
                                        onClick={() => removeItem.mutate(item.projectId)}
                                    >
                                        <XIcon />
                                    </Button>
                                </li>
                            ))}
                        </ul>
                    )}
                </div>
                <div className="flex flex-col gap-2">
                    <p className="text-sm font-medium">{text.instances}</p>
                    <p className="text-xs text-muted-foreground">{text.instancesHint}</p>
                    {(instances.data ?? []).length === 0 ? (
                        <p className="text-xs text-muted-foreground">{text.noInstances}</p>
                    ) : (
                        <ul className="flex flex-col gap-1">
                            {(instances.data ?? []).map((instance) => {
                                const subscribed = instance.syncPacks.some(
                                    (s) => s.packId === pack.id,
                                );
                                return (
                                    <li
                                        key={instance.id}
                                        className="flex items-center justify-between gap-2 py-1"
                                    >
                                        <span className="truncate text-sm">{instance.name}</span>
                                        <Switch
                                            checked={subscribed}
                                            disabled={subscribe.isPending}
                                            aria-label={text.subscribe(instance.name)}
                                            onCheckedChange={(on) =>
                                                subscribe.mutate({
                                                    instanceId: instance.id,
                                                    subscriptions: on
                                                        ? [
                                                              ...instance.syncPacks,
                                                              { packId: pack.id, excluded: [] },
                                                          ]
                                                        : instance.syncPacks.filter(
                                                              (s) => s.packId !== pack.id,
                                                          ),
                                                })
                                            }
                                        />
                                    </li>
                                );
                            })}
                        </ul>
                    )}
                </div>
            </CardContent>

            {adding && <AddProjectDialog pack={pack} onClose={() => setAdding(false)} />}
            <NameDialog key={pack.name} open={renaming} onOpenChange={setRenaming} pack={pack} />
            <AlertDialog open={deleting} onOpenChange={setDeleting}>
                <AlertDialogContent>
                    <AlertDialogHeader>
                        <AlertDialogTitle>{text.removeTitle(pack.name)}</AlertDialogTitle>
                        <AlertDialogDescription>{text.removeDescription}</AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel>{t.common.cancel}</AlertDialogCancel>
                        <AlertDialogAction variant="destructive" onClick={() => remove.mutate()}>
                            {text.removeConfirm}
                        </AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
        </Card>
    );
}

function AddProjectDialog({ pack, onClose }: { pack: SyncPack; onClose: () => void }) {
    const t = useT();
    const text = t.syncpacks;
    const queryClient = useQueryClient();
    const [kind, setKind] = useState<ContentKind>("mod");
    const [query, setQuery] = useState("");
    const debounced = useDebouncedValue(query, 300);
    const search = useInfiniteQuery({
        queryKey: ["sync-search", kind, debounced],
        queryFn: ({ pageParam }) => searchSyncProjects(kind, debounced, pageParam),
        initialPageParam: 0,
        getNextPageParam: (last) => {
            const next = last.offset + last.hits.length;
            return last.hits.length > 0 && next < last.total ? next : undefined;
        },
        staleTime: 60_000,
    });
    const add = useMutation({
        mutationFn: (item: Parameters<typeof addSyncItem>[1]) => addSyncItem(pack.id, item),
        onSuccess: announceSync,
        onError: (error) =>
            toast.error(getT().syncpacks.addFailed, { description: errorMessage(error) }),
        onSettled: () => queryClient.invalidateQueries({ queryKey: syncPacksKey }),
    });
    const inPack = new Set(pack.items.map((item) => item.projectId));
    const hits = search.data?.pages.flatMap((page) => page.hits) ?? [];

    return (
        <Dialog open onOpenChange={(open) => !open && onClose()}>
            <DialogContent className="sm:max-w-xl">
                <DialogHeader>
                    <DialogTitle>{text.addTitle(pack.name)}</DialogTitle>
                    <DialogDescription>{text.addDescription}</DialogDescription>
                </DialogHeader>
                <Tabs value={kind} onValueChange={(value) => setKind(value as ContentKind)}>
                    <TabsList>
                        {KINDS.map((value) => (
                            <TabsTrigger key={value} value={value}>
                                {t.content.kinds[value].tab}
                            </TabsTrigger>
                        ))}
                    </TabsList>
                </Tabs>
                <div className="relative">
                    <SearchIcon
                        className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground"
                        aria-hidden
                    />
                    <Input
                        value={query}
                        onChange={(event) => setQuery(event.target.value)}
                        placeholder={text.search}
                        aria-label={text.search}
                        className="pl-8"
                    />
                </div>
                <ul className="flex h-[22rem] flex-col divide-y overflow-y-auto pr-1">
                    {search.isPending
                        ? [0, 1, 2, 3].map((key) => (
                              <li key={key} className="py-2">
                                  <Skeleton className="h-10 w-full" />
                              </li>
                          ))
                        : hits.map((hit) => {
                              const already = inPack.has(hit.projectId);
                              return (
                                  <li key={hit.projectId} className="flex items-center gap-3 py-2">
                                      <ModIcon url={hit.iconUrl} />
                                      <div className="min-w-0 flex-1">
                                          <p className="truncate text-sm font-medium">
                                              {hit.title}
                                          </p>
                                          <p className="truncate text-xs text-muted-foreground tabular-nums">
                                              {hit.author} ·{" "}
                                              {t.common.downloads(
                                                  compactNumber.format(hit.downloads),
                                              )}
                                          </p>
                                      </div>
                                      {already ? (
                                          <Badge variant="secondary">
                                              <CheckIcon data-icon="inline-start" />
                                              {text.added}
                                          </Badge>
                                      ) : (
                                          <Button
                                              variant="outline"
                                              size="sm"
                                              disabled={add.isPending}
                                              onClick={() =>
                                                  add.mutate({
                                                      projectId: hit.projectId,
                                                      title: hit.title,
                                                      kind,
                                                      iconUrl: hit.iconUrl,
                                                  })
                                              }
                                          >
                                              <PlusIcon data-icon="inline-start" />
                                              {text.add}
                                          </Button>
                                      )}
                                  </li>
                              );
                          })}
                    {search.hasNextPage && (
                        <li className="py-2">
                            <Button
                                variant="ghost"
                                className="w-full"
                                disabled={search.isFetchingNextPage}
                                onClick={() => void search.fetchNextPage()}
                            >
                                {search.isFetchingNextPage ? t.common.loading : t.common.loadMore}
                            </Button>
                        </li>
                    )}
                </ul>
            </DialogContent>
        </Dialog>
    );
}

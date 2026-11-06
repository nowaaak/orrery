import {
    CheckIcon,
    DownloadIcon,
    Loader2Icon,
    PinIcon,
    RefreshCwIcon,
    SearchIcon,
    SparklesIcon,
    Trash2Icon,
    TriangleAlertIcon,
} from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { errorMessage } from "@/api/invoke";
import { installMod, updateMods } from "@/api/mods";
import type { ContentKind } from "@/bindings/ContentKind";
import type { ContentSource } from "@/bindings/ContentSource";
import type { ModUpdate } from "@/bindings/ModUpdate";
import type { Instance } from "@/bindings/Instance";
import type { InstalledMod } from "@/bindings/InstalledMod";
import type { ModSearchHit } from "@/bindings/ModSearchHit";
import { OrreryLogo } from "@/components/brand/OrreryLogo";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
    Card,
    CardAction,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useGameStore } from "@/features/launch/gameStore";
import { isContentTask } from "@/features/tasks/labels";
import { useTaskStore } from "@/features/tasks/taskStore";
import { compactNumber } from "@/lib/format";
import { useDebouncedValue } from "@/lib/useDebouncedValue";
import { getT, useT } from "@/i18n";
import { ModIcon } from "./ModIcon";
import {
    ORRERY_MENU,
    ORRERY_SHADERS,
    OrreryBundledCard,
    installBundleWithToast,
    orreryBundle,
} from "./OrreryBundled";
import { cn } from "@/lib/utils";
import {
    useCurseForgeAvailable,
    useInstalledMods,
    useModSearch,
    useModUpdates,
    usePinMod,
    useRemoveMod,
    useToggleMod,
} from "./useMods";
import { contentMismatch, isCurseForgeContent, isLocalContent } from "./compatibility";
import { InstanceSyncCard } from "@/features/syncpacks/InstanceSyncCard";
import { PendingDownloadsCard } from "@/features/modpacks/PendingDownloadsCard";
import { ContentPacks } from "./ContentPacks";
import { OptifineHint } from "./OptifineHint";

export function ModsTab({ instance }: { instance: Instance }) {
    const t = useT();
    const modded = instance.loader.type !== "vanilla";
    const kinds: ContentKind[] = modded ? ["mod", "resource_pack", "shader"] : ["resource_pack"];
    const [kind, setKind] = useState<ContentKind>(kinds[0]!);
    const [source, setSource] = useState<ContentSource>("modrinth");

    return (
        <div className="flex flex-col gap-4">
            <div className="flex flex-wrap items-center gap-3">
                <Tabs value={kind} onValueChange={(value) => setKind(value as ContentKind)}>
                    <TabsList>
                        {kinds.map((k) => (
                            <TabsTrigger key={k} value={k}>
                                {t.content.kinds[k].tab}
                            </TabsTrigger>
                        ))}
                    </TabsList>
                </Tabs>
                {!modded && (
                    <p className="text-sm text-muted-foreground">{t.content.needsLoader}</p>
                )}
            </div>
            <PendingDownloadsCard instanceId={instance.id} />
            <InstanceSyncCard instance={instance} />
            {kind === "shader" && (
                <OrreryBundledCard bundle={ORRERY_SHADERS} instanceId={instance.id} />
            )}
            {kind === "resource_pack" && (
                <OrreryBundledCard bundle={ORRERY_MENU} instanceId={instance.id} />
            )}
            {kind === "shader" && <IrisHint instanceId={instance.id} />}
            {kind === "resource_pack" && <OptifineHint instance={instance} />}
            {kind === "mod" && <ContentPacks instanceId={instance.id} />}
            <div className="@container">
                <div className="grid items-start gap-6 @3xl:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
                    <ModSearchCard
                        key={`${source}-${kind}`}
                        instance={instance}
                        kind={kind}
                        source={source}
                        onSourceChange={setSource}
                    />
                    <InstalledModsCard instance={instance} kind={kind} />
                </div>
            </div>
        </div>
    );
}

function IrisHint({ instanceId }: { instanceId: string }) {
    const t = useT();
    const mods = useInstalledMods(instanceId);
    const [pending, setPending] = useState(false);
    if (!mods.data) {
        return null;
    }
    const hasIris = mods.data.some(
        (m) => m.kind === "mod" && m.enabled && m.title.toLowerCase().startsWith("iris"),
    );
    if (hasIris) {
        return null;
    }
    const install = async () => {
        setPending(true);
        try {
            await installMod(instanceId, "mod", "iris", "Iris Shaders");
        } catch (error) {
            toast.error(getT().content.installIrisFailed, {
                description: errorMessage(error),
            });
        } finally {
            setPending(false);
        }
    };
    return (
        <div className="flex flex-wrap items-center gap-3 rounded-xl border border-primary/30 bg-primary/5 px-4 py-3 text-sm">
            <SparklesIcon className="size-4 shrink-0 text-primary" aria-hidden />
            <p className="min-w-0 flex-1">
                {t.content.irisHintBefore}
                <span className="font-medium">Iris</span>
                {t.content.irisHintAfter}
            </p>
            <Button size="sm" disabled={pending} onClick={() => void install()}>
                {t.content.installIris}
            </Button>
        </div>
    );
}

function InstalledModsCard({ instance, kind }: { instance: Instance; kind: ContentKind }) {
    const t = useT();
    const instanceId = instance.id;
    const allMods = useInstalledMods(instanceId);
    const mods = { ...allMods, data: allMods.data?.filter((mod) => mod.kind === kind) };
    const running = useGameStore((state) => instanceId in state.running);
    const updates = useModUpdates(instanceId, (mods.data?.length ?? 0) > 0);
    const updating = useTaskStore((state) =>
        Object.values(state.tasks).some(
            (task) =>
                isContentTask(task.kind) &&
                task.instanceId === instanceId &&
                task.status.type === "running",
        ),
    );
    const visibleIds = new Set(mods.data?.map((mod) => mod.projectId));
    const available = (updates.data ?? []).filter((update) => visibleIds.has(update.projectId));
    const byProject = new Map(available.map((update) => [update.projectId, update]));

    const updateAll = () =>
        updateMods(
            instanceId,
            available.map((update) => ({
                projectId: update.projectId,
                versionId: update.versionId,
            })),
        ).catch((error: unknown) =>
            toast.error(getT().content.updatesFailed, { description: errorMessage(error) }),
        );

    return (
        <Card>
            <CardHeader>
                <CardTitle>{t.content.installed}</CardTitle>
                <CardDescription>
                    {running ? t.content.closeGameToChange : t.content.disabledHint}
                </CardDescription>
                {(mods.data?.length ?? 0) > 0 && (
                    <CardAction>
                        <Button
                            size="sm"
                            variant={available.length > 0 ? "default" : "ghost"}
                            className="w-44"
                            disabled={available.length === 0 || running || updating}
                            onClick={() => void updateAll()}
                        >
                            {updates.isFetching ? (
                                <>
                                    <Loader2Icon
                                        data-icon="inline-start"
                                        className="animate-spin motion-reduce:animate-none"
                                    />
                                    {t.content.checkingUpdates}
                                </>
                            ) : available.length > 0 ? (
                                <>
                                    <RefreshCwIcon data-icon="inline-start" />
                                    {t.content.updateCount(available.length)}
                                </>
                            ) : (
                                <>
                                    <CheckIcon data-icon="inline-start" />
                                    {t.content.allUpToDate}
                                </>
                            )}
                        </Button>
                    </CardAction>
                )}
            </CardHeader>
            <CardContent>
                {mods.error ? (
                    <p className="text-sm text-destructive select-text">
                        {errorMessage(mods.error)}
                    </p>
                ) : !mods.data ? (
                    <Skeleton className="h-24 w-full" />
                ) : mods.data.length === 0 ? (
                    <p className="text-sm text-muted-foreground">{t.content.kinds[kind].empty}</p>
                ) : (
                    <ul className="flex flex-col divide-y">
                        {mods.data.map((mod) => (
                            <InstalledModRow
                                key={mod.projectId}
                                instance={instance}
                                mod={mod}
                                update={byProject.get(mod.projectId)}
                                disabled={running}
                            />
                        ))}
                    </ul>
                )}
            </CardContent>
        </Card>
    );
}

function InstalledModRow({
    instance,
    mod,
    update,
    disabled,
}: {
    instance: Instance;
    mod: InstalledMod;
    update: ModUpdate | undefined;
    disabled: boolean;
}) {
    const t = useT();
    const instanceId = instance.id;
    const mismatch = contentMismatch(mod, instance);
    const local = isLocalContent(mod.projectId);
    const toggle = useToggleMod(instanceId);
    const pin = usePinMod(instanceId);
    const remove = useRemoveMod(instanceId);
    const bundled = orreryBundle(mod.projectId);

    const onRemove = () =>
        remove.mutate(mod.projectId, {
            onSuccess: () =>
                toast.success(getT().content.removed(mod.title), {
                    action: local
                        ? undefined
                        : {
                              label: getT().content.undo,
                              onClick: () =>
                                  void (bundled
                                      ? installBundleWithToast(bundled, instanceId)
                                      : installMod(
                                            instanceId,
                                            mod.kind,
                                            mod.projectId,
                                            mod.title,
                                            mod.versionId,
                                        )),
                          },
                }),
            onError: (error) =>
                toast.error(getT().content.removeFailed, {
                    description: errorMessage(error),
                }),
        });

    return (
        <li className="flex items-center gap-3 py-2.5">
            <div
                className={cn(
                    "flex min-w-0 flex-1 items-center gap-3 transition-opacity motion-reduce:transition-none",
                    !mod.enabled && "opacity-50",
                )}
            >
                {bundled ? (
                    <OrreryLogo size={40} className="shrink-0" />
                ) : (
                    <ModIcon url={mod.iconUrl} />
                )}
                <div className="min-w-0 flex-1">
                    <p className="truncate text-sm font-medium">
                        {mod.title}
                        {mod.dependency && (
                            <Badge variant="outline" className="ml-2">
                                {t.content.dependency}
                            </Badge>
                        )}
                        {local && (
                            <Badge variant="outline" className="ml-2">
                                {t.content.localFile}
                            </Badge>
                        )}
                        {isCurseForgeContent(mod.projectId) && (
                            <Badge variant="outline" className="ml-2">
                                {t.content.fromCurseForge}
                            </Badge>
                        )}
                        {mismatch && (
                            <Badge
                                variant="outline"
                                className="ml-2 border-amber-500/50 text-amber-700 dark:text-amber-400"
                                title={t.content.mismatchHint(
                                    (mismatch === "version" ? mod.gameVersions : mod.loaders).join(
                                        ", ",
                                    ),
                                )}
                            >
                                <TriangleAlertIcon data-icon="inline-start" aria-hidden />
                                {t.content.mismatch[mismatch]}
                            </Badge>
                        )}
                    </p>
                    <p className="truncate text-xs text-muted-foreground">
                        {mod.versionNumber}
                        {update && <span className="text-primary"> → {update.versionNumber}</span>}
                    </p>
                </div>
            </div>
            {!local && !bundled && (
                <Button
                    variant="ghost"
                    size="icon"
                    aria-pressed={mod.pinned}
                    aria-label={
                        mod.pinned ? t.content.unpinItem(mod.title) : t.content.pinItem(mod.title)
                    }
                    title={mod.pinned ? t.content.pinnedHint : t.content.pinHint}
                    className={cn(mod.pinned ? "text-primary" : "text-muted-foreground")}
                    onClick={() =>
                        pin.mutate(
                            { projectId: mod.projectId, pinned: !mod.pinned },
                            {
                                onError: (error) =>
                                    toast.error(getT().content.pinFailed, {
                                        description: errorMessage(error),
                                    }),
                            },
                        )
                    }
                >
                    <PinIcon className={cn(mod.pinned && "fill-current")} />
                </Button>
            )}
            <Switch
                checked={mod.enabled}
                disabled={disabled}
                aria-label={
                    mod.enabled ? t.content.disableItem(mod.title) : t.content.enableItem(mod.title)
                }
                onCheckedChange={(enabled) =>
                    toggle.mutate(
                        { projectId: mod.projectId, enabled },
                        {
                            onError: (error) =>
                                toast.error(getT().content.toggleFailed, {
                                    description: errorMessage(error),
                                }),
                        },
                    )
                }
            />
            <Button
                variant="ghost"
                size="icon"
                disabled={disabled || remove.isPending}
                aria-label={t.content.removeItem(mod.title)}
                title={t.content.remove}
                onClick={onRemove}
            >
                <Trash2Icon />
            </Button>
        </li>
    );
}

const SOURCE_NAMES: Record<ContentSource, string> = {
    modrinth: "Modrinth",
    curse_forge: "CurseForge",
};

function ModSearchCard({
    instance,
    kind,
    source,
    onSourceChange,
}: {
    instance: Instance;
    kind: ContentKind;
    source: ContentSource;
    onSourceChange: (source: ContentSource) => void;
}) {
    const t = useT();
    const [query, setQuery] = useState("");
    const debounced = useDebouncedValue(query, 300);
    const curseforge = useCurseForgeAvailable().data === true;
    const active = curseforge ? source : "modrinth";
    const search = useModSearch(instance.id, active, kind, debounced);
    const installed = useInstalledMods(instance.id);
    const installedIds = new Set(installed.data?.map((mod) => mod.projectId));
    const hits = search.data?.pages.flatMap((page) => page.hits) ?? [];

    return (
        <Card>
            <CardHeader>
                <CardTitle>{t.content.searchSource(SOURCE_NAMES[active])}</CardTitle>
                <CardDescription>
                    {t.content.searchScope(t.content.kinds[kind].scope, instance.minecraftVersion)}
                </CardDescription>
                {curseforge && (
                    <CardAction>
                        <Tabs
                            value={active}
                            onValueChange={(value) => onSourceChange(value as ContentSource)}
                        >
                            <TabsList aria-label={t.content.source}>
                                {(["modrinth", "curse_forge"] as const).map((value) => (
                                    <TabsTrigger key={value} value={value}>
                                        {SOURCE_NAMES[value]}
                                    </TabsTrigger>
                                ))}
                            </TabsList>
                        </Tabs>
                    </CardAction>
                )}
            </CardHeader>
            <CardContent className="flex flex-col gap-3">
                <div className="relative">
                    <SearchIcon
                        className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground"
                        aria-hidden
                    />
                    <Input
                        type="search"
                        value={query}
                        placeholder={t.content.kinds[kind].placeholder}
                        aria-label={t.content.searchMods}
                        className="pl-8"
                        onChange={(event) => setQuery(event.target.value)}
                    />
                </div>
                {search.error ? (
                    <p className="text-sm text-destructive select-text">
                        {errorMessage(search.error)}
                    </p>
                ) : search.isPending ? (
                    <div className="flex flex-col gap-2">
                        {[0, 1, 2, 3].map((key) => (
                            <Skeleton key={key} className="h-14 w-full" />
                        ))}
                    </div>
                ) : hits.length === 0 ? (
                    <p className="text-sm text-muted-foreground">{t.content.kinds[kind].none}</p>
                ) : (
                    <ul className="flex flex-col divide-y">
                        {hits.map((hit) => (
                            <SearchHitRow
                                key={hit.projectId}
                                instanceId={instance.id}
                                kind={kind}
                                hit={hit}
                                installed={installedIds.has(hit.projectId)}
                            />
                        ))}
                    </ul>
                )}
                {search.hasNextPage && (
                    <Button
                        variant="outline"
                        disabled={search.isFetchingNextPage}
                        onClick={() => void search.fetchNextPage()}
                    >
                        {search.isFetchingNextPage ? t.common.loading : t.common.loadMore}
                    </Button>
                )}
            </CardContent>
        </Card>
    );
}

function SearchHitRow({
    instanceId,
    kind,
    hit,
    installed,
}: {
    instanceId: string;
    kind: ContentKind;
    hit: ModSearchHit;
    installed: boolean;
}) {
    const t = useT();
    const running = useGameStore((state) => instanceId in state.running);
    const installing = useTaskStore((state) =>
        Object.values(state.tasks).some(
            (task) =>
                isContentTask(task.kind) &&
                task.instanceId === instanceId &&
                task.status.type === "running",
        ),
    );
    const [pending, setPending] = useState(false);

    const onInstall = async () => {
        setPending(true);
        try {
            await installMod(instanceId, kind, hit.projectId, hit.title);
        } catch (error) {
            toast.error(getT().content.installFailed, { description: errorMessage(error) });
        } finally {
            setPending(false);
        }
    };

    return (
        <li className="flex items-center gap-3 py-2.5">
            <ModIcon url={hit.iconUrl} />
            <div className="min-w-0 flex-1">
                <p className="truncate text-sm font-medium">{hit.title}</p>
                <p className="line-clamp-1 text-xs text-muted-foreground">{hit.description}</p>
                <p className="text-xs text-muted-foreground tabular-nums">
                    {hit.author} · {t.common.downloads(compactNumber.format(hit.downloads))}
                </p>
            </div>
            {installed ? (
                <Badge variant="secondary">{t.content.installed}</Badge>
            ) : (
                <Button
                    size="sm"
                    variant="outline"
                    disabled={pending || installing || running}
                    onClick={() => void onInstall()}
                >
                    <DownloadIcon data-icon="inline-start" />
                    {t.common.install}
                </Button>
            )}
        </li>
    );
}

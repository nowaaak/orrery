import { useInfiniteQuery } from "@tanstack/react-query";
import { open } from "@tauri-apps/plugin-dialog";
import { FileArchiveIcon, LayersIcon, SearchIcon } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { errorMessage } from "@/api/invoke";
import { importModpack, installModpack, searchModpacks } from "@/api/modpacks";
import { useUiStore } from "@/app/uiStore";
import type { ContentSource } from "@/bindings/ContentSource";
import type { ModSearchHit } from "@/bindings/ModSearchHit";
import { Button } from "@/components/ui/button";
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ModIcon } from "@/features/mods/ModIcon";
import { useCurseForgeAvailable } from "@/features/mods/useMods";
import { compactNumber } from "@/lib/format";
import { useDebouncedValue } from "@/lib/useDebouncedValue";
import { getT, useT } from "@/i18n";

function useModpackSearch(source: ContentSource, query: string) {
    return useInfiniteQuery({
        queryKey: ["modpack-search", source, query],
        queryFn: ({ pageParam }) => searchModpacks(source, query, pageParam),
        initialPageParam: 0,
        getNextPageParam: (last) => {
            const next = last.offset + last.hits.length;
            return last.hits.length > 0 && next < last.total ? next : undefined;
        },
        staleTime: 5 * 60 * 1000,
    });
}

export function ModpackButton() {
    const t = useT();
    const openDialog = useUiStore((state) => state.openDialog);

    return (
        <Button variant="outline" onClick={() => openDialog("modpack")}>
            <LayersIcon data-icon="inline-start" />
            {t.content.modpacks.button}
        </Button>
    );
}

export function ModpackDialog() {
    const isOpen = useUiStore((state) => state.dialog === "modpack");
    const closeDialog = useUiStore((state) => state.closeDialog);

    return (
        <Dialog open={isOpen} onOpenChange={(next) => !next && closeDialog()}>
            <DialogContent className="sm:max-w-xl">
                {isOpen && <ModpackBrowser onDone={closeDialog} />}
            </DialogContent>
        </Dialog>
    );
}

function ModpackBrowser({ onDone }: { onDone: () => void }) {
    const t = useT();
    const [query, setQuery] = useState("");
    const debounced = useDebouncedValue(query, 300);
    const curseforge = useCurseForgeAvailable().data === true;
    const [source, setSource] = useState<ContentSource>("modrinth");
    const active = curseforge ? source : "modrinth";
    const search = useModpackSearch(active, debounced);
    const hits = search.data?.pages.flatMap((page) => page.hits) ?? [];
    const [busy, setBusy] = useState<string | null>(null);

    const onInstall = async (hit: ModSearchHit) => {
        setBusy(hit.projectId);
        try {
            await installModpack(hit.projectId, hit.title);
            const text = getT().content.modpacks;
            toast.success(text.installing(hit.title), { description: text.progressHint });
            onDone();
        } catch (error) {
            toast.error(getT().content.modpacks.installFailed, {
                description: errorMessage(error),
            });
            setBusy(null);
        }
    };

    const onImport = async () => {
        const path = await open({
            multiple: false,
            directory: false,
            filters: [
                {
                    name: t.content.modpacks.fileFilter,
                    extensions: curseforge ? ["mrpack", "zip"] : ["mrpack"],
                },
            ],
        });
        if (typeof path !== "string") {
            return;
        }
        setBusy("file");
        try {
            await importModpack(path);
            const text = getT().content.modpacks;
            toast.success(text.importing, { description: text.progressHint });
            onDone();
        } catch (error) {
            toast.error(getT().content.modpacks.importFailed, {
                description: errorMessage(error),
            });
            setBusy(null);
        }
    };

    return (
        <div className="flex flex-col gap-4">
            <DialogHeader>
                <DialogTitle>{t.content.modpacks.title}</DialogTitle>
                <DialogDescription>
                    {active === "curse_forge"
                        ? t.content.modpacks.descriptionCurseForge
                        : t.content.modpacks.description}
                </DialogDescription>
            </DialogHeader>
            {curseforge && (
                <Tabs value={active} onValueChange={(value) => setSource(value as ContentSource)}>
                    <TabsList aria-label={t.content.source}>
                        <TabsTrigger value="modrinth">Modrinth</TabsTrigger>
                        <TabsTrigger value="curse_forge">CurseForge</TabsTrigger>
                    </TabsList>
                </Tabs>
            )}
            <div className="relative">
                <SearchIcon
                    className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground"
                    aria-hidden
                />
                <Input
                    type="search"
                    value={query}
                    placeholder={t.content.modpacks.placeholder}
                    aria-label={t.content.modpacks.search}
                    className="pl-8"
                    onChange={(event) => setQuery(event.target.value)}
                />
            </div>
            <div className="h-[22rem] overflow-y-auto pr-1">
                {search.error ? (
                    <p className="text-sm text-destructive select-text">
                        {errorMessage(search.error)}
                    </p>
                ) : search.isPending ? (
                    <div className="flex flex-col gap-2">
                        {[0, 1, 2, 3, 4].map((key) => (
                            <Skeleton key={key} className="h-14 w-full" />
                        ))}
                    </div>
                ) : hits.length === 0 ? (
                    <p className="text-sm text-muted-foreground">{t.content.modpacks.none}</p>
                ) : (
                    <ul className="flex flex-col divide-y">
                        {hits.map((hit) => (
                            <li key={hit.projectId} className="flex items-center gap-3 py-2.5">
                                <ModIcon url={hit.iconUrl} />
                                <div className="min-w-0 flex-1">
                                    <p className="truncate text-sm font-medium">{hit.title}</p>
                                    <p className="line-clamp-1 text-xs text-muted-foreground">
                                        {hit.description}
                                    </p>
                                    <p className="text-xs text-muted-foreground tabular-nums">
                                        {hit.author} ·{" "}
                                        {t.common.downloads(compactNumber.format(hit.downloads))}
                                    </p>
                                </div>
                                <Button
                                    size="sm"
                                    variant="outline"
                                    className="w-28"
                                    disabled={busy !== null}
                                    onClick={() => void onInstall(hit)}
                                >
                                    {busy === hit.projectId
                                        ? t.content.modpacks.starting
                                        : t.common.install}
                                </Button>
                            </li>
                        ))}
                    </ul>
                )}
                {search.hasNextPage && (
                    <Button
                        variant="outline"
                        className="mt-2 w-full"
                        disabled={search.isFetchingNextPage}
                        onClick={() => void search.fetchNextPage()}
                    >
                        {search.isFetchingNextPage ? t.common.loading : t.common.loadMore}
                    </Button>
                )}
            </div>
            <DialogFooter className="sm:justify-between">
                <Button variant="outline" disabled={busy !== null} onClick={() => void onImport()}>
                    <FileArchiveIcon data-icon="inline-start" />
                    {t.content.modpacks.importFromFile}
                </Button>
                <Button variant="ghost" onClick={onDone}>
                    {t.common.close}
                </Button>
            </DialogFooter>
        </div>
    );
}

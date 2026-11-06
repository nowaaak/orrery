import { useInfiniteQuery, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
    checkModUpdates,
    curseforgeAvailable,
    listInstanceMods,
    removeMod,
    searchMods,
    setModEnabled,
    setModPinned,
} from "@/api/mods";
import type { ContentKind } from "@/bindings/ContentKind";
import type { ContentSource } from "@/bindings/ContentSource";
import type { InstalledMod } from "@/bindings/InstalledMod";

export const installedModsKey = (instanceId: string) => ["mods", instanceId] as const;

export function useInstalledMods(instanceId: string) {
    return useQuery({
        queryKey: installedModsKey(instanceId),
        queryFn: () => listInstanceMods(instanceId),
    });
}

export function useCurseForgeAvailable() {
    return useQuery({
        queryKey: ["curseforge-available"],
        queryFn: curseforgeAvailable,
        staleTime: Infinity,
    });
}

export function useModSearch(
    instanceId: string,
    source: ContentSource,
    kind: ContentKind,
    query: string,
) {
    return useInfiniteQuery({
        queryKey: ["mod-search", instanceId, source, kind, query],
        queryFn: ({ pageParam }) => searchMods(instanceId, source, kind, query, pageParam),
        initialPageParam: 0,
        getNextPageParam: (last) => {
            const next = last.offset + last.hits.length;
            return last.hits.length > 0 && next < last.total ? next : undefined;
        },
        staleTime: 5 * 60 * 1000,
    });
}

export function useToggleMod(instanceId: string) {
    const queryClient = useQueryClient();
    const key = installedModsKey(instanceId);
    return useMutation({
        mutationFn: ({ projectId, enabled }: { projectId: string; enabled: boolean }) =>
            setModEnabled(instanceId, projectId, enabled),
        onMutate: async ({ projectId, enabled }) => {
            await queryClient.cancelQueries({ queryKey: key });
            const previous = queryClient.getQueryData<InstalledMod[]>(key);
            queryClient.setQueryData<InstalledMod[]>(key, (mods) =>
                mods?.map((mod) => (mod.projectId === projectId ? { ...mod, enabled } : mod)),
            );
            return { previous };
        },
        onError: (_error, _input, context) => {
            if (context?.previous) {
                queryClient.setQueryData(key, context.previous);
            }
        },
        onSettled: () => queryClient.invalidateQueries({ queryKey: key }),
    });
}

export function usePinMod(instanceId: string) {
    const queryClient = useQueryClient();
    const key = installedModsKey(instanceId);
    return useMutation({
        mutationFn: ({ projectId, pinned }: { projectId: string; pinned: boolean }) =>
            setModPinned(instanceId, projectId, pinned),
        onMutate: async ({ projectId, pinned }) => {
            await queryClient.cancelQueries({ queryKey: key });
            const previous = queryClient.getQueryData<InstalledMod[]>(key);
            queryClient.setQueryData<InstalledMod[]>(key, (mods) =>
                mods?.map((mod) => (mod.projectId === projectId ? { ...mod, pinned } : mod)),
            );
            return { previous };
        },
        onError: (_error, _input, context) => {
            if (context?.previous) {
                queryClient.setQueryData(key, context.previous);
            }
        },
        onSettled: () => queryClient.invalidateQueries({ queryKey: key }),
    });
}

export function useRemoveMod(instanceId: string) {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: (projectId: string) => removeMod(instanceId, projectId),
        onSettled: () =>
            Promise.all(
                [
                    installedModsKey(instanceId),
                    ["content-packs", instanceId],
                    ["optifine-scan", instanceId],
                ].map((queryKey) => queryClient.invalidateQueries({ queryKey })),
            ),
    });
}

export function useModUpdates(instanceId: string, enabled: boolean) {
    return useQuery({
        queryKey: [...installedModsKey(instanceId), "updates"],
        queryFn: () => checkModUpdates(instanceId),
        enabled,
        staleTime: 5 * 60 * 1000,
        retry: false,
    });
}

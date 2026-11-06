import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
    createInstance,
    deleteInstance,
    duplicateInstance,
    getInstance,
    listInstances,
    setInstanceGroup,
    setInstanceIcon,
    setInstancePinned,
    updateInstance,
} from "@/api/instances";
import type { Instance } from "@/bindings/Instance";
import { listLoaderVersions, listMinecraftVersions } from "@/api/versions";
import type { ModLoader } from "@/bindings/ModLoader";
import type { InstanceUpdate } from "@/bindings/InstanceUpdate";

export const instancesKey = ["instances"] as const;

export function useInstances() {
    return useQuery({ queryKey: instancesKey, queryFn: listInstances });
}

export function useInstance(id: string) {
    return useQuery({ queryKey: [...instancesKey, id], queryFn: () => getInstance(id) });
}

export function useMinecraftVersions(includeSnapshots: boolean) {
    return useQuery({
        queryKey: ["versions", includeSnapshots],
        queryFn: () => listMinecraftVersions(includeSnapshots),
        staleTime: 10 * 60 * 1000,
    });
}

export function useLoaderVersions(loader: ModLoader | null, gameVersion: string) {
    return useQuery({
        queryKey: ["loader-versions", loader, gameVersion],
        queryFn: () => listLoaderVersions(loader ?? "fabric", gameVersion),
        enabled: loader !== null && gameVersion !== "",
        staleTime: 10 * 60 * 1000,
        retry: false,
    });
}

export function useCreateInstance() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: createInstance,
        onSuccess: () => queryClient.invalidateQueries({ queryKey: instancesKey }),
    });
}

export function useUpdateInstance(id: string) {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: (update: InstanceUpdate) => updateInstance(id, update),
        onSuccess: (instance) => {
            queryClient.setQueryData([...instancesKey, id], instance);
            return queryClient.invalidateQueries({ queryKey: instancesKey, exact: true });
        },
    });
}

function useInstanceChange<T>(change: (input: T) => Promise<Instance>) {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: change,
        onSuccess: (instance) => {
            queryClient.setQueryData([...instancesKey, instance.id], instance);
            queryClient.setQueryData<Instance[]>(instancesKey, (list) =>
                list?.map((item) => (item.id === instance.id ? instance : item)),
            );
        },
    });
}

export function useSetInstanceGroup() {
    return useInstanceChange(({ id, group }: { id: string; group: string | null }) =>
        setInstanceGroup(id, group),
    );
}

export function useSetInstancePinned() {
    return useInstanceChange(({ id, pinned }: { id: string; pinned: boolean }) =>
        setInstancePinned(id, pinned),
    );
}

export function useSetInstanceIcon() {
    return useInstanceChange(({ id, path }: { id: string; path: string | null }) =>
        setInstanceIcon(id, path),
    );
}

export function useDuplicateInstance() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: ({ id, name }: { id: string; name: string }) => duplicateInstance(id, name),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: instancesKey }),
    });
}

export function useDeleteInstance() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: deleteInstance,
        onSuccess: () => queryClient.invalidateQueries({ queryKey: instancesKey }),
    });
}

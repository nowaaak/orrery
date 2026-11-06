import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { getPaths, getSettings, updateSettings } from "@/api/settings";
import type { Settings } from "@/bindings/Settings";

export const settingsKey = ["settings"] as const;

export function useSettings() {
    return useQuery({ queryKey: settingsKey, queryFn: getSettings, staleTime: Infinity });
}

export function usePaths() {
    return useQuery({ queryKey: ["paths"], queryFn: getPaths, staleTime: Infinity });
}

export function useUpdateSettings() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: updateSettings,
        onMutate: async (next: Settings) => {
            await queryClient.cancelQueries({ queryKey: settingsKey });
            const previous = queryClient.getQueryData<Settings>(settingsKey);
            queryClient.setQueryData(settingsKey, next);
            return { previous };
        },
        onError: (_error, _next, context) => {
            if (context?.previous) {
                queryClient.setQueryData(settingsKey, context.previous);
            }
        },
        onSuccess: (saved) => queryClient.setQueryData(settingsKey, saved),
    });
}

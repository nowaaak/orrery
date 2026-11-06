import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { errorMessage } from "@/api/invoke";
import { listSyncPacks, setInstanceSyncPacks } from "@/api/syncpacks";
import type { SyncReport } from "@/bindings/SyncReport";
import type { SyncSubscription } from "@/bindings/SyncSubscription";
import { instancesKey } from "@/features/instances/useInstances";
import { getT } from "@/i18n";

export const syncPacksKey = ["sync-packs"] as const;

export function useSyncPacks() {
    return useQuery({ queryKey: syncPacksKey, queryFn: listSyncPacks });
}

export function announceSync(reports: SyncReport[]) {
    const text = getT().syncpacks;
    const started = reports.filter((report) => report.taskId !== null).length;
    const waiting = reports.filter((report) => report.waitingForStop).length;
    if (started > 0) {
        toast.info(text.syncing(started));
    }
    if (waiting > 0) {
        toast.info(text.waiting(waiting));
    }
}

export function useSetSubscriptions() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: ({
            instanceId,
            subscriptions,
        }: {
            instanceId: string;
            subscriptions: SyncSubscription[];
        }) => setInstanceSyncPacks(instanceId, subscriptions),
        onSuccess: (report) => announceSync([report]),
        onError: (error) =>
            toast.error(getT().syncpacks.subscribeFailed, { description: errorMessage(error) }),
        onSettled: () => queryClient.invalidateQueries({ queryKey: instancesKey }),
    });
}

import { useCallback } from "react";
import { toast } from "sonner";
import { errorMessage } from "@/api/invoke";
import { launchInstance } from "@/api/launch";
import { requestLogin } from "@/features/accounts/LoginRequiredDialog";
import { useActiveAccount } from "@/features/accounts/useAccounts";
import { getT } from "@/i18n";

export function useLaunchInstance() {
    const account = useActiveAccount();

    return useCallback(
        async (instanceId: string, world: string | null = null, server: string | null = null) => {
            if (!account) {
                requestLogin({ instanceId, world, server });
                return;
            }
            try {
                await launchInstance(instanceId, account.id, world, server);
            } catch (error) {
                toast.error(getT().common.launchFailed, { description: errorMessage(error) });
            }
        },
        [account],
    );
}

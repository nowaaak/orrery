import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { listAccounts, removeAccount, setActiveAccount } from "@/api/accounts";
import type { AccountsView } from "@/bindings/AccountsView";

export const accountsKey = ["accounts"] as const;

export function useAccounts() {
    return useQuery({ queryKey: accountsKey, queryFn: listAccounts });
}

export function useActiveAccount() {
    const { data } = useAccounts();
    return data?.accounts.find((account) => account.id === data.activeId) ?? null;
}

function useAccountMutation<T>(mutationFn: (input: T) => Promise<unknown>) {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn,
        onSuccess: () => queryClient.invalidateQueries({ queryKey: accountsKey }),
    });
}

export const useRemoveAccount = () => useAccountMutation(removeAccount);

export function useSetActiveAccount() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: setActiveAccount,
        onMutate: async (id: string) => {
            await queryClient.cancelQueries({ queryKey: accountsKey });
            const previous = queryClient.getQueryData<AccountsView>(accountsKey);
            if (previous) {
                queryClient.setQueryData<AccountsView>(accountsKey, { ...previous, activeId: id });
            }
            return { previous };
        },
        onError: (_error, _id, context) => {
            if (context?.previous) {
                queryClient.setQueryData(accountsKey, context.previous);
            }
        },
        onSettled: () => queryClient.invalidateQueries({ queryKey: accountsKey }),
    });
}

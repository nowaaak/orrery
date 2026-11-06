import { ChevronsUpDownIcon, CheckIcon, UsersIcon } from "lucide-react";
import { useNavigate } from "react-router";
import { toast } from "sonner";
import { errorMessage } from "@/api/invoke";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuLabel,
    DropdownMenuSeparator,
    DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { getT, useT } from "@/i18n";
import { SkinHead } from "./SkinHead";
import { useAccounts, useSetActiveAccount } from "./useAccounts";

export function AccountSwitcher() {
    const t = useT();
    const navigate = useNavigate();
    const { data } = useAccounts();
    const setActive = useSetActiveAccount();
    const active = data?.accounts.find((account) => account.id === data.activeId) ?? null;

    return (
        <DropdownMenu>
            <DropdownMenuTrigger className="flex h-14 w-full items-center gap-2.5 rounded-xl border border-sidebar-border bg-card px-2.5 text-left text-sm transition-colors outline-none hover:bg-sidebar-accent focus-visible:ring-2 focus-visible:ring-sidebar-ring aria-expanded:bg-sidebar-accent motion-reduce:transition-none">
                {active ? (
                    <SkinHead account={active} size={32} />
                ) : (
                    <UsersIcon className="size-5 text-muted-foreground" aria-hidden />
                )}
                <span className="min-w-0 flex-1">
                    <span className="block truncate font-semibold">
                        {active?.username ?? t.accounts.switcher.noAccount}
                    </span>
                    <span className="block text-xs text-muted-foreground">
                        {active ? t.accounts.kinds[active.kind] : t.accounts.switcher.createAccount}
                    </span>
                </span>
                <ChevronsUpDownIcon className="size-4 text-muted-foreground" aria-hidden />
            </DropdownMenuTrigger>
            <DropdownMenuContent side="top" align="start" className="w-56">
                {data && data.accounts.length > 0 && (
                    <DropdownMenuLabel>{t.accounts.switcher.switchAccount}</DropdownMenuLabel>
                )}
                {data?.accounts.map((account) => (
                    <DropdownMenuItem
                        key={account.id}
                        onSelect={() =>
                            setActive.mutate(account.id, {
                                onError: (error) =>
                                    toast.error(getT().accounts.switchFailed, {
                                        description: errorMessage(error),
                                    }),
                            })
                        }
                    >
                        <SkinHead account={account} size={20} />
                        <span className="flex-1 truncate">{account.username}</span>
                        {account.id === data.activeId && (
                            <CheckIcon aria-label={t.accounts.switcher.activeLabel} />
                        )}
                    </DropdownMenuItem>
                ))}
                {data && data.accounts.length > 0 && <DropdownMenuSeparator />}
                <DropdownMenuItem onSelect={() => void navigate("/accounts")}>
                    <UsersIcon />
                    {t.accounts.switcher.manage}
                </DropdownMenuItem>
            </DropdownMenuContent>
        </DropdownMenu>
    );
}

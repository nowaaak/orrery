import { CheckIcon, ShirtIcon, Trash2Icon, UserIcon } from "lucide-react";
import { useState } from "react";
import { Link } from "react-router";
import { toast } from "sonner";
import { errorMessage } from "@/api/invoke";
import type { Account } from "@/bindings/Account";
import { PageHeader } from "@/app/PageHeader";
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
import { Skeleton } from "@/components/ui/skeleton";
import { useT } from "@/i18n";
import { MicrosoftLoginCard } from "./MicrosoftLoginCard";
import { SkinHead } from "./SkinHead";
import { useAccounts, useRemoveAccount, useSetActiveAccount } from "./useAccounts";

function reportError(title: string) {
    return (error: unknown) => toast.error(title, { description: errorMessage(error) });
}

export function AccountsPage() {
    const t = useT();
    const accounts = useAccounts();

    return (
        <div className="flex max-w-3xl flex-col gap-6">
            <PageHeader title={t.accounts.title} />
            <Card>
                <CardHeader>
                    <CardTitle>{t.accounts.yourAccounts}</CardTitle>
                    <CardDescription>{t.accounts.activeHint}</CardDescription>
                </CardHeader>
                <CardContent>
                    {accounts.error ? (
                        <p className="text-destructive select-text">
                            {errorMessage(accounts.error)}
                        </p>
                    ) : !accounts.data ? (
                        <Skeleton className="h-16 w-full" />
                    ) : accounts.data.accounts.length === 0 ? (
                        <div className="flex items-center gap-3 rounded-lg border border-dashed p-6 text-sm text-muted-foreground">
                            <UserIcon className="size-5" aria-hidden />
                            {t.accounts.empty}
                        </div>
                    ) : (
                        <ul className="flex flex-col divide-y">
                            {accounts.data.accounts.map((account) => (
                                <AccountRow
                                    key={account.id}
                                    account={account}
                                    active={account.id === accounts.data.activeId}
                                />
                            ))}
                        </ul>
                    )}
                </CardContent>
            </Card>
            <MicrosoftLoginCard />
        </div>
    );
}

function AccountRow({ account, active }: { account: Account; active: boolean }) {
    const t = useT();
    const setActive = useSetActiveAccount();
    const remove = useRemoveAccount();
    const [confirmRemove, setConfirmRemove] = useState(false);

    return (
        <li className="flex items-center gap-3 py-3">
            <SkinHead account={account} size={40} />
            <div className="min-w-0 flex-1">
                <p className="truncate font-medium">{account.username}</p>
                <p className="truncate text-xs text-muted-foreground">
                    {t.accounts.kinds[account.kind]}
                </p>
            </div>
            <div className="flex w-28 shrink-0 justify-end">
                {active ? (
                    <Badge>{t.accounts.active}</Badge>
                ) : (
                    <Button
                        variant="outline"
                        size="sm"
                        disabled={setActive.isPending}
                        onClick={() =>
                            setActive.mutate(account.id, {
                                onError: reportError(t.accounts.switchFailed),
                            })
                        }
                    >
                        <CheckIcon data-icon="inline-start" />
                        {t.accounts.activate}
                    </Button>
                )}
            </div>
            <Button
                variant="ghost"
                size="icon"
                aria-label={t.accounts.chooseSkin}
                title={t.accounts.chooseSkin}
                asChild
            >
                <Link to={`/accounts/${account.id}`}>
                    <ShirtIcon />
                </Link>
            </Button>
            <Button
                variant="ghost"
                size="icon"
                aria-label={t.accounts.removeAccount(account.username)}
                title={t.accounts.remove}
                onClick={() => setConfirmRemove(true)}
            >
                <Trash2Icon />
            </Button>

            <AlertDialog open={confirmRemove} onOpenChange={setConfirmRemove}>
                <AlertDialogContent>
                    <AlertDialogHeader>
                        <AlertDialogTitle>
                            {t.accounts.confirmRemoveTitle(account.username)}
                        </AlertDialogTitle>
                        <AlertDialogDescription>
                            {t.accounts.removeMicrosoftDescription}
                        </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel>{t.common.cancel}</AlertDialogCancel>
                        <AlertDialogAction
                            variant="destructive"
                            onClick={() =>
                                remove.mutate(account.id, {
                                    onError: reportError(t.accounts.removeFailed),
                                })
                            }
                        >
                            {t.accounts.remove}
                        </AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
        </li>
    );
}

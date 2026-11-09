import { LogInIcon } from "lucide-react";
import { useCallback } from "react";
import { toast } from "sonner";
import { errorMessage } from "@/api/invoke";
import { launchInstance } from "@/api/launch";
import { useUiStore } from "@/app/uiStore";
import type { Account } from "@/bindings/Account";
import { Button } from "@/components/ui/button";
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from "@/components/ui/dialog";
import { useInstances } from "@/features/instances/useInstances";
import { getT, useT } from "@/i18n";
import { create } from "zustand";
import { LoginPanel, useMicrosoftLogin } from "./MicrosoftLoginCard";

export type PendingLaunch = {
    instanceId: string;
    world: string | null;
    server: string | null;
};

const usePendingLaunch = create<{ pending: PendingLaunch | null }>(() => ({ pending: null }));

export function requestLogin(pending: PendingLaunch) {
    usePendingLaunch.setState({ pending });
    useUiStore.getState().openDialog("login");
}

export function LoginRequiredDialog() {
    const isOpen = useUiStore((state) => state.dialog === "login");
    const closeDialog = useUiStore((state) => state.closeDialog);
    return (
        <Dialog open={isOpen} onOpenChange={(next) => !next && closeDialog()}>
            <DialogContent
                onPointerDownOutside={(event) => event.preventDefault()}
                onInteractOutside={(event) => event.preventDefault()}
            >
                {isOpen && <LoginRequired onDone={closeDialog} />}
            </DialogContent>
        </Dialog>
    );
}

function LoginRequired({ onDone }: { onDone: () => void }) {
    const t = useT();
    const text = t.onboarding;
    const pending = usePendingLaunch((state) => state.pending);
    const instances = useInstances();
    const login = useMicrosoftLogin();
    const name = instances.data?.find((item) => item.id === pending?.instanceId)?.name;

    const resume = useCallback(
        (account: Account) => {
            onDone();
            if (!pending) return;
            launchInstance(pending.instanceId, account.id, pending.world, pending.server).catch(
                (error: unknown) =>
                    toast.error(getT().common.launchFailed, { description: errorMessage(error) }),
            );
        },
        [onDone, pending],
    );

    if (login.pending) {
        return <LoginPanel login={login} onSuccess={resume} />;
    }

    return (
        <>
            <DialogHeader>
                <DialogTitle>{text.requiredTitle}</DialogTitle>
                <DialogDescription>
                    {text.requiredDescription(name ?? text.requiredFallback)}
                </DialogDescription>
            </DialogHeader>
            {login.available === false && (
                <p className="text-sm text-muted-foreground">{text.noClientId}</p>
            )}
            <DialogFooter>
                <Button variant="ghost" onClick={onDone}>
                    {t.common.cancel}
                </Button>
                <Button
                    disabled={login.starting || !login.available}
                    onClick={() => void login.start()}
                >
                    <LogInIcon data-icon="inline-start" />
                    {text.signIn}
                </Button>
            </DialogFooter>
        </>
    );
}

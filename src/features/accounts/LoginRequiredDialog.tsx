import { LogInIcon } from "lucide-react";
import { useCallback, useRef } from "react";
import { toast } from "sonner";
import { cancelMicrosoftLogin } from "@/api/auth";
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
import { DeviceCodePanel, useMicrosoftLogin } from "./MicrosoftLoginCard";

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
            <DialogContent>{isOpen && <LoginRequired onDone={closeDialog} />}</DialogContent>
        </Dialog>
    );
}

function LoginRequired({ onDone }: { onDone: () => void }) {
    const t = useT();
    const text = t.onboarding;
    const pending = usePendingLaunch((state) => state.pending);
    const instances = useInstances();
    const login = useMicrosoftLogin();
    const finished = useRef(false);
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

    if (login.device) {
        const device = login.device;
        return (
            <DeviceCodePanel
                device={device}
                finished={finished}
                onClose={() => {
                    if (!finished.current) {
                        void cancelMicrosoftLogin(device.loginId);
                    }
                    login.close();
                }}
                onDone={login.close}
                onSuccess={resume}
            />
        );
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
                    onClick={() => {
                        finished.current = false;
                        void login.start();
                    }}
                >
                    <LogInIcon data-icon="inline-start" />
                    {text.signIn}
                </Button>
            </DialogFooter>
        </>
    );
}

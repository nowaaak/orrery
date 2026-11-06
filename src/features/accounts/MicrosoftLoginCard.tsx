import { useQuery, useQueryClient } from "@tanstack/react-query";
import { openUrl } from "@tauri-apps/plugin-opener";
import { CopyIcon, ExternalLinkIcon, Loader2Icon, LogInIcon } from "lucide-react";
import { type RefObject, useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import {
    beginMicrosoftLogin,
    cancelMicrosoftLogin,
    microsoftLoginAvailable,
    onLoginEvent,
} from "@/api/auth";
import { friendlyMessage } from "@/api/errors";
import { errorMessage } from "@/api/invoke";
import type { Account } from "@/bindings/Account";
import type { DeviceCodeInfo } from "@/bindings/DeviceCodeInfo";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from "@/components/ui/dialog";
import { getT, useT } from "@/i18n";
import { accountsKey } from "./useAccounts";

export function useMicrosoftLogin() {
    const available = useQuery({
        queryKey: ["microsoft-login-available"],
        queryFn: microsoftLoginAvailable,
        staleTime: Infinity,
    });
    const [device, setDevice] = useState<DeviceCodeInfo | null>(null);
    const [starting, setStarting] = useState(false);
    const close = useCallback(() => setDevice(null), []);

    const start = async () => {
        setStarting(true);
        try {
            setDevice(await beginMicrosoftLogin());
        } catch (error) {
            toast.error(getT().accounts.microsoft.startFailed, {
                description: errorMessage(error),
            });
        } finally {
            setStarting(false);
        }
    };

    return { available: available.data, device, starting, start, close };
}

export function MicrosoftLoginCard() {
    const t = useT();
    const login = useMicrosoftLogin();

    return (
        <Card>
            <CardHeader>
                <CardTitle>{t.accounts.microsoft.title}</CardTitle>
                <CardDescription>{t.accounts.microsoft.description}</CardDescription>
            </CardHeader>
            <CardContent>
                {login.available === false ? (
                    <p className="text-sm text-muted-foreground">
                        {t.accounts.microsoft.noClientId}
                    </p>
                ) : (
                    <Button
                        disabled={login.starting || !login.available}
                        onClick={() => void login.start()}
                    >
                        <LogInIcon data-icon="inline-start" />
                        {t.accounts.microsoft.signIn}
                    </Button>
                )}
            </CardContent>
            {login.device && <DeviceCodeDialog device={login.device} onClose={login.close} />}
        </Card>
    );
}

export function DeviceCodeDialog({
    device,
    onClose,
    onSuccess,
}: {
    device: DeviceCodeInfo;
    onClose: () => void;
    onSuccess?: (account: Account) => void;
}) {
    const finished = useRef(false);
    const close = () => {
        if (!finished.current) {
            void cancelMicrosoftLogin(device.loginId);
        }
        onClose();
    };
    return (
        <Dialog open onOpenChange={(open) => !open && close()}>
            <DialogContent>
                <DeviceCodePanel
                    device={device}
                    finished={finished}
                    onClose={close}
                    onDone={onClose}
                    onSuccess={onSuccess}
                />
            </DialogContent>
        </Dialog>
    );
}

export function DeviceCodePanel({
    device,
    finished,
    onClose,
    onDone,
    onSuccess,
}: {
    device: DeviceCodeInfo;
    finished: RefObject<boolean>;
    onClose: () => void;
    onDone: () => void;
    onSuccess?: (account: Account) => void;
}) {
    const t = useT();
    const queryClient = useQueryClient();
    const successRef = useRef(onSuccess);

    useEffect(() => {
        successRef.current = onSuccess;
    }, [onSuccess]);

    useEffect(() => {
        let unlisten: (() => void) | undefined;
        let disposed = false;
        onLoginEvent((event) => {
            if (event.loginId !== device.loginId) {
                return;
            }
            finished.current = true;
            switch (event.status.type) {
                case "success":
                    toast.success(
                        getT().accounts.microsoft.signedInAs(event.status.account.username),
                    );
                    void queryClient.invalidateQueries({ queryKey: accountsKey });
                    successRef.current?.(event.status.account);
                    break;
                case "failed":
                    toast.error(getT().accounts.microsoft.failed, {
                        description: friendlyMessage(event.status.kind, event.status.message),
                    });
                    break;
                case "cancelled":
                    break;
            }
            onDone();
        }).then((fn) => {
            if (disposed) {
                fn();
            } else {
                unlisten = fn;
            }
        });
        return () => {
            disposed = true;
            unlisten?.();
        };
    }, [device.loginId, finished, onDone, queryClient]);

    const copy = async () => {
        try {
            await navigator.clipboard.writeText(device.userCode);
            toast.success(getT().accounts.microsoft.codeCopied);
        } catch {
            toast.error(getT().accounts.microsoft.copyFailed);
        }
    };

    const openPage = async () => {
        try {
            await openUrl(device.verificationUri);
        } catch (error) {
            const text = getT().accounts.microsoft;
            toast.error(text.openPageFailed, {
                description: text.openPageManually(device.verificationUri, String(error)),
            });
        }
    };

    return (
        <>
            <DialogHeader>
                <DialogTitle>{t.accounts.microsoft.dialogTitle}</DialogTitle>
                <DialogDescription>{t.accounts.microsoft.dialogDescription}</DialogDescription>
            </DialogHeader>
            <div className="flex flex-col items-center gap-3 py-2">
                <output
                    aria-label={t.accounts.microsoft.codeLabel}
                    className="rounded-lg border bg-muted px-5 py-3 font-mono text-2xl font-semibold tracking-widest select-all"
                >
                    {device.userCode}
                </output>
                <div className="flex gap-2">
                    <Button variant="outline" onClick={() => void copy()}>
                        <CopyIcon data-icon="inline-start" />
                        {t.accounts.microsoft.copyCode}
                    </Button>
                    <Button onClick={() => void openPage()}>
                        <ExternalLinkIcon data-icon="inline-start" />
                        {t.accounts.microsoft.openPage}
                    </Button>
                </div>
                <p className="text-center text-xs text-muted-foreground">
                    {t.accounts.microsoft.orOpenInBrowser}{" "}
                    <span className="font-mono text-foreground select-all">
                        {device.verificationUri}
                    </span>
                </p>
                <p className="flex items-center gap-2 text-sm text-muted-foreground" role="status">
                    <Loader2Icon
                        className="size-4 animate-spin motion-reduce:animate-none"
                        aria-hidden
                    />
                    {t.accounts.microsoft.waiting}
                </p>
            </div>
            <DialogFooter>
                <Button variant="ghost" onClick={onClose}>
                    {t.common.cancel}
                </Button>
            </DialogFooter>
        </>
    );
}

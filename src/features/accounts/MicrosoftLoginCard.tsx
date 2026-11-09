import { useQuery, useQueryClient } from "@tanstack/react-query";
import { openUrl } from "@tauri-apps/plugin-opener";
import { CopyIcon, ExternalLinkIcon, KeyRoundIcon, Loader2Icon, LogInIcon } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import {
    beginBrowserLogin,
    beginMicrosoftLogin,
    cancelMicrosoftLogin,
    microsoftLoginAvailable,
    onLoginEvent,
} from "@/api/auth";
import { friendlyMessage } from "@/api/errors";
import { errorMessage } from "@/api/invoke";
import type { Account } from "@/bindings/Account";
import type { BrowserLoginInfo } from "@/bindings/BrowserLoginInfo";
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

export type PendingLogin =
    { kind: "browser"; info: BrowserLoginInfo } | { kind: "device"; info: DeviceCodeInfo };

export type MicrosoftLogin = ReturnType<typeof useMicrosoftLogin>;

async function openInBrowser(url: string) {
    try {
        await openUrl(url);
    } catch (error) {
        toast.error(getT().accounts.microsoft.openPageFailed, {
            description: errorMessage(error),
        });
    }
}

export function useMicrosoftLogin() {
    const available = useQuery({
        queryKey: ["microsoft-login-available"],
        queryFn: microsoftLoginAvailable,
        staleTime: Infinity,
    });
    const [pending, setPending] = useState<PendingLogin | null>(null);
    const [starting, setStarting] = useState(false);
    const busy = useRef(false);
    const close = useCallback(() => setPending(null), []);

    const run = useCallback(async (begin: () => Promise<PendingLogin>) => {
        if (busy.current) {
            return;
        }
        busy.current = true;
        setStarting(true);
        try {
            setPending(await begin());
        } catch (error) {
            toast.error(getT().accounts.microsoft.startFailed, {
                description: errorMessage(error),
            });
        } finally {
            busy.current = false;
            setStarting(false);
        }
    }, []);

    const start = useCallback(
        () =>
            run(async () => {
                const text = getT().accounts.microsoft;
                const info = await beginBrowserLogin({
                    success: text.pageSuccess,
                    failure: text.pageFailure,
                });
                void openInBrowser(info.authorizeUrl);
                return { kind: "browser", info };
            }),
        [run],
    );

    const startDeviceCode = useCallback(
        () => run(async () => ({ kind: "device", info: await beginMicrosoftLogin() })),
        [run],
    );

    return { available: available.data, pending, starting, start, startDeviceCode, close };
}

function useLoginResult(
    loginId: string,
    onDone: () => void,
    onSuccess?: (account: Account) => void,
) {
    const queryClient = useQueryClient();
    const callbacks = useRef({ onDone, onSuccess });
    const mounted = useRef<string | null>(null);

    useEffect(() => {
        callbacks.current = { onDone, onSuccess };
    }, [onDone, onSuccess]);

    useEffect(() => {
        mounted.current = loginId;
        return () => {
            mounted.current = null;
            setTimeout(() => {
                if (mounted.current !== loginId) {
                    void cancelMicrosoftLogin(loginId);
                }
            }, 0);
        };
    }, [loginId]);

    useEffect(() => {
        let unlisten: (() => void) | undefined;
        let disposed = false;
        onLoginEvent((event) => {
            if (event.loginId !== loginId) {
                return;
            }
            switch (event.status.type) {
                case "success":
                    toast.success(
                        getT().accounts.microsoft.signedInAs(event.status.account.username),
                    );
                    void queryClient.invalidateQueries({ queryKey: accountsKey });
                    callbacks.current.onSuccess?.(event.status.account);
                    break;
                case "failed":
                    toast.error(getT().accounts.microsoft.failed, {
                        description: friendlyMessage(event.status.kind, event.status.message),
                    });
                    break;
                case "cancelled":
                    break;
            }
            callbacks.current.onDone();
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
    }, [loginId, queryClient]);
}

function keepOpen(event: Event) {
    event.preventDefault();
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
            <LoginDialog login={login} />
        </Card>
    );
}

export function LoginDialog({
    login,
    onSuccess,
}: {
    login: MicrosoftLogin;
    onSuccess?: (account: Account) => void;
}) {
    if (!login.pending) {
        return null;
    }
    return (
        <Dialog open onOpenChange={(open) => !open && login.close()}>
            <DialogContent onPointerDownOutside={keepOpen} onInteractOutside={keepOpen}>
                <LoginPanel login={login} onSuccess={onSuccess} />
            </DialogContent>
        </Dialog>
    );
}

export function LoginPanel({
    login,
    onSuccess,
}: {
    login: MicrosoftLogin;
    onSuccess?: (account: Account) => void;
}) {
    const pending = login.pending;
    if (!pending) {
        return null;
    }
    if (pending.kind === "browser") {
        return (
            <BrowserLoginPanel
                key={pending.info.loginId}
                info={pending.info}
                switching={login.starting}
                onUseCode={() => void login.startDeviceCode()}
                onClose={login.close}
                onSuccess={onSuccess}
            />
        );
    }
    return (
        <DeviceCodePanel
            key={pending.info.loginId}
            device={pending.info}
            onClose={login.close}
            onSuccess={onSuccess}
        />
    );
}

function WaitingStatus() {
    const t = useT();
    return (
        <p className="flex items-center gap-2 text-sm text-muted-foreground" role="status">
            <Loader2Icon className="size-4 animate-spin motion-reduce:animate-none" aria-hidden />
            {t.accounts.microsoft.waiting}
        </p>
    );
}

function BrowserLoginPanel({
    info,
    switching,
    onUseCode,
    onClose,
    onSuccess,
}: {
    info: BrowserLoginInfo;
    switching: boolean;
    onUseCode: () => void;
    onClose: () => void;
    onSuccess?: (account: Account) => void;
}) {
    const t = useT();
    const text = t.accounts.microsoft;
    useLoginResult(info.loginId, onClose, onSuccess);

    return (
        <>
            <DialogHeader>
                <DialogTitle>{text.browserTitle}</DialogTitle>
                <DialogDescription className="text-pretty">
                    {text.browserDescription}
                </DialogDescription>
            </DialogHeader>
            <div className="flex flex-col items-center gap-3 py-2">
                <WaitingStatus />
                <Button variant="outline" onClick={() => void openInBrowser(info.authorizeUrl)}>
                    <ExternalLinkIcon data-icon="inline-start" />
                    {text.reopenBrowser}
                </Button>
            </div>
            <DialogFooter className="sm:justify-between">
                <Button variant="ghost" disabled={switching} onClick={onUseCode}>
                    <KeyRoundIcon data-icon="inline-start" />
                    {text.useCode}
                </Button>
                <Button variant="ghost" onClick={onClose}>
                    {t.common.cancel}
                </Button>
            </DialogFooter>
        </>
    );
}

function DeviceCodePanel({
    device,
    onClose,
    onSuccess,
}: {
    device: DeviceCodeInfo;
    onClose: () => void;
    onSuccess?: (account: Account) => void;
}) {
    const t = useT();
    useLoginResult(device.loginId, onClose, onSuccess);

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
                <WaitingStatus />
            </div>
            <DialogFooter>
                <Button variant="ghost" onClick={onClose}>
                    {t.common.cancel}
                </Button>
            </DialogFooter>
        </>
    );
}

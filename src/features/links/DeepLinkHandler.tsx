import { useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router";
import { toast } from "sonner";
import { onDeepLink, takeDeepLinks } from "@/api/links";
import type { DeepLink } from "@/bindings/DeepLink";
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
import { useInstances } from "@/features/instances/useInstances";
import { useLaunchInstance } from "@/features/launch/useLaunchInstance";
import { getT, useT } from "@/i18n";

type LaunchLink = Extract<DeepLink["action"], { type: "launch" }>;

export function DeepLinkHandler() {
    const t = useT();
    const navigate = useNavigate();
    const launch = useLaunchInstance();
    const instances = useInstances();
    const [confirm, setConfirm] = useState<LaunchLink | null>(null);
    const actions = useRef({ launch, navigate });

    useEffect(() => {
        actions.current = { launch, navigate };
    }, [launch, navigate]);

    useEffect(() => {
        const process = async () => {
            for (const link of await takeDeepLinks()) {
                const action = link.action;
                if (action.type === "open") {
                    void actions.current.navigate(`/instances/${action.instanceId}`);
                } else if (link.trusted) {
                    void actions.current.launch(action.instanceId, action.world, action.server);
                } else {
                    setConfirm(action);
                }
            }
        };
        let disposed = false;
        let unlisten: (() => void) | null = null;
        void process().catch(() => undefined);
        onDeepLink(() => void process().catch(() => undefined))
            .then((stop) => {
                if (disposed) {
                    stop();
                } else {
                    unlisten = stop;
                }
            })
            .catch(() => undefined);
        return () => {
            disposed = true;
            unlisten?.();
        };
    }, []);

    const instance = confirm
        ? instances.data?.find((item) => item.id === confirm.instanceId)
        : undefined;

    return (
        <AlertDialog open={confirm !== null} onOpenChange={(open) => !open && setConfirm(null)}>
            <AlertDialogContent>
                <AlertDialogHeader>
                    <AlertDialogTitle>{t.links.confirmTitle}</AlertDialogTitle>
                    <AlertDialogDescription>{t.links.confirmDescription}</AlertDialogDescription>
                </AlertDialogHeader>
                {confirm && (
                    <ul className="flex flex-col gap-1 rounded-lg border bg-muted/40 p-3 text-sm">
                        <li>{t.links.confirmInstance(instance?.name ?? confirm.instanceId)}</li>
                        {confirm.world && <li>{t.links.confirmWorld(confirm.world)}</li>}
                        {confirm.server && (
                            <li className="font-mono">{t.links.confirmServer(confirm.server)}</li>
                        )}
                    </ul>
                )}
                <AlertDialogFooter>
                    <AlertDialogCancel>{t.common.cancel}</AlertDialogCancel>
                    <AlertDialogAction
                        onClick={() => {
                            if (!confirm) return;
                            if (!instance) {
                                toast.error(getT().links.unknownInstance);
                                return;
                            }
                            void launch(confirm.instanceId, confirm.world, confirm.server);
                        }}
                    >
                        {t.links.confirmStart}
                    </AlertDialogAction>
                </AlertDialogFooter>
            </AlertDialogContent>
        </AlertDialog>
    );
}

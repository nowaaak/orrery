import { useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import { toast } from "sonner";
import { errorMessage } from "@/api/invoke";
import { listRunning, onBridgeEvent, onGameLog, onGameState } from "@/api/launch";
import { instancesKey } from "@/features/instances/useInstances";
import { screenshotsKey } from "@/features/screenshots/ScreenshotsTab";
import { serversKey } from "@/features/servers/ServersTab";
import { getT } from "@/i18n";
import { crashHint } from "./crashHints";
import { useGameStore } from "./gameStore";

export function useGameSync() {
    const queryClient = useQueryClient();

    useEffect(() => {
        let disposed = false;
        const unlisteners: (() => void)[] = [];
        const store = useGameStore.getState();

        const register = (promise: Promise<() => void>) =>
            promise.then((unlisten) => {
                if (disposed) {
                    unlisten();
                } else {
                    unlisteners.push(unlisten);
                }
            });

        Promise.all([
            register(
                onGameState((event) => {
                    if (event.state.type === "running") {
                        store.setRunning(event.instanceId, event.state.pid);
                        return;
                    }
                    const stopRequested = useGameStore.getState().stopRequested[event.instanceId];
                    store.setStopped(event.instanceId);
                    void queryClient.invalidateQueries({ queryKey: instancesKey });
                    const { code, crash } = event.state;
                    if (crash && !stopRequested) {
                        store.setCrash(event.instanceId, crash);
                        toast.error(getT().launch.crashed, {
                            description: crashHint(crash.hint).title,
                            duration: 10000,
                            action: {
                                label: getT().launch.details,
                                onClick: () => store.showCrash(event.instanceId),
                            },
                        });
                    } else if (!stopRequested && code !== 0 && code !== null) {
                        toast.error(getT().launch.unexpectedExit, {
                            description: getT().launch.exitCode(code),
                        });
                    }
                }),
            ),
            register(onGameLog((event) => store.appendLog(event.instanceId, event.lines))),
            register(
                onBridgeEvent((event) => {
                    if (event.type === "connected") {
                        store.setCompanion(event.instanceId, event.modVersion);
                    } else if (event.type === "screenshot") {
                        void queryClient.invalidateQueries({
                            queryKey: screenshotsKey(event.instanceId),
                        });
                    } else {
                        if (event.type === "disconnected") {
                            store.setCompanion(event.instanceId, null);
                        } else {
                            store.setCompanionStatus(event.instanceId, event.status);
                        }
                        void queryClient.invalidateQueries({
                            queryKey: serversKey(event.instanceId),
                        });
                    }
                }),
            ),
        ])
            .then(listRunning)
            .then((games) => {
                if (!disposed) {
                    store.replaceRunning(games);
                }
            })
            .catch((error: unknown) =>
                toast.error(getT().launch.stateLoadFailed, {
                    description: errorMessage(error),
                }),
            );

        return () => {
            disposed = true;
            unlisteners.forEach((unlisten) => unlisten());
        };
    }, [queryClient]);
}

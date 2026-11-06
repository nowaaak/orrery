import { useEffect } from "react";
import { getGameLog } from "@/api/launch";
import { useT } from "@/i18n";
import { useGameStore } from "./gameStore";
import { LogView } from "./LogView";

const EMPTY: string[] = [];

export function GameLog({ instanceId }: { instanceId: string }) {
    const t = useT();
    const lines = useGameStore((state) => state.logs[instanceId] ?? EMPTY);
    const replaceLog = useGameStore((state) => state.replaceLog);

    useEffect(() => {
        let cancelled = false;
        getGameLog(instanceId)
            .then((tail) => {
                if (!cancelled && tail.length > 0) {
                    replaceLog(instanceId, tail);
                }
            })
            .catch(() => undefined);
        return () => {
            cancelled = true;
        };
    }, [instanceId, replaceLog]);

    if (lines.length === 0) {
        return (
            <div className="rounded-lg border border-dashed p-8 text-center text-sm text-muted-foreground">
                {t.launch.emptyLog}
            </div>
        );
    }

    return <LogView id={`game-log-${instanceId}`} lines={lines} />;
}

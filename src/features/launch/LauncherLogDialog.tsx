import { useQuery } from "@tanstack/react-query";
import { RefreshCwIcon } from "lucide-react";
import { errorMessage } from "@/api/invoke";
import { readLauncherLog } from "@/api/logs";
import { useUiStore } from "@/app/uiStore";
import { Button } from "@/components/ui/button";
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogHeader,
    DialogTitle,
} from "@/components/ui/dialog";
import { Skeleton } from "@/components/ui/skeleton";
import { useT } from "@/i18n";
import { LogView } from "./LogView";

export function LauncherLogDialog() {
    const open = useUiStore((state) => state.dialog === "launcher-log");
    const closeDialog = useUiStore((state) => state.closeDialog);

    return (
        <Dialog open={open} onOpenChange={(next) => !next && closeDialog()}>
            <DialogContent className="sm:max-w-4xl">{open && <LauncherLogContent />}</DialogContent>
        </Dialog>
    );
}

function LauncherLogContent() {
    const t = useT();
    const text = t.launch.launcherLog;
    const log = useQuery({ queryKey: ["launcher-log"], queryFn: readLauncherLog, gcTime: 0 });

    return (
        <>
            <DialogHeader>
                <DialogTitle>{text.title}</DialogTitle>
                <DialogDescription>{text.description}</DialogDescription>
            </DialogHeader>
            {log.data ? (
                log.data.length === 0 ? (
                    <p className="rounded-lg border border-dashed p-8 text-center text-sm text-muted-foreground">
                        {text.empty}
                    </p>
                ) : (
                    <LogView id="launcher-log" lines={log.data} />
                )
            ) : log.error ? (
                <p className="text-sm text-destructive select-text">
                    {text.loadFailed}: {errorMessage(log.error)}
                </p>
            ) : (
                <Skeleton className="h-[28rem] w-full" />
            )}
            <div className="flex justify-end">
                <Button
                    variant="ghost"
                    disabled={log.isFetching}
                    onClick={() => void log.refetch()}
                >
                    <RefreshCwIcon data-icon="inline-start" />
                    {text.refresh}
                </Button>
            </div>
        </>
    );
}

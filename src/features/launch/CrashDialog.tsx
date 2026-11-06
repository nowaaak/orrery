import { save } from "@tauri-apps/plugin-dialog";
import { FileDownIcon, FolderOpenIcon, SettingsIcon, TriangleAlertIcon } from "lucide-react";
import { useNavigate } from "react-router";
import { toast } from "sonner";
import { openInstanceFolder } from "@/api/instances";
import { errorMessage } from "@/api/invoke";
import { exportGameLog } from "@/api/launch";
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
import { useT } from "@/i18n";
import { crashHint, crashSummary } from "./crashHints";
import { useGameStore } from "./gameStore";

export function CrashDialog() {
    const t = useT();
    const navigate = useNavigate();
    const instanceId = useGameStore((state) => state.openCrash);
    const crash = useGameStore((state) => (instanceId ? state.crashes[instanceId] : undefined));
    const showCrash = useGameStore((state) => state.showCrash);
    const instances = useInstances();
    const instance = instances.data?.find((i) => i.id === instanceId);

    if (!instanceId || !crash) {
        return null;
    }
    const hint = crashHint(crash.hint);

    const exportLog = async () => {
        try {
            const path = await save({
                defaultPath: `${instanceId}-log.txt`,
                filters: [{ name: t.launch.textFile, extensions: ["txt", "log"] }],
            });
            if (path) {
                await exportGameLog(instanceId, path);
                toast.success(t.launch.logExported);
            }
        } catch (error) {
            toast.error(t.launch.exportFailed, { description: errorMessage(error) });
        }
    };

    return (
        <Dialog open onOpenChange={(open) => !open && showCrash(null)}>
            <DialogContent className="sm:max-w-lg">
                <DialogHeader>
                    <DialogTitle className="flex items-center gap-2">
                        <TriangleAlertIcon className="size-5 text-destructive" aria-hidden />
                        {instance ? t.launch.instanceCrashed(instance.name) : t.launch.crashed}
                    </DialogTitle>
                    <DialogDescription>{hint.title}</DialogDescription>
                </DialogHeader>
                <p className="text-sm text-pretty">{hint.advice}</p>
                <pre className="max-h-40 overflow-auto rounded-lg border bg-muted/50 p-3 font-mono text-xs whitespace-pre-wrap [overflow-wrap:anywhere]">
                    {crashSummary(crash)}
                </pre>
                <DialogFooter className="flex-wrap gap-2 sm:justify-between">
                    <div className="flex flex-wrap gap-2">
                        {crash.reportFile && (
                            <Button
                                variant="outline"
                                size="sm"
                                onClick={() =>
                                    openInstanceFolder(instanceId, "crash_reports").catch(
                                        (error: unknown) =>
                                            toast.error(t.common.openFolderFailed, {
                                                description: errorMessage(error),
                                            }),
                                    )
                                }
                            >
                                <FolderOpenIcon data-icon="inline-start" />
                                {t.launch.crashReports}
                            </Button>
                        )}
                        <Button variant="outline" size="sm" onClick={() => void exportLog()}>
                            <FileDownIcon data-icon="inline-start" />
                            {t.launch.exportLog}
                        </Button>
                    </div>
                    {hint.opensSettings && (
                        <Button
                            size="sm"
                            onClick={() => {
                                showCrash(null);
                                void navigate(`/instances/${instanceId}?tab=settings`);
                            }}
                        >
                            <SettingsIcon data-icon="inline-start" />
                            {t.launch.toSettings}
                        </Button>
                    )}
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
}

import { useQuery } from "@tanstack/react-query";
import { open } from "@tauri-apps/plugin-dialog";
import { FolderInputIcon, HardDriveIcon, ScrollTextIcon } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";
import { errorMessage } from "@/api/invoke";
import { changeDataDir, inspectDataDir } from "@/api/settings";
import { useUiStore } from "@/app/uiStore";
import { getStorageReport } from "@/api/storage";
import type { DataDirMode } from "@/bindings/DataDirMode";
import type { DataDirTarget } from "@/bindings/DataDirTarget";
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
import { Progress } from "@/components/ui/progress";
import { Skeleton } from "@/components/ui/skeleton";
import { stageLabel } from "@/features/tasks/labels";
import { useTaskStore } from "@/features/tasks/taskStore";
import { getT, useT, type Messages } from "@/i18n";
import { formatBytes, percent } from "@/lib/format";
import { usePaths } from "./useSettings";

const SUBFOLDER = "Orrery";

function joinSubfolder(dir: string): string {
    const separator = dir.includes("\\") ? "\\" : "/";
    return `${dir.replace(/[\\/]+$/, "")}${separator}${SUBFOLDER}`;
}

function moveProblem(target: DataDirTarget, t: Messages): string | null {
    const problems = t.settings.paths.problems;
    if (target.isCurrent) return problems.current;
    if (target.inAppFolder) return problems.appFolder;
    if (target.nested) return problems.nested;
    if (!target.writable) return problems.notWritable;
    if (!target.empty) return problems.notEmpty;
    if (!target.canMove) return problems.noSpace;
    return null;
}

function switchProblem(target: DataDirTarget, t: Messages): string | null {
    const problems = t.settings.paths.problems;
    if (target.isCurrent) return problems.current;
    if (target.inAppFolder) return problems.appFolder;
    if (target.nested) return problems.nested;
    if (!target.writable) return problems.notWritable;
    if (!target.canSwitch) return problems.notUsable;
    return null;
}

export function DataDirCard() {
    const t = useT();
    const paths = usePaths();
    const openDialog = useUiStore((state) => state.openDialog);
    const report = useQuery({ queryKey: ["storage"], queryFn: getStorageReport, staleTime: 0 });
    const [target, setTarget] = useState<DataDirTarget | null>(null);
    const [usedSubfolder, setUsedSubfolder] = useState(false);
    const [busy, setBusy] = useState(false);
    const [taskId, setTaskId] = useState<number | null>(null);
    const clearTask = useCallback(() => setTaskId(null), []);

    const used = report.data?.categories.reduce((sum, category) => sum + category.sizeBytes, 0);
    const isDefault = paths.data ? paths.data.root === paths.data.config : true;

    const inspect = async (dir: string, allowSubfolder: boolean) => {
        setBusy(true);
        try {
            let info = await inspectDataDir(dir);
            let subfolder = false;
            if (
                allowSubfolder &&
                info.exists &&
                !info.empty &&
                !info.isOrrery &&
                !info.nested &&
                !info.isCurrent
            ) {
                info = await inspectDataDir(joinSubfolder(dir));
                subfolder = true;
            }
            setUsedSubfolder(subfolder);
            setTarget(info);
        } catch (error) {
            toast.error(getT().settings.paths.failed, { description: errorMessage(error) });
        } finally {
            setBusy(false);
        }
    };

    const onPick = async () => {
        const dir = await open({ directory: true, multiple: false });
        if (typeof dir === "string") {
            await inspect(dir, true);
        }
    };

    const onStart = async (mode: DataDirMode) => {
        if (!target) return;
        setBusy(true);
        try {
            setTaskId(await changeDataDir(target.path, mode));
            setTarget(null);
        } catch (error) {
            toast.error(getT().settings.paths.failed, { description: errorMessage(error) });
        } finally {
            setBusy(false);
        }
    };

    return (
        <Card className="max-w-3xl">
            <CardHeader>
                <CardTitle>{t.settings.paths.title}</CardTitle>
                <CardDescription>{t.settings.paths.description}</CardDescription>
            </CardHeader>
            <CardContent className="flex flex-col gap-4">
                {paths.data ? (
                    <>
                        <div className="flex flex-wrap items-center gap-3 rounded-xl border bg-muted/30 p-4">
                            <HardDriveIcon
                                className="size-5 shrink-0 text-muted-foreground"
                                aria-hidden
                            />
                            <div className="min-w-0 flex-1">
                                <p className="font-mono text-sm break-all select-all">
                                    {paths.data.root}
                                </p>
                                <p className="text-sm text-muted-foreground tabular-nums">
                                    {isDefault ? `${t.settings.paths.defaultFolder} · ` : ""}
                                    {used === undefined
                                        ? t.common.loading
                                        : t.settings.paths.used(formatBytes(used))}
                                </p>
                            </div>
                            <div className="flex flex-wrap gap-2">
                                {!isDefault && (
                                    <Button
                                        variant="ghost"
                                        disabled={busy}
                                        onClick={() => void inspect(paths.data.config, false)}
                                    >
                                        {t.settings.paths.useDefault}
                                    </Button>
                                )}
                                <Button
                                    variant="outline"
                                    disabled={busy}
                                    onClick={() => void onPick()}
                                >
                                    <FolderInputIcon data-icon="inline-start" />
                                    {t.settings.paths.change}
                                </Button>
                            </div>
                        </div>
                        <dl className="grid grid-cols-[max-content_1fr] gap-x-4 gap-y-2 text-sm">
                            <dt className="text-muted-foreground">{t.settings.paths.cache}</dt>
                            <dd className="font-mono break-all select-all">{paths.data.cache}</dd>
                            <dt className="text-muted-foreground">{t.settings.paths.logs}</dt>
                            <dd className="flex flex-wrap items-center gap-x-3 gap-y-1">
                                <span className="font-mono break-all select-all">
                                    {paths.data.logs}
                                </span>
                                <Button
                                    variant="link"
                                    size="sm"
                                    className="h-auto p-0"
                                    onClick={() => openDialog("launcher-log")}
                                >
                                    <ScrollTextIcon data-icon="inline-start" />
                                    {t.settings.paths.showLog}
                                </Button>
                            </dd>
                        </dl>
                    </>
                ) : (
                    <Skeleton className="h-36 w-full" />
                )}
            </CardContent>
            <ChangeDialog
                target={target}
                usedSubfolder={usedSubfolder}
                busy={busy}
                onCancel={() => setTarget(null)}
                onStart={(mode) => void onStart(mode)}
            />
            <ProgressDialog taskId={taskId} onFailed={clearTask} />
        </Card>
    );
}

function ChangeDialog({
    target,
    usedSubfolder,
    busy,
    onCancel,
    onStart,
}: {
    target: DataDirTarget | null;
    usedSubfolder: boolean;
    busy: boolean;
    onCancel: () => void;
    onStart: (mode: DataDirMode) => void;
}) {
    const t = useT();
    const text = t.settings.paths;
    const moveBlocked = target ? moveProblem(target, t) : null;
    const switchBlocked = target ? switchProblem(target, t) : null;

    return (
        <Dialog open={target !== null} onOpenChange={(next) => !next && onCancel()}>
            <DialogContent className="sm:max-w-lg">
                <DialogHeader>
                    <DialogTitle>{text.dialogTitle}</DialogTitle>
                    <DialogDescription>{text.dialogDescription}</DialogDescription>
                </DialogHeader>
                {target && (
                    <div className="flex flex-col gap-3 text-sm">
                        <div>
                            <p className="text-muted-foreground">{text.target}</p>
                            <p className="font-mono break-all select-text">{target.path}</p>
                        </div>
                        {usedSubfolder && (
                            <p className="text-muted-foreground">{text.subfolder(SUBFOLDER)}</p>
                        )}
                        <ul className="flex flex-col gap-1 text-muted-foreground tabular-nums">
                            <li>{text.needed(formatBytes(target.neededBytes))}</li>
                            {target.freeBytes !== null && (
                                <li>{text.free(formatBytes(target.freeBytes))}</li>
                            )}
                            <li>{target.sameDrive ? text.sameDrive : text.otherDrive}</li>
                            {target.isOrrery && !target.isCurrent && <li>{text.existingData}</li>}
                        </ul>
                        <p className="text-muted-foreground">{text.switchHint}</p>
                        {(moveBlocked || switchBlocked) && (
                            <p className="text-destructive">{moveBlocked ?? switchBlocked}</p>
                        )}
                    </div>
                )}
                <DialogFooter>
                    <Button variant="ghost" onClick={onCancel}>
                        {t.common.cancel}
                    </Button>
                    <Button
                        variant="outline"
                        disabled={busy || switchBlocked !== null}
                        onClick={() => onStart("switch")}
                    >
                        {text.switchOnly}
                    </Button>
                    <Button disabled={busy || moveBlocked !== null} onClick={() => onStart("move")}>
                        {text.move}
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
}

function ProgressDialog({ taskId, onFailed }: { taskId: number | null; onFailed: () => void }) {
    const t = useT();
    const task = useTaskStore((state) => (taskId === null ? undefined : state.tasks[taskId]));
    const status = task?.status.type;
    const ended = taskId !== null && (status === "failed" || status === "cancelled");
    useEffect(() => {
        if (ended) {
            onFailed();
        }
    }, [ended, onFailed]);
    const progress = task?.progress;
    const value = progress ? percent(progress.bytesDone, progress.bytesTotal) : null;

    return (
        <Dialog open={taskId !== null}>
            <DialogContent
                showCloseButton={false}
                onEscapeKeyDown={(event) => event.preventDefault()}
                onPointerDownOutside={(event) => event.preventDefault()}
                onInteractOutside={(event) => event.preventDefault()}
            >
                <DialogHeader>
                    <DialogTitle>
                        {status === "completed"
                            ? t.settings.paths.restarting
                            : t.settings.paths.moving}
                    </DialogTitle>
                    <DialogDescription>{t.settings.paths.movingHint}</DialogDescription>
                </DialogHeader>
                <div className="flex flex-col gap-2 text-sm">
                    <Progress value={status === "completed" ? 100 : (value ?? 0)} />
                    <p className="text-muted-foreground tabular-nums">
                        {progress ? stageLabel(progress.stage) : t.common.loading}
                        {progress && progress.bytesTotal > 0
                            ? ` · ${formatBytes(progress.bytesDone)} / ${formatBytes(progress.bytesTotal)}`
                            : ""}
                    </p>
                </div>
            </DialogContent>
        </Dialog>
    );
}

import { XIcon } from "lucide-react";
import { useShallow } from "zustand/react/shallow";
import { cancelTask, dismissTask } from "@/api/tasks";
import type { TaskView } from "@/bindings/TaskView";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { friendlyMessage } from "@/api/errors";
import { formatBytes, percent } from "@/lib/format";
import { getT, useT } from "@/i18n";
import { stageLabel, taskTitle } from "./labels";
import { selectTaskList, useTaskStore } from "./taskStore";

export function TaskTray() {
    const tasks = useTaskStore(useShallow(selectTaskList));

    if (tasks.length === 0) {
        return null;
    }

    return (
        <section aria-label="Aufgaben" className="flex flex-col gap-2">
            {tasks.slice(0, 4).map((task) => (
                <TaskRow key={task.id} task={task} />
            ))}
        </section>
    );
}

function TaskRow({ task }: { task: TaskView }) {
    const t = useT();
    const remove = useTaskStore((state) => state.remove);
    const { progress, status } = task;
    const running = status.type === "running";
    const value = progress.bytesTotal
        ? percent(progress.bytesDone, progress.bytesTotal)
        : percent(progress.itemsDone, progress.itemsTotal);

    const onClose = () => {
        if (running) {
            void cancelTask(task.id);
        } else {
            remove(task.id);
            void dismissTask(task.id);
        }
    };

    return (
        <div className="rounded-lg border bg-card p-2.5 text-xs">
            <div className="flex items-start justify-between gap-2">
                <div className="min-w-0">
                    <p className="truncate font-medium">{taskTitle(task)}</p>
                    <p className="text-muted-foreground">{statusText(task)}</p>
                </div>
                <Button
                    variant="ghost"
                    size="icon-xs"
                    className={running && task.kind === "move_data" ? "invisible" : undefined}
                    onClick={onClose}
                    aria-label={running ? t.tasks.cancel : t.tasks.dismiss}
                    title={running ? t.tasks.cancel : t.tasks.dismiss}
                >
                    <XIcon />
                </Button>
            </div>
            {running && (
                <div className="mt-2 flex flex-col gap-1">
                    <Progress value={value ?? 0} aria-label={taskTitle(task)} />
                    <p className="text-muted-foreground tabular-nums">
                        {progress.bytesTotal > 0
                            ? `${formatBytes(progress.bytesDone)} / ${formatBytes(progress.bytesTotal)}`
                            : progress.itemsTotal > 0
                              ? `${progress.itemsDone} / ${progress.itemsTotal}`
                              : " "}
                    </p>
                </div>
            )}
        </div>
    );
}

function statusText(task: TaskView): string {
    switch (task.status.type) {
        case "running":
            return stageLabel(task.progress.stage);
        case "completed":
            return getT().tasks.completed;
        case "cancelled":
            return getT().tasks.cancelled;
        case "failed":
            return getT().tasks.failed(friendlyMessage(task.status.kind, task.status.message));
    }
}

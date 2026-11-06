import { useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import { toast } from "sonner";
import { dismissTask, listTasks, onTaskUpdated } from "@/api/tasks";
import { errorMessage } from "@/api/invoke";
import { friendlyMessage } from "@/api/errors";
import { sendGameToast } from "@/api/launch";
import { restartApp } from "@/api/settings";
import type { TaskView } from "@/bindings/TaskView";
import { useGameStore } from "@/features/launch/gameStore";
import { getT } from "@/i18n";
import { instancesKey } from "@/features/instances/useInstances";
import { isContentTask, isModpackTask, taskTitle } from "./labels";
import { useTaskStore } from "./taskStore";

const FINISHED_TASK_VISIBLE_MS = 4000;
const SILENT_IN_GAME: TaskView["kind"][] = ["launch_instance", "move_data"];

function toastInGame(task: TaskView) {
    if (SILENT_IN_GAME.includes(task.kind)) {
        return;
    }
    if (Object.keys(useGameStore.getState().companion).length === 0) {
        return;
    }
    const text = getT().tasks;
    const body =
        task.status.type === "completed"
            ? text.completed
            : task.status.type === "failed"
              ? text.failed(friendlyMessage(task.status.kind, task.status.message))
              : null;
    if (body !== null) {
        sendGameToast(taskTitle(task), body).catch(() => undefined);
    }
}

export function useTaskSync() {
    const queryClient = useQueryClient();

    useEffect(() => {
        let disposed = false;
        let unlisten: (() => void) | undefined;
        const { upsert, replaceAll, remove } = useTaskStore.getState();
        const timers = new Set<number>();
        const scheduleDismiss = (id: number) => {
            const timer = window.setTimeout(() => {
                timers.delete(timer);
                remove(id);
                void dismissTask(id);
            }, FINISHED_TASK_VISIBLE_MS);
            timers.add(timer);
        };

        onTaskUpdated((task) => {
            upsert(task);
            toastInGame(task);
            if (task.status.type === "completed" || task.status.type === "cancelled") {
                scheduleDismiss(task.id);
            }
            if (task.status.type === "failed") {
                toast.error(taskTitle(task), {
                    description: friendlyMessage(task.status.kind, task.status.message),
                });
            }
            if (task.kind === "move_data" && task.status.type === "completed") {
                void restartApp();
            }
            if (isModpackTask(task.kind) && task.status.type !== "running") {
                void queryClient.invalidateQueries({ queryKey: instancesKey });
            }
            if (task.status.type === "completed" && isContentTask(task.kind) && task.instanceId) {
                for (const key of ["mods", "content-packs", "optifine-scan"]) {
                    void queryClient.invalidateQueries({ queryKey: [key, task.instanceId] });
                }
            }
        })
            .then((fn) => {
                if (disposed) {
                    fn();
                } else {
                    unlisten = fn;
                }
            })
            .then(listTasks)
            .then((tasks) => {
                if (!disposed && tasks) {
                    replaceAll(
                        tasks.filter(
                            (task) =>
                                task.status.type === "running" || task.status.type === "failed",
                        ),
                    );
                }
            })
            .catch((error: unknown) => {
                toast.error("Aufgaben konnten nicht geladen werden", {
                    description: errorMessage(error),
                });
            });

        return () => {
            disposed = true;
            unlisten?.();
            timers.forEach((timer) => window.clearTimeout(timer));
        };
    }, [queryClient]);
}

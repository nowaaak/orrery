import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { TaskView } from "@/bindings/TaskView";
import { call } from "./invoke";

export const listTasks = () => call<TaskView[]>("list_tasks");

export const cancelTask = (id: number) => call<void>("cancel_task", { id });

export const dismissTask = (id: number) => call<void>("dismiss_task", { id });

export const onTaskUpdated = (handler: (task: TaskView) => void): Promise<UnlistenFn> =>
    listen<TaskView>("task:updated", (event) => handler(event.payload));

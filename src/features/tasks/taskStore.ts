import { create } from "zustand";
import type { TaskView } from "@/bindings/TaskView";

type TaskState = {
    tasks: Record<number, TaskView>;
    replaceAll: (tasks: TaskView[]) => void;
    upsert: (task: TaskView) => void;
    remove: (id: number) => void;
};

export const useTaskStore = create<TaskState>((set) => ({
    tasks: {},
    replaceAll: (tasks) =>
        set((state) => {
            const next: Record<number, TaskView> = {};
            for (const task of tasks) {
                const known = state.tasks[task.id];
                next[task.id] = known && known.status.type !== "running" ? known : task;
            }
            return { tasks: next };
        }),
    upsert: (task) =>
        set((state) => {
            const known = state.tasks[task.id];
            if (known && known.status.type !== "running" && task.status.type === "running") {
                return state;
            }
            return { tasks: { ...state.tasks, [task.id]: task } };
        }),
    remove: (id) =>
        set((state) => {
            const next = { ...state.tasks };
            delete next[id];
            return { tasks: next };
        }),
}));

export function selectTaskList(state: TaskState): TaskView[] {
    return Object.values(state.tasks).sort((a, b) => b.id - a.id);
}

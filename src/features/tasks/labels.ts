import type { Stage } from "@/bindings/Stage";
import type { TaskKind } from "@/bindings/TaskKind";
import type { TaskView } from "@/bindings/TaskView";
import { getT } from "@/i18n";

export function stageLabel(stage: Stage): string {
    return getT().tasks.stages[stage];
}

export function taskTitle(task: Pick<TaskView, "kind" | "subject">): string {
    return getT().tasks.titles[task.kind](task.subject);
}

const CONTENT_TASKS: TaskKind[] = [
    "install_mod",
    "update_mods",
    "install_pack",
    "install_orrery_shaders",
    "install_orrery_menu",
];
const MODPACK_TASKS: TaskKind[] = ["install_modpack", "import_modpack", "import_instance"];

export function isContentTask(kind: TaskKind): boolean {
    return CONTENT_TASKS.includes(kind);
}

export function isModpackTask(kind: TaskKind): boolean {
    return MODPACK_TASKS.includes(kind);
}

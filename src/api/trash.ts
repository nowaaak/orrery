import type { TrashEntry } from "@/bindings/TrashEntry";
import { call } from "./invoke";

export const listTrash = () => call<TrashEntry[]>("list_trash");

export const restoreTrash = (id: string) => call<TrashEntry>("restore_trash", { id });

export const deleteTrashEntry = (id: string) => call<void>("delete_trash_entry", { id });

export const emptyTrash = () => call<number>("empty_trash");

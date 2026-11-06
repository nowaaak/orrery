import type { TrashEntry } from "@/bindings/TrashEntry";
import type { DataPack } from "@/bindings/DataPack";
import type { WorldBackup } from "@/bindings/WorldBackup";
import type { WorldList } from "@/bindings/WorldList";
import { call } from "./invoke";

export const listWorlds = (instanceId: string) => call<WorldList>("list_worlds", { instanceId });

export const backupWorld = (instanceId: string, folder: string) =>
    call<WorldBackup>("backup_world", { instanceId, folder });

export const restoreWorldBackup = (instanceId: string, folder: string, fileName: string) =>
    call<void>("restore_world_backup", { instanceId, folder, fileName });

export const deleteWorld = (instanceId: string, folder: string) =>
    call<TrashEntry>("delete_world", { instanceId, folder });

export const deleteWorldBackup = (instanceId: string, folder: string, fileName: string) =>
    call<void>("delete_world_backup", { instanceId, folder, fileName });

export const listDatapacks = (instanceId: string, folder: string) =>
    call<DataPack[]>("list_datapacks", { instanceId, folder });

export const addDatapack = (instanceId: string, folder: string, path: string) =>
    call<DataPack>("add_datapack", { instanceId, folder, path });

export const removeDatapack = (instanceId: string, folder: string, name: string) =>
    call<void>("remove_datapack", { instanceId, folder, name });

export const openDatapacksFolder = (instanceId: string, folder: string) =>
    call<void>("open_datapacks_folder", { instanceId, folder });

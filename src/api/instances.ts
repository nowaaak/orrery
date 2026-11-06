import type { TrashEntry } from "@/bindings/TrashEntry";
import type { Instance } from "@/bindings/Instance";
import type { InstanceFolder } from "@/bindings/InstanceFolder";
import type { InstanceUpdate } from "@/bindings/InstanceUpdate";
import type { NewInstance } from "@/bindings/NewInstance";
import { call } from "./invoke";

export const listInstances = () => call<Instance[]>("list_instances");

export const getInstance = (id: string) => call<Instance>("get_instance", { id });

export const createInstance = (instance: NewInstance) =>
    call<Instance>("create_instance", { instance });

export const updateInstance = (id: string, update: InstanceUpdate) =>
    call<Instance>("update_instance", { id, update });

export const duplicateInstance = (id: string, name: string) =>
    call<Instance>("duplicate_instance", { id, name });

export const deleteInstance = (id: string) => call<TrashEntry>("delete_instance", { id });

export const setInstanceGroup = (id: string, group: string | null) =>
    call<Instance>("set_instance_group", { id, group });

export const setInstancePinned = (id: string, pinned: boolean) =>
    call<Instance>("set_instance_pinned", { id, pinned });

export const setInstanceIcon = (id: string, path: string | null) =>
    call<Instance>("set_instance_icon", { id, path });

export const getInstanceIcon = (id: string) => call<ArrayBuffer>("get_instance_icon", { id });

export const openInstanceFolder = (id: string, folder: InstanceFolder | null = null) =>
    call<void>("open_instance_folder", { id, folder });

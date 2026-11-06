import type { ContentKind } from "@/bindings/ContentKind";
import type { ModSearchPage } from "@/bindings/ModSearchPage";
import type { SyncItem } from "@/bindings/SyncItem";
import type { SyncPack } from "@/bindings/SyncPack";
import type { SyncReport } from "@/bindings/SyncReport";
import type { SyncSubscription } from "@/bindings/SyncSubscription";
import { call } from "./invoke";

export const listSyncPacks = () => call<SyncPack[]>("list_sync_packs");

export const createSyncPack = (name: string) => call<SyncPack>("create_sync_pack", { name });

export const renameSyncPack = (id: string, name: string) =>
    call<SyncPack>("rename_sync_pack", { id, name });

export const deleteSyncPack = (id: string) => call<SyncReport[]>("delete_sync_pack", { id });

export const addSyncItem = (id: string, item: SyncItem) =>
    call<SyncReport[]>("add_sync_item", { id, item });

export const removeSyncItem = (id: string, projectId: string) =>
    call<SyncReport[]>("remove_sync_item", { id, projectId });

export const setInstanceSyncPacks = (instanceId: string, subscriptions: SyncSubscription[]) =>
    call<SyncReport>("set_instance_sync_packs", { instanceId, subscriptions });

export const searchSyncProjects = (kind: ContentKind, query: string, offset: number) =>
    call<ModSearchPage>("search_sync_projects", { kind, query, offset });

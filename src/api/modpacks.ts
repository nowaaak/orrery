import type { ContentSource } from "@/bindings/ContentSource";
import type { ModpackExport } from "@/bindings/ModpackExport";
import type { ModSearchPage } from "@/bindings/ModSearchPage";
import type { PendingDownload } from "@/bindings/PendingDownload";
import { call } from "./invoke";

export const searchModpacks = (source: ContentSource, query: string, offset: number) =>
    call<ModSearchPage>("search_modpacks", { source, query, offset });

export const isModpack = (path: string) => call<boolean>("is_modpack", { path });

export const checkPendingDownloads = (instanceId: string) =>
    call<PendingDownload[]>("check_pending_downloads", { instanceId });

export const providePendingDownload = (instanceId: string, fileId: number, path: string) =>
    call<PendingDownload[]>("provide_pending_download", { instanceId, fileId, path });

export const skipPendingDownload = (instanceId: string, fileId: number) =>
    call<PendingDownload[]>("skip_pending_download", { instanceId, fileId });

export const installModpack = (projectId: string, title: string) =>
    call<number>("install_modpack", { projectId, title });

export const importModpack = (path: string) => call<number>("import_modpack", { path });

export const exportModpack = (instanceId: string, path: string) =>
    call<ModpackExport>("export_modpack", { instanceId, path });

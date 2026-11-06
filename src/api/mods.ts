import type { ContentKind } from "@/bindings/ContentKind";
import type { ContentSource } from "@/bindings/ContentSource";
import type { InstalledMod } from "@/bindings/InstalledMod";
import type { ModSearchPage } from "@/bindings/ModSearchPage";
import type { ModUpdate } from "@/bindings/ModUpdate";
import type { ModUpdateRequest } from "@/bindings/ModUpdateRequest";
import type { ModVersion } from "@/bindings/ModVersion";
import type { ContentPack } from "@/bindings/ContentPack";
import type { OptifineReport } from "@/bindings/OptifineReport";
import type { PackInstallResult } from "@/bindings/PackInstallResult";
import { call } from "./invoke";

export const searchMods = (
    instanceId: string,
    source: ContentSource,
    kind: ContentKind,
    query: string,
    offset: number,
) => call<ModSearchPage>("search_mods", { instanceId, source, kind, query, offset });

export const curseforgeAvailable = () => call<boolean>("curseforge_available");

export const getModVersions = (instanceId: string, kind: ContentKind, projectId: string) =>
    call<ModVersion[]>("get_mod_versions", { instanceId, kind, projectId });

export const installMod = (
    instanceId: string,
    kind: ContentKind,
    projectId: string,
    title: string,
    versionId: string | null = null,
) => call<number>("install_mod", { instanceId, kind, projectId, versionId, title });

export const listInstanceMods = (instanceId: string) =>
    call<InstalledMod[]>("list_instance_mods", { instanceId });

export const removeMod = (instanceId: string, projectId: string) =>
    call<void>("remove_mod", { instanceId, projectId });

export const setModEnabled = (instanceId: string, projectId: string, enabled: boolean) =>
    call<InstalledMod>("set_mod_enabled", { instanceId, projectId, enabled });

export const setModPinned = (instanceId: string, projectId: string, pinned: boolean) =>
    call<InstalledMod>("set_mod_pinned", { instanceId, projectId, pinned });

export const checkModUpdates = (instanceId: string) =>
    call<ModUpdate[]>("check_mod_updates", { instanceId });

export const updateMods = (instanceId: string, updates: ModUpdateRequest[]) =>
    call<number>("update_mods", { instanceId, updates });

export const listContentPacks = (instanceId: string) =>
    call<ContentPack[]>("list_content_packs", { instanceId });

export const installContentPack = (
    instanceId: string,
    packId: string,
    only: string[] | null = null,
) => call<PackInstallResult>("install_content_pack", { instanceId, packId, only });

export const removeContentPack = (instanceId: string, packId: string) =>
    call<number>("remove_content_pack", { instanceId, packId });

export const scanOptifinePacks = (instanceId: string) =>
    call<OptifineReport[]>("scan_optifine_packs", { instanceId });

export const installOrreryShaders = (instanceId: string) =>
    call<number>("install_orrery_shaders", { instanceId });

export const installOrreryMenu = (instanceId: string) =>
    call<number>("install_orrery_menu", { instanceId });

export const installLocalFile = (instanceId: string, path: string) =>
    call<InstalledMod>("install_local_file", { instanceId, path });

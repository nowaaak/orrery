import type { LoaderVersion } from "@/bindings/LoaderVersion";
import type { ModLoader } from "@/bindings/ModLoader";
import type { VersionList } from "@/bindings/VersionList";
import { call } from "./invoke";

export const listMinecraftVersions = (includeSnapshots: boolean, refresh = false) =>
    call<VersionList>("list_minecraft_versions", { includeSnapshots, refresh });

export const listLoaderVersions = (loader: ModLoader, gameVersion: string) =>
    call<LoaderVersion[]>("list_loader_versions", { loader, gameVersion });

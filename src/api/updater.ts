import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AppInfo } from "@/bindings/AppInfo";
import type { UpdateInfo } from "@/bindings/UpdateInfo";
import type { UpdateProgress } from "@/bindings/UpdateProgress";
import { call } from "./invoke";

export const getAppInfo = () => call<AppInfo>("get_app_info");

export const checkForUpdate = () => call<UpdateInfo | null>("check_for_update");

export const installUpdate = () => call<void>("install_update");

export const onUpdateProgress = (
    handler: (progress: UpdateProgress) => void,
): Promise<UnlistenFn> =>
    listen<UpdateProgress>("update:progress", (event) => handler(event.payload));

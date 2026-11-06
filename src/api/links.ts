import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { DeepLink } from "@/bindings/DeepLink";
import { call } from "./invoke";

export const takeDeepLinks = () => call<DeepLink[]>("take_deep_links");

export const createShortcut = (
    instanceId: string,
    world: string | null = null,
    server: string | null = null,
) => call<string>("create_shortcut", { instanceId, world, server });

export const onDeepLink = (handler: () => void): Promise<UnlistenFn> =>
    listen("deep-link", () => handler());

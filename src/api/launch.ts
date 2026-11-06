import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { BridgeEvent } from "@/bindings/BridgeEvent";
import type { GameLogEvent } from "@/bindings/GameLogEvent";
import type { GameStateEvent } from "@/bindings/GameStateEvent";
import type { JavaInstallation } from "@/bindings/JavaInstallation";
import type { RunningGame } from "@/bindings/RunningGame";
import { call } from "./invoke";

export const installInstance = (id: string) => call<number>("install_instance", { id });

export const repairInstance = (id: string) => call<number>("repair_instance", { id });

export const launchInstance = (
    id: string,
    accountId: string | null,
    world: string | null = null,
    server: string | null = null,
) => call<number>("launch_instance", { id, accountId, world, server });

export const listJavaInstallations = () => call<JavaInstallation[]>("list_java_installations");

export const killInstance = (id: string) => call<void>("kill_instance", { id });

export const listRunning = () => call<RunningGame[]>("list_running");

export const sendGameToast = (title: string, body: string) =>
    call<number>("send_game_toast", { title, body });

export const getGameLog = (id: string) => call<string[]>("get_game_log", { id });

export const exportGameLog = (id: string, path: string) =>
    call<void>("export_game_log", { id, path });

export const onGameState = (handler: (event: GameStateEvent) => void): Promise<UnlistenFn> =>
    listen<GameStateEvent>("game:state", (event) => handler(event.payload));

export const onGameLog = (handler: (event: GameLogEvent) => void): Promise<UnlistenFn> =>
    listen<GameLogEvent>("game:log", (event) => handler(event.payload));

export const onBridgeEvent = (handler: (event: BridgeEvent) => void): Promise<UnlistenFn> =>
    listen<BridgeEvent>("bridge:event", (event) => handler(event.payload));

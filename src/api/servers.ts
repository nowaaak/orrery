import type { SavedServer } from "@/bindings/SavedServer";
import type { ServerChange } from "@/bindings/ServerChange";
import type { ServerStatus } from "@/bindings/ServerStatus";
import { call } from "./invoke";

export const listServers = (id: string) => call<SavedServer[]>("list_servers", { id });

export const changeServers = (id: string, change: ServerChange) =>
    call<SavedServer[]>("change_servers", { id, change });

export const pingServer = (address: string) => call<ServerStatus>("ping_server", { address });

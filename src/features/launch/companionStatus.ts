import type { GameStatus } from "@/bindings/GameStatus";
import { getT } from "@/i18n";

const DEFAULT_PORT = "25565";

export function serverKey(address: string): string {
    const trimmed = address.trim().toLowerCase();
    const bracketed = /^\[([^\]]+)\](?::(\d+))?$/.exec(trimmed);
    if (bracketed) {
        const host = bracketed[1] ?? "";
        const port = bracketed[2] ?? DEFAULT_PORT;
        return port === DEFAULT_PORT ? host : `[${host}]:${port}`;
    }
    const [host = "", port = DEFAULT_PORT] = trimmed.split(":");
    const bare = host.replace(/\.+$/, "");
    return port === DEFAULT_PORT ? bare : `${bare}:${port}`;
}

export function isOnServer(status: GameStatus | null | undefined, address: string): boolean {
    return !!status?.server && serverKey(status.server) === serverKey(address);
}

export function dimensionLabel(id: string): string {
    const names = getT().instances.launch.dimensions;
    const path = id.includes(":") ? id.slice(id.indexOf(":") + 1) : id;
    if (path in names) {
        return names[path as keyof typeof names];
    }
    const last = path.split("/").pop() ?? path;
    return last
        .split("_")
        .filter(Boolean)
        .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
        .join(" ");
}

export function companionLocation(status: GameStatus | null | undefined): string | null {
    const where = status?.server ?? status?.world;
    if (!where) {
        return null;
    }
    return status?.dimension ? `${where} · ${dimensionLabel(status.dimension)}` : where;
}

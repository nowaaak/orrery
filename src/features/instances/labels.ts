import type { Instance } from "@/bindings/Instance";
import { getT } from "@/i18n";

export function loaderLabel(instance: Pick<Instance, "loader">): string {
    switch (instance.loader.type) {
        case "vanilla":
            return "Vanilla";
        case "fabric":
            return `Fabric ${instance.loader.loaderVersion}`;
        case "legacy_fabric":
            return `Legacy Fabric ${instance.loader.loaderVersion}`;
        case "quilt":
            return `Quilt ${instance.loader.loaderVersion}`;
        case "forge":
            return `Forge ${instance.loader.loaderVersion}`;
        case "neo_forge":
            return `NeoForge ${instance.loader.loaderVersion}`;
    }
}

export function relativeTime(timestamp: number): string {
    const format = new Intl.RelativeTimeFormat(getT().common.locale, { numeric: "auto" });
    const minutes = (timestamp - Date.now() / 1000) / 60;
    if (Math.abs(minutes) < 60) {
        return format.format(Math.round(minutes), "minute");
    }
    const hours = minutes / 60;
    if (Math.abs(hours) < 24) {
        return format.format(Math.round(hours), "hour");
    }
    return format.format(Math.round(hours / 24), "day");
}

export function lastPlayedShort(timestamp: number | null): string {
    return timestamp === null ? getT().instances.neverPlayed : relativeTime(timestamp);
}

export function lastPlayedLabel(timestamp: number | null): string {
    const t = getT().instances;
    return timestamp === null ? t.neverPlayed : t.playedAgo(relativeTime(timestamp));
}

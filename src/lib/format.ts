import { getT } from "@/i18n";

export const compactNumber = {
    format: (value: number) =>
        new Intl.NumberFormat(getT().common.locale, { notation: "compact" }).format(value),
};

export function formatBytes(bytes: number): string {
    if (!Number.isFinite(bytes) || bytes <= 0) {
        return "0 B";
    }
    const { bytes: units, locale } = getT().common;
    const exponent = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
    const value = bytes / 1024 ** exponent;
    const digits = exponent === 0 || value >= 100 ? 0 : 1;
    return `${value.toLocaleString(locale, { maximumFractionDigits: digits, minimumFractionDigits: digits })} ${units[exponent]}`;
}

export function percent(done: number, total: number): number | null {
    if (total <= 0) {
        return null;
    }
    return Math.min(100, Math.round((done / total) * 100));
}

export function formatPlaytime(seconds: number): string {
    const t = getT().common;
    if (seconds < 60) {
        return seconds > 0 ? t.playtimeUnderMinute : t.playtimeNone;
    }
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);
    const rest = minutes % 60;
    if (hours === 0) {
        return t.playtimeMinutes(minutes);
    }
    return rest === 0 ? t.playtimeHours(hours) : t.playtimeHoursMinutes(hours, rest);
}

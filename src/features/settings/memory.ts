import { getT } from "@/i18n";

const SYSTEM_RESERVE_MB = 3072;
const MAX_OPTION_MB = 65536;

export function memoryOptions(systemMb: number | null, floorMb = 1024): number[] {
    const limit = Math.min(Math.max(4096, systemMb ?? 16384), MAX_OPTION_MB);
    const options: number[] = [];
    for (let mb = floorMb; mb <= Math.min(4096, limit); mb += 512) {
        options.push(mb);
    }
    for (let mb = 5120; mb <= limit; mb += 1024) {
        options.push(mb);
    }
    return options;
}

export function formatGb(mb: number): string {
    const gb = mb / 1024;
    return `${gb.toLocaleString(getT().common.locale, { maximumFractionDigits: 2 })} GB`;
}

export function isTight(mb: number, systemMb: number | null): boolean {
    return systemMb !== null && systemMb - mb < SYSTEM_RESERVE_MB;
}

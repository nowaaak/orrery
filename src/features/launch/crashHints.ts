import type { CrashHint } from "@/bindings/CrashHint";
import type { CrashReport } from "@/bindings/CrashReport";
import { getT } from "@/i18n";

type HintText = {
    title: string;
    advice: string;
    opensSettings: boolean;
};

const OPENS_SETTINGS: ReadonlySet<CrashHint> = new Set<CrashHint>([
    "out_of_memory",
    "heap_too_large",
    "java_version",
]);

export function crashHint(hint: CrashHint): HintText {
    return { ...getT().launch.hints[hint], opensSettings: OPENS_SETTINGS.has(hint) };
}

export function crashSummary(crash: CrashReport): string {
    if (crash.summary !== null) {
        return crash.summary;
    }
    const fallback = getT().launch.summaryFallback;
    if (crash.reportFile?.toLowerCase().endsWith(".txt")) {
        return fallback.crashReport;
    }
    return crash.hint === "native_crash" ? fallback.nativeCrash : fallback.exited;
}

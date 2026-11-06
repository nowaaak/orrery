import { useQuery } from "@tanstack/react-query";
import { CheckIcon, SparklesIcon } from "lucide-react";
import { recommendSettings } from "@/api/tuning";
import type { SettingsRecommendation } from "@/bindings/SettingsRecommendation";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { getT, useT } from "@/i18n";
import type { Draft } from "./InstanceSettingsForm";

function gb(mb: number): string {
    const value = mb / 1024;
    return `${value.toLocaleString(getT().common.locale, { maximumFractionDigits: 1 })} GB`;
}

function reasons(r: SettingsRecommendation): string {
    const t = getT().instances.recommendation;
    const parts = [t.systemMemory(gb(r.systemMemoryMb))];
    if (r.modCount > 0) {
        parts.push(t.activeMods(r.modCount));
    }
    if (r.shaders) {
        parts.push(t.shaders);
    }
    return parts.join(" · ");
}

export function RecommendationBox({
    instanceId,
    current,
    onApply,
}: {
    instanceId: string;
    current: Draft;
    onApply: (recommendation: SettingsRecommendation) => void;
}) {
    const t = useT().instances.recommendation;
    const recommendation = useQuery({
        queryKey: ["recommendation", instanceId],
        queryFn: () => recommendSettings(instanceId),
    });

    if (recommendation.isPending) {
        return <Skeleton className="h-[7.5rem] w-full rounded-xl" />;
    }
    if (!recommendation.data) {
        return null;
    }
    const r = recommendation.data;
    const args = current.jvmArgs.split(/\s+/);
    const applied =
        current.customMemory &&
        current.maxMemoryMb === r.maxMemoryMb &&
        current.minMemoryMb === r.minMemoryMb &&
        r.jvmArgs.every((arg) => args.includes(arg));

    return (
        <div className="flex min-h-[7.5rem] flex-col gap-3 rounded-xl border border-primary/30 bg-primary/5 p-4 sm:flex-row sm:items-center">
            <SparklesIcon className="size-5 shrink-0 text-primary" aria-hidden />
            <div className="flex min-w-0 flex-1 flex-col gap-1 text-sm">
                <p className="font-medium">
                    {t.label} <span className="tabular-nums">{gb(r.maxMemoryMb)}</span> {t.ram}
                </p>
                <p className="text-muted-foreground">{reasons(r)}</p>
                {r.limit === "system_memory" && (
                    <p className="text-muted-foreground">{t.limitedBySystem(gb(r.neededMb))}</p>
                )}
                {r.limit === "useful_maximum" && (
                    <p className="text-muted-foreground">{t.usefulMaximum(gb(r.maxMemoryMb))}</p>
                )}
                <p
                    className="truncate font-mono text-xs text-muted-foreground select-text"
                    title={r.jvmArgs.join(" ")}
                >
                    {r.jvmArgs.join(" ")}
                </p>
            </div>
            <Button
                type="button"
                variant={applied ? "ghost" : "default"}
                disabled={applied}
                className="w-32 shrink-0"
                onClick={() => onApply(r)}
            >
                {applied ? (
                    <>
                        <CheckIcon data-icon="inline-start" />
                        {t.applied}
                    </>
                ) : (
                    t.apply
                )}
            </Button>
        </div>
    );
}

import { useMutation, useQuery } from "@tanstack/react-query";
import { SparklesIcon } from "lucide-react";
import { toast } from "sonner";
import { errorMessage } from "@/api/invoke";
import { installContentPack, scanOptifinePacks } from "@/api/mods";
import type { Instance } from "@/bindings/Instance";
import type { OptifineReport } from "@/bindings/OptifineReport";
import { Button } from "@/components/ui/button";
import { useGameStore } from "@/features/launch/gameStore";
import { getT, useT } from "@/i18n";
import { notifyPackResult } from "./ContentPacks";

export function OptifineHint({ instance }: { instance: Instance }) {
    const t = useT();
    const modded = instance.loader.type !== "vanilla";
    const reports = useQuery({
        queryKey: ["optifine-scan", instance.id],
        queryFn: () => scanOptifinePacks(instance.id),
    });
    const relevant = (reports.data ?? []).filter((report) => report.missing.length > 0 || !modded);
    if (relevant.length === 0) {
        return null;
    }
    return (
        <div className="flex flex-col gap-2">
            {relevant.map((report) => (
                <ReportRow key={report.fileName} instance={instance} report={report} />
            ))}
            {!modded && (
                <p className="text-sm text-muted-foreground">{t.content.optifine.needsLoader}</p>
            )}
        </div>
    );
}

function ReportRow({ instance, report }: { instance: Instance; report: OptifineReport }) {
    const t = useT();
    const text = t.content.optifine;
    const running = useGameStore((state) => instance.id in state.running);
    const modded = instance.loader.type !== "vanilla";
    const install = useMutation({
        mutationFn: () =>
            installContentPack(
                instance.id,
                "optifine",
                report.missing.map((mod) => mod.slug),
            ),
        onSuccess: (result) => notifyPackResult("optifine", result),
        onError: (error) =>
            toast.error(getT().content.packs.failed, { description: errorMessage(error) }),
    });

    return (
        <div className="flex flex-wrap items-center gap-3 rounded-xl border border-primary/30 bg-primary/5 px-4 py-3 text-sm">
            <SparklesIcon className="size-5 shrink-0 text-primary" aria-hidden />
            <div className="min-w-0 flex-1">
                <p className="font-medium">{text.title(report.fileName)}</p>
                <p className="text-muted-foreground">
                    {text.features(
                        report.features.map((feature) => text.names[feature]).join(", "),
                    )}
                    {modded &&
                        report.missing.length > 0 &&
                        ` ${text.needs(report.missing.map((mod) => mod.name).join(", "))}`}
                </p>
            </div>
            {modded && (
                <Button
                    size="sm"
                    disabled={running || install.isPending}
                    onClick={() => install.mutate()}
                >
                    {t.common.install}
                </Button>
            )}
        </div>
    );
}

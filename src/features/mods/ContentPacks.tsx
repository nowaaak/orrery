import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
    GaugeIcon,
    PackageIcon,
    SparklesIcon,
    SunIcon,
    Trash2Icon,
    TurtleIcon,
    type LucideIcon,
} from "lucide-react";
import { toast } from "sonner";
import { errorMessage } from "@/api/invoke";
import { installContentPack, listContentPacks, removeContentPack } from "@/api/mods";
import type { ContentPack } from "@/bindings/ContentPack";
import type { PackInstallResult } from "@/bindings/PackInstallResult";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { useGameStore } from "@/features/launch/gameStore";
import { getT, useT } from "@/i18n";
import { installedModsKey } from "./useMods";

const ICONS: Record<string, LucideIcon> = {
    performance: GaugeIcon,
    optifine: SparklesIcon,
    shader_ready: SunIcon,
    low_end: TurtleIcon,
    quality_of_life: PackageIcon,
};

export const contentPacksKey = (instanceId: string) => ["content-packs", instanceId] as const;

export function packName(id: string): string {
    const names: Record<string, { name: string }> = getT().content.packs.items;
    return names[id]?.name ?? id;
}

export function notifyPackResult(packId: string, result: PackInstallResult) {
    const text = getT().content.packs;
    const details = [
        result.alreadyInstalled.length > 0 &&
            text.alreadyInstalled(result.alreadyInstalled.join(", ")),
        result.unavailable.length > 0 && text.unavailable(result.unavailable.join(", ")),
    ]
        .filter(Boolean)
        .join(" ");
    if (result.installing.length > 0) {
        toast.success(text.installing(packName(packId), result.installing.length), {
            description: details || undefined,
        });
    } else {
        toast.info(text.nothingToInstall, { description: details || undefined });
    }
}

export function ContentPacks({ instanceId }: { instanceId: string }) {
    const t = useT();
    const packs = useQuery({
        queryKey: contentPacksKey(instanceId),
        queryFn: () => listContentPacks(instanceId),
    });

    return (
        <Card>
            <CardHeader>
                <CardTitle>{t.content.packs.title}</CardTitle>
                <CardDescription>{t.content.packs.description}</CardDescription>
            </CardHeader>
            <CardContent>
                {packs.error ? (
                    <p className="text-sm text-destructive select-text">
                        {errorMessage(packs.error)}
                    </p>
                ) : !packs.data ? (
                    <Skeleton className="h-40 w-full" />
                ) : (
                    <ul className="flex flex-col divide-y">
                        {packs.data.map((pack) => (
                            <PackRow key={pack.id} instanceId={instanceId} pack={pack} />
                        ))}
                    </ul>
                )}
            </CardContent>
        </Card>
    );
}

function PackRow({ instanceId, pack }: { instanceId: string; pack: ContentPack }) {
    const t = useT();
    const queryClient = useQueryClient();
    const running = useGameStore((state) => instanceId in state.running);
    const Icon = ICONS[pack.id] ?? PackageIcon;
    const item = (t.content.packs.items as Record<string, { name: string; description: string }>)[
        pack.id
    ];
    const refresh = () =>
        Promise.all([
            queryClient.invalidateQueries({ queryKey: contentPacksKey(instanceId) }),
            queryClient.invalidateQueries({ queryKey: installedModsKey(instanceId) }),
        ]);
    const install = useMutation({
        mutationFn: () => installContentPack(instanceId, pack.id),
        onSuccess: (result) => notifyPackResult(pack.id, result),
        onError: (error) =>
            toast.error(getT().content.packs.failed, { description: errorMessage(error) }),
    });
    const remove = useMutation({
        mutationFn: () => removeContentPack(instanceId, pack.id),
        onSuccess: async (count) => {
            toast.success(getT().content.packs.removed(packName(pack.id), count));
            await refresh();
        },
        onError: (error) =>
            toast.error(getT().content.packs.removeFailed, { description: errorMessage(error) }),
    });

    return (
        <li className="flex flex-wrap items-center gap-3 py-3">
            <Icon className="size-5 shrink-0 text-primary" aria-hidden />
            <div className="min-w-0 flex-1">
                <p className="text-sm font-medium">{item?.name ?? pack.id}</p>
                {item?.description && (
                    <p className="text-sm text-muted-foreground">{item.description}</p>
                )}
                <p className="text-xs text-muted-foreground">
                    {pack.mods.map((mod) => mod.name).join(" · ")}
                </p>
            </div>
            <div className="flex gap-2">
                {pack.installed && (
                    <Button
                        size="sm"
                        variant="ghost"
                        aria-label={t.content.packs.removePack(item?.name ?? pack.id)}
                        disabled={running || remove.isPending}
                        onClick={() => remove.mutate()}
                    >
                        <Trash2Icon />
                    </Button>
                )}
                <Button
                    size="sm"
                    variant="outline"
                    className="w-28"
                    disabled={running || install.isPending}
                    onClick={() => install.mutate()}
                >
                    {pack.installed ? t.content.packs.complete : t.common.install}
                </Button>
            </div>
        </li>
    );
}

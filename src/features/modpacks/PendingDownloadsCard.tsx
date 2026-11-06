import { useQuery, useQueryClient } from "@tanstack/react-query";
import { open } from "@tauri-apps/plugin-dialog";
import { ExternalLinkIcon, FileSearchIcon, RefreshCwIcon } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { errorMessage } from "@/api/invoke";
import { checkPendingDownloads, providePendingDownload, skipPendingDownload } from "@/api/modpacks";
import type { PendingDownload } from "@/bindings/PendingDownload";
import { Button } from "@/components/ui/button";
import {
    Card,
    CardAction,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle,
} from "@/components/ui/card";
import { ModIcon } from "@/features/mods/ModIcon";
import { installedModsKey } from "@/features/mods/useMods";
import { openExternal } from "@/lib/external";
import { getT, useT } from "@/i18n";

const POLL_MS = 4000;

export const pendingDownloadsKey = (instanceId: string) =>
    ["pending-downloads", instanceId] as const;

function extensionOf(name: string) {
    const dot = name.lastIndexOf(".");
    return dot > 0 ? name.slice(dot + 1) : "";
}

export function PendingDownloadsCard({ instanceId }: { instanceId: string }) {
    const t = useT();
    const text = t.content.pending;
    const queryClient = useQueryClient();
    const key = pendingDownloadsKey(instanceId);
    const pending = useQuery({
        queryKey: key,
        queryFn: () => checkPendingDownloads(instanceId),
        refetchInterval: (query) => ((query.state.data?.length ?? 0) > 0 ? POLL_MS : false),
        refetchOnWindowFocus: true,
    });
    const [busy, setBusy] = useState<number | null>(null);
    const count = pending.data?.length ?? 0;
    const previous = useRef(count);

    useEffect(() => {
        if (count < previous.current) {
            void queryClient.invalidateQueries({ queryKey: installedModsKey(instanceId) });
        }
        previous.current = count;
    }, [count, instanceId, queryClient]);

    if (count === 0) {
        return null;
    }

    const apply = (next: PendingDownload[]) => queryClient.setQueryData(key, next);

    const onChoose = async (item: PendingDownload) => {
        const extension = extensionOf(item.fileName);
        const path = await open({
            multiple: false,
            directory: false,
            filters: extension ? [{ name: item.fileName, extensions: [extension] }] : undefined,
        });
        if (typeof path !== "string") {
            return;
        }
        setBusy(item.fileId);
        try {
            apply(await providePendingDownload(instanceId, item.fileId, path));
            toast.success(getT().content.pending.added(item.title));
        } catch (error) {
            toast.error(getT().content.pending.wrongFile, { description: errorMessage(error) });
        } finally {
            setBusy(null);
        }
    };

    const onSkip = async (item: PendingDownload) => {
        setBusy(item.fileId);
        try {
            apply(await skipPendingDownload(instanceId, item.fileId));
            toast.info(getT().content.pending.skipped(item.title));
        } catch (error) {
            toast.error(getT().content.pending.skipFailed, { description: errorMessage(error) });
        } finally {
            setBusy(null);
        }
    };

    return (
        <Card className="border-amber-500/40">
            <CardHeader>
                <CardTitle>{text.title(count)}</CardTitle>
                <CardDescription>{text.description}</CardDescription>
                <CardAction>
                    <Button
                        variant="outline"
                        size="sm"
                        disabled={pending.isFetching}
                        onClick={() => void pending.refetch()}
                    >
                        <RefreshCwIcon data-icon="inline-start" />
                        {text.check}
                    </Button>
                </CardAction>
            </CardHeader>
            <CardContent>
                <ul className="flex flex-col divide-y">
                    {pending.data?.map((item) => (
                        <li key={item.fileId} className="flex flex-wrap items-center gap-3 py-2.5">
                            <ModIcon url={item.iconUrl} />
                            <div className="min-w-0 flex-1">
                                <p className="truncate text-sm font-medium">{item.title}</p>
                                <p className="truncate text-xs text-muted-foreground select-text">
                                    {item.fileName}
                                </p>
                            </div>
                            <div className="flex flex-wrap gap-2">
                                <Button size="sm" onClick={() => openExternal(item.pageUrl)}>
                                    <ExternalLinkIcon data-icon="inline-start" />
                                    {text.open}
                                </Button>
                                <Button
                                    size="sm"
                                    variant="outline"
                                    disabled={busy !== null}
                                    onClick={() => void onChoose(item)}
                                >
                                    <FileSearchIcon data-icon="inline-start" />
                                    {text.choose}
                                </Button>
                                <Button
                                    size="sm"
                                    variant="ghost"
                                    disabled={busy !== null}
                                    onClick={() => void onSkip(item)}
                                >
                                    {text.skip}
                                </Button>
                            </div>
                        </li>
                    ))}
                </ul>
            </CardContent>
        </Card>
    );
}

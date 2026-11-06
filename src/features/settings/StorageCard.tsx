import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { BrushCleaningIcon } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { errorMessage } from "@/api/invoke";
import { cleanStorage, getStorageReport } from "@/api/storage";
import type { CleanupSummary } from "@/bindings/CleanupSummary";
import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { getT, useT } from "@/i18n";
import { formatBytes } from "@/lib/format";

const storageKey = ["storage"] as const;

function cleanupDetails(summary: CleanupSummary): string[] {
    const t = getT().settings.storage;
    const details: string[] = [];
    if (summary.unusedVersions.length > 0) {
        details.push(t.unusedVersions(summary.unusedVersions.join(", ")));
    }
    if (summary.unusedRuntimes.length > 0) {
        details.push(t.unusedRuntimes(summary.unusedRuntimes.join(", ")));
    }
    if (summary.libraryFiles > 0) {
        details.push(t.libraryFiles(summary.libraryFiles));
    }
    if (summary.assetFiles > 0) {
        details.push(t.assetFiles(summary.assetFiles));
    }
    if (summary.cacheBytes > 0) {
        details.push(t.cache(formatBytes(summary.cacheBytes)));
    }
    return details;
}

export function StorageCard() {
    const t = useT();
    const queryClient = useQueryClient();
    const report = useQuery({ queryKey: storageKey, queryFn: getStorageReport, staleTime: 0 });
    const [confirm, setConfirm] = useState(false);
    const clean = useMutation({
        mutationFn: cleanStorage,
        onSuccess: (result) => {
            toast.success(getT().settings.storage.freed(formatBytes(result.freedBytes)));
            return queryClient.invalidateQueries({ queryKey: storageKey });
        },
        onError: (error) =>
            toast.error(getT().settings.storage.failed, { description: errorMessage(error) }),
    });

    const data = report.data;
    const total = data?.categories.reduce((sum, c) => sum + c.sizeBytes, 0) ?? 0;
    const largest = Math.max(1, ...(data?.categories.map((c) => c.sizeBytes) ?? [1]));
    const details = data ? cleanupDetails(data.cleanup) : [];

    return (
        <Card className="max-w-3xl">
            <CardHeader>
                <CardTitle>{t.settings.storage.title}</CardTitle>
                <CardDescription>
                    {data
                        ? t.settings.storage.usage(formatBytes(total))
                        : t.settings.storage.calculating}
                </CardDescription>
            </CardHeader>
            <CardContent className="flex flex-col gap-5">
                {report.error ? (
                    <p className="text-sm text-destructive select-text">
                        {errorMessage(report.error)}
                    </p>
                ) : !data ? (
                    <Skeleton className="h-[19rem] w-full" />
                ) : (
                    <>
                        <dl className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-x-4 gap-y-2.5 text-sm">
                            {data.categories.map((category) => (
                                <div key={category.category} className="contents">
                                    <dt className="flex min-w-0 flex-col gap-1">
                                        <span className="truncate">
                                            {t.settings.storage.categories[category.category]}
                                        </span>
                                        <span aria-hidden className="h-1.5 rounded-full bg-muted">
                                            <span
                                                className="block h-full rounded-full bg-primary/70"
                                                style={{
                                                    width: `${Math.max(1, (category.sizeBytes / largest) * 100)}%`,
                                                }}
                                            />
                                        </span>
                                    </dt>
                                    <dd className="text-right text-muted-foreground tabular-nums">
                                        {formatBytes(category.sizeBytes)}
                                    </dd>
                                </div>
                            ))}
                        </dl>
                        {data.instances.length > 0 && (
                            <div className="flex flex-col gap-1.5 text-sm">
                                <p className="font-medium">{t.settings.storage.largestInstances}</p>
                                <ul className="flex flex-col gap-1">
                                    {data.instances.slice(0, 5).map((instance) => (
                                        <li
                                            key={instance.id}
                                            className="flex justify-between gap-4"
                                        >
                                            <span className="truncate text-muted-foreground">
                                                {instance.name}
                                            </span>
                                            <span className="shrink-0 text-muted-foreground tabular-nums">
                                                {formatBytes(instance.sizeBytes)}
                                            </span>
                                        </li>
                                    ))}
                                </ul>
                            </div>
                        )}
                        <div className="flex flex-col gap-3 rounded-xl border bg-muted/30 p-4">
                            <div className="flex flex-wrap items-center justify-between gap-3">
                                <div className="min-w-0">
                                    <p className="font-medium">
                                        {data.cleanup.totalBytes > 0
                                            ? t.settings.storage.cleanupFrees(
                                                  formatBytes(data.cleanup.totalBytes),
                                              )
                                            : t.settings.storage.nothingToClean}
                                    </p>
                                    <p className="text-sm text-muted-foreground">
                                        {t.settings.storage.keepsInstanceData}
                                    </p>
                                </div>
                                <Button
                                    variant="outline"
                                    disabled={data.cleanup.totalBytes === 0 || clean.isPending}
                                    onClick={() => setConfirm(true)}
                                >
                                    <BrushCleaningIcon data-icon="inline-start" />
                                    {clean.isPending
                                        ? t.settings.storage.cleaning
                                        : t.settings.storage.cleanUp}
                                </Button>
                            </div>
                            {details.length > 0 && (
                                <ul className="list-disc pl-5 text-sm text-muted-foreground">
                                    {details.map((line) => (
                                        <li key={line} className="[overflow-wrap:anywhere]">
                                            {line}
                                        </li>
                                    ))}
                                </ul>
                            )}
                            {!data.cleanup.complete && (
                                <p className="text-sm text-muted-foreground">
                                    {t.settings.storage.incomplete}
                                </p>
                            )}
                        </div>
                    </>
                )}
            </CardContent>
            <AlertDialog open={confirm} onOpenChange={setConfirm}>
                <AlertDialogContent>
                    <AlertDialogHeader>
                        <AlertDialogTitle>{t.settings.storage.confirmTitle}</AlertDialogTitle>
                        <AlertDialogDescription>
                            {t.settings.storage.confirmDescription}
                        </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel>{t.common.cancel}</AlertDialogCancel>
                        <AlertDialogAction onClick={() => clean.mutate()}>
                            {t.settings.storage.cleanUp}
                        </AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
        </Card>
    );
}

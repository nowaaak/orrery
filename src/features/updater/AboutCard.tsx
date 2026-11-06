import { useQuery, useQueryClient } from "@tanstack/react-query";
import { ExternalLinkIcon, RefreshCwIcon, ScaleIcon } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { errorMessage } from "@/api/invoke";
import { checkForUpdate, getAppInfo } from "@/api/updater";
import type { UpdateInfo } from "@/bindings/UpdateInfo";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { getT, useT } from "@/i18n";
import { openExternal } from "@/lib/external";
import { LicensesDialog } from "./LicensesDialog";
import { UpdateDialog, updateKey } from "./UpdateNotice";

export const appInfoKey = ["app-info"] as const;
const RELEASES_URL = "https://github.com/nowaaak/orrery/releases";
const PRIVACY_URL = "https://nowaaak.github.io/orrery/privacy.html";

function releaseUrl(version: string): string {
    return `${RELEASES_URL}/tag/v${version}`;
}

export function useAppInfo() {
    return useQuery({ queryKey: appInfoKey, queryFn: getAppInfo, staleTime: Infinity });
}

export function AboutCard() {
    const t = useT();
    const info = useAppInfo();
    const queryClient = useQueryClient();
    const [checking, setChecking] = useState(false);
    const [found, setFound] = useState<UpdateInfo | null>(null);
    const [showLicenses, setShowLicenses] = useState(false);
    const isDev = info.data?.channel === "dev";

    const onCheck = async () => {
        setChecking(true);
        try {
            const update = await queryClient.fetchQuery({
                queryKey: updateKey,
                queryFn: checkForUpdate,
                staleTime: 0,
            });
            if (update) {
                setFound(update);
            } else {
                const about = getT().updater.about;
                toast.success(about.upToDate, {
                    description: about.upToDateDetail(info.data?.version ?? ""),
                });
            }
        } catch (error) {
            toast.error(getT().updater.failed, { description: errorMessage(error) });
        } finally {
            setChecking(false);
        }
    };

    return (
        <Card className="max-w-3xl">
            <CardHeader>
                <CardTitle>{t.updater.about.title}</CardTitle>
                <CardDescription>{t.updater.about.description}</CardDescription>
            </CardHeader>
            <CardContent className="flex flex-col gap-3">
                {info.data ? (
                    <>
                        <div className="flex flex-wrap items-center justify-between gap-3">
                            <div className="flex items-center gap-2">
                                <span className="font-medium tabular-nums">
                                    {t.updater.about.version(info.data.version)}
                                </span>
                                <Badge variant={isDev ? "secondary" : "outline"}>
                                    {isDev ? t.updater.about.dev : t.updater.about.release}
                                </Badge>
                            </div>
                            {!isDev && (
                                <Button
                                    variant="outline"
                                    disabled={checking}
                                    onClick={() => void onCheck()}
                                >
                                    <RefreshCwIcon
                                        data-icon="inline-start"
                                        className={
                                            checking ? "motion-safe:animate-spin" : undefined
                                        }
                                    />
                                    {checking ? t.updater.about.checking : t.updater.about.check}
                                </Button>
                            )}
                        </div>
                        <p className="text-sm text-muted-foreground">
                            {isDev ? t.updater.about.devHint : t.updater.about.autoCheck}
                        </p>
                        <div className="flex flex-wrap gap-x-6 gap-y-2">
                            <Button
                                variant="link"
                                className="h-auto p-0"
                                onClick={() => openExternal(releaseUrl(info.data.version))}
                            >
                                <ExternalLinkIcon data-icon="inline-start" />
                                {t.updater.about.whatsNew}
                            </Button>
                            <Button
                                variant="link"
                                className="h-auto p-0"
                                onClick={() => openExternal(PRIVACY_URL)}
                            >
                                <ExternalLinkIcon data-icon="inline-start" />
                                {t.updater.about.privacy}
                            </Button>
                            <Button
                                variant="link"
                                className="h-auto p-0"
                                onClick={() => setShowLicenses(true)}
                            >
                                <ScaleIcon data-icon="inline-start" />
                                {t.updater.about.licensesLink}
                            </Button>
                        </div>
                        <p className="text-xs text-muted-foreground">
                            {t.updater.about.disclaimer}
                        </p>
                    </>
                ) : (
                    <Skeleton className="h-16 w-full" />
                )}
            </CardContent>
            <LicensesDialog open={showLicenses} onOpenChange={setShowLicenses} />
            {found && (
                <UpdateDialog
                    version={found.version}
                    currentVersion={found.currentVersion}
                    notes={found.notes}
                    onClose={() => setFound(null)}
                />
            )}
        </Card>
    );
}

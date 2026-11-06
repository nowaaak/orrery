import { ArrowLeftIcon, ClockIcon, TriangleAlertIcon, XIcon } from "lucide-react";
import { Link, useParams, useSearchParams } from "react-router";
import { errorMessage } from "@/api/invoke";
import type { Instance } from "@/bindings/Instance";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { GameLog } from "@/features/launch/GameLog";
import { LaunchButton } from "@/features/launch/LaunchButton";
import { companionLocation } from "@/features/launch/companionStatus";
import { crashHint } from "@/features/launch/crashHints";
import { useGameStore } from "@/features/launch/gameStore";
import { ModsTab } from "@/features/mods/ModsTab";
import { ScreenshotsTab } from "@/features/screenshots/ScreenshotsTab";
import { ServersTab } from "@/features/servers/ServersTab";
import { WorldsTab } from "@/features/worlds/WorldsTab";
import { useT } from "@/i18n";
import { formatPlaytime } from "@/lib/format";
import { FolderMenu } from "./FolderMenu";
import { InstanceActions } from "./InstanceActions";
import { InstanceAvatar } from "./InstanceAvatar";
import { InstanceSettingsForm } from "./InstanceSettingsForm";
import { lastPlayedLabel, loaderLabel } from "./labels";
import { useInstance } from "./useInstances";

const TABS = ["log", "mods", "worlds", "servers", "screenshots", "settings"] as const;
type Tab = (typeof TABS)[number];

function formKey(instance: Instance): string {
    const { name, minecraftVersion, loader, java, resolution, launch } = instance;
    return JSON.stringify({ name, minecraftVersion, loader, java, resolution, launch });
}

export function InstanceDetailPage() {
    const t = useT();
    const { id = "" } = useParams();
    const instance = useInstance(id);
    const [params, setParams] = useSearchParams();
    const requested = params.get("tab");
    const tab: Tab = TABS.includes(requested as Tab) ? (requested as Tab) : "log";

    if (instance.error) {
        return (
            <div className="flex flex-col items-start gap-4">
                <BackLink />
                <p className="text-destructive select-text">{errorMessage(instance.error)}</p>
            </div>
        );
    }
    if (!instance.data) {
        return (
            <div className="flex flex-col gap-6">
                <BackLink />
                <Skeleton className="h-36 w-full rounded-2xl" />
            </div>
        );
    }
    const data = instance.data;

    return (
        <div className="flex flex-col gap-6">
            <BackLink />
            <header className="rounded-2xl border bg-card p-6">
                <div className="flex flex-wrap items-center gap-5">
                    <InstanceAvatar instance={data} size="lg" />
                    <div className="flex min-w-0 flex-1 flex-col gap-2">
                        <h1 className="truncate text-2xl font-semibold tracking-tight">
                            {data.name}
                        </h1>
                        <div className="flex flex-wrap items-center gap-x-3 gap-y-1.5 text-sm text-muted-foreground">
                            <Badge variant="secondary">{data.minecraftVersion}</Badge>
                            <Badge variant="outline">{loaderLabel(data)}</Badge>
                            <CompanionBadge instanceId={data.id} />
                            <span>{lastPlayedLabel(data.lastPlayedAt)}</span>
                            <span className="inline-flex items-center gap-1 tabular-nums">
                                <ClockIcon className="size-3.5" aria-hidden />
                                {formatPlaytime(data.totalPlaySecs)}
                            </span>
                        </div>
                    </div>
                    <div className="flex items-center gap-2">
                        <FolderMenu instance={data} />
                        <LaunchButton instanceId={data.id} size="lg" className="w-48" />
                        <InstanceActions instance={data} />
                    </div>
                </div>
            </header>

            <Tabs value={tab} onValueChange={(next) => setParams({ tab: next }, { replace: true })}>
                <TabsList>
                    <TabsTrigger value="log">{t.instances.tabs.log}</TabsTrigger>
                    <TabsTrigger value="mods">{t.instances.tabs.mods}</TabsTrigger>
                    <TabsTrigger value="worlds">{t.instances.tabs.worlds}</TabsTrigger>
                    <TabsTrigger value="servers">{t.instances.tabs.servers}</TabsTrigger>
                    <TabsTrigger value="screenshots">{t.instances.tabs.screenshots}</TabsTrigger>
                    <TabsTrigger value="settings">{t.instances.tabs.settings}</TabsTrigger>
                </TabsList>
                <TabsContent value="log" className="flex flex-col gap-4 pt-4">
                    <CrashBanner instanceId={data.id} />
                    <GameLog instanceId={data.id} />
                </TabsContent>
                <TabsContent value="mods" className="pt-4">
                    <ModsTab instance={data} />
                </TabsContent>
                <TabsContent value="worlds" className="pt-4">
                    <WorldsTab instance={data} />
                </TabsContent>
                <TabsContent value="servers" className="pt-4">
                    <ServersTab instance={data} />
                </TabsContent>
                <TabsContent value="screenshots" className="pt-4">
                    <ScreenshotsTab instance={data} />
                </TabsContent>
                <TabsContent value="settings" className="pt-4">
                    <InstanceSettingsForm key={formKey(data)} instance={data} />
                </TabsContent>
            </Tabs>
        </div>
    );
}

function CrashBanner({ instanceId }: { instanceId: string }) {
    const t = useT();
    const crash = useGameStore((state) => state.crashes[instanceId]);
    const showCrash = useGameStore((state) => state.showCrash);
    const clearCrash = useGameStore((state) => state.clearCrash);

    if (!crash) {
        return null;
    }
    return (
        <div
            role="alert"
            className="flex items-center gap-3 rounded-xl border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm"
        >
            <TriangleAlertIcon className="size-4 shrink-0 text-destructive" aria-hidden />
            <p className="min-w-0 flex-1">
                <span className="font-medium">{t.instances.crashBanner}</span>{" "}
                {crashHint(crash.hint).title}
            </p>
            <Button size="sm" variant="outline" onClick={() => showCrash(instanceId)}>
                {t.instances.details}
            </Button>
            <Button
                size="icon-sm"
                variant="ghost"
                aria-label={t.instances.dismissCrash}
                onClick={() => clearCrash(instanceId)}
            >
                <XIcon />
            </Button>
        </div>
    );
}

function CompanionBadge({ instanceId }: { instanceId: string }) {
    const t = useT();
    const companion = useGameStore((state) => state.companion[instanceId]);
    if (!companion) {
        return null;
    }
    const where = companionLocation(companion.status);
    return (
        <Badge variant="secondary" title={`Orrery Companion ${companion.modVersion}`}>
            {t.instances.launch.companionConnected}
            {where ? ` · ${where}` : ""}
        </Badge>
    );
}

function BackLink() {
    const t = useT();
    return (
        <Button variant="ghost" size="sm" asChild className="self-start">
            <Link to="/">
                <ArrowLeftIcon data-icon="inline-start" />
                {t.instances.allInstances}
            </Link>
        </Button>
    );
}

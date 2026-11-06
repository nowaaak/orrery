import { ChevronDownIcon, ClockIcon, OrbitIcon, PinIcon } from "lucide-react";
import { useState } from "react";
import { Link } from "react-router";
import { toast } from "sonner";
import { errorMessage } from "@/api/invoke";
import type { Instance } from "@/bindings/Instance";
import type { InstanceSort } from "@/bindings/InstanceSort";
import { PageHeader } from "@/app/PageHeader";
import { Badge } from "@/components/ui/badge";
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { useSettings, useUpdateSettings } from "@/features/settings/useSettings";
import { LaunchButton } from "@/features/launch/LaunchButton";
import { useGameStore } from "@/features/launch/gameStore";
import { useT } from "@/i18n";
import { formatPlaytime } from "@/lib/format";
import { cn } from "@/lib/utils";
import { ImportButton } from "@/features/imports/ImportDialog";
import { ModpackButton } from "@/features/modpacks/ModpackDialog";
import { CreateInstanceButton } from "./CreateInstanceDialog";
import { InstanceActions } from "./InstanceActions";
import { InstanceAvatar } from "./InstanceAvatar";
import { instanceSections, type InstanceSection } from "./instanceOrder";
import { lastPlayedLabel, lastPlayedShort, loaderLabel } from "./labels";
import { useInstances } from "./useInstances";

const SORTS: InstanceSort[] = ["last_played", "name", "created", "playtime"];
const COLLAPSED_KEY = "orrery.collapsedSections";

function readCollapsed(): string[] {
    try {
        const stored: unknown = JSON.parse(localStorage.getItem(COLLAPSED_KEY) ?? "[]");
        return Array.isArray(stored) ? stored.filter((key) => typeof key === "string") : [];
    } catch {
        return [];
    }
}

function writeCollapsed(keys: string[]) {
    try {
        localStorage.setItem(COLLAPSED_KEY, JSON.stringify(keys));
    } catch {
        return;
    }
}

export function InstancesPage() {
    const t = useT();
    const instances = useInstances();
    const settings = useSettings();
    const updateSettings = useUpdateSettings();
    const sort = settings.data?.instanceSort ?? "last_played";
    const recent = instances.data?.find((instance) => instance.lastPlayedAt !== null);
    const sections = instances.data ? instanceSections(instances.data, sort) : [];
    const plain = sections.length === 1 && sections[0]!.kind === "ungrouped";

    const onSortChange = (next: InstanceSort) => {
        if (settings.data) {
            updateSettings.mutate(
                { ...settings.data, instanceSort: next },
                {
                    onError: (error) =>
                        toast.error(t.settings.saveFailed, { description: errorMessage(error) }),
                },
            );
        }
    };

    return (
        <div className="flex flex-col gap-8">
            <PageHeader
                title={t.instances.title}
                actions={
                    <>
                        <ImportButton />
                        <ModpackButton />
                        <CreateInstanceButton />
                    </>
                }
            />
            {instances.error ? (
                <p className="text-destructive select-text">{errorMessage(instances.error)}</p>
            ) : !instances.data ? (
                <div className="flex flex-col gap-8">
                    <Skeleton className="h-40 rounded-2xl" />
                    <InstanceGrid>
                        {[0, 1, 2].map((key) => (
                            <Skeleton key={key} className="h-44 rounded-2xl" />
                        ))}
                    </InstanceGrid>
                </div>
            ) : instances.data.length === 0 ? (
                <EmptyState />
            ) : (
                <>
                    {recent && <ContinueCard instance={recent} />}
                    <div className="flex flex-col gap-6">
                        <div className="flex items-center justify-between gap-4">
                            <h2 className="text-sm font-medium tracking-wide text-muted-foreground uppercase">
                                {t.instances.allInstances}
                            </h2>
                            <Select
                                value={sort}
                                disabled={!settings.data}
                                onValueChange={(value) => onSortChange(value as InstanceSort)}
                            >
                                <SelectTrigger
                                    className="w-48"
                                    size="sm"
                                    aria-label={t.instances.sort.label}
                                >
                                    <SelectValue />
                                </SelectTrigger>
                                <SelectContent align="end">
                                    {SORTS.map((value) => (
                                        <SelectItem key={value} value={value}>
                                            {t.instances.sort[value]}
                                        </SelectItem>
                                    ))}
                                </SelectContent>
                            </Select>
                        </div>
                        {plain ? (
                            <InstanceGrid>
                                {sections[0]!.instances.map((instance) => (
                                    <InstanceCard key={instance.id} instance={instance} />
                                ))}
                            </InstanceGrid>
                        ) : (
                            <Sections sections={sections} />
                        )}
                    </div>
                </>
            )}
        </div>
    );
}

function sectionKey(section: InstanceSection): string {
    return section.kind === "group" ? `group:${section.name}` : section.kind;
}

function Sections({ sections }: { sections: InstanceSection[] }) {
    const t = useT();
    const [collapsed, setCollapsed] = useState(readCollapsed);

    const toggle = (key: string) =>
        setCollapsed((current) => {
            const next = current.includes(key)
                ? current.filter((item) => item !== key)
                : [...current, key];
            writeCollapsed(next);
            return next;
        });

    return sections.map((section) => {
        const key = sectionKey(section);
        const open = !collapsed.includes(key);
        const title =
            section.kind === "pinned"
                ? t.instances.pinnedSection
                : section.kind === "group"
                  ? section.name
                  : t.instances.ungroupedSection;
        const headingId = `section-${key}`;
        return (
            <section key={key} aria-labelledby={headingId} className="flex flex-col gap-3">
                <h3 id={headingId}>
                    <button
                        type="button"
                        aria-expanded={open}
                        aria-label={t.instances.toggleSection(title)}
                        onClick={() => toggle(key)}
                        className="-mx-1.5 flex items-center gap-2 rounded-md px-1.5 py-1 text-sm font-medium text-foreground/85 outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring"
                    >
                        <ChevronDownIcon
                            className={cn(
                                "size-4 text-muted-foreground transition-transform motion-reduce:transition-none",
                                !open && "-rotate-90",
                            )}
                            aria-hidden
                        />
                        {section.kind === "pinned" && (
                            <PinIcon className="size-3.5 text-primary" aria-hidden />
                        )}
                        <span>{title}</span>
                        <span className="text-muted-foreground tabular-nums">
                            {section.instances.length}
                        </span>
                    </button>
                </h3>
                {open && (
                    <InstanceGrid>
                        {section.instances.map((instance) => (
                            <InstanceCard key={instance.id} instance={instance} />
                        ))}
                    </InstanceGrid>
                )}
            </section>
        );
    });
}

function InstanceGrid({ children }: { children: React.ReactNode }) {
    return (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(17rem,1fr))] gap-4">{children}</div>
    );
}

function ContinueCard({ instance }: { instance: Instance }) {
    const t = useT();
    return (
        <section
            aria-label={t.instances.continuePlaying}
            className="rounded-2xl border bg-card p-6"
        >
            <div className="flex flex-wrap items-center gap-5">
                <InstanceAvatar instance={instance} size="lg" />
                <div className="flex min-w-0 flex-1 flex-col gap-1.5">
                    <p className="text-xs font-medium tracking-wide text-primary uppercase">
                        {t.instances.continuePlaying}
                    </p>
                    <Link
                        to={`/instances/${instance.id}`}
                        className="truncate rounded-sm text-2xl font-semibold tracking-tight outline-none hover:underline focus-visible:ring-2 focus-visible:ring-ring"
                    >
                        {instance.name}
                    </Link>
                    <p className="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm text-muted-foreground">
                        <span>
                            {instance.minecraftVersion} · {loaderLabel(instance)}
                        </span>
                        <span>{lastPlayedLabel(instance.lastPlayedAt)}</span>
                        <span className="inline-flex items-center gap-1 tabular-nums">
                            <ClockIcon className="size-3.5" aria-hidden />
                            {formatPlaytime(instance.totalPlaySecs)}
                        </span>
                    </p>
                </div>
                <LaunchButton instanceId={instance.id} size="lg" className="w-48" />
            </div>
        </section>
    );
}

function InstanceCard({ instance }: { instance: Instance }) {
    const t = useT();
    const running = useGameStore((state) => instance.id in state.running);

    return (
        <article className="group relative flex flex-col gap-4 rounded-2xl border bg-card p-4 transition-colors hover:border-foreground/15 motion-reduce:transition-none">
            <div className="flex items-start gap-3">
                <InstanceAvatar instance={instance} />
                <div className="flex min-w-0 flex-1 flex-col gap-1">
                    <Link
                        to={`/instances/${instance.id}`}
                        className="truncate rounded-sm font-semibold outline-none after:absolute after:inset-0 after:rounded-2xl hover:underline focus-visible:ring-2 focus-visible:ring-ring"
                    >
                        {instance.name}
                    </Link>
                    <div className="flex flex-wrap items-center gap-1.5">
                        <Badge variant="secondary">{instance.minecraftVersion}</Badge>
                        <Badge variant="outline">{loaderLabel(instance)}</Badge>
                        {running && <Badge>{t.instances.running}</Badge>}
                        {instance.pinned && (
                            <PinIcon
                                className="size-3.5 text-primary"
                                aria-label={t.instances.pinnedSection}
                            />
                        )}
                    </div>
                </div>
            </div>
            <dl className="grid grid-cols-2 gap-2 text-xs">
                <div className="rounded-lg bg-muted/50 px-2.5 py-2">
                    <dt className="text-muted-foreground">{t.instances.playtime}</dt>
                    <dd className="font-medium tabular-nums">
                        {formatPlaytime(instance.totalPlaySecs)}
                    </dd>
                </div>
                <div className="rounded-lg bg-muted/50 px-2.5 py-2">
                    <dt className="text-muted-foreground">{t.instances.lastPlayed}</dt>
                    <dd className="truncate font-medium">
                        {lastPlayedShort(instance.lastPlayedAt)}
                    </dd>
                </div>
            </dl>
            <div className="relative z-10 mt-auto flex items-center gap-2">
                <LaunchButton instanceId={instance.id} className="flex-1" />
                <InstanceActions instance={instance} />
            </div>
        </article>
    );
}

function EmptyState() {
    const t = useT();
    return (
        <div className="flex flex-col items-center gap-4 rounded-2xl border border-dashed p-14 text-center">
            <OrbitIcon className="size-10 text-primary" aria-hidden />
            <div>
                <p className="text-lg font-semibold">{t.instances.emptyTitle}</p>
                <p className="text-sm text-muted-foreground">{t.instances.emptyDescription}</p>
            </div>
            <div className="flex gap-2">
                <ImportButton />
                <ModpackButton />
                <CreateInstanceButton />
            </div>
        </div>
    );
}

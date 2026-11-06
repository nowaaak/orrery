import { useQuery } from "@tanstack/react-query";
import { useState, type FormEvent } from "react";
import { toast } from "sonner";
import { errorMessage } from "@/api/invoke";
import { discordAvailable } from "@/api/settings";
import type { Language } from "@/bindings/Language";
import type { LauncherWindow } from "@/bindings/LauncherWindow";
import type { PresenceLevel } from "@/bindings/PresenceLevel";
import type { Settings } from "@/bindings/Settings";
import type { Theme } from "@/bindings/Theme";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { PageHeader } from "@/app/PageHeader";
import { getT, useT } from "@/i18n";
import { AboutCard } from "@/features/updater/AboutCard";
import { DataDirCard } from "./DataDirCard";
import { MemorySelect, useSystemMemory } from "./MemorySelect";
import { TrashCard } from "@/features/trash/TrashCard";
import { StorageCard } from "./StorageCard";
import { useSettings, useUpdateSettings } from "./useSettings";

const IS_WINDOWS = navigator.userAgent.includes("Windows");

const THEMES: Theme[] = ["system", "light", "dark"];
const LANGUAGES: Language[] = ["system", "de", "en"];
const LAUNCHER_WINDOWS: LauncherWindow[] = ["keep_open", "minimize", "hide"];
const PRESENCE_LEVELS: PresenceLevel[] = ["off", "basic", "detailed"];

export function SettingsPage() {
    const t = useT();
    const settings = useSettings();

    return (
        <div className="flex flex-col gap-6">
            <PageHeader title={t.settings.title} />
            {settings.data ? (
                <SettingsForm initial={settings.data} />
            ) : settings.error ? (
                <p className="text-destructive select-text">{errorMessage(settings.error)}</p>
            ) : (
                <Skeleton className="h-80 w-full max-w-3xl" />
            )}
            <StorageCard />
            <TrashCard />
            <DataDirCard />
            <AboutCard />
        </div>
    );
}

function SettingsForm({ initial }: { initial: Settings }) {
    const t = useT();
    const memory = useSystemMemory();
    const discord = useQuery({
        queryKey: ["discord-available"],
        queryFn: discordAvailable,
        staleTime: Infinity,
    });
    const [draft, setDraft] = useState(initial);
    const update = useUpdateSettings();
    const dirty = JSON.stringify(draft) !== JSON.stringify(initial);

    const set = <K extends keyof Settings>(key: K, value: Settings[K]) =>
        setDraft((current) => ({ ...current, [key]: value }));

    const onSubmit = (event: FormEvent) => {
        event.preventDefault();
        update.mutate(draft, {
            onSuccess: (saved) => {
                setDraft(saved);
                toast.success(getT().settings.saved);
            },
            onError: (error) =>
                toast.error(getT().settings.saveFailed, { description: errorMessage(error) }),
        });
    };

    const onThemeChange = (theme: Theme) => {
        set("theme", theme);
        update.mutate({ ...initial, theme });
    };

    const onLanguageChange = (language: Language) => {
        set("language", language);
        update.mutate({ ...initial, language });
    };

    return (
        <Card className="max-w-3xl">
            <CardHeader>
                <CardTitle>{t.settings.general}</CardTitle>
                <CardDescription>{t.settings.generalDescription}</CardDescription>
            </CardHeader>
            <CardContent>
                <form onSubmit={onSubmit} className="flex flex-col gap-5">
                    <div className="grid gap-2">
                        <Label htmlFor="theme">{t.settings.theme}</Label>
                        <Select
                            value={draft.theme}
                            onValueChange={(value) => onThemeChange(value as Theme)}
                        >
                            <SelectTrigger id="theme" className="w-48">
                                <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                                {THEMES.map((theme) => (
                                    <SelectItem key={theme} value={theme}>
                                        {t.settings.themes[theme]}
                                    </SelectItem>
                                ))}
                            </SelectContent>
                        </Select>
                    </div>

                    <div className="grid gap-2">
                        <Label htmlFor="language">{t.settings.language}</Label>
                        <div className="flex items-center gap-3">
                            <Select
                                value={draft.language}
                                disabled={dirty}
                                onValueChange={(value) => onLanguageChange(value as Language)}
                            >
                                <SelectTrigger
                                    id="language"
                                    className="w-48"
                                    aria-describedby={dirty ? "language-locked" : undefined}
                                >
                                    <SelectValue />
                                </SelectTrigger>
                                <SelectContent>
                                    {LANGUAGES.map((language) => (
                                        <SelectItem key={language} value={language}>
                                            {t.settings.languages[language]}
                                        </SelectItem>
                                    ))}
                                </SelectContent>
                            </Select>
                            {dirty && (
                                <span
                                    id="language-locked"
                                    className="text-sm text-muted-foreground"
                                >
                                    {t.settings.languageLocked}
                                </span>
                            )}
                        </div>
                    </div>

                    <div className="grid grid-cols-2 gap-4">
                        <MemorySelect
                            id="min-memory"
                            label={t.settings.minMemory}
                            value={draft.defaultMinMemoryMb}
                            floorMb={512}
                            maxMb={draft.defaultMaxMemoryMb}
                            onChange={(value) => set("defaultMinMemoryMb", value)}
                        />
                        <MemorySelect
                            id="max-memory"
                            label={t.settings.maxMemory}
                            value={draft.defaultMaxMemoryMb}
                            onChange={(value) =>
                                setDraft((current) => ({
                                    ...current,
                                    defaultMaxMemoryMb: value,
                                    defaultMinMemoryMb: Math.min(current.defaultMinMemoryMb, value),
                                }))
                            }
                        />
                    </div>
                    <p className="-mt-2 text-sm text-muted-foreground">
                        {memory.data ? t.settings.systemMemory(Math.round(memory.data / 1024)) : ""}
                        {t.settings.memoryHint}
                    </p>

                    <NumberField
                        id="downloads"
                        label={t.settings.downloads}
                        value={draft.maxConcurrentDownloads}
                        min={1}
                        max={64}
                        onChange={(value) => set("maxConcurrentDownloads", value)}
                    />

                    <div className="flex items-center justify-between gap-4">
                        <div>
                            <Label htmlFor="snapshots">{t.settings.snapshots}</Label>
                            <p className="text-sm text-muted-foreground">
                                {t.settings.snapshotsDescription}
                            </p>
                        </div>
                        <Switch
                            id="snapshots"
                            checked={draft.showSnapshots}
                            onCheckedChange={(checked) => set("showSnapshots", checked)}
                        />
                    </div>

                    <div className="flex items-center justify-between gap-4">
                        <div>
                            <Label htmlFor="game-options">{t.settings.gameOptions}</Label>
                            <p className="text-sm text-muted-foreground">
                                {t.settings.gameOptionsDescription}
                            </p>
                        </div>
                        <Switch
                            id="game-options"
                            checked={draft.optimizeGameOptions}
                            onCheckedChange={(checked) => set("optimizeGameOptions", checked)}
                        />
                    </div>

                    {IS_WINDOWS && (
                        <div className="flex items-center justify-between gap-4">
                            <div>
                                <Label htmlFor="dedicated-gpu">{t.settings.dedicatedGpu}</Label>
                                <p className="text-sm text-muted-foreground">
                                    {t.settings.dedicatedGpuDescription}
                                </p>
                            </div>
                            <Switch
                                id="dedicated-gpu"
                                checked={draft.preferDedicatedGpu}
                                onCheckedChange={(checked) => set("preferDedicatedGpu", checked)}
                            />
                        </div>
                    )}

                    {discord.data && (
                        <div className="grid gap-2">
                            <Label htmlFor="discord-presence">{t.settings.discordPresence}</Label>
                            <Select
                                value={draft.discordPresence}
                                onValueChange={(value) =>
                                    set("discordPresence", value as PresenceLevel)
                                }
                            >
                                <SelectTrigger
                                    id="discord-presence"
                                    className="w-80"
                                    aria-describedby="discord-presence-hint"
                                >
                                    <SelectValue />
                                </SelectTrigger>
                                <SelectContent>
                                    {PRESENCE_LEVELS.map((value) => (
                                        <SelectItem key={value} value={value}>
                                            {t.settings.discordPresences[value]}
                                        </SelectItem>
                                    ))}
                                </SelectContent>
                            </Select>
                            <p id="discord-presence-hint" className="text-sm text-muted-foreground">
                                {t.settings.discordPresenceDescription}
                            </p>
                        </div>
                    )}

                    <div className="grid gap-2">
                        <Label htmlFor="launcher-window">{t.settings.launcherWindow}</Label>
                        <Select
                            value={draft.launcherWindow}
                            onValueChange={(value) =>
                                set("launcherWindow", value as LauncherWindow)
                            }
                        >
                            <SelectTrigger id="launcher-window" className="w-80">
                                <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                                {LAUNCHER_WINDOWS.map((value) => (
                                    <SelectItem key={value} value={value}>
                                        {t.settings.launcherWindows[value]}
                                    </SelectItem>
                                ))}
                            </SelectContent>
                        </Select>
                    </div>

                    <div className="flex gap-2">
                        <Button type="submit" disabled={!dirty || update.isPending}>
                            {update.isPending ? t.common.saving : t.common.save}
                        </Button>
                        <Button
                            type="button"
                            variant="ghost"
                            disabled={!dirty || update.isPending}
                            onClick={() => setDraft(initial)}
                        >
                            {t.common.discard}
                        </Button>
                    </div>
                </form>
            </CardContent>
        </Card>
    );
}

type NumberFieldProps = {
    id: string;
    label: string;
    value: number;
    min?: number;
    max?: number;
    step?: number;
    onChange: (value: number) => void;
};

function NumberField({ id, label, value, min, max, step, onChange }: NumberFieldProps) {
    return (
        <div className="grid gap-2">
            <Label htmlFor={id}>{label}</Label>
            <Input
                id={id}
                type="number"
                inputMode="numeric"
                className="tabular-nums"
                value={Number.isNaN(value) ? "" : value}
                min={min}
                max={max}
                step={step}
                onChange={(event) => onChange(event.target.valueAsNumber)}
            />
        </div>
    );
}

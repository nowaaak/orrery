import { useQuery } from "@tanstack/react-query";
import { useState, type FormEvent } from "react";
import { toast } from "sonner";
import { errorMessage } from "@/api/invoke";
import { recommendSettings } from "@/api/tuning";
import type { Instance } from "@/bindings/Instance";
import type { InstanceUpdate } from "@/bindings/InstanceUpdate";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { useGameStore } from "@/features/launch/gameStore";
import { MemorySelect } from "@/features/settings/MemorySelect";
import { useSettings } from "@/features/settings/useSettings";
import { useT } from "@/i18n";
import { JavaPathField } from "./JavaPathField";
import {
    LaunchOptionsCard,
    toLaunchDraft,
    toLaunchSettings,
    type LaunchDraft,
} from "./LaunchOptionsCard";
import { RecommendationBox } from "./RecommendationBox";
import { useUpdateInstance } from "./useInstances";
import { isSelectionComplete, VersionFields } from "./VersionFields";

export type Draft = {
    name: string;
    minecraftVersion: string;
    loader: Instance["loader"];
    javaPath: string;
    customMemory: boolean;
    minMemoryMb: number;
    maxMemoryMb: number;
    jvmArgs: string;
    customResolution: boolean;
    width: number;
    height: number;
    launch: LaunchDraft;
};

function toDraft(instance: Instance, defaults: { min: number; max: number }): Draft {
    return {
        name: instance.name,
        minecraftVersion: instance.minecraftVersion,
        loader: instance.loader,
        javaPath: instance.java.javaPath ?? "",
        customMemory: instance.java.maxMemoryMb !== null,
        minMemoryMb: instance.java.minMemoryMb ?? defaults.min,
        maxMemoryMb: instance.java.maxMemoryMb ?? defaults.max,
        jvmArgs: instance.java.extraJvmArgs.join(" "),
        customResolution: instance.resolution !== null,
        width: instance.resolution?.width ?? 1280,
        height: instance.resolution?.height ?? 720,
        launch: toLaunchDraft(instance.launch),
    };
}

function toUpdate(draft: Draft): InstanceUpdate {
    return {
        name: draft.name,
        minecraftVersion: draft.minecraftVersion,
        loader: draft.loader,
        java: {
            javaPath: draft.javaPath.trim() === "" ? null : draft.javaPath.trim(),
            minMemoryMb: draft.customMemory ? draft.minMemoryMb : null,
            maxMemoryMb: draft.customMemory ? draft.maxMemoryMb : null,
            extraJvmArgs: draft.jvmArgs.split(/\s+/).filter((arg) => arg !== ""),
        },
        resolution: draft.customResolution ? { width: draft.width, height: draft.height } : null,
        launch: toLaunchSettings(draft.launch),
    };
}

export function InstanceSettingsForm({ instance }: { instance: Instance }) {
    const t = useT();
    const settings = useSettings();
    const defaults = {
        min: settings.data?.defaultMinMemoryMb ?? 512,
        max: settings.data?.defaultMaxMemoryMb ?? 4096,
    };
    const [draft, setDraft] = useState(() => toDraft(instance, defaults));
    const [showSnapshots, setShowSnapshots] = useState(false);
    const update = useUpdateInstance(instance.id);
    const recommendation = useQuery({
        queryKey: ["recommendation", instance.id],
        queryFn: () => recommendSettings(instance.id),
    });
    const running = useGameStore((state) => instance.id in state.running);
    const dirty =
        JSON.stringify(toUpdate(draft)) !== JSON.stringify(toUpdate(toDraft(instance, defaults)));

    const set = <K extends keyof Draft>(key: K, value: Draft[K]) =>
        setDraft((current) => ({ ...current, [key]: value }));

    const onSubmit = (event: FormEvent) => {
        event.preventDefault();
        update.mutate(toUpdate(draft), {
            onSuccess: (saved) => {
                setDraft(toDraft(saved, defaults));
                toast.success(t.instances.settings.saved);
            },
            onError: (error) =>
                toast.error(t.instances.settings.saveFailed, { description: errorMessage(error) }),
        });
    };

    return (
        <form onSubmit={onSubmit} className="flex max-w-3xl flex-col gap-6">
            <Card>
                <CardHeader>
                    <CardTitle>{t.instances.settings.general}</CardTitle>
                </CardHeader>
                <CardContent className="flex flex-col gap-4">
                    <div className="grid gap-2">
                        <Label htmlFor="edit-name">{t.instances.settings.name}</Label>
                        <Input
                            id="edit-name"
                            value={draft.name}
                            maxLength={64}
                            onChange={(event) => set("name", event.target.value)}
                        />
                    </div>
                    <VersionFields
                        value={{ minecraftVersion: draft.minecraftVersion, loader: draft.loader }}
                        onChange={(selection) =>
                            setDraft((current) => ({ ...current, ...selection }))
                        }
                        showSnapshots={showSnapshots}
                        onShowSnapshotsChange={setShowSnapshots}
                    />
                </CardContent>
            </Card>

            <Card>
                <CardHeader>
                    <CardTitle>{t.instances.settings.java}</CardTitle>
                    <CardDescription>{t.instances.settings.javaDescription}</CardDescription>
                </CardHeader>
                <CardContent className="flex flex-col gap-4">
                    <RecommendationBox
                        instanceId={instance.id}
                        current={draft}
                        onApply={(recommendation) =>
                            setDraft((current) => {
                                const args = current.jvmArgs.split(/\s+/).filter(Boolean);
                                const missing = recommendation.jvmArgs.filter(
                                    (arg) => !args.includes(arg),
                                );
                                return {
                                    ...current,
                                    customMemory: true,
                                    minMemoryMb: recommendation.minMemoryMb,
                                    maxMemoryMb: recommendation.maxMemoryMb,
                                    jvmArgs: [...args, ...missing].join(" "),
                                };
                            })
                        }
                    />
                    <JavaPathField
                        value={draft.javaPath}
                        onChange={(value) => set("javaPath", value)}
                    />
                    <ToggleRow
                        id="custom-memory"
                        label={t.instances.settings.customMemory}
                        checked={draft.customMemory}
                        onChange={(checked) => set("customMemory", checked)}
                    />
                    {draft.customMemory && (
                        <div className="grid grid-cols-2 gap-4">
                            <MemorySelect
                                id="instance-min-memory"
                                label={t.instances.settings.minimum}
                                value={draft.minMemoryMb}
                                floorMb={512}
                                maxMb={draft.maxMemoryMb}
                                recommendedMb={recommendation.data?.minMemoryMb}
                                onChange={(value) => set("minMemoryMb", value)}
                            />
                            <MemorySelect
                                id="instance-max-memory"
                                label={t.instances.settings.maximum}
                                value={draft.maxMemoryMb}
                                recommendedMb={recommendation.data?.maxMemoryMb}
                                onChange={(value) =>
                                    setDraft((current) => ({
                                        ...current,
                                        maxMemoryMb: value,
                                        minMemoryMb: Math.min(current.minMemoryMb, value),
                                    }))
                                }
                            />
                        </div>
                    )}
                    <div className="grid gap-2">
                        <Label htmlFor="jvm-args">{t.instances.settings.jvmArgs}</Label>
                        <Input
                            id="jvm-args"
                            value={draft.jvmArgs}
                            placeholder="-XX:+UseG1GC"
                            spellCheck={false}
                            className="font-mono"
                            onChange={(event) => set("jvmArgs", event.target.value)}
                        />
                    </div>
                </CardContent>
            </Card>

            <Card>
                <CardHeader>
                    <CardTitle>{t.instances.settings.window}</CardTitle>
                </CardHeader>
                <CardContent className="flex flex-col gap-4">
                    <ToggleRow
                        id="custom-resolution"
                        label={t.instances.settings.customResolution}
                        checked={draft.customResolution}
                        onChange={(checked) => set("customResolution", checked)}
                    />
                    {draft.customResolution && (
                        <div className="grid grid-cols-2 gap-4">
                            <NumberInput
                                id="width"
                                label={t.instances.settings.width}
                                value={draft.width}
                                onChange={(value) => set("width", value)}
                            />
                            <NumberInput
                                id="height"
                                label={t.instances.settings.height}
                                value={draft.height}
                                onChange={(value) => set("height", value)}
                            />
                        </div>
                    )}
                </CardContent>
            </Card>

            <LaunchOptionsCard
                value={draft.launch}
                companionAvailable={draft.loader.type === "fabric" || draft.loader.type === "quilt"}
                onChange={(patch) =>
                    setDraft((current) => ({ ...current, launch: { ...current.launch, ...patch } }))
                }
            />

            <div className="flex items-center gap-3">
                <Button
                    type="submit"
                    disabled={
                        !dirty ||
                        running ||
                        update.isPending ||
                        !isSelectionComplete({
                            minecraftVersion: draft.minecraftVersion,
                            loader: draft.loader,
                        })
                    }
                >
                    {update.isPending ? t.common.saving : t.common.save}
                </Button>
                <Button
                    type="button"
                    variant="ghost"
                    disabled={!dirty || update.isPending}
                    onClick={() => setDraft(toDraft(instance, defaults))}
                >
                    {t.common.discard}
                </Button>
                {running && (
                    <p className="text-sm text-muted-foreground">
                        {t.instances.settings.stopToSave}
                    </p>
                )}
            </div>
        </form>
    );
}

function ToggleRow(props: {
    id: string;
    label: string;
    checked: boolean;
    onChange: (checked: boolean) => void;
}) {
    return (
        <div className="flex items-center justify-between gap-4">
            <Label htmlFor={props.id}>{props.label}</Label>
            <Switch id={props.id} checked={props.checked} onCheckedChange={props.onChange} />
        </div>
    );
}

function NumberInput(props: {
    id: string;
    label: string;
    value: number;
    step?: number;
    onChange: (value: number) => void;
}) {
    return (
        <div className="grid gap-2">
            <Label htmlFor={props.id}>{props.label}</Label>
            <Input
                id={props.id}
                type="number"
                inputMode="numeric"
                className="tabular-nums"
                step={props.step}
                value={Number.isNaN(props.value) ? "" : props.value}
                onChange={(event) => props.onChange(event.target.valueAsNumber)}
            />
        </div>
    );
}

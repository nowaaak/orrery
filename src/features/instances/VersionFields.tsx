import { ExternalLinkIcon } from "lucide-react";
import { useEffect } from "react";
import { errorMessage } from "@/api/invoke";
import type { LoaderSpec } from "@/bindings/LoaderSpec";
import { Button } from "@/components/ui/button";
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
import type { ModLoader } from "@/bindings/ModLoader";
import { useT } from "@/i18n";
import { openExternal } from "@/lib/external";
import { useLoaderVersions, useMinecraftVersions } from "./useInstances";

export type VersionSelection = {
    minecraftVersion: string;
    loader: LoaderSpec;
};

type VersionFieldsProps = {
    value: VersionSelection;
    onChange: (value: VersionSelection) => void;
    showSnapshots: boolean;
    onShowSnapshotsChange: (value: boolean) => void;
};

const LOADER_NAMES: Record<ModLoader, string> = {
    fabric: "Fabric",
    legacy_fabric: "Legacy Fabric",
    quilt: "Quilt",
    forge: "Forge",
    neo_forge: "NeoForge",
};
const FORGE_PAGE = "https://files.minecraftforge.net/net/minecraftforge/forge/";
const FORGE_PATREON = "https://www.patreon.com/LexManos/";

function modLoaderOf(loader: LoaderSpec): ModLoader | null {
    return loader.type === "vanilla" ? null : loader.type;
}

export function isSelectionComplete(value: VersionSelection): boolean {
    if (value.minecraftVersion === "") {
        return false;
    }
    return value.loader.type === "vanilla" || value.loader.loaderVersion !== "";
}

export function VersionFields({
    value,
    onChange,
    showSnapshots,
    onShowSnapshotsChange,
}: VersionFieldsProps) {
    const t = useT().instances.versions;
    const versions = useMinecraftVersions(showSnapshots);
    const modLoader = modLoaderOf(value.loader);
    const loaderVersions = useLoaderVersions(modLoader, value.minecraftVersion);

    useEffect(() => {
        if (value.minecraftVersion === "" && versions.data) {
            onChange({ ...value, minecraftVersion: versions.data.latestRelease });
        }
    }, [versions.data, value, onChange]);

    useEffect(() => {
        if (value.loader.type === "vanilla" || !loaderVersions.data) {
            return;
        }
        const current = value.loader.loaderVersion;
        if (!loaderVersions.data.some((loader) => loader.version === current)) {
            const preferred =
                loaderVersions.data.find((loader) => loader.stable) ?? loaderVersions.data[0];
            onChange({
                ...value,
                loader: { type: value.loader.type, loaderVersion: preferred?.version ?? "" },
            });
        }
    }, [loaderVersions.data, value, onChange]);

    const knownVersion =
        versions.data?.versions.some((v) => v.id === value.minecraftVersion) ?? true;

    return (
        <div className="flex flex-col gap-4">
            <div className="grid gap-2">
                <div className="flex items-center justify-between gap-2">
                    <Label htmlFor="mc-version">{t.minecraftVersion}</Label>
                    <label className="flex items-center gap-2 text-xs text-muted-foreground">
                        <Switch
                            size="sm"
                            checked={showSnapshots}
                            onCheckedChange={onShowSnapshotsChange}
                        />
                        {t.snapshots}
                    </label>
                </div>
                {versions.data ? (
                    <Select
                        value={value.minecraftVersion}
                        onValueChange={(minecraftVersion) =>
                            onChange({ ...value, minecraftVersion })
                        }
                    >
                        <SelectTrigger id="mc-version" className="w-full">
                            <SelectValue placeholder={t.placeholder} />
                        </SelectTrigger>
                        <SelectContent className="max-h-80">
                            {!knownVersion && (
                                <SelectItem value={value.minecraftVersion}>
                                    {value.minecraftVersion}
                                </SelectItem>
                            )}
                            {versions.data.versions.map((version) => (
                                <SelectItem key={version.id} value={version.id}>
                                    {version.id}
                                    {version.kind === "snapshot" && (
                                        <span className="text-muted-foreground"> {t.snapshot}</span>
                                    )}
                                </SelectItem>
                            ))}
                        </SelectContent>
                    </Select>
                ) : versions.error ? (
                    <p className="flex min-h-8 items-center text-sm text-destructive select-text">
                        {errorMessage(versions.error)}
                    </p>
                ) : (
                    <Skeleton className="h-8 w-full" />
                )}
            </div>

            <div className="grid grid-cols-2 gap-4">
                <div className="grid gap-2">
                    <Label htmlFor="loader">{t.modLoader}</Label>
                    <Select
                        value={value.loader.type}
                        onValueChange={(type) =>
                            onChange({
                                ...value,
                                loader:
                                    type === "fabric" ||
                                    type === "legacy_fabric" ||
                                    type === "quilt" ||
                                    type === "forge" ||
                                    type === "neo_forge"
                                        ? { type, loaderVersion: "" }
                                        : { type: "vanilla" },
                            })
                        }
                    >
                        <SelectTrigger id="loader" className="w-full">
                            <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                            <SelectItem value="vanilla">Vanilla</SelectItem>
                            <SelectItem value="fabric">Fabric</SelectItem>
                            <SelectItem value="legacy_fabric">Legacy Fabric</SelectItem>
                            <SelectItem value="quilt">Quilt</SelectItem>
                            <SelectItem value="forge">Forge</SelectItem>
                            <SelectItem value="neo_forge">NeoForge</SelectItem>
                        </SelectContent>
                    </Select>
                </div>

                {value.loader.type !== "vanilla" && modLoader && (
                    <div className="grid gap-2">
                        <Label htmlFor="loader-version">
                            {t.loaderVersion(LOADER_NAMES[modLoader])}
                        </Label>
                        {loaderVersions.data && loaderVersions.data.length > 0 ? (
                            <Select
                                value={value.loader.loaderVersion}
                                onValueChange={(loaderVersion) =>
                                    onChange({
                                        ...value,
                                        loader: { type: modLoader, loaderVersion },
                                    })
                                }
                            >
                                <SelectTrigger id="loader-version" className="w-full">
                                    <SelectValue />
                                </SelectTrigger>
                                <SelectContent className="max-h-80">
                                    {loaderVersions.data.map((loader) => (
                                        <SelectItem key={loader.version} value={loader.version}>
                                            {loader.version}
                                            {!loader.stable && (
                                                <span className="text-muted-foreground">
                                                    {" "}
                                                    {t.beta}
                                                </span>
                                            )}
                                        </SelectItem>
                                    ))}
                                </SelectContent>
                            </Select>
                        ) : loaderVersions.isLoading ? (
                            <Skeleton className="h-8 w-full" />
                        ) : (
                            <p className="flex h-8 items-center text-sm text-destructive">
                                {t.unsupported(LOADER_NAMES[modLoader])}
                            </p>
                        )}
                    </div>
                )}
            </div>

            {value.loader.type === "forge" && (
                <div className="flex flex-col gap-1 text-xs text-muted-foreground">
                    <p>{t.forgeNotice}</p>
                    <div className="flex gap-4">
                        <Button
                            type="button"
                            variant="link"
                            className="h-auto p-0 text-xs"
                            onClick={() => openExternal(FORGE_PAGE)}
                        >
                            <ExternalLinkIcon data-icon="inline-start" />
                            {t.forgeWebsite}
                        </Button>
                        <Button
                            type="button"
                            variant="link"
                            className="h-auto p-0 text-xs"
                            onClick={() => openExternal(FORGE_PATREON)}
                        >
                            <ExternalLinkIcon data-icon="inline-start" />
                            {t.forgePatreon}
                        </Button>
                    </div>
                </div>
            )}
        </div>
    );
}

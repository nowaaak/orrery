import { PlusIcon } from "lucide-react";
import { useState, type FormEvent } from "react";
import { toast } from "sonner";
import { errorMessage } from "@/api/invoke";
import { useUiStore } from "@/app/uiStore";
import { installContentPack } from "@/api/mods";
import { Button } from "@/components/ui/button";
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { notifyPackResult } from "@/features/mods/ContentPacks";
import { useSettings } from "@/features/settings/useSettings";
import { useT } from "@/i18n";
import { loaderLabel } from "./labels";
import { useCreateInstance, useInstances } from "./useInstances";
import { isSelectionComplete, VersionFields, type VersionSelection } from "./VersionFields";

const EMPTY_SELECTION: VersionSelection = { minecraftVersion: "", loader: { type: "vanilla" } };
const NO_SETTINGS_SOURCE = "none";

export function CreateInstanceButton() {
    const t = useT();
    const openDialog = useUiStore((state) => state.openDialog);

    return (
        <Button onClick={() => openDialog("create-instance")}>
            <PlusIcon data-icon="inline-start" />
            {t.instances.create.button}
        </Button>
    );
}

export function CreateInstanceDialog() {
    const open = useUiStore((state) => state.dialog === "create-instance");
    const closeDialog = useUiStore((state) => state.closeDialog);

    return (
        <Dialog open={open} onOpenChange={(next) => !next && closeDialog()}>
            <DialogContent>{open && <CreateInstanceForm onDone={closeDialog} />}</DialogContent>
        </Dialog>
    );
}

function CreateInstanceForm({ onDone }: { onDone: () => void }) {
    const t = useT();
    const settings = useSettings();
    const [name, setName] = useState("");
    const [selection, setSelection] = useState(EMPTY_SELECTION);
    const [showSnapshots, setShowSnapshots] = useState(settings.data?.showSnapshots ?? false);
    const [performancePack, setPerformancePack] = useState(true);
    const modded = selection.loader.type !== "vanilla";
    const create = useCreateInstance();
    const instances = useInstances().data ?? [];
    const [settingsSource, setSettingsSource] = useState<string | null>(null);
    const lastPlayed = instances.find((instance) => instance.lastPlayedAt !== null);
    const copySettingsFrom = settingsSource ?? lastPlayed?.id ?? NO_SETTINGS_SOURCE;

    const suggestedName =
        selection.minecraftVersion === ""
            ? t.instances.create.defaultName
            : selection.loader.type === "vanilla"
              ? selection.minecraftVersion
              : `${selection.minecraftVersion} ${loaderLabel(selection).split(" ")[0]}`;
    const finalName = name.trim() === "" ? suggestedName : name.trim();

    const onSubmit = (event: FormEvent) => {
        event.preventDefault();
        create.mutate(
            {
                name: finalName,
                ...selection,
                copySettingsFrom: copySettingsFrom === NO_SETTINGS_SOURCE ? null : copySettingsFrom,
            },
            {
                onSuccess: (instance) => {
                    toast.success(t.instances.created(instance.name));
                    onDone();
                    if (modded && performancePack) {
                        installContentPack(instance.id, "performance").then(
                            (result) => notifyPackResult("performance", result),
                            (error) =>
                                toast.error(t.instances.create.performancePackFailed, {
                                    description: errorMessage(error),
                                }),
                        );
                    }
                },
                onError: (error) =>
                    toast.error(t.instances.create.failed, {
                        description: errorMessage(error),
                    }),
            },
        );
    };

    return (
        <form onSubmit={onSubmit} className="flex flex-col gap-5">
            <DialogHeader>
                <DialogTitle>{t.instances.create.title}</DialogTitle>
                <DialogDescription>{t.instances.create.description}</DialogDescription>
            </DialogHeader>
            <div className="grid gap-2">
                <Label htmlFor="instance-name">{t.instances.create.name}</Label>
                <Input
                    id="instance-name"
                    value={name}
                    placeholder={suggestedName}
                    maxLength={64}
                    onChange={(event) => setName(event.target.value)}
                />
            </div>
            <VersionFields
                value={selection}
                onChange={setSelection}
                showSnapshots={showSnapshots}
                onShowSnapshotsChange={setShowSnapshots}
            />
            {instances.length > 0 && (
                <div className="grid gap-2">
                    <Label htmlFor="copy-settings">{t.instances.create.copySettings}</Label>
                    <Select value={copySettingsFrom} onValueChange={setSettingsSource}>
                        <SelectTrigger
                            id="copy-settings"
                            className="w-full"
                            aria-describedby="copy-settings-hint"
                        >
                            <SelectValue />
                        </SelectTrigger>
                        <SelectContent className="max-h-80">
                            <SelectItem value={NO_SETTINGS_SOURCE}>
                                {t.instances.create.copySettingsNone}
                            </SelectItem>
                            {instances.map((instance) => (
                                <SelectItem key={instance.id} value={instance.id}>
                                    {instance.name}
                                    <span className="text-muted-foreground">
                                        {" "}
                                        {instance.minecraftVersion}
                                    </span>
                                </SelectItem>
                            ))}
                        </SelectContent>
                    </Select>
                    <p id="copy-settings-hint" className="text-sm text-muted-foreground">
                        {t.instances.create.copySettingsHint}
                    </p>
                </div>
            )}
            <div className="flex items-center justify-between gap-4">
                <div>
                    <Label htmlFor="performance-pack" className={modded ? "" : "opacity-50"}>
                        {t.instances.create.performancePack}
                    </Label>
                    <p className="text-sm text-muted-foreground">
                        {modded
                            ? t.instances.create.performancePackHint
                            : t.instances.create.performancePackUnavailable}
                    </p>
                </div>
                <Switch
                    id="performance-pack"
                    checked={modded && performancePack}
                    disabled={!modded}
                    onCheckedChange={setPerformancePack}
                />
            </div>
            <DialogFooter>
                <Button type="button" variant="ghost" onClick={onDone}>
                    {t.common.cancel}
                </Button>
                <Button
                    type="submit"
                    disabled={!isSelectionComplete(selection) || create.isPending}
                >
                    {create.isPending ? t.instances.create.submitting : t.instances.create.submit}
                </Button>
            </DialogFooter>
        </form>
    );
}

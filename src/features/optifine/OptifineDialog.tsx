import { useMutation, useQueryClient } from "@tanstack/react-query";
import { ExternalLinkIcon } from "lucide-react";
import { useState } from "react";
import { useNavigate } from "react-router";
import { toast } from "sonner";
import { create } from "zustand";
import { createInstance, deleteInstance } from "@/api/instances";
import { errorMessage } from "@/api/invoke";
import { installLocalFile } from "@/api/mods";
import { useUiStore } from "@/app/uiStore";
import type { OptifineJar } from "@/bindings/OptifineJar";
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
import { Skeleton } from "@/components/ui/skeleton";
import { instancesKey, useInstances, useLoaderVersions } from "@/features/instances/useInstances";
import { installedModsKey } from "@/features/mods/useMods";
import { getT, useT } from "@/i18n";
import { openExternal } from "@/lib/external";

const DOWNLOAD_PAGE = "https://optifine.net/downloads";
const NEW_INSTANCE = "new";

type OptifineState = {
    path: string;
    jar: OptifineJar | null;
    show: (path: string, jar: OptifineJar) => void;
};

const useOptifineStore = create<OptifineState>((set) => ({
    path: "",
    jar: null,
    show: (path, jar) => set({ path, jar }),
}));

export function openOptifine(path: string, jar: OptifineJar) {
    useOptifineStore.getState().show(path, jar);
    useUiStore.getState().openDialog("optifine");
}

export function optifineLabel(jar: OptifineJar): string {
    const label = `${jar.edition.replaceAll("_", " ")} ${jar.release}`;
    return jar.preview ? `${label} preview` : label;
}

export function OptifineDialog() {
    const isOpen = useUiStore((state) => state.dialog === "optifine");
    const closeDialog = useUiStore((state) => state.closeDialog);
    const path = useOptifineStore((state) => state.path);
    const jar = useOptifineStore((state) => state.jar);
    return (
        <Dialog open={isOpen} onOpenChange={(next) => !next && closeDialog()}>
            <DialogContent className="sm:max-w-lg">
                {isOpen && jar && <OptifineForm path={path} jar={jar} onDone={closeDialog} />}
            </DialogContent>
        </Dialog>
    );
}

async function install(
    path: string,
    jar: OptifineJar,
    target: string,
    name: string,
    forgeVersion: string,
): Promise<string> {
    if (target !== NEW_INSTANCE) {
        await installLocalFile(target, path);
        return target;
    }
    const instance = await createInstance({
        name,
        minecraftVersion: jar.minecraftVersion,
        loader: { type: "forge", loaderVersion: forgeVersion },
        copySettingsFrom: null,
    });
    try {
        await installLocalFile(instance.id, path);
    } catch (error) {
        await deleteInstance(instance.id).catch(() => undefined);
        throw error;
    }
    return instance.id;
}

function OptifineForm({
    path,
    jar,
    onDone,
}: {
    path: string;
    jar: OptifineJar;
    onDone: () => void;
}) {
    const t = useT().optifine;
    const queryClient = useQueryClient();
    const navigate = useNavigate();
    const instances = useInstances();
    const forgeVersions = useLoaderVersions("forge", jar.minecraftVersion);
    const matching = (instances.data ?? []).filter(
        (instance) =>
            instance.loader.type === "forge" && instance.minecraftVersion === jar.minecraftVersion,
    );
    const [target, setTarget] = useState(NEW_INSTANCE);
    const [name, setName] = useState(() => t.defaultName(jar.minecraftVersion));
    const [pickedForge, setForgeVersion] = useState<string | null>(null);
    const recommended = forgeVersions.data?.find(
        (version) => version.version === jar.forgeVersion,
    )?.version;
    const forgeVersion = pickedForge ?? recommended ?? forgeVersions.data?.[0]?.version ?? "";

    const run = useMutation({
        mutationFn: () => install(path, jar, target, name.trim(), forgeVersion),
        onSuccess: async (instanceId) => {
            const title =
                target === NEW_INSTANCE
                    ? name.trim()
                    : (matching.find((item) => item.id === target)?.name ?? "");
            toast.success(getT().optifine.done(title));
            await queryClient.invalidateQueries({ queryKey: instancesKey });
            await queryClient.invalidateQueries({ queryKey: installedModsKey(instanceId) });
            onDone();
            void navigate(`/instances/${instanceId}`);
        },
        onError: (error: unknown) =>
            toast.error(getT().optifine.failed, { description: errorMessage(error) }),
    });

    const creating = target === NEW_INSTANCE;
    const noForge = forgeVersions.isSuccess && forgeVersions.data.length === 0;
    const blocked =
        run.isPending || (creating && (name.trim() === "" || forgeVersion === "" || noForge));

    return (
        <div className="flex min-w-0 flex-col gap-4">
            <DialogHeader>
                <DialogTitle>{t.title}</DialogTitle>
                <DialogDescription>
                    {t.description(optifineLabel(jar), jar.minecraftVersion)}
                </DialogDescription>
            </DialogHeader>

            <div className="grid gap-2">
                <Label htmlFor="optifine-target">{t.target}</Label>
                <Select value={target} onValueChange={setTarget}>
                    <SelectTrigger id="optifine-target" className="w-full">
                        <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                        <SelectItem value={NEW_INSTANCE}>{t.newInstance}</SelectItem>
                        {matching.map((instance) => (
                            <SelectItem key={instance.id} value={instance.id}>
                                {instance.name}
                            </SelectItem>
                        ))}
                    </SelectContent>
                </Select>
            </div>

            {creating && (
                <div className="grid grid-cols-2 gap-4">
                    <div className="grid gap-2">
                        <Label htmlFor="optifine-name">{t.name}</Label>
                        <Input
                            id="optifine-name"
                            value={name}
                            maxLength={64}
                            onChange={(event) => setName(event.target.value)}
                        />
                    </div>
                    <div className="grid gap-2">
                        <Label htmlFor="optifine-forge">{t.forgeVersion}</Label>
                        {forgeVersions.isPending ? (
                            <Skeleton className="h-8 w-full" />
                        ) : noForge || !forgeVersions.data ? (
                            <p className="text-sm text-destructive">{t.noForge}</p>
                        ) : (
                            <Select value={forgeVersion} onValueChange={setForgeVersion}>
                                <SelectTrigger id="optifine-forge" className="w-full">
                                    <SelectValue />
                                </SelectTrigger>
                                <SelectContent className="max-h-80">
                                    {forgeVersions.data.map((version) => (
                                        <SelectItem key={version.version} value={version.version}>
                                            {version.version}
                                        </SelectItem>
                                    ))}
                                </SelectContent>
                            </Select>
                        )}
                    </div>
                </div>
            )}

            <div className="flex flex-col gap-1 text-xs text-muted-foreground">
                <p>{recommended ? t.compatibilityKnown(recommended) : t.compatibility}</p>
                <Button
                    type="button"
                    variant="link"
                    className="h-auto self-start p-0 text-xs"
                    onClick={() => openExternal(DOWNLOAD_PAGE)}
                >
                    <ExternalLinkIcon data-icon="inline-start" />
                    {t.downloadPage}
                </Button>
            </div>

            <DialogFooter>
                <Button disabled={blocked} onClick={() => run.mutate()}>
                    {run.isPending ? t.installing : t.install}
                </Button>
            </DialogFooter>
        </div>
    );
}

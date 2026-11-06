import { useMutation, useQuery } from "@tanstack/react-query";
import { open } from "@tauri-apps/plugin-dialog";
import {
    ArrowLeftIcon,
    FileArchiveIcon,
    FileCodeIcon,
    FolderInputIcon,
    FolderOpenIcon,
    TriangleAlertIcon,
} from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { create } from "zustand";
import { inspectImport, previewImport, scanImports, startImport } from "@/api/imports";
import { inspectOptifine } from "@/api/optifine";
import { errorMessage } from "@/api/invoke";
import { useUiStore } from "@/app/uiStore";
import type { ForeignInstance } from "@/bindings/ForeignInstance";
import type { ForeignLoader } from "@/bindings/ForeignLoader";
import type { ImportWarning } from "@/bindings/ImportWarning";
import { Badge } from "@/components/ui/badge";
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
import { Skeleton } from "@/components/ui/skeleton";
import { getT, useT } from "@/i18n";
import { openOptifine } from "@/features/optifine/OptifineDialog";
import { formatBytes } from "@/lib/format";

type ImportState = {
    found: ForeignInstance[] | null;
    show: (found: ForeignInstance[]) => void;
    clear: () => void;
};

export const useImportStore = create<ImportState>((set) => ({
    found: null,
    show: (found) => set({ found }),
    clear: () => set({ found: null }),
}));

export function openImport(found: ForeignInstance[] | null) {
    if (found) {
        useImportStore.getState().show(found);
    } else {
        useImportStore.getState().clear();
    }
    useUiStore.getState().openDialog("import");
}

export function loaderLabel(loader: ForeignLoader): string {
    switch (loader.type) {
        case "vanilla":
            return "Vanilla";
        case "fabric":
            return `Fabric ${loader.version}`;
        case "quilt":
            return `Quilt ${loader.version}`;
        case "forge":
            return `Forge ${loader.version}`;
        case "neo_forge":
            return `NeoForge ${loader.version}`;
        case "other":
            return loader.name;
    }
}

export function warningText(warning: ImportWarning): string {
    const text = getT().imports.warnings;
    switch (warning.type) {
        case "loader_not_supported":
            return text.loaderNotSupported(loaderLabel(warning.loader));
        case "unknown_version":
            return text.unknownVersion;
        case "java_not_taken":
            return text.javaNotTaken(warning.path);
        case "jvm_args_not_taken":
            return text.jvmArgsNotTaken(warning.args);
        case "commands_not_taken":
            return text.commandsNotTaken(warning.commands.join(" · "));
        case "skipped_links":
            return text.skippedLinks(warning.count);
        case "skipped_unsafe":
            return text.skippedUnsafe(warning.count);
    }
}

export function ImportButton() {
    const t = useT();
    return (
        <Button variant="outline" onClick={() => openImport(null)}>
            <FolderInputIcon data-icon="inline-start" />
            {t.imports.button}
        </Button>
    );
}

export function ImportDialog() {
    const isOpen = useUiStore((state) => state.dialog === "import");
    const closeDialog = useUiStore((state) => state.closeDialog);
    const close = () => {
        closeDialog();
        useImportStore.getState().clear();
    };
    return (
        <Dialog open={isOpen} onOpenChange={(next) => !next && close()}>
            <DialogContent className="sm:max-w-xl">
                {isOpen && <ImportFlow onDone={close} />}
            </DialogContent>
        </Dialog>
    );
}

function instanceKey(instance: ForeignInstance): string {
    return `${instance.path}#${instance.profile ?? ""}`;
}

function ImportFlow({ onDone }: { onDone: () => void }) {
    const [selected, setSelected] = useState<ForeignInstance | null>(() => {
        const dropped = useImportStore.getState().found;
        return dropped?.length === 1 ? (dropped[0] ?? null) : null;
    });
    return selected ? (
        <ImportPreviewStep instance={selected} onBack={() => setSelected(null)} onDone={onDone} />
    ) : (
        <ImportList onSelect={setSelected} />
    );
}

function ImportList({ onSelect }: { onSelect: (instance: ForeignInstance) => void }) {
    const t = useT();
    const dropped = useImportStore((state) => state.found);
    const showFound = useImportStore((state) => state.show);
    const scan = useQuery({
        queryKey: ["import-scan"],
        queryFn: scanImports,
        enabled: dropped === null,
        staleTime: 30_000,
    });
    const inspect = useMutation({
        mutationFn: inspectImport,
        onSuccess: (found) => {
            if (found.length === 0) {
                toast.info(getT().imports.noneFound);
            } else if (found.length === 1 && found[0]) {
                onSelect(found[0]);
            } else {
                showFound(found);
            }
        },
        onError: (error: unknown) =>
            toast.error(getT().imports.inspectFailed, { description: errorMessage(error) }),
    });
    const list = dropped ?? scan.data ?? [];

    const chooseOptifine = async () => {
        const path = await open({
            multiple: false,
            filters: [{ name: t.optifine.jarFilter, extensions: ["jar"] }],
        });
        if (typeof path !== "string") return;
        try {
            const jar = await inspectOptifine(path);
            if (jar) {
                openOptifine(path, jar);
            } else {
                toast.error(getT().optifine.notOptifine);
            }
        } catch (error) {
            toast.error(getT().optifine.failed, { description: errorMessage(error) });
        }
    };

    const choose = async (directory: boolean) => {
        const path = await open({
            multiple: false,
            directory,
            filters: directory
                ? undefined
                : [{ name: t.imports.archiveFilter, extensions: ["zip"] }],
        });
        if (typeof path === "string") inspect.mutate(path);
    };

    return (
        <div className="flex min-w-0 flex-col gap-4">
            <DialogHeader>
                <DialogTitle>{t.imports.title}</DialogTitle>
                <DialogDescription>{t.imports.description}</DialogDescription>
            </DialogHeader>
            <div className="h-[20rem] overflow-y-auto pr-1">
                {dropped === null && scan.error ? (
                    <p className="text-sm text-destructive select-text">
                        {t.imports.scanFailed}: {errorMessage(scan.error)}
                    </p>
                ) : dropped === null && scan.isPending ? (
                    <div className="flex flex-col gap-2" aria-label={t.imports.scanning}>
                        {[0, 1, 2, 3].map((key) => (
                            <Skeleton key={key} className="h-14 w-full" />
                        ))}
                    </div>
                ) : list.length === 0 ? (
                    <p className="rounded-xl border border-dashed p-6 text-center text-sm text-muted-foreground">
                        {t.imports.empty}
                    </p>
                ) : (
                    <ul className="flex flex-col divide-y">
                        {list.map((instance) => (
                            <li key={instanceKey(instance)}>
                                <button
                                    type="button"
                                    onClick={() => onSelect(instance)}
                                    className="flex w-full items-center gap-3 rounded-lg px-2 py-2.5 text-left outline-none hover:bg-muted/60 focus-visible:ring-2 focus-visible:ring-ring"
                                >
                                    <div className="min-w-0 flex-1">
                                        <p className="truncate text-sm font-medium">
                                            {instance.name}
                                        </p>
                                        <p className="truncate text-xs text-muted-foreground select-text">
                                            {instance.gameDir}
                                        </p>
                                    </div>
                                    <div className="flex shrink-0 flex-col items-end gap-1">
                                        <Badge variant="secondary">
                                            {t.imports.sources[instance.source]}
                                        </Badge>
                                        <span className="text-xs text-muted-foreground tabular-nums">
                                            {instance.minecraftVersion ??
                                                (instance.latest
                                                    ? t.imports.latest(instance.latest)
                                                    : t.imports.unknownVersion)}{" "}
                                            · {loaderLabel(instance.loader)}
                                        </span>
                                    </div>
                                </button>
                            </li>
                        ))}
                    </ul>
                )}
            </div>
            <DialogFooter className="sm:justify-start">
                <Button
                    variant="outline"
                    disabled={inspect.isPending}
                    onClick={() => void choose(true)}
                >
                    <FolderOpenIcon data-icon="inline-start" />
                    {inspect.isPending ? t.imports.inspecting : t.imports.chooseFolder}
                </Button>
                <Button
                    variant="outline"
                    disabled={inspect.isPending}
                    onClick={() => void choose(false)}
                >
                    <FileArchiveIcon data-icon="inline-start" />
                    {t.imports.chooseArchive}
                </Button>
                <Button variant="ghost" onClick={() => void chooseOptifine()}>
                    <FileCodeIcon data-icon="inline-start" />
                    {t.optifine.chooseJar}
                </Button>
            </DialogFooter>
        </div>
    );
}

function ImportPreviewStep({
    instance,
    onBack,
    onDone,
}: {
    instance: ForeignInstance;
    onBack: () => void;
    onDone: () => void;
}) {
    const t = useT();
    const [name, setName] = useState(instance.name);
    const target = { path: instance.path, profile: instance.profile };
    const preview = useQuery({
        queryKey: ["import-preview", instance.path, instance.profile],
        queryFn: () => previewImport(target),
        staleTime: 0,
        gcTime: 0,
    });
    const start = useMutation({
        mutationFn: () => startImport(target, name.trim() || null),
        onSuccess: () => {
            const text = getT().imports;
            toast.success(text.started(name.trim() || instance.name), {
                description: text.progressHint,
            });
            onDone();
        },
        onError: (error: unknown) =>
            toast.error(getT().imports.startFailed, { description: errorMessage(error) }),
    });
    const data = preview.data;
    const blocked = !data || data.minecraftVersion === null || start.isPending;

    return (
        <div className="flex min-w-0 flex-col gap-4">
            <DialogHeader>
                <DialogTitle>{instance.name}</DialogTitle>
                <DialogDescription className="break-all select-text">
                    {t.imports.sources[instance.source]} · {instance.gameDir}
                </DialogDescription>
            </DialogHeader>
            <div className="flex flex-col gap-2">
                <Label htmlFor="import-name">{t.imports.name}</Label>
                <Input
                    id="import-name"
                    value={name}
                    maxLength={64}
                    onChange={(event) => setName(event.target.value)}
                />
            </div>
            <div className="h-[16rem] overflow-y-auto pr-1">
                {preview.error ? (
                    <p className="text-sm text-destructive select-text">
                        {t.imports.previewFailed}: {errorMessage(preview.error)}
                    </p>
                ) : !data ? (
                    <div className="flex flex-col gap-2" aria-label={t.imports.previewLoading}>
                        {[0, 1, 2].map((key) => (
                            <Skeleton key={key} className="h-8 w-full" />
                        ))}
                    </div>
                ) : (
                    <div className="flex flex-col gap-3">
                        <div className="flex flex-wrap gap-2">
                            <Badge variant="secondary">
                                {data.minecraftVersion ?? t.imports.unknownVersion}
                            </Badge>
                            <Badge variant="secondary">{loaderLabel(instance.loader)}</Badge>
                            <Badge variant="outline" className="tabular-nums">
                                {t.imports.copies(data.files, formatBytes(data.bytes))}
                            </Badge>
                        </div>
                        {data.warnings.length > 0 && (
                            <ul className="flex flex-col gap-1.5">
                                {data.warnings.map((warning, index) => (
                                    <li
                                        key={`${warning.type}-${index}`}
                                        className="flex gap-2 rounded-lg bg-amber-500/10 px-3 py-2 text-xs text-amber-800 select-text dark:text-amber-200"
                                    >
                                        <TriangleAlertIcon
                                            className="mt-0.5 size-3.5 shrink-0"
                                            aria-hidden
                                        />
                                        {warningText(warning)}
                                    </li>
                                ))}
                            </ul>
                        )}
                        {data.folders.length === 0 ? (
                            <p className="text-sm text-muted-foreground">
                                {t.imports.nothingToCopy}
                            </p>
                        ) : (
                            <ul className="flex flex-col divide-y text-sm">
                                {data.folders.map((folder) => (
                                    <li
                                        key={folder.name || "-"}
                                        className="flex items-center justify-between gap-3 py-1.5"
                                    >
                                        <span className="truncate font-mono text-xs">
                                            {folder.name || t.imports.rootFiles}
                                        </span>
                                        <span className="shrink-0 text-xs text-muted-foreground tabular-nums">
                                            {t.imports.copies(
                                                folder.files,
                                                formatBytes(folder.bytes),
                                            )}
                                        </span>
                                    </li>
                                ))}
                            </ul>
                        )}
                    </div>
                )}
            </div>
            <DialogFooter>
                <Button variant="ghost" onClick={onBack}>
                    <ArrowLeftIcon data-icon="inline-start" />
                    {t.imports.back}
                </Button>
                <Button disabled={blocked} onClick={() => start.mutate()}>
                    {start.isPending ? t.imports.importing : t.imports.import}
                </Button>
            </DialogFooter>
        </div>
    );
}

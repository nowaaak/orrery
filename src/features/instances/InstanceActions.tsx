import { useQueryClient } from "@tanstack/react-query";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
    CopyIcon,
    EllipsisVerticalIcon,
    FileArchiveIcon,
    FolderOpenIcon,
    FolderTreeIcon,
    MonitorUpIcon,
    ImageIcon,
    ImageOffIcon,
    PinIcon,
    PinOffIcon,
    SettingsIcon,
    Trash2Icon,
    WrenchIcon,
} from "lucide-react";
import { useState } from "react";
import { useNavigate } from "react-router";
import { toast } from "sonner";
import { openInstanceFolder } from "@/api/instances";
import { errorMessage } from "@/api/invoke";
import { repairInstance } from "@/api/launch";
import { exportModpack } from "@/api/modpacks";
import type { Instance } from "@/bindings/Instance";
import { Button } from "@/components/ui/button";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuSeparator,
    DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useGameStore } from "@/features/launch/gameStore";
import { useCurseForgeAvailable } from "@/features/mods/useMods";
import { useT } from "@/i18n";
import { showTrashUndo } from "@/features/trash/undo";
import { createShortcutWithToast } from "@/features/links/shortcuts";
import { GroupDialog } from "./GroupDialog";
import {
    instancesKey,
    useDeleteInstance,
    useDuplicateInstance,
    useSetInstanceIcon,
    useSetInstancePinned,
} from "./useInstances";

const ICON_EXTENSIONS = ["png", "jpg", "jpeg", "gif", "webp"];

function reportError(title: string) {
    return (error: unknown) => toast.error(title, { description: errorMessage(error) });
}

export function InstanceActions({ instance }: { instance: Instance }) {
    const t = useT();
    const navigate = useNavigate();
    const running = useGameStore((state) => instance.id in state.running);
    const duplicate = useDuplicateInstance();
    const remove = useDeleteInstance();
    const queryClient = useQueryClient();
    const [exporting, setExporting] = useState(false);
    const curseforge = useCurseForgeAvailable().data === true;
    const [editingGroup, setEditingGroup] = useState(false);
    const setPinned = useSetInstancePinned();
    const setIcon = useSetInstanceIcon();

    const onPin = () =>
        setPinned.mutate(
            { id: instance.id, pinned: !instance.pinned },
            { onError: reportError(t.instances.actions.pinFailed) },
        );

    const onChangeIcon = async () => {
        const path = await open({
            multiple: false,
            directory: false,
            filters: [{ name: t.instances.actions.iconFilter, extensions: ICON_EXTENSIONS }],
        });
        if (typeof path === "string") {
            setIcon.mutate(
                { id: instance.id, path },
                { onError: reportError(t.instances.actions.iconFailed) },
            );
        }
    };

    const onExport = async () => {
        const path = await save({
            defaultPath: `${instance.name}.mrpack`,
            filters: [
                { name: t.instances.actions.exportFilter, extensions: ["mrpack"] },
                ...(curseforge
                    ? [{ name: t.instances.actions.exportCurseForgeFilter, extensions: ["zip"] }]
                    : []),
            ],
        });
        if (!path) {
            return;
        }
        const source = path.toLowerCase().endsWith(".zip") ? "CurseForge" : "Modrinth";
        setExporting(true);
        const pending = toast.loading(t.instances.actions.exporting);
        try {
            const result = await exportModpack(instance.id, path);
            const details = [
                t.instances.actions.exportLinked(result.linked, result.overrideFiles, source),
                result.skipped.length > 0 &&
                    t.instances.actions.exportSkipped(result.skipped.join(", "), source),
            ]
                .filter(Boolean)
                .join(" ");
            toast.success(t.instances.actions.exported, { id: pending, description: details });
        } catch (error) {
            toast.error(t.instances.actions.exportFailed, {
                id: pending,
                description: errorMessage(error),
            });
        } finally {
            setExporting(false);
        }
    };

    const onDuplicate = () =>
        duplicate.mutate(
            { id: instance.id, name: t.instances.actions.duplicateName(instance.name) },
            {
                onSuccess: (copy) => toast.success(t.instances.created(copy.name)),
                onError: reportError(t.instances.actions.duplicateFailed),
            },
        );

    const onDelete = () =>
        remove.mutate(instance.id, {
            onSuccess: (entry) => {
                void navigate("/");
                showTrashUndo(entry, (restored) => {
                    void queryClient.invalidateQueries({ queryKey: instancesKey });
                    void navigate(`/instances/${restored.instanceId}`);
                });
            },
            onError: reportError(t.instances.actions.deleteFailed),
        });

    return (
        <>
            <DropdownMenu>
                <DropdownMenuTrigger asChild>
                    <Button
                        variant="ghost"
                        size="icon"
                        aria-label={t.instances.actions.menu(instance.name)}
                    >
                        <EllipsisVerticalIcon />
                    </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end" className="w-60">
                    <DropdownMenuItem onSelect={() => void navigate(`/instances/${instance.id}`)}>
                        <SettingsIcon />
                        {t.instances.actions.details}
                    </DropdownMenuItem>
                    <DropdownMenuItem disabled={setPinned.isPending} onSelect={onPin}>
                        {instance.pinned ? <PinOffIcon /> : <PinIcon />}
                        {instance.pinned ? t.instances.actions.unpin : t.instances.actions.pin}
                    </DropdownMenuItem>
                    <DropdownMenuItem onSelect={() => setEditingGroup(true)}>
                        <FolderTreeIcon />
                        {t.instances.actions.group}
                    </DropdownMenuItem>
                    <DropdownMenuItem
                        disabled={setIcon.isPending}
                        onSelect={() => void onChangeIcon()}
                    >
                        <ImageIcon />
                        {t.instances.actions.changeIcon}
                    </DropdownMenuItem>
                    {instance.icon && (
                        <DropdownMenuItem
                            disabled={setIcon.isPending}
                            onSelect={() =>
                                setIcon.mutate(
                                    { id: instance.id, path: null },
                                    { onError: reportError(t.instances.actions.iconFailed) },
                                )
                            }
                        >
                            <ImageOffIcon />
                            {t.instances.actions.removeIcon}
                        </DropdownMenuItem>
                    )}
                    <DropdownMenuSeparator />
                    <DropdownMenuItem
                        onSelect={() =>
                            openInstanceFolder(instance.id).catch(
                                reportError(t.common.openFolderFailed),
                            )
                        }
                    >
                        <FolderOpenIcon />
                        {t.instances.actions.openFolder}
                    </DropdownMenuItem>
                    <DropdownMenuItem
                        disabled={running}
                        onSelect={() =>
                            repairInstance(instance.id).catch(
                                reportError(t.instances.actions.repairFailed),
                            )
                        }
                    >
                        <WrenchIcon />
                        {t.instances.actions.repair}
                    </DropdownMenuItem>
                    <DropdownMenuItem
                        disabled={running || duplicate.isPending}
                        onSelect={onDuplicate}
                    >
                        <CopyIcon />
                        {t.instances.actions.duplicate}
                    </DropdownMenuItem>
                    <DropdownMenuItem onSelect={() => void createShortcutWithToast(instance.id)}>
                        <MonitorUpIcon />
                        {t.links.createShortcut}
                    </DropdownMenuItem>
                    <DropdownMenuItem disabled={exporting} onSelect={() => void onExport()}>
                        <FileArchiveIcon />
                        {t.instances.actions.export}
                    </DropdownMenuItem>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem variant="destructive" disabled={running} onSelect={onDelete}>
                        <Trash2Icon />
                        {t.common.delete}
                    </DropdownMenuItem>
                </DropdownMenuContent>
            </DropdownMenu>

            <GroupDialog instance={instance} open={editingGroup} onOpenChange={setEditingGroup} />
        </>
    );
}

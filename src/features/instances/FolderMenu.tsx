import {
    CameraIcon,
    ChevronDownIcon,
    FileWarningIcon,
    FolderIcon,
    FolderOpenIcon,
    GlobeIcon,
    PackageIcon,
    PaletteIcon,
    ScrollTextIcon,
    SlidersHorizontalIcon,
    SparklesIcon,
    type LucideIcon,
} from "lucide-react";
import { toast } from "sonner";
import { openInstanceFolder } from "@/api/instances";
import { errorMessage } from "@/api/invoke";
import type { Instance } from "@/bindings/Instance";
import type { InstanceFolder } from "@/bindings/InstanceFolder";
import { Button } from "@/components/ui/button";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuSeparator,
    DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useT } from "@/i18n";

type Entry = { folder: InstanceFolder; icon: LucideIcon; modded?: boolean };

const ENTRIES: Entry[] = [
    { folder: "saves", icon: GlobeIcon },
    { folder: "screenshots", icon: CameraIcon },
    { folder: "mods", icon: PackageIcon, modded: true },
    { folder: "resourcepacks", icon: PaletteIcon },
    { folder: "shaderpacks", icon: SparklesIcon, modded: true },
    { folder: "config", icon: SlidersHorizontalIcon },
    { folder: "logs", icon: ScrollTextIcon },
    { folder: "crash_reports", icon: FileWarningIcon },
];

export function FolderMenu({ instance }: { instance: Instance }) {
    const t = useT();
    const modded = instance.loader.type !== "vanilla";
    const open = (folder: InstanceFolder) =>
        openInstanceFolder(instance.id, folder).catch((error: unknown) =>
            toast.error(t.common.openFolderFailed, {
                description: errorMessage(error),
            }),
        );

    return (
        <DropdownMenu>
            <DropdownMenuTrigger asChild>
                <Button variant="outline" size="lg">
                    <FolderIcon data-icon="inline-start" />
                    {t.instances.folderMenu}
                    <ChevronDownIcon data-icon="inline-end" />
                </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="w-56">
                <DropdownMenuItem onSelect={() => void open("root")}>
                    <FolderOpenIcon />
                    {t.instances.folders.root}
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                {ENTRIES.filter((entry) => !entry.modded || modded).map((entry) => (
                    <DropdownMenuItem key={entry.folder} onSelect={() => void open(entry.folder)}>
                        <entry.icon />
                        {t.instances.folders[entry.folder]}
                    </DropdownMenuItem>
                ))}
            </DropdownMenuContent>
        </DropdownMenu>
    );
}

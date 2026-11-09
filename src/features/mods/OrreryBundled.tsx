import { useState } from "react";
import { toast } from "sonner";
import { errorMessage } from "@/api/invoke";
import { installOrreryMenu } from "@/api/mods";
import { OrreryLogo } from "@/components/brand/OrreryLogo";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useGameStore } from "@/features/launch/gameStore";
import { isContentTask } from "@/features/tasks/labels";
import { useTaskStore } from "@/features/tasks/taskStore";
import { getT, useT } from "@/i18n";
import type { Messages } from "@/i18n";
import { useInstalledMods } from "./useMods";

type BundledText = { description: string; install: string; failed: string };

type OrreryBundle = {
    id: string;
    title: string;
    install: (instanceId: string) => Promise<number>;
    text: (t: Messages) => BundledText;
};

export const ORRERY_MENU: OrreryBundle = {
    id: "orrery-menu",
    title: "Orrery Menu",
    install: installOrreryMenu,
    text: (t) => t.content.orreryMenu,
};

export function orreryBundle(projectId: string): OrreryBundle | undefined {
    return [ORRERY_MENU].find((bundle) => bundle.id === projectId);
}

export async function installBundleWithToast(bundle: OrreryBundle, instanceId: string) {
    try {
        await bundle.install(instanceId);
    } catch (error) {
        toast.error(bundle.text(getT()).failed, { description: errorMessage(error) });
    }
}

export function OrreryBundledCard({
    bundle,
    instanceId,
}: {
    bundle: OrreryBundle;
    instanceId: string;
}) {
    const t = useT();
    const text = bundle.text(t);
    const mods = useInstalledMods(instanceId);
    const running = useGameStore((state) => instanceId in state.running);
    const busy = useTaskStore((state) =>
        Object.values(state.tasks).some(
            (task) =>
                isContentTask(task.kind) &&
                task.instanceId === instanceId &&
                task.status.type === "running",
        ),
    );
    const [pending, setPending] = useState(false);
    const installed = mods.data?.some((mod) => mod.projectId === bundle.id) ?? false;

    const install = async () => {
        setPending(true);
        await installBundleWithToast(bundle, instanceId);
        setPending(false);
    };

    return (
        <div className="flex flex-wrap items-center gap-3 rounded-xl border bg-card px-4 py-3 text-sm">
            <OrreryLogo size={36} className="shrink-0" />
            <div className="min-w-0 flex-1">
                <p className="font-medium">{bundle.title}</p>
                <p className="text-muted-foreground">{text.description}</p>
            </div>
            {installed ? (
                <Badge variant="secondary">{t.content.installed}</Badge>
            ) : (
                <Button
                    size="sm"
                    disabled={!mods.data || pending || busy || running}
                    onClick={() => void install()}
                >
                    {text.install}
                </Button>
            )}
        </div>
    );
}

import { PlayIcon, SquareIcon, XIcon } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { errorMessage } from "@/api/invoke";
import { killInstance } from "@/api/launch";
import { cancelTask } from "@/api/tasks";
import { Button } from "@/components/ui/button";
import { stageLabel } from "@/features/tasks/labels";
import { useTaskStore } from "@/features/tasks/taskStore";
import { useT } from "@/i18n";
import { percent } from "@/lib/format";
import { cn } from "@/lib/utils";
import { useGameStore } from "./gameStore";
import { useLaunchInstance } from "./useLaunchInstance";

type LaunchButtonProps = {
    instanceId: string;
    size?: "default" | "lg";
    className?: string;
};

export function LaunchButton({ instanceId, size = "default", className }: LaunchButtonProps) {
    const t = useT();
    const launch = useLaunchInstance();
    const running = useGameStore((state) => instanceId in state.running);
    const markStopRequested = useGameStore((state) => state.markStopRequested);
    const task = useTaskStore((state) =>
        Object.values(state.tasks).find(
            (t) =>
                t.instanceId === instanceId &&
                t.status.type === "running" &&
                (t.kind === "launch_instance" ||
                    t.kind === "install_instance" ||
                    t.kind === "repair_instance"),
        ),
    );
    const [pending, setPending] = useState(false);

    if (running) {
        return (
            <Button
                variant="secondary"
                size={size}
                className={className}
                onClick={() => {
                    markStopRequested(instanceId);
                    killInstance(instanceId).catch((error: unknown) =>
                        toast.error(t.common.stopFailed, {
                            description: errorMessage(error),
                        }),
                    );
                }}
            >
                <SquareIcon data-icon="inline-start" />
                {t.common.stop}
            </Button>
        );
    }

    if (task) {
        const { progress } = task;
        const value = progress.bytesTotal
            ? percent(progress.bytesDone, progress.bytesTotal)
            : percent(progress.itemsDone, progress.itemsTotal);
        return (
            <Button
                variant="outline"
                size={size}
                className={cn("relative overflow-hidden", className)}
                onClick={() => void cancelTask(task.id)}
                title={t.common.cancel}
            >
                <span
                    aria-hidden
                    className="absolute inset-y-0 left-0 bg-primary/15 transition-[width] duration-200 motion-reduce:transition-none"
                    style={{ width: `${value ?? 0}%` }}
                />
                <XIcon data-icon="inline-start" className="relative" />
                <span className="relative min-w-0 truncate tabular-nums">
                    {t.launch.progress(stageLabel(progress.stage), value)}
                </span>
            </Button>
        );
    }

    const onLaunch = async () => {
        setPending(true);
        try {
            await launch(instanceId);
        } finally {
            setPending(false);
        }
    };

    return (
        <Button
            size={size}
            className={className}
            disabled={pending}
            onClick={() => void onLaunch()}
        >
            <PlayIcon data-icon="inline-start" />
            {t.common.play}
        </Button>
    );
}

import { HardDriveIcon, RotateCwIcon } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { errorMessage } from "@/api/invoke";
import { resetDataDir, restartApp } from "@/api/settings";
import type { DataDirIssue } from "@/bindings/DataDirIssue";
import { Button } from "@/components/ui/button";
import { getT, useT } from "@/i18n";
import { TitleBar } from "./TitleBar";

export function StartupIssueScreen({ issue }: { issue: DataDirIssue }) {
    const t = useT();
    const text = t.settings.startup;
    const [busy, setBusy] = useState(false);

    const run = async (action: () => Promise<void>) => {
        setBusy(true);
        try {
            await action();
        } catch (error) {
            toast.error(getT().settings.startup.failed, { description: errorMessage(error) });
            setBusy(false);
        }
    };

    return (
        <div className="grid h-full grid-cols-[15rem_minmax(0,1fr)] grid-rows-[2.5rem_minmax(0,1fr)] bg-background text-foreground">
            <TitleBar />
            <main className="col-span-2 flex items-center justify-center p-6">
                <div
                    role="alert"
                    className="flex w-full max-w-lg flex-col gap-4 rounded-2xl border bg-card p-6"
                >
                    <HardDriveIcon className="size-8 text-destructive" aria-hidden />
                    <div className="flex flex-col gap-2">
                        <h1 className="text-xl font-semibold text-balance">{text.title}</h1>
                        <p className="text-sm break-words text-muted-foreground">
                            {text.description(issue.path)}
                        </p>
                        <p className="text-sm">{text.problems[issue.problem]}</p>
                    </div>
                    <p className="text-sm text-muted-foreground">{text.resetHint}</p>
                    <div className="flex flex-wrap gap-2">
                        <Button disabled={busy} onClick={() => void run(restartApp)}>
                            <RotateCwIcon data-icon="inline-start" />
                            {text.retry}
                        </Button>
                        <Button
                            variant="outline"
                            disabled={busy}
                            onClick={() => void run(resetDataDir)}
                        >
                            {text.reset}
                        </Button>
                    </div>
                </div>
            </main>
        </div>
    );
}

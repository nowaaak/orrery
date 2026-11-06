import { useMutation, useQuery } from "@tanstack/react-query";
import { openUrl } from "@tauri-apps/plugin-opener";
import { CopyIcon, ExternalLinkIcon, UploadIcon } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { errorMessage } from "@/api/invoke";
import { prepareLogShare, shareLog } from "@/api/logs";
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

type ShareLogDialogProps = {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    lines: string[];
};

export function ShareLogDialog({ open, onOpenChange, lines }: ShareLogDialogProps) {
    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent className="sm:max-w-2xl">
                {open && <ShareLogContent lines={lines} />}
            </DialogContent>
        </Dialog>
    );
}

function ShareLogContent({ lines }: { lines: string[] }) {
    const t = useT();
    const text = t.launch.share;
    const [snapshot] = useState(() => lines.join("\n"));
    const preview = useQuery({
        queryKey: ["log-share", snapshot],
        queryFn: () => prepareLogShare(snapshot),
        gcTime: 0,
    });
    const upload = useMutation({
        mutationFn: shareLog,
        onSuccess: (shared) => {
            toast.success(getT().launch.share.uploaded);
            void copyLink(shared.url);
        },
        onError: (error) =>
            toast.error(getT().launch.share.failed, { description: errorMessage(error) }),
    });
    const lineCount = preview.data ? preview.data.split("\n").length : 0;

    return (
        <>
            <DialogHeader>
                <DialogTitle>{text.title}</DialogTitle>
                <DialogDescription className="text-pretty">{text.description}</DialogDescription>
            </DialogHeader>
            <div className="grid min-w-0 gap-2">
                <div className="flex items-baseline justify-between gap-2">
                    <Label htmlFor="share-preview">{text.preview}</Label>
                    {preview.data !== undefined && (
                        <span className="text-xs text-muted-foreground tabular-nums">
                            {text.lines(lineCount)}
                        </span>
                    )}
                </div>
                {preview.data !== undefined ? (
                    <pre
                        id="share-preview"
                        tabIndex={0}
                        className="h-72 overflow-auto rounded-lg border bg-muted/40 p-3 font-mono text-xs whitespace-pre-wrap select-text [overflow-wrap:anywhere]"
                    >
                        {preview.data}
                    </pre>
                ) : preview.error ? (
                    <p className="text-sm text-destructive select-text">
                        {errorMessage(preview.error)}
                    </p>
                ) : (
                    <Skeleton className="h-72 w-full" aria-label={text.preparing} />
                )}
            </div>
            {upload.data && (
                <div className="grid gap-2">
                    <Label htmlFor="share-link">{text.link}</Label>
                    <div className="flex gap-2">
                        <Input
                            id="share-link"
                            readOnly
                            value={upload.data.url}
                            className="font-mono"
                            onFocus={(event) => event.currentTarget.select()}
                        />
                        <Button
                            variant="outline"
                            size="icon"
                            aria-label={text.copy}
                            title={text.copy}
                            onClick={() => void copyLink(upload.data.url)}
                        >
                            <CopyIcon />
                        </Button>
                        <Button
                            variant="outline"
                            size="icon"
                            aria-label={text.open}
                            title={text.open}
                            onClick={() =>
                                openUrl(upload.data.url).catch((error: unknown) =>
                                    toast.error(errorMessage(error)),
                                )
                            }
                        >
                            <ExternalLinkIcon />
                        </Button>
                    </div>
                </div>
            )}
            <DialogFooter>
                {!upload.data && (
                    <Button
                        disabled={
                            preview.data === undefined ||
                            preview.data.trim() === "" ||
                            upload.isPending
                        }
                        onClick={() => preview.data !== undefined && upload.mutate(preview.data)}
                    >
                        <UploadIcon data-icon="inline-start" />
                        {upload.isPending ? text.uploading : text.upload}
                    </Button>
                )}
            </DialogFooter>
        </>
    );
}

function copyLink(url: string) {
    return navigator.clipboard.writeText(url).then(
        () => toast.success(getT().launch.share.copied),
        (error: unknown) => toast.error(errorMessage(error)),
    );
}

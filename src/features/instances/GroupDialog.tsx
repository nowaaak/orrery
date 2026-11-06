import { useState, type FormEvent } from "react";
import { toast } from "sonner";
import { errorMessage } from "@/api/invoke";
import type { Instance } from "@/bindings/Instance";
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
import { getT, useT } from "@/i18n";
import { groupNames } from "./instanceOrder";
import { useInstances, useSetInstanceGroup } from "./useInstances";

const MAX_GROUP_CHARS = 40;

export function GroupDialog({
    instance,
    open,
    onOpenChange,
}: {
    instance: Instance;
    open: boolean;
    onOpenChange: (open: boolean) => void;
}) {
    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent>
                {open && <GroupForm instance={instance} onDone={() => onOpenChange(false)} />}
            </DialogContent>
        </Dialog>
    );
}

function GroupForm({ instance, onDone }: { instance: Instance; onDone: () => void }) {
    const t = useT();
    const text = t.instances.group;
    const [value, setValue] = useState(instance.group ?? "");
    const existing = groupNames(useInstances().data ?? []);
    const setGroup = useSetInstanceGroup();

    const submit = (group: string | null) =>
        setGroup.mutate(
            { id: instance.id, group },
            {
                onSuccess: onDone,
                onError: (error) =>
                    toast.error(getT().instances.group.failed, {
                        description: errorMessage(error),
                    }),
            },
        );

    const onSubmit = (event: FormEvent) => {
        event.preventDefault();
        submit(value.trim() === "" ? null : value.trim());
    };

    return (
        <form onSubmit={onSubmit} className="grid gap-4">
            <DialogHeader>
                <DialogTitle>{text.title(instance.name)}</DialogTitle>
                <DialogDescription>{text.description}</DialogDescription>
            </DialogHeader>
            <div className="grid gap-2">
                <Label htmlFor="group-name">{text.label}</Label>
                <Input
                    id="group-name"
                    list="group-names"
                    value={value}
                    maxLength={MAX_GROUP_CHARS}
                    placeholder={text.placeholder}
                    autoComplete="off"
                    autoFocus
                    onChange={(event) => setValue(event.target.value)}
                />
                <datalist id="group-names">
                    {existing.map((name) => (
                        <option key={name} value={name} />
                    ))}
                </datalist>
            </div>
            <DialogFooter className="gap-2">
                {instance.group && (
                    <Button
                        type="button"
                        variant="ghost"
                        disabled={setGroup.isPending}
                        onClick={() => submit(null)}
                    >
                        {text.remove}
                    </Button>
                )}
                <Button type="submit" disabled={setGroup.isPending}>
                    {setGroup.isPending ? t.common.saving : t.common.save}
                </Button>
            </DialogFooter>
        </form>
    );
}

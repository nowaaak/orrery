import { useQuery } from "@tanstack/react-query";
import { RefreshCwIcon } from "lucide-react";
import { useState } from "react";
import { listJavaInstallations } from "@/api/launch";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from "@/components/ui/select";
import { useT } from "@/i18n";

const AUTO = "__auto";
const CUSTOM = "__custom";

export function JavaPathField({
    value,
    onChange,
}: {
    value: string;
    onChange: (value: string) => void;
}) {
    const t = useT();
    const text = t.instances.settings;
    const installations = useQuery({
        queryKey: ["java-installations"],
        queryFn: listJavaInstallations,
        staleTime: 5 * 60 * 1000,
    });
    const known = installations.data?.some((java) => java.path === value) ?? false;
    const [custom, setCustom] = useState(value !== "" && !known);
    const showCustom = custom || (value !== "" && installations.isSuccess && !known);
    const selected = showCustom ? CUSTOM : value === "" ? AUTO : value;

    return (
        <div className="grid gap-2">
            <Label htmlFor="java-installation">{text.javaInstallation}</Label>
            <div className="flex gap-2">
                <Select
                    value={selected}
                    onValueChange={(next) => {
                        setCustom(next === CUSTOM);
                        if (next === AUTO) {
                            onChange("");
                        } else if (next !== CUSTOM) {
                            onChange(next);
                        }
                    }}
                >
                    <SelectTrigger id="java-installation" className="min-w-0 flex-1">
                        <SelectValue
                            placeholder={installations.isPending ? text.javaDetecting : undefined}
                        />
                    </SelectTrigger>
                    <SelectContent>
                        <SelectItem value={AUTO}>{text.javaAuto}</SelectItem>
                        {installations.data?.map((java) => (
                            <SelectItem key={java.path} value={java.path}>
                                <span className="flex min-w-0 flex-col items-start">
                                    <span>
                                        {text.javaOption(java.major, java.version, java.vendor)}
                                        {java.managed ? ` · ${text.javaManaged}` : ""}
                                    </span>
                                    <span className="max-w-[32rem] truncate font-mono text-xs text-muted-foreground">
                                        {java.path}
                                    </span>
                                </span>
                            </SelectItem>
                        ))}
                        <SelectItem value={CUSTOM}>{text.javaCustom}</SelectItem>
                    </SelectContent>
                </Select>
                <Button
                    type="button"
                    variant="outline"
                    size="icon"
                    aria-label={text.javaRescan}
                    title={text.javaRescan}
                    disabled={installations.isFetching}
                    onClick={() => void installations.refetch()}
                >
                    <RefreshCwIcon className={installations.isFetching ? "animate-spin" : ""} />
                </Button>
            </div>
            {installations.isFetching && (
                <p className="text-sm text-muted-foreground">{text.javaDetecting}</p>
            )}
            {showCustom && (
                <Input
                    id="java-path"
                    aria-label={text.javaPath}
                    value={value}
                    placeholder={text.javaPath}
                    spellCheck={false}
                    className="font-mono"
                    onChange={(event) => onChange(event.target.value)}
                />
            )}
        </div>
    );
}

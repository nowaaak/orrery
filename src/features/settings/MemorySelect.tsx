import { useQuery } from "@tanstack/react-query";
import { systemMemory } from "@/api/tuning";
import { Label } from "@/components/ui/label";
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from "@/components/ui/select";
import { useT } from "@/i18n";
import { formatGb, isTight, memoryOptions } from "./memory";

type MemorySelectProps = {
    id: string;
    label: string;
    value: number;
    onChange: (mb: number) => void;
    floorMb?: number;
    maxMb?: number;
    recommendedMb?: number;
};

export function useSystemMemory() {
    return useQuery({ queryKey: ["system-memory"], queryFn: systemMemory, staleTime: Infinity });
}

export function MemorySelect({
    id,
    label,
    value,
    onChange,
    floorMb = 1024,
    maxMb,
    recommendedMb,
}: MemorySelectProps) {
    const t = useT();
    const system = useSystemMemory().data ?? null;
    const options = memoryOptions(system, floorMb).filter(
        (mb) => maxMb === undefined || mb <= maxMb,
    );
    if (!options.includes(value)) {
        options.push(value);
        options.sort((a, b) => a - b);
    }

    return (
        <div className="grid gap-2">
            <Label htmlFor={id}>{label}</Label>
            <Select value={String(value)} onValueChange={(next) => onChange(Number(next))}>
                <SelectTrigger id={id} className="w-full tabular-nums">
                    <SelectValue />
                </SelectTrigger>
                <SelectContent className="max-h-72">
                    {options.map((mb) => (
                        <SelectItem key={mb} value={String(mb)} className="tabular-nums">
                            {formatGb(mb)}
                            {mb === recommendedMb && (
                                <span className="text-primary">
                                    {" "}
                                    · {t.settings.memoryRecommended}
                                </span>
                            )}
                            {isTight(mb, system) && (
                                <span className="text-muted-foreground">
                                    {" "}
                                    · {t.settings.memoryTight}
                                </span>
                            )}
                        </SelectItem>
                    ))}
                </SelectContent>
            </Select>
        </div>
    );
}

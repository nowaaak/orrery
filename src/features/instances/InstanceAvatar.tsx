import type { Instance } from "@/bindings/Instance";
import { cn } from "@/lib/utils";
import { useInstanceIcon } from "./useInstanceIcon";

const GRADIENTS: Record<Instance["loader"]["type"], string> = {
    vanilla: "from-emerald-400 via-teal-500 to-cyan-600",
    fabric: "from-indigo-400 via-violet-500 to-fuchsia-600",
    legacy_fabric: "from-sky-400 via-indigo-500 to-violet-700",
    quilt: "from-amber-400 via-orange-500 to-rose-600",
    forge: "from-slate-400 via-zinc-500 to-stone-700",
    neo_forge: "from-orange-400 via-red-500 to-rose-700",
};

function initials(name: string): string {
    const words = name.trim().split(/\s+/).filter(Boolean);
    if (words.length >= 2) {
        return (words[0]![0]! + words[1]![0]!).toUpperCase();
    }
    return (words[0] ?? "?").slice(0, 2).toUpperCase();
}

export function InstanceAvatar({
    instance,
    size = "md",
    className,
}: {
    instance: Pick<Instance, "id" | "name" | "loader" | "icon">;
    size?: "md" | "lg";
    className?: string;
}) {
    const iconUrl = useInstanceIcon(instance.id, instance.icon);
    const shape = size === "lg" ? "size-16 rounded-2xl text-xl" : "size-11 rounded-xl text-sm";

    if (iconUrl) {
        return (
            <img
                src={iconUrl}
                alt=""
                aria-hidden
                className={cn(
                    "shrink-0 bg-muted object-cover shadow-lg ring-1 ring-white/15",
                    shape,
                    className,
                )}
            />
        );
    }

    return (
        <span
            aria-hidden
            className={cn(
                "relative inline-flex shrink-0 items-center justify-center overflow-hidden bg-gradient-to-br font-semibold text-white shadow-lg ring-1 ring-white/15",
                GRADIENTS[instance.loader.type],
                shape,
                className,
            )}
        >
            <span className="absolute inset-0 bg-[radial-gradient(circle_at_30%_25%,rgba(255,255,255,0.35),transparent_55%)]" />
            <span className="relative tracking-wide">{initials(instance.name)}</span>
        </span>
    );
}

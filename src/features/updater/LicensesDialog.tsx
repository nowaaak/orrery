import { useQuery } from "@tanstack/react-query";
import { ChevronRightIcon, SearchIcon } from "lucide-react";
import { useDeferredValue, useMemo, useState } from "react";
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogHeader,
    DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { useT } from "@/i18n";
import { cn } from "@/lib/utils";

type LicensePackage = {
    ecosystem: "cargo" | "npm";
    name: string;
    version: string;
    license: string;
    repository: string | null;
    texts: string[];
};

type LicenseList = {
    packages: LicensePackage[];
    texts: Record<string, string>;
};

async function loadLicenses(): Promise<LicenseList | null> {
    const response = await fetch("/licenses.json");
    if (!response.ok) {
        return null;
    }
    return (await response.json()) as LicenseList;
}

export function filterLicenses(packages: LicensePackage[], query: string): LicensePackage[] {
    const needle = query.trim().toLowerCase();
    if (needle === "") {
        return packages;
    }
    return packages.filter(
        (pkg) =>
            pkg.name.toLowerCase().includes(needle) || pkg.license.toLowerCase().includes(needle),
    );
}

export function LicensesDialog({
    open,
    onOpenChange,
}: {
    open: boolean;
    onOpenChange: (open: boolean) => void;
}) {
    const t = useT().updater.licenses;
    const list = useQuery({
        queryKey: ["licenses"],
        queryFn: loadLicenses,
        enabled: open,
        staleTime: Infinity,
    });
    const [query, setQuery] = useState("");
    const deferred = useDeferredValue(query);
    const [expanded, setExpanded] = useState<string | null>(null);
    const visible = useMemo(
        () => filterLicenses(list.data?.packages ?? [], deferred),
        [list.data, deferred],
    );

    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent className="sm:max-w-2xl">
                <DialogHeader>
                    <DialogTitle>{t.title}</DialogTitle>
                    <DialogDescription>{t.description}</DialogDescription>
                </DialogHeader>
                {list.isPending ? (
                    <div className="flex flex-col gap-2">
                        {[0, 1, 2, 3].map((key) => (
                            <Skeleton key={key} className="h-10 w-full" />
                        ))}
                    </div>
                ) : !list.data ? (
                    <p className="rounded-xl border border-dashed p-5 text-center text-sm text-muted-foreground">
                        {t.missing}
                    </p>
                ) : (
                    <div className="flex min-w-0 flex-col gap-3">
                        <div className="relative">
                            <SearchIcon
                                className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground"
                                aria-hidden
                            />
                            <Input
                                value={query}
                                onChange={(event) => setQuery(event.target.value)}
                                placeholder={t.search}
                                aria-label={t.search}
                                className="pl-8"
                            />
                        </div>
                        <p className="text-xs text-muted-foreground tabular-nums">
                            {t.count(visible.length, list.data.packages.length)}
                        </p>
                        <ul className="h-[24rem] overflow-y-auto pr-1">
                            {visible.map((pkg) => {
                                const key = `${pkg.ecosystem}:${pkg.name}@${pkg.version}`;
                                const isOpen = expanded === key;
                                return (
                                    <li key={key} className="border-b last:border-b-0">
                                        <button
                                            type="button"
                                            aria-expanded={isOpen}
                                            onClick={() => setExpanded(isOpen ? null : key)}
                                            className="flex w-full items-center gap-2 rounded-md px-2 py-2 text-left outline-none hover:bg-muted/60 focus-visible:ring-2 focus-visible:ring-ring"
                                        >
                                            <ChevronRightIcon
                                                className={cn(
                                                    "size-3.5 shrink-0 text-muted-foreground transition-transform motion-reduce:transition-none",
                                                    isOpen && "rotate-90",
                                                )}
                                                aria-hidden
                                            />
                                            <span className="min-w-0 flex-1 truncate text-sm">
                                                {pkg.name}{" "}
                                                <span className="text-muted-foreground tabular-nums">
                                                    {pkg.version}
                                                </span>
                                            </span>
                                            <span className="shrink-0 text-xs text-muted-foreground">
                                                {pkg.license}
                                            </span>
                                        </button>
                                        {isOpen && (
                                            <div className="flex flex-col gap-2 px-2 pb-3">
                                                {pkg.repository && (
                                                    <p className="text-xs break-all text-muted-foreground select-text">
                                                        {pkg.repository}
                                                    </p>
                                                )}
                                                {pkg.texts.length === 0 ? (
                                                    <p className="text-xs text-muted-foreground">
                                                        {t.noText(pkg.license)}
                                                    </p>
                                                ) : (
                                                    pkg.texts.map((hash) => (
                                                        <pre
                                                            key={hash}
                                                            className="max-h-64 overflow-auto rounded-lg bg-muted p-3 font-mono text-[0.6875rem] leading-relaxed whitespace-pre-wrap select-text"
                                                        >
                                                            {list.data?.texts[hash]}
                                                        </pre>
                                                    ))
                                                )}
                                            </div>
                                        )}
                                    </li>
                                );
                            })}
                        </ul>
                    </div>
                )}
            </DialogContent>
        </Dialog>
    );
}

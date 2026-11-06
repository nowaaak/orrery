import { SearchIcon, Share2Icon } from "lucide-react";
import {
    useDeferredValue,
    useLayoutEffect,
    useMemo,
    useRef,
    useState,
    type ReactNode,
} from "react";
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
import { Switch } from "@/components/ui/switch";
import { useT } from "@/i18n";
import { cn } from "@/lib/utils";
import { filterLog, splitMatches, type LevelFilter, type LogLevel } from "./logFilter";
import { ShareLogDialog } from "./ShareLogDialog";

const LEVELS: LevelFilter[] = ["all", "warn", "error"];
const IS_MAC = navigator.userAgent.includes("Macintosh");

const LEVEL_CLASS: Record<LogLevel, string> = {
    error: "text-destructive",
    warn: "text-amber-600 dark:text-amber-400",
    info: "",
};

type LogViewProps = {
    id: string;
    lines: string[];
    notice?: ReactNode;
    className?: string;
};

export function LogView({ id, lines, notice, className }: LogViewProps) {
    const t = useT();
    const text = t.launch.log;
    const [query, setQuery] = useState("");
    const [level, setLevel] = useState<LevelFilter>("all");
    const [follow, setFollow] = useState(true);
    const [sharing, setSharing] = useState(false);
    const deferredQuery = useDeferredValue(query);
    const containerRef = useRef<HTMLDivElement>(null);
    const searchRef = useRef<HTMLInputElement>(null);
    const visible = useMemo(
        () => filterLog(lines, deferredQuery, level),
        [lines, deferredQuery, level],
    );

    useLayoutEffect(() => {
        const container = containerRef.current;
        if (container && follow) {
            container.scrollTop = container.scrollHeight;
        }
    }, [visible, follow]);

    return (
        <div
            className={cn("flex min-w-0 flex-col gap-3", className)}
            onKeyDown={(event) => {
                if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "f") {
                    event.preventDefault();
                    searchRef.current?.focus();
                }
            }}
        >
            <div className="flex flex-wrap items-center gap-2">
                <div className="relative min-w-48 flex-1">
                    <SearchIcon
                        className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground"
                        aria-hidden
                    />
                    <Input
                        ref={searchRef}
                        type="search"
                        value={query}
                        aria-label={text.search}
                        aria-keyshortcuts={IS_MAC ? "Meta+F" : "Control+F"}
                        placeholder={`${text.searchPlaceholder}  ${IS_MAC ? "⌘F" : text.searchShortcut}`}
                        className="pl-8"
                        onChange={(event) => setQuery(event.target.value)}
                    />
                </div>
                <Select value={level} onValueChange={(value) => setLevel(value as LevelFilter)}>
                    <SelectTrigger className="w-52" aria-label={text.level}>
                        <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                        {LEVELS.map((value) => (
                            <SelectItem key={value} value={value}>
                                {text.levels[value]}
                            </SelectItem>
                        ))}
                    </SelectContent>
                </Select>
                <div className="flex items-center gap-2 px-1">
                    <Switch id={`${id}-follow`} checked={follow} onCheckedChange={setFollow} />
                    <Label htmlFor={`${id}-follow`}>{text.follow}</Label>
                </div>
                <Button
                    variant="outline"
                    disabled={lines.length === 0}
                    onClick={() => setSharing(true)}
                >
                    <Share2Icon data-icon="inline-start" />
                    {text.share}
                </Button>
            </div>
            <div
                ref={containerRef}
                role="log"
                aria-live="off"
                tabIndex={0}
                onScroll={(event) => {
                    const el = event.currentTarget;
                    const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 24;
                    if (atBottom !== follow) {
                        setFollow(atBottom);
                    }
                }}
                className="h-[28rem] overflow-auto rounded-lg border bg-muted/40 p-3 font-mono text-xs leading-relaxed select-text outline-none focus-visible:ring-2 focus-visible:ring-ring"
            >
                {notice}
                {visible.length === 0 ? (
                    <p className="p-4 text-center font-sans text-sm text-muted-foreground">
                        {text.noMatches}
                    </p>
                ) : (
                    visible.map((entry) => (
                        <div
                            key={entry.index}
                            className={cn(
                                "whitespace-pre-wrap [overflow-wrap:anywhere]",
                                LEVEL_CLASS[entry.level],
                            )}
                        >
                            {deferredQuery.trim() === ""
                                ? entry.line
                                : splitMatches(entry.line, deferredQuery).map((part, index) =>
                                      part.match ? (
                                          <mark
                                              key={index}
                                              className="rounded-sm bg-amber-300/60 text-inherit dark:bg-amber-500/40"
                                          >
                                              {part.text}
                                          </mark>
                                      ) : (
                                          part.text
                                      ),
                                  )}
                        </div>
                    ))
                )}
            </div>
            <p className="text-xs text-muted-foreground tabular-nums">
                {text.count(visible.length, lines.length)}
            </p>
            <ShareLogDialog open={sharing} onOpenChange={setSharing} lines={lines} />
        </div>
    );
}

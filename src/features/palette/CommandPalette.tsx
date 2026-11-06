import {
    BoxesIcon,
    BrushCleaningIcon,
    EarthIcon,
    FolderOpenIcon,
    LanguagesIcon,
    LayersIcon,
    MonitorIcon,
    MoonIcon,
    PackageIcon,
    PlayIcon,
    PlusIcon,
    ScrollTextIcon,
    SearchIcon,
    SettingsIcon,
    SlidersHorizontalIcon,
    SquareIcon,
    SunIcon,
    UserPlusIcon,
    UsersIcon,
    type LucideIcon,
} from "lucide-react";
import { useEffect, useId, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { useNavigate } from "react-router";
import { toast } from "sonner";
import { errorMessage } from "@/api/invoke";
import { killInstance } from "@/api/launch";
import { useUiStore } from "@/app/uiStore";
import type { Language } from "@/bindings/Language";
import type { Theme } from "@/bindings/Theme";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { loaderLabel } from "@/features/instances/labels";
import { useInstances } from "@/features/instances/useInstances";
import { useGameStore } from "@/features/launch/gameStore";
import { useLaunchInstance } from "@/features/launch/useLaunchInstance";
import { useSettings, useUpdateSettings } from "@/features/settings/useSettings";
import { useT, type Messages } from "@/i18n";
import { fuzzyScore } from "@/lib/fuzzy";
import { cn } from "@/lib/utils";

export const IS_MAC = navigator.userAgent.includes("Macintosh");

export function paletteShortcut(t: Messages): string {
    return IS_MAC ? "⌘ K" : t.palette.shortcut;
}

const RECENT_KEY = "orrery.palette.recent";
const MAX_RECENT = 5;
const FEATURED_INSTANCES = 5;
const GROUP_ORDER = ["instances", "actions", "pages"] as const;

type Group = (typeof GROUP_ORDER)[number];

type Command = {
    id: string;
    label: string;
    group: Group;
    icon: LucideIcon;
    keywords?: string;
    hint?: string;
    featured: boolean;
    run: () => unknown;
};

type Section = { key: Group | "recent"; items: Command[] };

const THEMES: { theme: Theme; icon: LucideIcon }[] = [
    { theme: "dark", icon: MoonIcon },
    { theme: "light", icon: SunIcon },
    { theme: "system", icon: MonitorIcon },
];

const LANGUAGES: Language[] = ["de", "en", "system"];

function readRecent(): string[] {
    try {
        const value: unknown = JSON.parse(localStorage.getItem(RECENT_KEY) ?? "[]");
        return Array.isArray(value) ? value.filter((id) => typeof id === "string") : [];
    } catch {
        return [];
    }
}

function rememberRecent(id: string) {
    try {
        const next = [id, ...readRecent().filter((known) => known !== id)].slice(0, MAX_RECENT);
        localStorage.setItem(RECENT_KEY, JSON.stringify(next));
    } catch {
        return;
    }
}

function reportError(title: string) {
    return (error: unknown) => toast.error(title, { description: errorMessage(error) });
}

function useCommands(): Command[] {
    const t = useT();
    const navigate = useNavigate();
    const instances = useInstances();
    const running = useGameStore((state) => state.running);
    const markStopRequested = useGameStore((state) => state.markStopRequested);
    const launch = useLaunchInstance();
    const settings = useSettings();
    const updateSettings = useUpdateSettings();
    const openDialog = useUiStore((state) => state.openDialog);

    return useMemo(() => {
        const sorted = [...(instances.data ?? [])].sort(
            (a, b) => (b.lastPlayedAt ?? b.createdAt) - (a.lastPlayedAt ?? a.createdAt),
        );
        const text = t.palette.commands;
        const commands: Command[] = [];
        sorted.forEach((instance, index) => {
            const hint = `${instance.minecraftVersion} · ${loaderLabel(instance).split(" ")[0]}`;
            const isRunning = instance.id in running;
            commands.push(
                isRunning
                    ? {
                          id: `stop:${instance.id}`,
                          label: text.stop(instance.name),
                          group: "instances",
                          icon: SquareIcon,
                          keywords: text.stopKeywords,
                          hint,
                          featured: index < FEATURED_INSTANCES,
                          run: () => {
                              markStopRequested(instance.id);
                              return killInstance(instance.id).catch(
                                  reportError(t.common.stopFailed),
                              );
                          },
                      }
                    : {
                          id: `play:${instance.id}`,
                          label: text.play(instance.name),
                          group: "instances",
                          icon: PlayIcon,
                          keywords: text.playKeywords,
                          hint,
                          featured: index < FEATURED_INSTANCES,
                          run: () => launch(instance.id),
                      },
                {
                    id: `open:${instance.id}`,
                    label: text.open(instance.name),
                    group: "instances",
                    icon: FolderOpenIcon,
                    keywords: text.openKeywords,
                    hint,
                    featured: false,
                    run: () => navigate(`/instances/${instance.id}`),
                },
                {
                    id: `mods:${instance.id}`,
                    label: text.content(instance.name),
                    group: "instances",
                    icon: PackageIcon,
                    keywords: text.contentKeywords,
                    featured: false,
                    run: () => navigate(`/instances/${instance.id}?tab=mods`),
                },
                {
                    id: `worlds:${instance.id}`,
                    label: text.worlds(instance.name),
                    group: "instances",
                    icon: EarthIcon,
                    keywords: text.worldsKeywords,
                    featured: false,
                    run: () => navigate(`/instances/${instance.id}?tab=worlds`),
                },
                {
                    id: `settings:${instance.id}`,
                    label: text.settings(instance.name),
                    group: "instances",
                    icon: SlidersHorizontalIcon,
                    keywords: text.settingsKeywords,
                    featured: false,
                    run: () => navigate(`/instances/${instance.id}?tab=settings`),
                },
            );
        });

        commands.push(
            {
                id: "create-instance",
                label: text.createInstance,
                group: "actions",
                icon: PlusIcon,
                keywords: text.createInstanceKeywords,
                featured: true,
                run: () => openDialog("create-instance"),
            },
            {
                id: "modpack",
                label: text.modpack,
                group: "actions",
                icon: LayersIcon,
                keywords: text.modpackKeywords,
                featured: true,
                run: () => openDialog("modpack"),
            },
            {
                id: "add-account",
                label: text.addAccount,
                group: "actions",
                icon: UserPlusIcon,
                keywords: text.addAccountKeywords,
                featured: false,
                run: () => navigate("/accounts"),
            },
            {
                id: "storage",
                label: text.storage,
                group: "actions",
                icon: BrushCleaningIcon,
                keywords: text.storageKeywords,
                featured: false,
                run: () => navigate("/settings"),
            },
            {
                id: "launcher-log",
                label: text.launcherLog,
                group: "actions",
                icon: ScrollTextIcon,
                keywords: text.launcherLogKeywords,
                featured: false,
                run: () => openDialog("launcher-log"),
            },
        );
        const current = settings.data;
        if (current) {
            for (const { theme, icon } of THEMES) {
                commands.push({
                    id: `theme:${theme}`,
                    label: text.theme(t.settings.themes[theme]),
                    group: "actions",
                    icon,
                    keywords: text.themeKeywords,
                    hint: current.theme === theme ? t.palette.active : undefined,
                    featured: false,
                    run: () =>
                        updateSettings.mutate(
                            { ...current, theme },
                            { onError: reportError(text.themeFailed) },
                        ),
                });
            }
            for (const language of LANGUAGES) {
                commands.push({
                    id: `language:${language}`,
                    label: text.language(t.settings.languages[language]),
                    group: "actions",
                    icon: LanguagesIcon,
                    keywords: text.languageKeywords,
                    hint: current.language === language ? t.palette.active : undefined,
                    featured: false,
                    run: () =>
                        updateSettings.mutate(
                            { ...current, language },
                            { onError: reportError(text.languageFailed) },
                        ),
                });
            }
        }
        commands.push(
            {
                id: "page:instances",
                label: t.app.nav.instances,
                group: "pages",
                icon: BoxesIcon,
                featured: true,
                run: () => navigate("/"),
            },
            {
                id: "page:accounts",
                label: t.app.nav.accounts,
                group: "pages",
                icon: UsersIcon,
                featured: true,
                run: () => navigate("/accounts"),
            },
            {
                id: "page:settings",
                label: t.app.nav.settings,
                group: "pages",
                icon: SettingsIcon,
                featured: true,
                run: () => navigate("/settings"),
            },
        );
        return commands;
    }, [
        t,
        instances.data,
        running,
        markStopRequested,
        launch,
        navigate,
        openDialog,
        settings.data,
        updateSettings,
    ]);
}

function buildSections(commands: Command[], query: string, recent: string[]): Section[] {
    if (query.trim() === "") {
        const byId = new Map(commands.map((command) => [command.id, command]));
        const recentItems = recent.flatMap((id) => byId.get(id) ?? []);
        const recentIds = new Set(recentItems.map((command) => command.id));
        const sections: Section[] = [];
        if (recentItems.length > 0) {
            sections.push({ key: "recent", items: recentItems });
        }
        for (const group of GROUP_ORDER) {
            const items = commands.filter(
                (command) =>
                    command.group === group && command.featured && !recentIds.has(command.id),
            );
            if (items.length > 0) {
                sections.push({ key: group, items });
            }
        }
        return sections;
    }

    const scored = commands.flatMap((command) => {
        const label = fuzzyScore(query, command.label);
        const extended = fuzzyScore(query, `${command.label} ${command.keywords ?? ""}`);
        const score = Math.max(label ?? -1, extended === null ? -1 : extended * 0.8);
        return score < 0 ? [] : [{ command, score }];
    });
    return GROUP_ORDER.map((group) => {
        const items = scored
            .filter((entry) => entry.command.group === group)
            .sort((a, b) => b.score - a.score);
        return { key: group, best: items[0]?.score ?? -1, items: items.map((e) => e.command) };
    })
        .filter((section) => section.items.length > 0)
        .sort((a, b) => b.best - a.best)
        .map(({ key, items }) => ({ key, items }));
}

export function CommandPalette() {
    const t = useT();
    const open = useUiStore((state) => state.paletteOpen);
    const setOpen = useUiStore((state) => state.setPaletteOpen);

    useEffect(() => {
        const onKeyDown = (event: globalThis.KeyboardEvent) => {
            if (
                (event.ctrlKey || event.metaKey) &&
                !event.altKey &&
                !event.shiftKey &&
                event.key.toLowerCase() === "k"
            ) {
                event.preventDefault();
                setOpen(!useUiStore.getState().paletteOpen);
            }
        };
        window.addEventListener("keydown", onKeyDown);
        return () => window.removeEventListener("keydown", onKeyDown);
    }, [setOpen]);

    return (
        <Dialog open={open} onOpenChange={setOpen}>
            <DialogContent
                showCloseButton={false}
                className="top-[12%] translate-y-0 gap-0 overflow-hidden p-0 sm:max-w-xl"
            >
                <DialogTitle className="sr-only">{t.palette.title}</DialogTitle>
                <DialogDescription className="sr-only">{t.palette.description}</DialogDescription>
                {open && <Palette onClose={() => setOpen(false)} />}
            </DialogContent>
        </Dialog>
    );
}

function Palette({ onClose }: { onClose: () => void }) {
    const t = useT();
    const commands = useCommands();
    const [query, setQuery] = useState("");
    const [active, setActive] = useState(0);
    const [recent] = useState(readRecent);
    const listRef = useRef<HTMLDivElement>(null);
    const baseId = useId();
    const sections = useMemo(
        () => buildSections(commands, query, recent),
        [commands, query, recent],
    );
    const flat = sections.flatMap((section) => section.items);
    const activeIndex = flat.length === 0 ? -1 : Math.min(active, flat.length - 1);
    const optionId = (index: number) => `${baseId}-option-${index}`;

    useEffect(() => {
        if (activeIndex < 0) {
            return;
        }
        listRef.current
            ?.querySelector(`[data-index="${activeIndex}"]`)
            ?.scrollIntoView({ block: "nearest" });
    }, [activeIndex]);

    const execute = (command: Command | undefined) => {
        if (!command) {
            return;
        }
        rememberRecent(command.id);
        onClose();
        window.setTimeout(() => void command.run(), 0);
    };

    const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
        if (flat.length === 0) {
            return;
        }
        if (event.key === "ArrowDown") {
            event.preventDefault();
            setActive((activeIndex + 1) % flat.length);
        } else if (event.key === "ArrowUp") {
            event.preventDefault();
            setActive((activeIndex - 1 + flat.length) % flat.length);
        } else if (event.key === "PageDown") {
            event.preventDefault();
            setActive(Math.min(flat.length - 1, activeIndex + 5));
        } else if (event.key === "PageUp") {
            event.preventDefault();
            setActive(Math.max(0, activeIndex - 5));
        } else if (event.key === "Enter" && !event.nativeEvent.isComposing) {
            event.preventDefault();
            execute(flat[activeIndex]);
        }
    };

    let index = -1;
    return (
        <div className="flex flex-col">
            <div className="flex items-center gap-2.5 border-b px-4">
                <SearchIcon className="size-4 shrink-0 text-muted-foreground" aria-hidden />
                <input
                    autoFocus
                    role="combobox"
                    aria-expanded
                    aria-controls={`${baseId}-list`}
                    aria-activedescendant={activeIndex >= 0 ? optionId(activeIndex) : undefined}
                    aria-autocomplete="list"
                    aria-label={t.palette.inputLabel}
                    value={query}
                    placeholder={t.palette.placeholder}
                    spellCheck={false}
                    className="h-12 min-w-0 flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground"
                    onChange={(event) => {
                        setQuery(event.target.value);
                        setActive(0);
                    }}
                    onKeyDown={onKeyDown}
                />
            </div>
            <div
                ref={listRef}
                id={`${baseId}-list`}
                role="listbox"
                aria-label={t.palette.results}
                className="h-80 overflow-y-auto p-2"
            >
                {flat.length === 0 ? (
                    <p className="px-3 py-8 text-center text-sm text-muted-foreground">
                        {t.palette.noResults(query.trim())}
                    </p>
                ) : (
                    sections.map((section) => (
                        <div
                            key={section.key}
                            role="group"
                            aria-labelledby={`${baseId}-${section.key}`}
                            className="pb-1"
                        >
                            <p
                                id={`${baseId}-${section.key}`}
                                className="px-3 pt-2 pb-1 text-xs font-medium text-muted-foreground"
                            >
                                {t.palette.groups[section.key]}
                            </p>
                            {section.items.map((command) => {
                                index += 1;
                                const itemIndex = index;
                                const selected = itemIndex === activeIndex;
                                return (
                                    <div
                                        key={command.id}
                                        id={optionId(itemIndex)}
                                        data-index={itemIndex}
                                        role="option"
                                        aria-selected={selected}
                                        className={cn(
                                            "flex h-10 cursor-pointer items-center gap-3 rounded-lg px-3 text-sm select-none",
                                            selected && "bg-accent text-accent-foreground",
                                        )}
                                        onPointerMove={() => {
                                            if (!selected) {
                                                setActive(itemIndex);
                                            }
                                        }}
                                        onPointerDown={(event) => event.preventDefault()}
                                        onClick={() => execute(command)}
                                    >
                                        <command.icon
                                            className="size-4 shrink-0 text-muted-foreground"
                                            aria-hidden
                                        />
                                        <span className="min-w-0 flex-1 truncate">
                                            {command.label}
                                        </span>
                                        {command.hint && (
                                            <span className="shrink-0 text-xs text-muted-foreground">
                                                {command.hint}
                                            </span>
                                        )}
                                    </div>
                                );
                            })}
                        </div>
                    ))
                )}
            </div>
            <div className="flex items-center gap-4 border-t px-4 py-2 text-xs text-muted-foreground">
                <span>
                    <Kbd>↑</Kbd> <Kbd>↓</Kbd> {t.palette.footerSelect}
                </span>
                <span>
                    <Kbd>Enter</Kbd> {t.palette.footerRun}
                </span>
                <span>
                    <Kbd>Esc</Kbd> {t.palette.footerClose}
                </span>
            </div>
        </div>
    );
}

export function Kbd({ children }: { children: string }) {
    return (
        <kbd className="inline-flex h-5 min-w-5 items-center justify-center rounded border bg-muted px-1 font-sans text-[0.6875rem] font-medium">
            {children}
        </kbd>
    );
}

export function PaletteButton() {
    const t = useT();
    const setOpen = useUiStore((state) => state.setPaletteOpen);

    return (
        <button
            type="button"
            onClick={() => setOpen(true)}
            aria-keyshortcuts={IS_MAC ? "Meta+K" : "Control+K"}
            className="flex h-9 items-center gap-2 rounded-lg bg-sidebar-accent/60 px-3 text-sm text-sidebar-foreground/60 transition-colors outline-none hover:bg-sidebar-accent hover:text-sidebar-foreground focus-visible:ring-2 focus-visible:ring-sidebar-ring motion-reduce:transition-none"
        >
            <SearchIcon className="size-4" aria-hidden />
            <span className="flex-1 text-left">{t.palette.open}</span>
            <Kbd>{paletteShortcut(t)}</Kbd>
        </button>
    );
}

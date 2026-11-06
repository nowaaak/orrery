import { useLayoutEffect, useRef } from "react";
import { Link, Outlet, useLocation } from "react-router";
import { ScrollArea } from "@/components/ui/scroll-area";
import { CreateInstanceDialog } from "@/features/instances/CreateInstanceDialog";
import { DropZone } from "@/features/drop/DropZone";
import { CrashDialog } from "@/features/launch/CrashDialog";
import { LauncherLogDialog } from "@/features/launch/LauncherLogDialog";
import { DeepLinkHandler } from "@/features/links/DeepLinkHandler";
import { ImportDialog } from "@/features/imports/ImportDialog";
import { ModpackDialog } from "@/features/modpacks/ModpackDialog";
import { OptifineDialog } from "@/features/optifine/OptifineDialog";
import { CommandPalette, PaletteButton } from "@/features/palette/CommandPalette";
import { NAV_ITEMS } from "./navigation";
import { TitleBar } from "./TitleBar";
import { AccountSwitcher } from "@/features/accounts/AccountSwitcher";
import { LoginRequiredDialog } from "@/features/accounts/LoginRequiredDialog";
import { useAccounts } from "@/features/accounts/useAccounts";
import { WelcomeScreen } from "@/features/onboarding/WelcomeScreen";
import { useSettings } from "@/features/settings/useSettings";
import { useGameSync } from "@/features/launch/useGameSync";
import { cn } from "@/lib/utils";
import { TaskTray } from "@/features/tasks/TaskTray";
import { UpdateNotice } from "@/features/updater/UpdateNotice";
import { useT } from "@/i18n";

export function AppLayout() {
    useGameSync();
    const t = useT();
    const settings = useSettings();
    const accounts = useAccounts();
    const welcome = settings.data?.welcomeDone === false && accounts.data?.accounts.length === 0;
    const { pathname } = useLocation();
    const scrollRef = useRef<HTMLDivElement>(null);

    useLayoutEffect(() => {
        const viewport = scrollRef.current?.querySelector<HTMLElement>(
            '[data-slot="scroll-area-viewport"]',
        );
        if (viewport) {
            viewport.scrollTop = 0;
        }
    }, [pathname]);

    return (
        <div className="grid h-full grid-cols-[15rem_minmax(0,1fr)] grid-rows-[2.5rem_minmax(0,1fr)] bg-sidebar text-foreground">
            <TitleBar hidePage={welcome} />
            {welcome ? (
                <WelcomeScreen />
            ) : (
                <>
                    <aside className="flex min-h-0 flex-col gap-4 p-2.5 pt-1">
                        <PaletteButton />
                        <nav aria-label={t.app.nav.label} className="flex flex-col gap-0.5">
                            {NAV_ITEMS.map((item) => {
                                const active = item.isActive(pathname);
                                return (
                                    <Link
                                        key={item.to}
                                        to={item.to}
                                        aria-current={active ? "page" : undefined}
                                        className={cn(
                                            "flex h-9 items-center gap-3 rounded-lg px-3 text-sm font-medium text-sidebar-foreground/65 transition-colors outline-none hover:bg-sidebar-accent/50 hover:text-sidebar-foreground focus-visible:ring-2 focus-visible:ring-sidebar-ring motion-reduce:transition-none",
                                            active &&
                                                "bg-sidebar-accent text-sidebar-accent-foreground",
                                        )}
                                    >
                                        <item.icon className="size-4" aria-hidden />
                                        {t.app.nav[item.labelKey]}
                                    </Link>
                                );
                            })}
                        </nav>
                        <div className="mt-auto flex flex-col gap-3">
                            <UpdateNotice />
                            <TaskTray />
                            <AccountSwitcher />
                        </div>
                    </aside>
                    <div className="min-h-0 overflow-hidden rounded-tl-2xl border-t border-l border-border bg-background">
                        <ScrollArea ref={scrollRef} className="h-full">
                            <main className="mx-auto w-full max-w-6xl p-6">
                                <Outlet />
                            </main>
                        </ScrollArea>
                    </div>
                </>
            )}
            <CrashDialog />
            <CreateInstanceDialog />
            <ModpackDialog />
            <ImportDialog />
            <OptifineDialog />
            <LoginRequiredDialog />
            <LauncherLogDialog />
            <DropZone />
            <DeepLinkHandler />
            <CommandPalette />
        </div>
    );
}

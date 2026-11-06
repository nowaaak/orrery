import { QueryClient, QueryClientProvider, useQuery } from "@tanstack/react-query";
import { RouterProvider } from "react-router";
import { Toaster } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";
import { useLanguageSync, useResolvedTheme } from "@/features/settings/useThemeSync";
import { useTaskSync } from "@/features/tasks/useTaskSync";
import { getStartupIssue } from "@/api/settings";
import { resolveLocale, setLocale } from "@/i18n";
import { router } from "./router";
import { StartupIssueScreen } from "./StartupIssueScreen";

const queryClient = new QueryClient({
    defaultOptions: {
        queries: {
            retry: 1,
            refetchOnWindowFocus: false,
        },
    },
});

export function App() {
    return (
        <QueryClientProvider client={queryClient}>
            <TooltipProvider delayDuration={300}>
                <StartupGate />
            </TooltipProvider>
        </QueryClientProvider>
    );
}

function StartupGate() {
    const issue = useQuery({
        queryKey: ["startup-issue"],
        queryFn: async () => {
            const startup = await getStartupIssue();
            if (startup) {
                setLocale(resolveLocale(startup.language));
            }
            return startup;
        },
        staleTime: Infinity,
        retry: false,
    });
    const theme = useResolvedTheme();

    if (issue.isPending) {
        return null;
    }
    if (issue.data) {
        return (
            <>
                <StartupIssueScreen issue={issue.data.issue} />
                <Toaster theme={theme} position="bottom-right" />
            </>
        );
    }
    return <AppShell />;
}

function AppShell() {
    const theme = useResolvedTheme();
    const locale = useLanguageSync();
    useTaskSync();

    return (
        <>
            <RouterProvider key={locale} router={router} />
            <Toaster theme={theme} position="bottom-right" />
        </>
    );
}

import { useEffect, useState } from "react";
import type { Language } from "@/bindings/Language";
import type { Theme } from "@/bindings/Theme";
import { resolveLocale, setLocale, useLocale, type Locale } from "@/i18n";
import { useSettings } from "./useSettings";

const DARK_QUERY = "(prefers-color-scheme: dark)";

export function useLanguageSync(): Locale {
    const language: Language = useSettings().data?.language ?? "system";
    const locale = resolveLocale(language);

    useEffect(() => {
        setLocale(locale);
    }, [locale]);

    return useLocale();
}

export function useResolvedTheme(): "light" | "dark" {
    const theme: Theme = useSettings().data?.theme ?? "dark";
    const [systemDark, setSystemDark] = useState(() => window.matchMedia(DARK_QUERY).matches);

    useEffect(() => {
        const media = window.matchMedia(DARK_QUERY);
        const onChange = (event: MediaQueryListEvent) => setSystemDark(event.matches);
        media.addEventListener("change", onChange);
        return () => media.removeEventListener("change", onChange);
    }, []);

    const resolved = theme === "system" ? (systemDark ? "dark" : "light") : theme;

    useEffect(() => {
        document.documentElement.classList.toggle("dark", resolved === "dark");
        document.documentElement.style.colorScheme = resolved;
    }, [resolved]);

    return resolved;
}

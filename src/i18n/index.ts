import { create } from "zustand";
import type { Language } from "@/bindings/Language";
import { messages, type Locale, type Messages } from "./messages";

export type { Locale, Messages };

export function resolveLocale(language: Language): Locale {
    if (language === "system") {
        return navigator.language.toLowerCase().startsWith("de") ? "de" : "en";
    }
    return language;
}

const useLocaleStore = create<{ locale: Locale }>(() => ({ locale: resolveLocale("system") }));

export function setLocale(locale: Locale) {
    document.documentElement.lang = locale;
    useLocaleStore.setState({ locale });
}

export function useLocale(): Locale {
    return useLocaleStore((state) => state.locale);
}

export function useT(): Messages {
    return messages[useLocale()];
}

export function getT(): Messages {
    return messages[useLocaleStore.getState().locale];
}

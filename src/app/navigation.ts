import { BoxesIcon, LayersIcon, SettingsIcon, UsersIcon, type LucideIcon } from "lucide-react";
import type { Messages } from "@/i18n";

export type NavItem = {
    to: string;
    labelKey: Exclude<keyof Messages["app"]["nav"], "label">;
    icon: LucideIcon;
    isActive: (pathname: string) => boolean;
};

export const NAV_ITEMS: NavItem[] = [
    {
        to: "/",
        labelKey: "instances",
        icon: BoxesIcon,
        isActive: (pathname) => pathname === "/" || pathname.startsWith("/instances/"),
    },
    {
        to: "/sync-packs",
        labelKey: "syncPacks",
        icon: LayersIcon,
        isActive: (pathname) => pathname.startsWith("/sync-packs"),
    },
    {
        to: "/accounts",
        labelKey: "accounts",
        icon: UsersIcon,
        isActive: (pathname) => pathname.startsWith("/accounts"),
    },
    {
        to: "/settings",
        labelKey: "settings",
        icon: SettingsIcon,
        isActive: (pathname) => pathname.startsWith("/settings"),
    },
];

export function findNavItem(pathname: string): NavItem | undefined {
    return NAV_ITEMS.find((item) => item.isActive(pathname));
}

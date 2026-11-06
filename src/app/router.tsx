import { SyncPacksPage } from "@/features/syncpacks/SyncPacksPage";
import { createHashRouter } from "react-router";
import { AccountsPage } from "@/features/accounts/AccountsPage";
import { InstanceDetailPage } from "@/features/instances/InstanceDetailPage";
import { InstancesPage } from "@/features/instances/InstancesPage";
import { SettingsPage } from "@/features/settings/SettingsPage";
import { AccountSkinPage } from "@/features/skins/AccountSkinPage";
import { AppLayout } from "./AppLayout";

export const router = createHashRouter([
    {
        path: "/",
        element: <AppLayout />,
        children: [
            { index: true, element: <InstancesPage /> },
            { path: "instances/:id", element: <InstanceDetailPage /> },
            { path: "sync-packs", element: <SyncPacksPage /> },
            { path: "accounts", element: <AccountsPage /> },
            { path: "accounts/:id", element: <AccountSkinPage /> },
            { path: "settings", element: <SettingsPage /> },
        ],
    },
]);

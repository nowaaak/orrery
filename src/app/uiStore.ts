import { create } from "zustand";

export type AppDialog =
    "create-instance" | "modpack" | "launcher-log" | "import" | "optifine" | "login";

type UiState = {
    paletteOpen: boolean;
    dialog: AppDialog | null;
    welcomeDismissed: boolean;
    setPaletteOpen: (open: boolean) => void;
    openDialog: (dialog: AppDialog) => void;
    closeDialog: () => void;
    dismissWelcome: () => void;
};

export const useUiStore = create<UiState>((set) => ({
    paletteOpen: false,
    dialog: null,
    welcomeDismissed: false,
    setPaletteOpen: (paletteOpen) => set({ paletteOpen }),
    openDialog: (dialog) => set({ dialog, paletteOpen: false }),
    closeDialog: () => set({ dialog: null }),
    dismissWelcome: () => set({ welcomeDismissed: true }),
}));

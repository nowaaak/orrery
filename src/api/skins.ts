import type { LibrarySkin } from "@/bindings/LibrarySkin";
import type { ProfileView } from "@/bindings/ProfileView";
import { call } from "./invoke";

export const listSkinLibrary = () => call<LibrarySkin[]>("list_skin_library");

export const getLibrarySkin = (id: string) => call<ArrayBuffer>("get_library_skin", { id });

export const importSkinFile = (path: string, slim: boolean) =>
    call<LibrarySkin>("import_skin_file", { path, slim });

export const importPlayerSkin = (player: string) =>
    call<LibrarySkin>("import_player_skin", { player });

export const updateLibrarySkin = (id: string, name: string, slim: boolean) =>
    call<LibrarySkin>("update_library_skin", { id, name, slim });

export const removeLibrarySkin = (id: string) => call<void>("remove_library_skin", { id });

export const applyLibrarySkin = (accountId: string, skinId: string) =>
    call<void>("apply_library_skin", { accountId, skinId });

export const getAccountProfile = (accountId: string) =>
    call<ProfileView>("get_account_profile", { accountId });

export const setAccountCape = (accountId: string, capeId: string | null) =>
    call<void>("set_account_cape", { accountId, capeId });

export const fetchTexture = (url: string) => call<ArrayBuffer>("fetch_texture", { url });

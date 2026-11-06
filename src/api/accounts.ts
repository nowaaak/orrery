import type { AccountsView } from "@/bindings/AccountsView";
import { call } from "./invoke";

export const listAccounts = () => call<AccountsView>("list_accounts");

export const removeAccount = (id: string) => call<void>("remove_account", { id });

export const setActiveAccount = (id: string) => call<void>("set_active_account", { id });

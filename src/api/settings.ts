import type { DataDirMode } from "@/bindings/DataDirMode";
import type { DataDirTarget } from "@/bindings/DataDirTarget";
import type { LauncherPaths } from "@/bindings/LauncherPaths";
import type { StartupIssue } from "@/bindings/StartupIssue";
import type { Settings } from "@/bindings/Settings";
import { call } from "./invoke";

export const getSettings = () => call<Settings>("get_settings");

export const updateSettings = (settings: Settings) =>
    call<Settings>("update_settings", { settings });

export const getPaths = () => call<LauncherPaths>("get_paths");

export const discordAvailable = () => call<boolean>("discord_available");

export const getStartupIssue = () => call<StartupIssue | null>("get_startup_issue");

export const inspectDataDir = (path: string) => call<DataDirTarget>("inspect_data_dir", { path });

export const changeDataDir = (path: string, mode: DataDirMode) =>
    call<number>("change_data_dir", { path, mode });

export const resetDataDir = () => call<void>("reset_data_dir");

export const restartApp = () => call<void>("restart_app");

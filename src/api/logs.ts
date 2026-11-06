import type { SharedLog } from "@/bindings/SharedLog";
import { call } from "./invoke";

export const prepareLogShare = (text: string) => call<string>("prepare_log_share", { text });

export const shareLog = (text: string) => call<SharedLog>("share_log", { text });

export const readLauncherLog = () => call<string[]>("read_launcher_log");

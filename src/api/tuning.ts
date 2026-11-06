import type { SettingsRecommendation } from "@/bindings/SettingsRecommendation";
import { call } from "./invoke";

export const systemMemory = () => call<number | null>("system_memory");

export const recommendSettings = (instanceId: string) =>
    call<SettingsRecommendation>("recommend_settings", { instanceId });

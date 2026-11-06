import type { Screenshot } from "@/bindings/Screenshot";
import { call } from "./invoke";

export const listScreenshots = (id: string) => call<Screenshot[]>("list_screenshots", { id });

export const readScreenshot = (id: string, name: string) =>
    call<ArrayBuffer>("read_screenshot", { id, name });

export const deleteScreenshot = (id: string, name: string) =>
    call<void>("delete_screenshot", { id, name });

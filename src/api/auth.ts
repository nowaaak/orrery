import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { BrowserLoginInfo } from "@/bindings/BrowserLoginInfo";
import type { DeviceCodeInfo } from "@/bindings/DeviceCodeInfo";
import type { LoginEvent } from "@/bindings/LoginEvent";
import type { LoginPage } from "@/bindings/LoginPage";
import { call } from "./invoke";

export const microsoftLoginAvailable = () => call<boolean>("microsoft_login_available");

export const beginMicrosoftLogin = () => call<DeviceCodeInfo>("begin_microsoft_login");

export const beginBrowserLogin = (page: LoginPage) =>
    call<BrowserLoginInfo>("begin_browser_login", { page });

export const cancelMicrosoftLogin = (loginId: string) =>
    call<void>("cancel_microsoft_login", { loginId });

export const onLoginEvent = (handler: (event: LoginEvent) => void): Promise<UnlistenFn> =>
    listen<LoginEvent>("auth:login", (event) => handler(event.payload));

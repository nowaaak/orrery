import type { OptifineJar } from "@/bindings/OptifineJar";
import { call } from "./invoke";

export const inspectOptifine = (path: string) =>
    call<OptifineJar | null>("inspect_optifine", { path });

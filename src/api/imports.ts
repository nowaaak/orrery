import type { ForeignInstance } from "@/bindings/ForeignInstance";
import type { ImportPreview } from "@/bindings/ImportPreview";
import type { ImportTarget } from "@/bindings/ImportTarget";
import { call } from "./invoke";

export const scanImports = () => call<ForeignInstance[]>("scan_imports");

export const inspectImport = (path: string) => call<ForeignInstance[]>("inspect_import", { path });

export const previewImport = (target: ImportTarget) =>
    call<ImportPreview>("preview_import", { target });

export const startImport = (target: ImportTarget, name: string | null) =>
    call<number>("start_import", { target, name });

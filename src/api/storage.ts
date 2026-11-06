import type { CleanupResult } from "@/bindings/CleanupResult";
import type { StorageReport } from "@/bindings/StorageReport";
import { call } from "./invoke";

export const getStorageReport = () => call<StorageReport>("storage_report");

export const cleanStorage = () => call<CleanupResult>("clean_storage");

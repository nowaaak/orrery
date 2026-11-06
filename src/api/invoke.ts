import { invoke, type InvokeArgs } from "@tauri-apps/api/core";
import type { CommandError } from "@/bindings/CommandError";
import { friendlyMessage } from "./errors";

export class AppError extends Error {
    readonly kind: string;

    constructor(kind: string, message: string) {
        super(message);
        this.name = "AppError";
        this.kind = kind;
    }
}

function isCommandError(value: unknown): value is CommandError {
    return (
        typeof value === "object" &&
        value !== null &&
        "kind" in value &&
        "message" in value &&
        typeof value.kind === "string" &&
        typeof value.message === "string"
    );
}

export async function call<T>(command: string, args?: InvokeArgs): Promise<T> {
    try {
        return await invoke<T>(command, args);
    } catch (error) {
        if (isCommandError(error)) {
            throw new AppError(error.kind, error.message);
        }
        throw new AppError("internal", String(error));
    }
}

export function errorMessage(error: unknown): string {
    if (error instanceof AppError) {
        return friendlyMessage(error.kind, error.message);
    }
    if (error instanceof Error) {
        return error.message;
    }
    return String(error);
}

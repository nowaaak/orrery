import { create } from "zustand";
import type { CrashReport } from "@/bindings/CrashReport";
import type { GameStatus } from "@/bindings/GameStatus";

const MAX_LOG_LINES = 5000;

type GameStoreState = {
    running: Record<string, number>;
    logs: Record<string, string[]>;
    stopRequested: Record<string, boolean>;
    crashes: Record<string, CrashReport>;
    companion: Record<string, { modVersion: string; status: GameStatus | null }>;
    openCrash: string | null;
    setCompanion: (instanceId: string, modVersion: string | null) => void;
    setCompanionStatus: (instanceId: string, status: GameStatus) => void;
    setCrash: (instanceId: string, crash: CrashReport) => void;
    clearCrash: (instanceId: string) => void;
    showCrash: (instanceId: string | null) => void;
    setRunning: (instanceId: string, pid: number) => void;
    setStopped: (instanceId: string) => void;
    replaceRunning: (games: { instanceId: string; pid: number }[]) => void;
    appendLog: (instanceId: string, lines: string[]) => void;
    replaceLog: (instanceId: string, lines: string[]) => void;
    markStopRequested: (instanceId: string) => void;
};

function trimLog(lines: string[]): string[] {
    return lines.length > MAX_LOG_LINES ? lines.slice(lines.length - MAX_LOG_LINES) : lines;
}

export const useGameStore = create<GameStoreState>((set) => ({
    running: {},
    logs: {},
    stopRequested: {},
    crashes: {},
    companion: {},
    openCrash: null,
    setCompanion: (instanceId, modVersion) =>
        set((state) => {
            const companion = { ...state.companion };
            if (modVersion === null) {
                delete companion[instanceId];
            } else {
                companion[instanceId] = { modVersion, status: null };
            }
            return { companion };
        }),
    setCompanionStatus: (instanceId, status) =>
        set((state) => {
            const current = state.companion[instanceId];
            return current
                ? { companion: { ...state.companion, [instanceId]: { ...current, status } } }
                : {};
        }),
    setCrash: (instanceId, crash) =>
        set((state) => ({ crashes: { ...state.crashes, [instanceId]: crash } })),
    clearCrash: (instanceId) =>
        set((state) => {
            const crashes = { ...state.crashes };
            delete crashes[instanceId];
            return {
                crashes,
                openCrash: state.openCrash === instanceId ? null : state.openCrash,
            };
        }),
    showCrash: (instanceId) => set(() => ({ openCrash: instanceId })),
    setRunning: (instanceId, pid) =>
        set((state) => ({
            running: { ...state.running, [instanceId]: pid },
            logs: { ...state.logs, [instanceId]: [] },
            stopRequested: { ...state.stopRequested, [instanceId]: false },
        })),
    setStopped: (instanceId) =>
        set((state) => {
            const running = { ...state.running };
            delete running[instanceId];
            const companion = { ...state.companion };
            delete companion[instanceId];
            return { running, companion };
        }),
    replaceRunning: (games) =>
        set(() => ({
            running: Object.fromEntries(games.map((game) => [game.instanceId, game.pid])),
        })),
    appendLog: (instanceId, lines) =>
        set((state) => ({
            logs: {
                ...state.logs,
                [instanceId]: trimLog([...(state.logs[instanceId] ?? []), ...lines]),
            },
        })),
    replaceLog: (instanceId, lines) =>
        set((state) => ({ logs: { ...state.logs, [instanceId]: trimLog(lines) } })),
    markStopRequested: (instanceId) =>
        set((state) => ({ stopRequested: { ...state.stopRequested, [instanceId]: true } })),
}));

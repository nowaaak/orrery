export type LogLevel = "error" | "warn" | "info";
export type LevelFilter = "all" | "warn" | "error";

const ERROR_PATTERN = /\/(ERROR|FATAL)\]|\[(ERROR|FATAL)\]|Exception|^\s+at |^Caused by:/;
const WARN_PATTERN = /\/WARN\]|\[WARN\]/;

export function lineLevel(line: string): LogLevel {
    if (ERROR_PATTERN.test(line)) {
        return "error";
    }
    if (WARN_PATTERN.test(line)) {
        return "warn";
    }
    return "info";
}

export type FilteredLine = { index: number; line: string; level: LogLevel };

export function filterLog(lines: string[], query: string, level: LevelFilter): FilteredLine[] {
    const needle = query.trim().toLowerCase();
    const result: FilteredLine[] = [];
    lines.forEach((line, index) => {
        const lineLevelValue = lineLevel(line);
        if (level === "error" && lineLevelValue !== "error") {
            return;
        }
        if (level === "warn" && lineLevelValue === "info") {
            return;
        }
        if (needle !== "" && !line.toLowerCase().includes(needle)) {
            return;
        }
        result.push({ index, line, level: lineLevelValue });
    });
    return result;
}

export function splitMatches(line: string, query: string): { text: string; match: boolean }[] {
    const needle = query.trim().toLowerCase();
    if (needle === "") {
        return [{ text: line, match: false }];
    }
    const lower = line.toLowerCase();
    const parts: { text: string; match: boolean }[] = [];
    let position = 0;
    let found = lower.indexOf(needle);
    while (found !== -1) {
        if (found > position) {
            parts.push({ text: line.slice(position, found), match: false });
        }
        parts.push({ text: line.slice(found, found + needle.length), match: true });
        position = found + needle.length;
        found = lower.indexOf(needle, position);
    }
    if (position < line.length) {
        parts.push({ text: line.slice(position), match: false });
    }
    return parts;
}

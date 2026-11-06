const BOUNDARY = /[\s\-_:./()]/;

export function normalize(text: string): string {
    return text
        .normalize("NFD")
        .replace(/\p{Diacritic}/gu, "")
        .toLowerCase();
}

function isBoundary(text: string, index: number): boolean {
    return index === 0 || BOUNDARY.test(text[index - 1] ?? "");
}

function scoreWord(word: string, text: string): number | null {
    const direct = text.indexOf(word);
    if (direct >= 0) {
        return 100 + word.length * 4 + (isBoundary(text, direct) ? 50 : 0) - direct * 0.1;
    }
    let score = 0;
    let from = 0;
    let previous = -2;
    for (const char of word) {
        const found = text.indexOf(char, from);
        if (found < 0) {
            return null;
        }
        score += 1;
        if (found === previous + 1) {
            score += 5;
        }
        if (isBoundary(text, found)) {
            score += 8;
        }
        previous = found;
        from = found + 1;
    }
    return score;
}

export function fuzzyScore(query: string, target: string): number | null {
    const words = normalize(query).trim().split(/\s+/).filter(Boolean);
    if (words.length === 0) {
        return 0;
    }
    const text = normalize(target);
    let total = 0;
    for (const word of words) {
        const score = scoreWord(word, text);
        if (score === null) {
            return null;
        }
        total += score;
    }
    return total;
}

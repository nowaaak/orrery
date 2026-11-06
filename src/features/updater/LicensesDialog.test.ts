import { describe, expect, it } from "vitest";
import { filterLicenses } from "./LicensesDialog";

const pkg = (name: string, license: string) => ({
    ecosystem: "cargo" as const,
    name,
    version: "1.0.0",
    license,
    repository: null,
    texts: [],
});

describe("filterLicenses", () => {
    it("matches names and licenses case-insensitively", () => {
        const all = [pkg("serde", "MIT OR Apache-2.0"), pkg("zip", "MIT"), pkg("ring", "ISC")];
        expect(filterLicenses(all, "").length).toBe(3);
        expect(filterLicenses(all, " SER ").map((p) => p.name)).toEqual(["serde"]);
        expect(filterLicenses(all, "isc").map((p) => p.name)).toEqual(["ring"]);
        expect(filterLicenses(all, "mit").map((p) => p.name)).toEqual(["serde", "zip"]);
    });
});

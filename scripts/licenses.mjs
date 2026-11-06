import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const output = join(root, "public", "licenses.json");
const LICENSE_FILE = /^(licen[cs]e|copying|notice|unlicense)([-._].*)?$/i;
const MAX_TEXT_BYTES = 64 * 1024;

const texts = new Map();

function addText(text) {
    const hash = createHash("sha256").update(text).digest("hex").slice(0, 16);
    texts.set(hash, text);
    return hash;
}

function licenseTexts(dir) {
    if (!existsSync(dir)) {
        return [];
    }
    return readdirSync(dir)
        .filter((name) => LICENSE_FILE.test(name))
        .map((name) => join(dir, name))
        .filter((path) => statSync(path).isFile() && statSync(path).size <= MAX_TEXT_BYTES)
        .sort()
        .map((path) => addText(readFileSync(path, "utf8").replace(/\r\n/g, "\n").trim()));
}

function cargoPackages() {
    const metadata = JSON.parse(
        execFileSync(
            process.env.CARGO ?? "cargo",
            ["metadata", "--format-version", "1", "--locked"],
            {
                cwd: root,
                encoding: "utf8",
                maxBuffer: 256 * 1024 * 1024,
            },
        ),
    );
    const packages = new Map(metadata.packages.map((pkg) => [pkg.id, pkg]));
    const nodes = new Map(metadata.resolve.nodes.map((node) => [node.id, node]));
    const members = new Set(metadata.workspace_members);
    const seen = new Set();
    const queue = [...members];
    while (queue.length > 0) {
        const id = queue.pop();
        if (seen.has(id)) {
            continue;
        }
        seen.add(id);
        for (const dep of nodes.get(id)?.deps ?? []) {
            if (dep.dep_kinds.some((kind) => kind.kind !== "dev")) {
                queue.push(dep.pkg);
            }
        }
    }
    return [...seen]
        .filter((id) => !members.has(id))
        .map((id) => packages.get(id))
        .map((pkg) => ({
            ecosystem: "cargo",
            name: pkg.name,
            version: pkg.version,
            license: pkg.license ?? (pkg.license_file ? "see license text" : "unknown"),
            repository: pkg.repository ?? null,
            texts: licenseTexts(dirname(pkg.manifest_path)),
        }));
}

function npmPackages() {
    const lock = JSON.parse(readFileSync(join(root, "package-lock.json"), "utf8"));
    return Object.entries(lock.packages)
        .filter(([path, entry]) => path.startsWith("node_modules/") && !entry.dev)
        .filter(([path]) => existsSync(join(root, path, "package.json")))
        .map(([path, entry]) => {
            const manifest = JSON.parse(readFileSync(join(root, path, "package.json"), "utf8"));
            const repository =
                typeof manifest.repository === "string"
                    ? manifest.repository
                    : (manifest.repository?.url ?? null);
            return {
                ecosystem: "npm",
                name: manifest.name ?? path.slice(path.lastIndexOf("node_modules/") + 13),
                version: manifest.version ?? entry.version,
                license: entry.license ?? manifest.license ?? "unknown",
                repository,
                texts: licenseTexts(join(root, path)),
            };
        });
}

const unique = new Map();
for (const pkg of [...cargoPackages(), ...npmPackages()]) {
    unique.set(`${pkg.ecosystem}:${pkg.name}@${pkg.version}`, pkg);
}
const packages = [...unique.values()].sort(
    (a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version),
);
const used = new Set(packages.flatMap((pkg) => pkg.texts));
const result = {
    packages,
    texts: Object.fromEntries([...texts].filter(([hash]) => used.has(hash))),
};
mkdirSync(dirname(output), { recursive: true });
writeFileSync(output, JSON.stringify(result));
const missing = packages.filter((pkg) => pkg.texts.length === 0).length;
console.log(
    `licenses: ${packages.length} packages, ${used.size} texts, ${missing} without a text file, ${Math.round(JSON.stringify(result).length / 1024)} KB`,
);

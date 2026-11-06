import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { copyFileSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const companion = join(root, "companion");
const built = join(companion, "build", "libs", "companion");
const target = join(root, "src-tauri", "resources", "companion");
const skipBuild = process.argv.includes("--skip-build");

if (!skipBuild) {
    const windows = process.platform === "win32";
    const wrapper = join(companion, windows ? "gradlew.bat" : "gradlew");
    const result = windows
        ? spawnSync(`"${wrapper}" collectJar --console=plain`, {
              cwd: companion,
              stdio: "inherit",
              shell: true,
          })
        : spawnSync(wrapper, ["collectJar", "--console=plain"], {
              cwd: companion,
              stdio: "inherit",
          });
    if (result.status !== 0) {
        process.exit(result.status ?? 1);
    }
}

const version = readFileSync(join(companion, "stonecutter.properties.toml"), "utf8").match(
    /^mod\.version = "([^"]+)"/m,
)?.[1];
if (!version) {
    throw new Error("mod.version is missing in stonecutter.properties.toml");
}

mkdirSync(target, { recursive: true });
for (const name of readdirSync(target)) {
    if (name.endsWith(".jar") || name === "companion.json") {
        rmSync(join(target, name));
    }
}

const jars = readdirSync(built)
    .filter((name) => /^orrery-companion-.+\.jar$/.test(name))
    .sort()
    .map((file) => {
        copyFileSync(join(built, file), join(target, file));
        const sha512 = createHash("sha512")
            .update(readFileSync(join(target, file)))
            .digest("hex");
        const minecraft = file.slice("orrery-companion-".length, -".jar".length);
        return { minecraft, file, sha512 };
    });

writeFileSync(join(target, "companion.json"), `${JSON.stringify({ version, jars }, null, 4)}\n`);
console.log(
    `Bundled ${jars.length} companion jars: ${jars.map((jar) => jar.minecraft).join(", ")}`,
);

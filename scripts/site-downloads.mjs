import { writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const output = join(root, "site", "downloads.json");
const repository = process.env.GITHUB_REPOSITORY ?? "nowaaak/orrery";

const KINDS = [
    {
        id: "windows-exe",
        os: "windows",
        pattern: /_x64-setup\.exe$/,
        name: "Windows",
        detail: "Installer (.exe)",
    },
    {
        id: "windows-msi",
        os: "windows",
        pattern: /_x64_[\w-]+\.msi$/,
        name: "Windows",
        detail: "MSI package",
    },
    {
        id: "mac-arm",
        os: "mac",
        pattern: /_aarch64\.dmg$/,
        name: "macOS Apple Silicon",
        detail: "Disk image (.dmg)",
    },
    {
        id: "mac-intel",
        os: "mac",
        pattern: /_x64\.dmg$/,
        name: "macOS Intel",
        detail: "Disk image (.dmg)",
    },
    {
        id: "linux-appimage",
        os: "linux",
        pattern: /_amd64\.AppImage$/,
        name: "Linux",
        detail: "AppImage, updates itself",
    },
    {
        id: "linux-deb",
        os: "linux",
        pattern: /_amd64\.deb$/,
        name: "Linux",
        detail: "Debian and Ubuntu (.deb)",
    },
    {
        id: "linux-rpm",
        os: "linux",
        pattern: /\.x86_64\.rpm$/,
        name: "Linux",
        detail: "Fedora and openSUSE (.rpm)",
    },
];

export function pickFiles(assets) {
    return KINDS.flatMap((kind) => {
        const asset = assets.find((item) => kind.pattern.test(item.name));
        return asset
            ? [
                  {
                      id: kind.id,
                      os: kind.os,
                      name: kind.name,
                      detail: kind.detail,
                      file: asset.name,
                      url: asset.browser_download_url,
                      size: asset.size,
                  },
              ]
            : [];
    });
}

async function main() {
    const headers = { Accept: "application/vnd.github+json", "User-Agent": "orrery-site" };
    if (process.env.GITHUB_TOKEN) {
        headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`;
    }
    const response = await fetch(`https://api.github.com/repos/${repository}/releases/latest`, {
        headers,
    });
    if (!response.ok) {
        throw new Error(`GitHub answered ${response.status} for the latest release`);
    }
    const release = await response.json();
    const files = pickFiles(release.assets ?? []);
    if (files.length === 0) {
        throw new Error(`the latest release ${release.tag_name} has no installers`);
    }
    const result = {
        version: release.tag_name.replace(/^v/, ""),
        published: release.published_at,
        page: release.html_url,
        files,
    };
    writeFileSync(output, `${JSON.stringify(result, null, 4)}\n`);
    console.log(`downloads: ${result.version} with ${files.length} files`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
    main().catch((error) => {
        console.error(error.message);
        process.exit(1);
    });
}

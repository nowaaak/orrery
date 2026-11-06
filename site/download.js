(() => {
    const section = document.getElementById("download-files");
    if (!section) {
        return;
    }

    const PRIMARY = { windows: "windows-exe", mac: "mac-arm", linux: "linux-appimage" };

    function detectOs() {
        const hint = navigator.userAgentData && navigator.userAgentData.platform;
        const text = hint || navigator.userAgent || "";
        if (/android|iphone|ipad/i.test(text)) {
            return null;
        }
        if (/win/i.test(text)) {
            return "windows";
        }
        if (/mac/i.test(text)) {
            return "mac";
        }
        if (/linux|x11/i.test(text)) {
            return "linux";
        }
        return null;
    }

    function megabytes(bytes) {
        return `${Math.round(bytes / 1024 / 1024)} MB`;
    }

    function element(tag, className, text) {
        const node = document.createElement(tag);
        if (className) {
            node.className = className;
        }
        if (text) {
            node.textContent = text;
        }
        return node;
    }

    function fileLink(file, primary) {
        const link = element("a", primary ? "button primary download-main" : "download-file");
        link.href = file.url;
        if (primary) {
            link.append(element("span", null, `Download for ${file.name}`));
            link.append(
                element("span", "download-meta", `${file.detail} · ${megabytes(file.size)}`),
            );
        } else {
            link.append(element("span", "download-name", file.name));
            link.append(
                element("span", "download-meta", `${file.detail} · ${megabytes(file.size)}`),
            );
        }
        return link;
    }

    fetch("downloads.json", { cache: "no-cache" })
        .then((response) => (response.ok ? response.json() : Promise.reject(response.status)))
        .then((data) => {
            const os = detectOs();
            const primary = data.files.find((file) => file.id === PRIMARY[os]);
            const content = element("div", "download-box");
            content.append(element("p", "download-version", `Version ${data.version}`));
            if (primary) {
                content.append(fileLink(primary, true));
                if (os === "mac") {
                    content.append(
                        element(
                            "p",
                            "note",
                            "For Macs with an Intel processor, take the Intel disk image below.",
                        ),
                    );
                }
            } else {
                content.append(
                    element("p", "note", "Pick the file for your computer from the list below."),
                );
            }
            content.append(element("h3", "download-all", "All files"));
            const list = element("div", "download-list");
            for (const file of data.files) {
                list.append(fileLink(file, false));
            }
            content.append(list);
            const page = element("a", "download-page", "Release notes and older versions");
            page.href = data.page;
            content.append(page);
            section.replaceChildren(content);
        })
        .catch(() => {});
})();

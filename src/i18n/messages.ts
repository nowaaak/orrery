import * as accounts from "./modules/accounts";
import * as app from "./modules/app";
import * as common from "./modules/common";
import * as content from "./modules/content";
import * as drop from "./modules/drop";
import * as errors from "./modules/errors";
import * as imports from "./modules/imports";
import * as instances from "./modules/instances";
import * as launch from "./modules/launch";
import * as links from "./modules/links";
import * as onboarding from "./modules/onboarding";
import * as optifine from "./modules/optifine";
import * as palette from "./modules/palette";
import * as screenshots from "./modules/screenshots";
import * as servers from "./modules/servers";
import * as settings from "./modules/settings";
import * as skins from "./modules/skins";
import * as syncpacks from "./modules/syncpacks";
import * as tasks from "./modules/tasks";
import * as trash from "./modules/trash";
import * as updater from "./modules/updater";
import * as worlds from "./modules/worlds";

const de = {
    accounts: accounts.de,
    app: app.de,
    common: common.de,
    content: content.de,
    drop: drop.de,
    errors: errors.de,
    imports: imports.de,
    instances: instances.de,
    launch: launch.de,
    links: links.de,
    onboarding: onboarding.de,
    optifine: optifine.de,
    palette: palette.de,
    screenshots: screenshots.de,
    servers: servers.de,
    settings: settings.de,
    skins: skins.de,
    syncpacks: syncpacks.de,
    tasks: tasks.de,
    trash: trash.de,
    updater: updater.de,
    worlds: worlds.de,
};

export type Messages = typeof de;
export type Locale = "de" | "en";

const en: Messages = {
    accounts: accounts.en,
    app: app.en,
    common: common.en,
    content: content.en,
    drop: drop.en,
    errors: errors.en,
    imports: imports.en,
    instances: instances.en,
    launch: launch.en,
    links: links.en,
    onboarding: onboarding.en,
    optifine: optifine.en,
    palette: palette.en,
    screenshots: screenshots.en,
    servers: servers.en,
    settings: settings.en,
    skins: skins.en,
    syncpacks: syncpacks.en,
    tasks: tasks.en,
    trash: trash.en,
    updater: updater.en,
    worlds: worlds.en,
};

export const messages: Record<Locale, Messages> = { de, en };

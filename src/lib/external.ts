import { openUrl } from "@tauri-apps/plugin-opener";
import { toast } from "sonner";
import { errorMessage } from "@/api/invoke";
import { getT } from "@/i18n";

export function openExternal(url: string) {
    openUrl(url).catch((error: unknown) =>
        toast.error(getT().common.openFolderFailed, { description: errorMessage(error) }),
    );
}

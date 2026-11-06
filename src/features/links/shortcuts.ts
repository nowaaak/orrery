import { toast } from "sonner";
import { errorMessage } from "@/api/invoke";
import { createShortcut } from "@/api/links";
import { getT } from "@/i18n";

export async function createShortcutWithToast(
    instanceId: string,
    world: string | null = null,
    server: string | null = null,
) {
    try {
        const path = await createShortcut(instanceId, world, server);
        toast.success(getT().links.shortcutCreated, { description: path });
    } catch (error) {
        toast.error(getT().links.shortcutFailed, { description: errorMessage(error) });
    }
}

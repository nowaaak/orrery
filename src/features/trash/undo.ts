import { toast } from "sonner";
import { errorMessage } from "@/api/invoke";
import { restoreTrash } from "@/api/trash";
import type { TrashEntry } from "@/bindings/TrashEntry";
import { getT } from "@/i18n";

const UNDO_MS = 8000;

export const trashKey = ["trash"] as const;

export function showTrashUndo(entry: TrashEntry, onRestored: (restored: TrashEntry) => void) {
    const text = getT().trash;
    toast.success(text.moved(entry.name), {
        duration: UNDO_MS,
        action: {
            label: text.undo,
            onClick: () => {
                restoreTrash(entry.id)
                    .then((restored) => {
                        toast.success(getT().trash.restored(restored.name));
                        onRestored(restored);
                    })
                    .catch((error: unknown) =>
                        toast.error(getT().trash.restoreFailed, {
                            description: errorMessage(error),
                        }),
                    );
            },
        },
    });
}

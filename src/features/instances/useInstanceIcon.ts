import { useQuery } from "@tanstack/react-query";
import { getInstanceIcon } from "@/api/instances";

const MIME: Record<string, string> = {
    png: "image/png",
    jpg: "image/jpeg",
    gif: "image/gif",
    webp: "image/webp",
};

export function useInstanceIcon(id: string, icon: string | null): string | null {
    const query = useQuery({
        queryKey: ["instance-icon", id, icon],
        queryFn: async () => {
            const bytes = await getInstanceIcon(id);
            if (bytes.byteLength === 0) {
                return null;
            }
            const extension = icon?.split(".").pop() ?? "png";
            return URL.createObjectURL(new Blob([bytes], { type: MIME[extension] ?? "image/png" }));
        },
        enabled: icon !== null,
        staleTime: Infinity,
        gcTime: Infinity,
    });
    return icon === null ? null : (query.data ?? null);
}

import { PackageIcon } from "lucide-react";

export function ModIcon({ url }: { url: string | null }) {
    return url ? (
        <img
            src={url}
            alt=""
            loading="lazy"
            className="size-10 shrink-0 rounded-md bg-muted object-cover"
        />
    ) : (
        <span className="flex size-10 shrink-0 items-center justify-center rounded-md bg-muted">
            <PackageIcon className="size-5 text-muted-foreground" aria-hidden />
        </span>
    );
}

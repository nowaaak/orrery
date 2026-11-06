import { LayersIcon } from "lucide-react";
import { useNavigate } from "react-router";
import type { Instance } from "@/bindings/Instance";
import type { SyncSubscription } from "@/bindings/SyncSubscription";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { ModIcon } from "@/features/mods/ModIcon";
import { useT } from "@/i18n";
import { useSetSubscriptions, useSyncPacks } from "./useSyncPacks";

export function InstanceSyncCard({ instance }: { instance: Instance }) {
    const t = useT();
    const text = t.syncpacks;
    const navigate = useNavigate();
    const packs = useSyncPacks();
    const subscribe = useSetSubscriptions();

    if (!packs.data || packs.data.length === 0) {
        return null;
    }
    const subscriptions = instance.syncPacks;
    const available = packs.data.filter(
        (pack) => !subscriptions.some((subscription) => subscription.packId === pack.id),
    );
    const save = (next: SyncSubscription[]) =>
        subscribe.mutate({ instanceId: instance.id, subscriptions: next });

    return (
        <Card>
            <CardHeader className="flex flex-row items-start justify-between gap-4">
                <div className="flex flex-col gap-1.5">
                    <CardTitle>{text.cardTitle}</CardTitle>
                    <CardDescription>{text.cardDescription}</CardDescription>
                </div>
                <Button variant="ghost" onClick={() => void navigate("/sync-packs")}>
                    <LayersIcon data-icon="inline-start" />
                    {text.manage}
                </Button>
            </CardHeader>
            <CardContent className="flex flex-col gap-4">
                {subscriptions.length === 0 && (
                    <p className="text-sm text-muted-foreground">{text.cardEmpty}</p>
                )}
                {subscriptions.map((subscription) => {
                    const pack = packs.data.find((item) => item.id === subscription.packId);
                    if (!pack) {
                        return null;
                    }
                    return (
                        <section key={pack.id} className="flex flex-col gap-2">
                            <div className="flex items-center justify-between gap-3">
                                <p className="text-sm font-medium">
                                    {pack.name}{" "}
                                    <span className="font-normal text-muted-foreground">
                                        · {text.items(pack.items.length)}
                                    </span>
                                </p>
                                <Button
                                    variant="outline"
                                    size="sm"
                                    disabled={subscribe.isPending}
                                    onClick={() =>
                                        save(subscriptions.filter((s) => s.packId !== pack.id))
                                    }
                                >
                                    {text.unsubscribe}
                                </Button>
                            </div>
                            <ul className="flex flex-col divide-y rounded-xl border px-3">
                                {pack.items.map((item) => {
                                    const included = !subscription.excluded.includes(
                                        item.projectId,
                                    );
                                    return (
                                        <li
                                            key={item.projectId}
                                            className="flex items-center gap-3 py-2"
                                        >
                                            <ModIcon url={item.iconUrl} />
                                            <div className="min-w-0 flex-1">
                                                <p className="truncate text-sm">{item.title}</p>
                                                <p className="text-xs text-muted-foreground">
                                                    {text.kindLabel[item.kind]}
                                                </p>
                                            </div>
                                            <Switch
                                                checked={included}
                                                disabled={subscribe.isPending}
                                                aria-label={text.includeItem(item.title)}
                                                onCheckedChange={(on) =>
                                                    save(
                                                        subscriptions.map((s) =>
                                                            s.packId === pack.id
                                                                ? {
                                                                      ...s,
                                                                      excluded: on
                                                                          ? s.excluded.filter(
                                                                                (id) =>
                                                                                    id !==
                                                                                    item.projectId,
                                                                            )
                                                                          : [
                                                                                ...s.excluded,
                                                                                item.projectId,
                                                                            ],
                                                                  }
                                                                : s,
                                                        ),
                                                    )
                                                }
                                            />
                                        </li>
                                    );
                                })}
                            </ul>
                        </section>
                    );
                })}
                {available.length > 0 && (
                    <Select
                        value=""
                        disabled={subscribe.isPending}
                        onValueChange={(packId) =>
                            save([...subscriptions, { packId, excluded: [] }])
                        }
                    >
                        <SelectTrigger className="w-64" aria-label={text.subscribeTo}>
                            <SelectValue placeholder={text.subscribeTo} />
                        </SelectTrigger>
                        <SelectContent>
                            {available.map((pack) => (
                                <SelectItem key={pack.id} value={pack.id}>
                                    {pack.name}
                                </SelectItem>
                            ))}
                        </SelectContent>
                    </Select>
                )}
            </CardContent>
        </Card>
    );
}

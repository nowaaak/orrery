import { useMutation, useQuery, useQueryClient, type UseQueryResult } from "@tanstack/react-query";
import { open } from "@tauri-apps/plugin-dialog";
import { ArrowLeftIcon, FileUpIcon, Trash2Icon, UserSearchIcon } from "lucide-react";
import { useState, type FormEvent } from "react";
import { Link, useParams } from "react-router";
import { toast } from "sonner";
import { errorMessage } from "@/api/invoke";
import {
    applyLibrarySkin,
    fetchTexture,
    getAccountProfile,
    getLibrarySkin,
    importPlayerSkin,
    importSkinFile,
    listSkinLibrary,
    removeLibrarySkin,
    setAccountCape,
    updateLibrarySkin,
} from "@/api/skins";
import type { Account } from "@/bindings/Account";
import type { LibrarySkin } from "@/bindings/LibrarySkin";
import type { ProfileView } from "@/bindings/ProfileView";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { accountsKey, useAccounts } from "@/features/accounts/useAccounts";
import { getT, useT } from "@/i18n";
import { cn } from "@/lib/utils";
import { SkinFront } from "./SkinFront";
import { SkinViewer } from "./SkinViewer";

const libraryKey = ["skin-library"] as const;

function pngBlob(bytes: ArrayBuffer): Blob | null {
    return bytes.byteLength > 0 ? new Blob([bytes], { type: "image/png" }) : null;
}

function useTexture(url: string | null) {
    return useQuery({
        queryKey: ["texture", url],
        queryFn: async () => pngBlob(await fetchTexture(url!)),
        enabled: url !== null,
        staleTime: Infinity,
    });
}

function useLibrarySkin(id: string | null) {
    return useQuery({
        queryKey: ["library-skin", id],
        queryFn: async () => pngBlob(await getLibrarySkin(id!)),
        enabled: id !== null,
        staleTime: Infinity,
    });
}

function reportError(title: string) {
    return (error: unknown) => toast.error(title, { description: errorMessage(error) });
}

export function AccountSkinPage() {
    const t = useT();
    const { id = "" } = useParams();
    const accounts = useAccounts();
    const account = accounts.data?.accounts.find((item) => item.id === id);

    return (
        <div className="flex flex-col gap-6">
            <Button variant="ghost" size="sm" asChild className="self-start">
                <Link to="/accounts">
                    <ArrowLeftIcon data-icon="inline-start" />
                    {t.skins.back}
                </Link>
            </Button>
            {account ? (
                <SkinEditor account={account} />
            ) : accounts.error ? (
                <p className="text-destructive select-text">{errorMessage(accounts.error)}</p>
            ) : (
                <Skeleton className="h-96 w-full rounded-2xl" />
            )}
        </div>
    );
}

function SkinEditor({ account }: { account: Account }) {
    const t = useT();
    const queryClient = useQueryClient();
    const [selected, setSelected] = useState<string | null>(null);
    const library = useQuery({ queryKey: libraryKey, queryFn: listSkinLibrary });
    const profile = useQuery({
        queryKey: ["account-profile", account.id],
        queryFn: () => getAccountProfile(account.id),
        retry: false,
    });
    const remoteUrl = account.skin?.url ?? null;
    const remoteSkin = useTexture(remoteUrl);
    const activeCape = profile.data?.capes.find((cape) => cape.active) ?? null;
    const capeTexture = useTexture(activeCape?.url ?? null);
    const selectedSkin = library.data?.find((skin) => skin.id === selected) ?? null;
    const selectedBlob = useLibrarySkin(selected);

    const viewerSkin = selectedSkin ? (selectedBlob.data ?? null) : (remoteSkin.data ?? null);
    const viewerSlim = selectedSkin ? selectedSkin.slim : (account.skin?.slim ?? false);

    const apply = useMutation({
        mutationFn: (skin: LibrarySkin) => applyLibrarySkin(account.id, skin.id),
        onSuccess: async () => {
            toast.success(getT().skins.applied);
            setSelected(null);
            await Promise.all([
                queryClient.invalidateQueries({ queryKey: accountsKey }),
                queryClient.invalidateQueries({ queryKey: ["skin", account.id] }),
                queryClient.invalidateQueries({ queryKey: ["account-profile", account.id] }),
            ]);
        },
        onError: reportError(t.skins.applyFailed),
    });

    return (
        <>
            <header className="flex flex-col gap-1">
                <h1 className="text-2xl font-semibold tracking-tight">
                    {t.skins.title(account.username)}
                </h1>
                <p className="text-sm text-muted-foreground">{t.skins.microsoftUpload}</p>
            </header>
            <div className="grid items-start gap-6 lg:grid-cols-[minmax(0,22rem)_minmax(0,1fr)]">
                <Card>
                    <CardHeader>
                        <CardTitle>{selectedSkin?.name ?? t.skins.current}</CardTitle>
                        {!selectedSkin && !remoteSkin.data && (
                            <CardDescription>{t.skins.noSkin}</CardDescription>
                        )}
                    </CardHeader>
                    <CardContent className="flex flex-col gap-4">
                        <SkinViewer
                            skin={viewerSkin}
                            cape={capeTexture.data ?? null}
                            slim={viewerSlim}
                            label={
                                selectedSkin
                                    ? t.skins.previewLabel(selectedSkin.name)
                                    : t.skins.viewerLabel(account.username)
                            }
                            className="h-96 w-full bg-gradient-to-b from-muted/60 to-muted/10"
                        />
                        {selectedSkin && (
                            <SelectedSkinControls
                                skin={selectedSkin}
                                pending={apply.isPending}
                                onApply={() => apply.mutate(selectedSkin)}
                            />
                        )}
                    </CardContent>
                </Card>
                <div className="flex flex-col gap-6">
                    <LibraryCard selected={selected} onSelect={setSelected} />
                    <CapesCard account={account} profile={profile} />
                </div>
            </div>
        </>
    );
}

function SelectedSkinControls({
    skin,
    pending,
    onApply,
}: {
    skin: LibrarySkin;
    pending: boolean;
    onApply: () => void;
}) {
    const t = useT();
    const queryClient = useQueryClient();
    const update = useMutation({
        mutationFn: (slim: boolean) => updateLibrarySkin(skin.id, skin.name, slim),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: libraryKey }),
        onError: reportError(t.skins.applyFailed),
    });
    return (
        <div className="flex flex-col gap-3">
            <div className="flex items-center justify-between gap-3">
                <Label htmlFor="skin-slim">{t.skins.importSlim}</Label>
                <Switch
                    id="skin-slim"
                    checked={skin.slim}
                    disabled={update.isPending}
                    onCheckedChange={(slim) => update.mutate(slim)}
                />
            </div>
            <Button disabled={pending} onClick={onApply}>
                {pending ? t.skins.applying : t.skins.apply}
            </Button>
        </div>
    );
}

function LibraryCard({
    selected,
    onSelect,
}: {
    selected: string | null;
    onSelect: (id: string | null) => void;
}) {
    const t = useT();
    const queryClient = useQueryClient();
    const library = useQuery({ queryKey: libraryKey, queryFn: listSkinLibrary });
    const [player, setPlayer] = useState("");
    const [slim, setSlim] = useState(false);
    const refresh = () => queryClient.invalidateQueries({ queryKey: libraryKey });
    const imported = (skin: LibrarySkin) => {
        toast.success(getT().skins.imported(skin.name));
        onSelect(skin.id);
        return refresh();
    };
    const fromFile = useMutation({
        mutationFn: (path: string) => importSkinFile(path, slim),
        onSuccess: imported,
        onError: reportError(t.skins.importFailed),
    });
    const fromPlayer = useMutation({
        mutationFn: importPlayerSkin,
        onSuccess: async (skin) => {
            setPlayer("");
            await imported(skin);
        },
        onError: reportError(t.skins.importFailed),
    });
    const remove = useMutation({
        mutationFn: (skin: LibrarySkin) => removeLibrarySkin(skin.id),
        onSuccess: async (_, skin) => {
            toast.success(getT().skins.removed(skin.name));
            if (selected === skin.id) onSelect(null);
            await refresh();
        },
        onError: reportError(t.skins.removeFailed),
    });

    const chooseFile = async () => {
        const path = await open({
            multiple: false,
            directory: false,
            filters: [{ name: t.skins.fileFilter, extensions: ["png"] }],
        });
        if (typeof path === "string") fromFile.mutate(path);
    };

    const onPlayer = (event: FormEvent) => {
        event.preventDefault();
        if (player.trim() !== "") fromPlayer.mutate(player.trim());
    };

    return (
        <Card>
            <CardHeader>
                <CardTitle>{t.skins.library}</CardTitle>
                <CardDescription>{t.skins.libraryDescription}</CardDescription>
            </CardHeader>
            <CardContent className="flex flex-col gap-4">
                <div className="flex flex-wrap items-center gap-3">
                    <Button
                        variant="outline"
                        disabled={fromFile.isPending}
                        onClick={() => void chooseFile()}
                    >
                        <FileUpIcon data-icon="inline-start" />
                        {fromFile.isPending ? t.skins.importing : t.skins.importFile}
                    </Button>
                    <div className="flex items-center gap-2">
                        <Switch id="import-slim" checked={slim} onCheckedChange={setSlim} />
                        <Label htmlFor="import-slim">{t.skins.importSlim}</Label>
                    </div>
                    <form onSubmit={onPlayer} className="ml-auto flex gap-2">
                        <Input
                            aria-label={t.skins.importPlayer}
                            value={player}
                            placeholder={t.skins.playerPlaceholder}
                            maxLength={16}
                            autoComplete="off"
                            spellCheck={false}
                            className="w-40"
                            onChange={(event) => setPlayer(event.target.value)}
                        />
                        <Button
                            type="submit"
                            variant="outline"
                            disabled={fromPlayer.isPending || player.trim() === ""}
                        >
                            <UserSearchIcon data-icon="inline-start" />
                            {fromPlayer.isPending ? t.skins.importing : t.skins.importPlayer}
                        </Button>
                    </form>
                </div>
                {library.error ? (
                    <p className="text-sm text-destructive select-text">
                        {errorMessage(library.error)}
                    </p>
                ) : !library.data ? (
                    <Skeleton className="h-40 w-full" />
                ) : library.data.length === 0 ? (
                    <p className="rounded-xl border border-dashed p-6 text-center text-sm text-muted-foreground">
                        {t.skins.empty}
                    </p>
                ) : (
                    <ul className="grid grid-cols-[repeat(auto-fill,minmax(7rem,1fr))] gap-3">
                        {library.data.map((skin) => (
                            <LibraryTile
                                key={skin.id}
                                skin={skin}
                                selected={selected === skin.id}
                                removing={remove.isPending}
                                onSelect={() => onSelect(selected === skin.id ? null : skin.id)}
                                onRemove={() => remove.mutate(skin)}
                            />
                        ))}
                    </ul>
                )}
            </CardContent>
        </Card>
    );
}

function LibraryTile({
    skin,
    selected,
    removing,
    onSelect,
    onRemove,
}: {
    skin: LibrarySkin;
    selected: boolean;
    removing: boolean;
    onSelect: () => void;
    onRemove: () => void;
}) {
    const t = useT();
    const blob = useLibrarySkin(skin.id);
    return (
        <li className="relative">
            <button
                type="button"
                aria-pressed={selected}
                onClick={onSelect}
                className={cn(
                    "flex w-full flex-col items-center gap-2 rounded-xl border bg-muted/30 p-3 outline-none hover:border-foreground/20 focus-visible:ring-2 focus-visible:ring-ring",
                    selected && "border-primary ring-1 ring-primary",
                )}
            >
                <SkinFront skin={blob.data} slim={skin.slim} className="h-24 w-12" />
                <span className="w-full truncate text-xs font-medium">{skin.name}</span>
                <Badge variant="outline" className="text-[10px]">
                    {skin.slim ? t.skins.slim : t.skins.classic}
                </Badge>
            </button>
            <Button
                variant="ghost"
                size="icon-sm"
                aria-label={t.skins.remove(skin.name)}
                disabled={removing}
                onClick={onRemove}
                className="absolute top-1 right-1"
            >
                <Trash2Icon />
            </Button>
        </li>
    );
}

function CapesCard({
    account,
    profile,
}: {
    account: Account;
    profile: UseQueryResult<ProfileView>;
}) {
    const t = useT();
    const queryClient = useQueryClient();
    const change = useMutation({
        mutationFn: (capeId: string | null) => setAccountCape(account.id, capeId),
        onSuccess: async () => {
            toast.success(getT().skins.capeSaved);
            await queryClient.invalidateQueries({ queryKey: ["account-profile", account.id] });
        },
        onError: reportError(t.skins.capeFailed),
    });
    const capes = profile.data?.capes ?? [];
    const active = capes.find((cape) => cape.active)?.id ?? null;

    return (
        <Card>
            <CardHeader>
                <CardTitle>{t.skins.capes}</CardTitle>
                <CardDescription>{t.skins.capesDescription}</CardDescription>
            </CardHeader>
            <CardContent>
                {profile.isPending ? (
                    <p className="text-sm text-muted-foreground">{t.skins.capesLoading}</p>
                ) : profile.error ? (
                    <p className="text-sm text-destructive select-text">
                        {t.skins.capesFailed}: {errorMessage(profile.error)}
                    </p>
                ) : (
                    <div
                        role="radiogroup"
                        aria-label={t.skins.capes}
                        className="flex flex-wrap gap-2"
                    >
                        {[
                            { id: null, label: t.skins.noCape },
                            ...capes.map((cape) => ({ id: cape.id, label: cape.alias ?? cape.id })),
                        ].map((option) => (
                            <Button
                                key={option.id ?? "none"}
                                role="radio"
                                aria-checked={active === option.id}
                                variant={active === option.id ? "default" : "outline"}
                                size="sm"
                                disabled={change.isPending}
                                onClick={() => change.mutate(option.id)}
                            >
                                {option.label}
                            </Button>
                        ))}
                    </div>
                )}
            </CardContent>
        </Card>
    );
}

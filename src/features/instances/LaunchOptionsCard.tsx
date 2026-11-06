import { PlusIcon, Trash2Icon } from "lucide-react";
import type { EnvVar } from "@/bindings/EnvVar";
import type { LaunchSettings } from "@/bindings/LaunchSettings";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from "@/components/ui/select";
import { Separator } from "@/components/ui/separator";
import { Switch } from "@/components/ui/switch";
import { useAccounts } from "@/features/accounts/useAccounts";
import { useT } from "@/i18n";

const IS_LINUX = navigator.userAgent.includes("Linux") && !navigator.userAgent.includes("Android");
const ACTIVE_ACCOUNT = "__active";
const PLACEHOLDERS = [
    "$INST_NAME",
    "$INST_ID",
    "$INST_DIR",
    "$INST_MC_DIR",
    "$INST_JAVA",
    "$INST_JAVA_ARGS",
];

export type LaunchDraft = {
    accountId: string;
    joinServer: string;
    gameArgs: string;
    env: EnvVar[];
    wrapper: string;
    preLaunch: string;
    postExit: string;
    gamemode: boolean;
    mangohud: boolean;
    primeOffload: boolean;
    companion: boolean;
};

export function toLaunchDraft(launch: LaunchSettings): LaunchDraft {
    return {
        accountId: launch.accountId ?? "",
        joinServer: launch.joinServer ?? "",
        gameArgs: launch.extraGameArgs.join(" "),
        env: launch.env,
        wrapper: launch.wrapper ?? "",
        preLaunch: launch.preLaunch ?? "",
        postExit: launch.postExit ?? "",
        gamemode: launch.gamemode,
        mangohud: launch.mangohud,
        primeOffload: launch.primeOffload,
        companion: !launch.companionDisabled,
    };
}

const optional = (value: string) => (value.trim() === "" ? null : value.trim());

export function toLaunchSettings(draft: LaunchDraft): LaunchSettings {
    return {
        accountId: optional(draft.accountId),
        joinServer: optional(draft.joinServer),
        extraGameArgs: draft.gameArgs.split(/\s+/).filter((arg) => arg !== ""),
        env: draft.env
            .map((variable) => ({ key: variable.key.trim(), value: variable.value }))
            .filter((variable) => variable.key !== "" || variable.value !== ""),
        wrapper: optional(draft.wrapper),
        preLaunch: optional(draft.preLaunch),
        postExit: optional(draft.postExit),
        gamemode: draft.gamemode,
        mangohud: draft.mangohud,
        primeOffload: draft.primeOffload,
        companionDisabled: !draft.companion,
    };
}

type LaunchOptionsCardProps = {
    value: LaunchDraft;
    onChange: (patch: Partial<LaunchDraft>) => void;
    companionAvailable: boolean;
};

export function LaunchOptionsCard({ value, onChange, companionAvailable }: LaunchOptionsCardProps) {
    const t = useT();
    const text = t.instances.launch;
    const accounts = useAccounts();
    const savedAccountMissing =
        value.accountId !== "" &&
        accounts.isSuccess &&
        !accounts.data.accounts.some((account) => account.id === value.accountId);

    const setEnv = (index: number, patch: Partial<EnvVar>) =>
        onChange({
            env: value.env.map((variable, i) =>
                i === index ? { ...variable, ...patch } : variable,
            ),
        });

    return (
        <Card>
            <CardHeader>
                <CardTitle>{text.title}</CardTitle>
                <CardDescription>{text.description}</CardDescription>
            </CardHeader>
            <CardContent className="flex flex-col gap-4">
                <div className="grid gap-2">
                    <Label htmlFor="launch-account">{text.account}</Label>
                    <Select
                        value={value.accountId === "" ? ACTIVE_ACCOUNT : value.accountId}
                        onValueChange={(next) =>
                            onChange({ accountId: next === ACTIVE_ACCOUNT ? "" : next })
                        }
                    >
                        <SelectTrigger id="launch-account" className="w-80">
                            <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                            <SelectItem value={ACTIVE_ACCOUNT}>{text.accountActive}</SelectItem>
                            {accounts.data?.accounts.map((account) => (
                                <SelectItem key={account.id} value={account.id}>
                                    {account.username}
                                </SelectItem>
                            ))}
                            {savedAccountMissing && (
                                <SelectItem value={value.accountId}>
                                    {text.accountMissing}
                                </SelectItem>
                            )}
                        </SelectContent>
                    </Select>
                </div>

                {companionAvailable && (
                    <SwitchRow
                        id="companion"
                        label={text.companion}
                        description={text.companionDescription}
                        checked={value.companion}
                        onChange={(companion) => onChange({ companion })}
                    />
                )}

                <div className="grid gap-2">
                    <Label htmlFor="join-server">{text.joinServer}</Label>
                    <Input
                        id="join-server"
                        value={value.joinServer}
                        placeholder={text.joinServerPlaceholder}
                        spellCheck={false}
                        autoComplete="off"
                        aria-describedby="join-server-hint"
                        onChange={(event) => onChange({ joinServer: event.target.value })}
                    />
                    <p id="join-server-hint" className="text-sm text-muted-foreground">
                        {text.joinServerHint}
                    </p>
                </div>

                <div className="grid gap-2">
                    <Label htmlFor="game-args">{text.gameArgs}</Label>
                    <Input
                        id="game-args"
                        value={value.gameArgs}
                        placeholder="--fullscreen"
                        spellCheck={false}
                        className="font-mono"
                        onChange={(event) => onChange({ gameArgs: event.target.value })}
                    />
                </div>

                <fieldset className="grid gap-2">
                    <legend className="mb-2 text-sm font-medium">{text.env}</legend>
                    {value.env.map((variable, index) => (
                        <div key={index} className="flex gap-2">
                            <Input
                                aria-label={text.envKey}
                                value={variable.key}
                                placeholder={text.envKey}
                                spellCheck={false}
                                className="w-56 font-mono"
                                onChange={(event) => setEnv(index, { key: event.target.value })}
                            />
                            <Input
                                aria-label={text.envValue}
                                value={variable.value}
                                placeholder={text.envValue}
                                spellCheck={false}
                                className="min-w-0 flex-1 font-mono"
                                onChange={(event) => setEnv(index, { value: event.target.value })}
                            />
                            <Button
                                type="button"
                                variant="ghost"
                                size="icon"
                                aria-label={text.envRemove(variable.key)}
                                onClick={() =>
                                    onChange({ env: value.env.filter((_, i) => i !== index) })
                                }
                            >
                                <Trash2Icon />
                            </Button>
                        </div>
                    ))}
                    <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        className="justify-self-start"
                        onClick={() => onChange({ env: [...value.env, { key: "", value: "" }] })}
                    >
                        <PlusIcon data-icon="inline-start" />
                        {text.envAdd}
                    </Button>
                </fieldset>

                <Separator />

                <div className="flex flex-col gap-1">
                    <p className="text-sm font-medium">{text.commands}</p>
                    <p className="text-sm text-pretty text-muted-foreground">
                        {text.commandsDescription}
                    </p>
                    <p className="flex flex-wrap items-center gap-1.5 text-sm text-muted-foreground">
                        {text.placeholders}
                        {PLACEHOLDERS.map((placeholder) => (
                            <code
                                key={placeholder}
                                className="rounded bg-muted px-1.5 py-0.5 font-mono text-xs select-all"
                            >
                                {placeholder}
                            </code>
                        ))}
                    </p>
                </div>
                <CommandInput
                    id="pre-launch"
                    label={text.preLaunch}
                    value={value.preLaunch}
                    onChange={(preLaunch) => onChange({ preLaunch })}
                />
                <CommandInput
                    id="wrapper"
                    label={text.wrapper}
                    value={value.wrapper}
                    placeholder={text.wrapperPlaceholder}
                    onChange={(wrapper) => onChange({ wrapper })}
                />
                <CommandInput
                    id="post-exit"
                    label={text.postExit}
                    value={value.postExit}
                    onChange={(postExit) => onChange({ postExit })}
                />

                {IS_LINUX && (
                    <>
                        <Separator />
                        <p className="text-sm font-medium">{text.linux}</p>
                        <SwitchRow
                            id="gamemode"
                            label={text.gamemode}
                            description={text.gamemodeDescription}
                            checked={value.gamemode}
                            onChange={(gamemode) => onChange({ gamemode })}
                        />
                        <SwitchRow
                            id="mangohud"
                            label={text.mangohud}
                            description={text.mangohudDescription}
                            checked={value.mangohud}
                            onChange={(mangohud) => onChange({ mangohud })}
                        />
                        <SwitchRow
                            id="prime-offload"
                            label={text.prime}
                            description={text.primeDescription}
                            checked={value.primeOffload}
                            onChange={(primeOffload) => onChange({ primeOffload })}
                        />
                    </>
                )}
            </CardContent>
        </Card>
    );
}

function CommandInput(props: {
    id: string;
    label: string;
    value: string;
    placeholder?: string;
    onChange: (value: string) => void;
}) {
    return (
        <div className="grid gap-2">
            <Label htmlFor={props.id}>{props.label}</Label>
            <Input
                id={props.id}
                value={props.value}
                placeholder={props.placeholder}
                spellCheck={false}
                autoComplete="off"
                className="font-mono"
                onChange={(event) => props.onChange(event.target.value)}
            />
        </div>
    );
}

function SwitchRow(props: {
    id: string;
    label: string;
    description: string;
    checked: boolean;
    onChange: (checked: boolean) => void;
}) {
    return (
        <div className="flex items-center justify-between gap-4">
            <div>
                <Label htmlFor={props.id}>{props.label}</Label>
                <p className="text-sm text-muted-foreground">{props.description}</p>
            </div>
            <Switch id={props.id} checked={props.checked} onCheckedChange={props.onChange} />
        </div>
    );
}

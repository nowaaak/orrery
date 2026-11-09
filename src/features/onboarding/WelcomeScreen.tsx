import { BoxesIcon, LogInIcon, PackageIcon, ShieldCheckIcon, SparklesIcon } from "lucide-react";
import { useUiStore } from "@/app/uiStore";
import { OrreryLogo } from "@/components/brand/OrreryLogo";
import { Button } from "@/components/ui/button";
import { LoginDialog, useMicrosoftLogin } from "@/features/accounts/MicrosoftLoginCard";
import { UpdateNotice } from "@/features/updater/UpdateNotice";
import { useT } from "@/i18n";
import panorama from "../../../menupack/assets/minecraft/textures/gui/title/background/panorama_0.png";

export function WelcomeScreen() {
    const t = useT();
    const text = t.onboarding;
    const login = useMicrosoftLogin();

    const dismiss = useUiStore((state) => state.dismissWelcome);

    const features = [
        { icon: BoxesIcon, label: text.features.loaders },
        { icon: PackageIcon, label: text.features.content },
        { icon: SparklesIcon, label: text.features.menu },
        { icon: ShieldCheckIcon, label: text.features.privacy },
    ];

    return (
        <div className="col-span-2 grid min-h-0 grid-cols-[minmax(0,1fr)_24rem] overflow-hidden rounded-t-2xl border-t border-border bg-background">
            <section className="relative flex min-h-0 flex-col justify-between gap-10 overflow-y-auto p-10 text-white">
                <img
                    src={panorama}
                    alt=""
                    className="pointer-events-none absolute inset-0 size-full object-cover"
                />
                <div
                    aria-hidden
                    className="pointer-events-none absolute inset-0 bg-linear-to-t from-black/85 via-black/40 to-black/10"
                />
                <OrreryLogo size={56} className="relative" />
                <div className="relative flex max-w-xl flex-col gap-6">
                    <h1 className="text-4xl font-semibold tracking-tight text-balance">
                        {text.headline}
                    </h1>
                    <p className="text-base text-pretty text-white/75">{text.tagline}</p>
                    <ul className="grid grid-cols-2 gap-2">
                        {features.map((feature) => (
                            <li
                                key={feature.label}
                                className="flex items-center gap-3 rounded-xl border border-white/15 bg-black/35 px-3 py-2.5 text-sm backdrop-blur-sm"
                            >
                                <feature.icon
                                    className="size-4 shrink-0 text-white/70"
                                    aria-hidden
                                />
                                {feature.label}
                            </li>
                        ))}
                    </ul>
                    <p className="text-xs text-pretty text-white/60">
                        {t.updater.about.disclaimer}
                    </p>
                </div>
            </section>
            <aside className="flex min-h-0 flex-col justify-center gap-6 overflow-y-auto border-l bg-sidebar p-8">
                <div className="flex flex-col gap-2 text-center">
                    <h2 className="text-2xl font-semibold tracking-tight text-balance">
                        {text.welcome}
                    </h2>
                    <p className="text-sm text-pretty text-muted-foreground">{text.signInHint}</p>
                </div>
                <div className="flex flex-col gap-2">
                    <Button
                        size="lg"
                        disabled={login.starting || !login.available}
                        onClick={() => void login.start()}
                    >
                        <LogInIcon data-icon="inline-start" />
                        {text.signIn}
                    </Button>
                    <Button variant="ghost" size="lg" onClick={dismiss}>
                        {text.continueWithout}
                    </Button>
                </div>
                {login.available === false && (
                    <p className="text-center text-xs text-muted-foreground">{text.noClientId}</p>
                )}
                <p className="border-t pt-4 text-center text-xs text-pretty text-muted-foreground">
                    {text.withoutHint}
                </p>
                <UpdateNotice />
            </aside>
            <LoginDialog login={login} onSuccess={dismiss} />
        </div>
    );
}

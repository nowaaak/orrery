export const de = {
    headline: "Ein unabhängiger Launcher für Minecraft: Java Edition.",
    tagline:
        "Instanzen, Mods, Modpacks und ein eigenes Menü. Ohne Server von Orrery, ohne Werbung, Open Source.",
    features: {
        loaders: "Fabric, Quilt, Forge und NeoForge",
        content: "Mods von Modrinth mit einem Klick",
        menu: "Eigenes Hauptmenü eingebaut",
        privacy: "Keine Telemetrie, kein Konto bei uns",
    },
    welcome: "Willkommen",
    signInHint:
        "Melde dich mit dem Microsoft-Konto an, mit dem du Minecraft: Java Edition gekauft hast.",
    signIn: "Mit Microsoft anmelden",
    continueWithout: "Ohne Konto fortfahren",
    withoutHint:
        "Ohne Konto kannst du dich umsehen, Instanzen anlegen und Inhalte installieren. Zum Spielen brauchst du die Anmeldung.",
    noClientId:
        "In diesem Build ist keine Microsoft-Client-ID hinterlegt, deshalb ist keine Anmeldung möglich.",
    requiredTitle: "Anmeldung zum Spielen",
    requiredDescription: (instance: string) =>
        `Um ${instance} zu starten, brauchst du ein Microsoft-Konto, das Minecraft: Java Edition besitzt. Nach der Anmeldung startet das Spiel von selbst.`,
    requiredFallback: "das Spiel",
};

export const en: typeof de = {
    headline: "An independent launcher for Minecraft: Java Edition.",
    tagline: "Instances, mods, modpacks and an own menu. No Orrery servers, no ads, open source.",
    features: {
        loaders: "Fabric, Quilt, Forge and NeoForge",
        content: "Mods from Modrinth in one click",
        menu: "Own main menu built in",
        privacy: "No telemetry, no account with us",
    },
    welcome: "Welcome",
    signInHint: "Sign in with the Microsoft account you bought Minecraft: Java Edition with.",
    signIn: "Sign in with Microsoft",
    continueWithout: "Continue without an account",
    withoutHint:
        "Without an account you can look around, create instances and install content. Playing needs the sign-in.",
    noClientId: "This build has no Microsoft client id, so signing in is not possible.",
    requiredTitle: "Sign in to play",
    requiredDescription: (instance) =>
        `To start ${instance} you need a Microsoft account that owns Minecraft: Java Edition. The game starts by itself after you signed in.`,
    requiredFallback: "the game",
};

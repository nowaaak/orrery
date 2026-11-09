import type { Account } from "@/bindings/Account";

export const de = {
    kinds: {
        microsoft: "Microsoft",
    } satisfies Record<Account["kind"], string>,
    title: "Accounts",
    yourAccounts: "Deine Accounts",
    activeHint: "Der aktive Account wird beim Spielstart verwendet.",
    empty: "Noch kein Account angelegt.",
    active: "Aktiv",
    activate: "Aktivieren",
    switchFailed: "Wechsel fehlgeschlagen",
    chooseSkin: "Skin wählen",
    removeAccount: (name: string) => `${name} entfernen`,
    remove: "Entfernen",
    confirmRemoveTitle: (name: string) => `${name} entfernen?`,
    removeMicrosoftDescription:
        "Die gespeicherte Anmeldung wird gelöscht. Du kannst dich jederzeit erneut anmelden.",
    removeFailed: "Entfernen fehlgeschlagen",
    microsoft: {
        startFailed: "Anmeldung konnte nicht gestartet werden",
        title: "Microsoft-Account hinzufügen",
        description:
            "Anmeldung im Browser auf Microsofts eigener Seite. Unabhängig vom Windows-, Store- oder Xbox-Login. Die Anmeldung wird im Schlüsselbund des Systems gespeichert.",
        noClientId:
            "In diesem Build ist keine Microsoft-Client-ID hinterlegt. Setze beim Bauen die Umgebungsvariable MS_CLIENT_ID.",
        signIn: "Mit Microsoft anmelden",
        signedInAs: (name: string) => `Angemeldet als ${name}`,
        failed: "Anmeldung fehlgeschlagen",
        codeCopied: "Code kopiert",
        copyFailed: "Kopieren nicht möglich",
        openPageFailed: "Anmeldeseite konnte nicht geöffnet werden",
        openPageManually: (uri: string, error: string) =>
            `Öffne ${uri} bitte selbst im Browser. (${error})`,
        dialogTitle: "Bei Microsoft anmelden",
        dialogDescription:
            "Öffne die Anmeldeseite und gib diesen Code ein. Danach geht es hier automatisch weiter.",
        codeLabel: "Anmeldecode",
        copyCode: "Code kopieren",
        openPage: "Anmeldeseite öffnen",
        orOpenInBrowser: "Oder im Browser öffnen:",
        waiting: "Warte auf die Anmeldung …",
        browserTitle: "Im Browser anmelden",
        browserDescription:
            "Wähle im Browser das Microsoft-Konto, mit dem du Minecraft: Java Edition gekauft hast. Danach geht es hier automatisch weiter.",
        reopenBrowser: "Browser erneut öffnen",
        useCode: "Stattdessen mit Code anmelden",
        pageSuccess: "Du kannst dieses Fenster schließen und zu Orrery zurückkehren.",
        pageFailure:
            "Die Anmeldung wurde nicht abgeschlossen. Du kannst dieses Fenster schließen und es in Orrery erneut versuchen.",
    },
    switcher: {
        noAccount: "Kein Account",
        createAccount: "Account anlegen",
        switchAccount: "Account wechseln",
        activeLabel: "aktiv",
        manage: "Accounts verwalten",
    },
};

export const en: typeof de = {
    kinds: {
        microsoft: "Microsoft",
    },
    title: "Accounts",
    yourAccounts: "Your accounts",
    activeHint: "The active account is used when you start the game.",
    empty: "No accounts added yet.",
    active: "Active",
    activate: "Activate",
    switchFailed: "Could not switch accounts",
    chooseSkin: "Choose skin",
    removeAccount: (name) => `Remove ${name}`,
    remove: "Remove",
    confirmRemoveTitle: (name) => `Remove ${name}?`,
    removeMicrosoftDescription:
        "The saved sign-in will be deleted. You can sign in again at any time.",
    removeFailed: "Could not remove the account",
    microsoft: {
        startFailed: "Could not start the sign-in",
        title: "Add Microsoft account",
        description:
            "Sign in on Microsoft's own page in your browser. Separate from your Windows, Store or Xbox sign-in. The sign-in is stored in the system keychain.",
        noClientId:
            "This build has no Microsoft client ID. Set the MS_CLIENT_ID environment variable when building.",
        signIn: "Sign in with Microsoft",
        signedInAs: (name) => `Signed in as ${name}`,
        failed: "Sign-in failed",
        codeCopied: "Code copied",
        copyFailed: "Could not copy the code",
        openPageFailed: "Could not open the sign-in page",
        openPageManually: (uri, error) => `Please open ${uri} in your browser. (${error})`,
        dialogTitle: "Sign in with Microsoft",
        dialogDescription:
            "Open the sign-in page and enter this code. The launcher continues automatically once you are done.",
        codeLabel: "Sign-in code",
        copyCode: "Copy code",
        openPage: "Open sign-in page",
        orOpenInBrowser: "Or open in your browser:",
        waiting: "Waiting for sign-in …",
        browserTitle: "Sign in with your browser",
        browserDescription:
            "In your browser, choose the Microsoft account you bought Minecraft: Java Edition with. The launcher continues automatically once you are done.",
        reopenBrowser: "Open browser again",
        useCode: "Sign in with a code instead",
        pageSuccess: "You can close this window and return to Orrery.",
        pageFailure:
            "The sign-in was not completed. You can close this window and try again in Orrery.",
    },
    switcher: {
        noAccount: "No account",
        createAccount: "Add account",
        switchAccount: "Switch account",
        activeLabel: "active",
        manage: "Manage accounts",
    },
};

export const de = {
    friendly: {
        auth_not_configured:
            "Dieser Build hat keine Microsoft-Client-ID. Setze MS_CLIENT_ID beim Bauen des Launchers.",
        auth_declined: "Die Anmeldung wurde im Browser abgelehnt.",
        auth_expired: "Der Anmeldecode ist abgelaufen. Bitte starte die Anmeldung erneut.",
        auth_no_xbox_account:
            "Dieses Microsoft-Konto hat noch kein Xbox-Profil. Lege es auf xbox.com an und versuche es erneut.",
        auth_child_account:
            "Kinderkonten müssen von einem Erwachsenen zu einer Microsoft-Familie hinzugefügt werden.",
        auth_region_unavailable: "Xbox Live ist in deiner Region nicht verfügbar.",
        auth_adult_verification: "Für dieses Konto ist eine Altersverifizierung bei Xbox nötig.",
        auth_app_not_approved:
            "Die Minecraft-Dienste haben diese App-Registrierung abgelehnt. Sie muss erst von Mojang freigeschaltet werden.",
        auth_no_minecraft_profile:
            "Dieses Konto besitzt Minecraft: Java Edition nicht oder hat noch keinen Spielernamen.",
        auth_session_expired:
            "Die gespeicherte Anmeldung ist abgelaufen. Bitte melde dich erneut an.",
        auth_service: "Die Anmeldung bei Microsoft, Xbox oder Minecraft ist fehlgeschlagen.",
        secret_store:
            "Auf den Schlüsselbund des Systems kann nicht zugegriffen werden. Unter Linux wird ein Secret Service wie GNOME Keyring oder KWallet benötigt.",
        network: "Keine Verbindung zum Server. Prüfe deine Internetverbindung.",
        checksum: "Eine heruntergeladene Datei war beschädigt. Bitte versuche es erneut.",
        launch_hook:
            "Ein eigener Befehl der Instanz ist fehlgeschlagen. Du findest ihn in den Einstellungen der Instanz unter Start.",
    },
};

export const en: typeof de = {
    friendly: {
        auth_not_configured:
            "This build has no Microsoft client ID. Set MS_CLIENT_ID when building the launcher.",
        auth_declined: "Sign-in was declined in the browser.",
        auth_expired: "The sign-in code has expired. Please start the sign-in again.",
        auth_no_xbox_account:
            "This Microsoft account has no Xbox profile yet. Create one on xbox.com and try again.",
        auth_child_account: "An adult must add child accounts to a Microsoft family.",
        auth_region_unavailable: "Xbox Live is not available in your region.",
        auth_adult_verification: "This account needs age verification with Xbox.",
        auth_app_not_approved:
            "Minecraft services rejected this app registration. Mojang has to approve it first.",
        auth_no_minecraft_profile:
            "This account does not own Minecraft: Java Edition or has no player name yet.",
        auth_session_expired: "The saved sign-in has expired. Please sign in again.",
        auth_service: "Signing in to Microsoft, Xbox or Minecraft failed.",
        secret_store:
            "Cannot access the system keychain. On Linux, a Secret Service such as GNOME Keyring or KWallet is required.",
        network: "Cannot reach the server. Check your internet connection.",
        checksum: "A downloaded file was corrupted. Please try again.",
        launch_hook:
            "A custom command of this instance failed. You can find it in the instance settings under Launch.",
    },
};

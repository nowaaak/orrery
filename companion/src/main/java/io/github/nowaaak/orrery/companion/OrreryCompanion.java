package io.github.nowaaak.orrery.companion;

import com.google.gson.JsonObject;
import io.github.nowaaak.orrery.companion.emotes.EmoteScreen;
import io.github.nowaaak.orrery.companion.emotes.Emotes;
import io.github.nowaaak.orrery.companion.hud.HudRenderer;
import io.github.nowaaak.orrery.companion.modules.Modules;
import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.Minecraft;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public class OrreryCompanion implements ClientModInitializer {
    public static final String ID = "orrery-companion";
    public static final Logger LOGGER = LoggerFactory.getLogger(ID);
    public static final String VERSION = /*$ mod_version*/ "0.1.0";
    public static final String MINECRAFT = /*$ minecraft*/ "26.3";
    private static final int STATUS_INTERVAL_TICKS = 20;

    private static Bridge bridge;
    private static JsonObject lastStatus;
    private static int ticks;

    @Override
    public void onInitializeClient() {
        LOGGER.info("Orrery Companion {} for Minecraft {} loaded", VERSION, MINECRAFT);
        Keys.register();
        Policy.register();
        TitleScreenExtras.register();
        HudRenderer.register();
        Modules.AUTO_RECONNECT.register();
        bridge = Bridge.fromSystemProperties(OrreryCompanion::onLauncherMessage);
        if (bridge != null) {
            bridge.start();
        }
        ClientTickEvents.END_CLIENT_TICK.register(OrreryCompanion::onTick);
    }

    public static Bridge bridge() {
        return bridge;
    }

    private static void onLauncherMessage(JsonObject message) {
        String type = message.has("type") ? message.get("type").getAsString() : "";
        Minecraft minecraft = Minecraft.getInstance();
        switch (type) {
            case "welcome" -> minecraft.execute(() -> lastStatus = null);
            case "toast" -> {
                String title = text(message, "title");
                String body = text(message, "body");
                minecraft.execute(() -> Toasts.show(minecraft, title, body));
            }
            default -> {
            }
        }
    }

    private static String text(JsonObject message, String key) {
        return message.has(key) ? message.get(key).getAsString() : "";
    }

    private static void onTick(Minecraft minecraft) {
        while (Keys.menu.consumeClick()) {
            Screens.open(minecraft, new OrreryScreen(Screens.current(minecraft)));
        }
        while (Keys.emote.consumeClick()) {
            if (minecraft.player != null) {
                Screens.open(minecraft, new EmoteScreen());
            }
        }
        Emotes.tick(minecraft);
        Modules.tick(minecraft);
        if (++ticks % STATUS_INTERVAL_TICKS != 0) {
            return;
        }
        LastPlayed.remember(minecraft);
        if (bridge == null || !bridge.isConnected()) {
            return;
        }
        JsonObject status = GameState.capture(minecraft);
        if (!status.equals(lastStatus)) {
            lastStatus = status;
            bridge.send(status);
        }
    }
}

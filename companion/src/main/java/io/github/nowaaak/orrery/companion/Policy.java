package io.github.nowaaak.orrery.companion;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import java.util.LinkedHashSet;
import java.util.Set;
import net.fabricmc.fabric.api.client.networking.v1.ClientPlayConnectionEvents;
import net.fabricmc.fabric.api.client.networking.v1.ClientPlayNetworking;
//? if >=1.20.5 {
import net.fabricmc.fabric.api.networking.v1.PayloadTypeRegistry;
//?} else {
/*import net.minecraft.resources.Identifier;
*///?}

public final class Policy {
    public static final int MAX_BYTES = 4096;
    private static final int MAX_MODULES = 64;
    private static volatile Set<String> disabled = Set.of();

    private Policy() {
    }

    public static Set<String> disabled() {
        return disabled;
    }

    public static boolean allows(String module) {
        return !disabled.contains(module);
    }

    static void apply(String json) {
        Set<String> modules = new LinkedHashSet<>();
        try {
            JsonObject root = JsonParser.parseString(json).getAsJsonObject();
            JsonArray list = root.has("disabled") ? root.getAsJsonArray("disabled") : new JsonArray();
            for (JsonElement element : list) {
                if (modules.size() >= MAX_MODULES) {
                    break;
                }
                String module = element.getAsString();
                if (module.matches("[a-z0-9_]{1,32}")) {
                    modules.add(module);
                }
            }
        } catch (RuntimeException e) {
            OrreryCompanion.LOGGER.warn("Ignoring an invalid orrery:policy message");
            return;
        }
        disabled = Set.copyOf(modules);
        OrreryCompanion.LOGGER.info("Server policy disables {}", modules.isEmpty() ? "nothing" : String.join(", ", modules));
    }

    static void register() {
        //? if >=26.1 {
        PayloadTypeRegistry.clientboundPlay().register(PolicyPayload.TYPE, PolicyPayload.CODEC);
        ClientPlayNetworking.registerGlobalReceiver(PolicyPayload.TYPE, (payload, context) -> context.client().execute(() -> apply(payload.json())));
        //?} else if >=1.20.5 {
        /*PayloadTypeRegistry.playS2C().register(PolicyPayload.TYPE, PolicyPayload.CODEC);
        ClientPlayNetworking.registerGlobalReceiver(PolicyPayload.TYPE, (payload, context) -> context.client().execute(() -> apply(payload.json())));
        *///?} else {
        /*ClientPlayNetworking.registerGlobalReceiver(new Identifier("orrery", "policy"), (client, handler, buf, sender) -> {
            String json = buf.readUtf(MAX_BYTES);
            client.execute(() -> apply(json));
        });
        *///?}
        ClientPlayConnectionEvents.DISCONNECT.register((handler, client) -> disabled = Set.of());
    }
}

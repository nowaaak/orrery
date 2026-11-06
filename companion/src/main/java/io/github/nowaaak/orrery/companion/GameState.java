package io.github.nowaaak.orrery.companion;

import com.google.gson.JsonObject;
import net.minecraft.client.Minecraft;
import net.minecraft.client.multiplayer.ServerData;
import net.minecraft.client.server.IntegratedServer;

public final class GameState {
    private GameState() {
    }

    public static JsonObject capture(Minecraft minecraft) {
        JsonObject status = new JsonObject();
        status.addProperty("type", "status");
        if (minecraft.level == null) {
            status.addProperty("screen", "menu");
            return status;
        }
        IntegratedServer server = minecraft.getSingleplayerServer();
        if (server != null) {
            status.addProperty("screen", "world");
            status.addProperty("world", server.getWorldData().getLevelName());
        } else {
            status.addProperty("screen", "server");
            ServerData data = minecraft.getCurrentServer();
            if (data != null) {
                status.addProperty("server", data.ip);
            }
        }
        //? if >=1.21.11 {
        status.addProperty("dimension", minecraft.level.dimension().identifier().toString());
        //?} else {
        /*status.addProperty("dimension", minecraft.level.dimension().location().toString());
        *///?}
        return status;
    }
}

package io.github.nowaaak.orrery.companion.modules.hud;

import io.github.nowaaak.orrery.companion.modules.TextHudModule;
import java.util.List;
import net.minecraft.client.Minecraft;
import net.minecraft.client.multiplayer.ClientPacketListener;
import net.minecraft.client.multiplayer.PlayerInfo;
import net.minecraft.client.multiplayer.ServerData;
import net.minecraft.client.resources.language.I18n;

public class ServerModule extends TextHudModule {
    public ServerModule() {
        super("server", false, false, 0.01f, 0.3f);
    }

    @Override
    public List<String> lines(Minecraft minecraft) {
        ServerData server = minecraft.getCurrentServer();
        ClientPacketListener connection = minecraft.getConnection();
        if (server == null || minecraft.getSingleplayerServer() != null || connection == null || minecraft.player == null) {
            return List.of();
        }
        PlayerInfo info = connection.getPlayerInfo(minecraft.player.getUUID());
        if (info == null) {
            return List.of(server.ip);
        }
        return List.of(I18n.get("orrery-companion.hud.server", server.ip, info.getLatency()));
    }
}

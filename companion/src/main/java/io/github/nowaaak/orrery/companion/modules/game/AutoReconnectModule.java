package io.github.nowaaak.orrery.companion.modules.game;

import io.github.nowaaak.orrery.companion.LastPlayed;
import io.github.nowaaak.orrery.companion.Screens;
import io.github.nowaaak.orrery.companion.modules.Module;
import java.util.List;
import net.fabricmc.fabric.api.client.screen.v1.ScreenEvents;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.components.AbstractWidget;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.DisconnectedScreen;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.gui.screens.TitleScreen;
import net.minecraft.client.gui.screens.multiplayer.JoinMultiplayerScreen;
import net.minecraft.client.multiplayer.ServerData;
import net.minecraft.network.chat.Component;

public class AutoReconnectModule extends Module {
    private static final int MAX_ATTEMPTS = 5;
    private static final long STABLE_MILLIS = 60_000;

    private final Option delay = option("delay", 5, List.of(3, 5, 10, 30),
            value -> Component.translatable("orrery-companion.module.auto_reconnect.seconds", value));

    private String serverName;
    private String serverAddress;
    private long connectedSince;
    private int attempts;
    private long deadline;
    private boolean cancelled;
    private Button reconnect;

    public AutoReconnectModule() {
        super("auto_reconnect", false, false);
    }

    @Override
    public List<Option> options() {
        return List.of(delay);
    }

    public void register() {
        ScreenEvents.AFTER_INIT.register((minecraft, screen, width, height) -> {
            if (screen instanceof DisconnectedScreen && isActive() && serverAddress != null) {
                decorate(minecraft, screen, width);
            }
        });
    }

    private void decorate(Minecraft minecraft, Screen screen, int width) {
        List<AbstractWidget> widgets = widgets(screen);
        int bottom = 0;
        for (AbstractWidget widget : widgets) {
            bottom = Math.max(bottom, widget.getY() + widget.getHeight());
        }
        int y = bottom + 4;
        reconnect = Button.builder(label(), button -> connect(minecraft)).bounds(width / 2 - 100, y, 150, 20).build();
        Button cancel = Button.builder(Component.translatable("gui.cancel"), button -> {
            cancelled = true;
            reconnect.setMessage(Component.translatable("orrery-companion.auto_reconnect.manual"));
            button.active = false;
        }).bounds(width / 2 + 54, y, 46, 20).build();
        cancel.active = canRetry();
        widgets.add(reconnect);
        widgets.add(cancel);
    }

    public void tick(Minecraft minecraft) {
        long now = System.currentTimeMillis();
        ServerData server = minecraft.getCurrentServer();
        if (minecraft.level != null && server != null && minecraft.getSingleplayerServer() == null && !server.isLan()) {
            serverName = server.name;
            serverAddress = server.ip;
            if (connectedSince == 0) {
                connectedSince = now;
            } else if (now - connectedSince > STABLE_MILLIS) {
                attempts = 0;
            }
        } else {
            connectedSince = 0;
        }
        if (!(Screens.current(minecraft) instanceof DisconnectedScreen)) {
            deadline = 0;
            cancelled = false;
            reconnect = null;
            return;
        }
        if (!isActive() || serverAddress == null || !canRetry()) {
            return;
        }
        if (deadline == 0) {
            deadline = now + option(delay) * 1000L;
        }
        if (reconnect != null) {
            reconnect.setMessage(label());
        }
        if (now >= deadline) {
            connect(minecraft);
        }
    }

    private boolean canRetry() {
        return !cancelled && attempts < MAX_ATTEMPTS;
    }

    private Component label() {
        if (!canRetry()) {
            return Component.translatable("orrery-companion.auto_reconnect.manual");
        }
        long seconds = deadline == 0 ? option(delay) : Math.max(0, (deadline - System.currentTimeMillis() + 999) / 1000);
        return Component.translatable("orrery-companion.auto_reconnect.countdown", seconds);
    }

    private void connect(Minecraft minecraft) {
        attempts++;
        deadline = 0;
        reconnect = null;
        LastPlayed.connect(minecraft, new JoinMultiplayerScreen(new TitleScreen()), serverName, serverAddress);
    }

    private static List<AbstractWidget> widgets(Screen screen) {
        //? if >=26.1 {
        return net.fabricmc.fabric.api.client.screen.v1.Screens.getWidgets(screen);
        //?} else {
        /*return net.fabricmc.fabric.api.client.screen.v1.Screens.getButtons(screen);
        *///?}
    }
}

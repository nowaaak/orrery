package io.github.nowaaak.orrery.companion.modules.game;

import io.github.nowaaak.orrery.companion.Keys;
import io.github.nowaaak.orrery.companion.LastPlayed;
import io.github.nowaaak.orrery.companion.Screens;
import io.github.nowaaak.orrery.companion.Toasts;
import io.github.nowaaak.orrery.companion.WaypointsScreen;
import io.github.nowaaak.orrery.companion.hud.Canvas;
import io.github.nowaaak.orrery.companion.modules.Module;
import java.util.List;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.Font;
import net.minecraft.client.gui.screens.DeathScreen;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.player.LocalPlayer;
import net.minecraft.client.resources.language.I18n;
import net.minecraft.util.Mth;

public class WaypointsModule extends Module {
    public static final int DEATH_POINT = 1;
    private static final float HALF_FIELD = 60f;
    private static final int MARKER = 0xFF8B8FFF;
    private static final int DEATH_MARKER = 0xFFF87171;
    private static final int OUTLINE = 0xFF000000;
    private static final int TEXT = 0xFFFFFFFF;
    private static final int DISTANCE = 0xFFB4B4BE;

    private final Option death = option("death", DEATH_POINT, List.of(0, 1),
            value -> net.minecraft.network.chat.Component.translatable(value == 1 ? "options.on" : "options.off"));
    private boolean deathRecorded;

    public WaypointsModule() {
        super("waypoints", false, false);
    }

    @Override
    public List<Option> options() {
        return List.of(death);
    }

    public Option deathOption() {
        return death;
    }

    @Override
    public Screen settingsScreen(Screen parent) {
        return new WaypointsScreen(parent);
    }

    public void tick(Minecraft minecraft) {
        LocalPlayer player = minecraft.player;
        boolean dead = Screens.current(minecraft) instanceof DeathScreen;
        int presses = 0;
        while (Keys.waypoint.consumeClick()) {
            presses++;
        }
        if (!isActive() || !available(minecraft)) {
            deathRecorded = dead;
            return;
        }
        if (presses > 0) {
            String name = I18n.get("orrery-companion.waypoints.default", list(minecraft).size() + 1);
            add(minecraft, name);
            Toasts.show(minecraft, I18n.get("orrery-companion.waypoints.added"), name);
        }
        if (dead && !deathRecorded && option(death) == 1) {
            String context = LastPlayed.context(minecraft);
            Waypoints.setDeath(context, new Waypoints.Waypoint(I18n.get("orrery-companion.waypoints.death"),
                    player.getBlockX(), player.getBlockY(), player.getBlockZ(), dimension(minecraft), true));
        }
        deathRecorded = dead;
    }

    public static boolean available(Minecraft minecraft) {
        return minecraft.player != null && minecraft.level != null && !minecraft.showOnlyReducedInfo();
    }

    public List<Waypoints.Waypoint> list(Minecraft minecraft) {
        if (!available(minecraft)) {
            return List.of();
        }
        return Waypoints.list(LastPlayed.context(minecraft));
    }

    public void add(Minecraft minecraft, String name) {
        LocalPlayer player = minecraft.player;
        String clean = Waypoints.cleanName(name);
        if (!available(minecraft) || clean.isEmpty()) {
            return;
        }
        Waypoints.add(LastPlayed.context(minecraft), new Waypoints.Waypoint(clean, player.getBlockX(), player.getBlockY(), player.getBlockZ(),
                dimension(minecraft), false));
    }

    public void remove(Minecraft minecraft, Waypoints.Waypoint waypoint) {
        Waypoints.remove(LastPlayed.context(minecraft), waypoint);
    }

    public void drawCompass(Canvas canvas, Minecraft minecraft) {
        LocalPlayer player = minecraft.player;
        if (!isActive() || !available(minecraft)) {
            return;
        }
        String dimension = dimension(minecraft);
        Font font = minecraft.font;
        int center = canvas.width() / 2;
        int half = Math.round(canvas.width() * 0.35f);
        for (Waypoints.Waypoint waypoint : list(minecraft)) {
            if (!waypoint.dimension().equals(dimension)) {
                continue;
            }
            double dx = waypoint.x() + 0.5 - player.getX();
            double dy = waypoint.y() + 0.5 - player.getY();
            double dz = waypoint.z() + 0.5 - player.getZ();
            double bearing = Math.toDegrees(Math.atan2(-dx, dz));
            float offset = Mth.wrapDegrees((float) bearing - player.getYRot());
            if (Math.abs(offset) > HALF_FIELD) {
                continue;
            }
            int x = center + Math.round(offset / HALF_FIELD * half);
            int color = waypoint.death() ? DEATH_MARKER : MARKER;
            canvas.fill(x - 3, 3, x + 3, 9, OUTLINE);
            canvas.fill(x - 2, 4, x + 2, 8, color);
            String label = waypoint.name();
            canvas.text(font, label, x - font.width(label) / 2, 12, TEXT);
            String distance = I18n.get("orrery-companion.waypoints.distance", Math.round(Math.sqrt(dx * dx + dy * dy + dz * dz)));
            canvas.text(font, distance, x - font.width(distance) / 2, 22, DISTANCE);
        }
    }

    public static String dimension(Minecraft minecraft) {
        //? if >=1.21.11 {
        return minecraft.level.dimension().identifier().toString();
        //?} else {
        /*return minecraft.level.dimension().location().toString();
        *///?}
    }
}

package io.github.nowaaak.orrery.companion.modules;

import io.github.nowaaak.orrery.companion.modules.game.ChatModule;
import io.github.nowaaak.orrery.companion.modules.game.CrosshairModule;
import io.github.nowaaak.orrery.companion.modules.game.HandPositionModule;
import io.github.nowaaak.orrery.companion.modules.game.LowFireModule;
import io.github.nowaaak.orrery.companion.modules.game.WaypointsModule;
import io.github.nowaaak.orrery.companion.modules.game.ZoomModule;
import io.github.nowaaak.orrery.companion.modules.hud.ClockModule;
import io.github.nowaaak.orrery.companion.modules.hud.CoordinatesModule;
import io.github.nowaaak.orrery.companion.modules.hud.CpsModule;
import io.github.nowaaak.orrery.companion.modules.hud.DurabilityModule;
import io.github.nowaaak.orrery.companion.modules.hud.EffectsModule;
import io.github.nowaaak.orrery.companion.modules.hud.FpsModule;
import io.github.nowaaak.orrery.companion.modules.hud.KeystrokesModule;
import io.github.nowaaak.orrery.companion.modules.hud.MemoryModule;
import io.github.nowaaak.orrery.companion.modules.hud.ServerModule;
import java.util.List;
import net.minecraft.client.Minecraft;

public final class Modules {
    public static final ZoomModule ZOOM = new ZoomModule();
    public static final CrosshairModule CROSSHAIR = new CrosshairModule();
    public static final LowFireModule LOW_FIRE = new LowFireModule();
    public static final HandPositionModule HAND_POSITION = new HandPositionModule();
    public static final ChatModule CHAT = new ChatModule();
    public static final WaypointsModule WAYPOINTS = new WaypointsModule();

    private static final List<Module> ALL = List.of(
            new FpsModule(),
            new CoordinatesModule(),
            new ClockModule(),
            new MemoryModule(),
            new ServerModule(),
            new KeystrokesModule(),
            new CpsModule(),
            new DurabilityModule(),
            new EffectsModule(),
            ZOOM,
            CROSSHAIR,
            LOW_FIRE,
            HAND_POSITION,
            CHAT,
            WAYPOINTS);
    private static final List<HudModule> HUD = ALL.stream()
            .filter(HudModule.class::isInstance)
            .map(HudModule.class::cast)
            .toList();

    private Modules() {
    }

    public static List<Module> all() {
        return ALL;
    }

    public static List<HudModule> hud() {
        return HUD;
    }

    public static void tick(Minecraft minecraft) {
        ZOOM.tick(minecraft);
        WAYPOINTS.tick(minecraft);
    }
}

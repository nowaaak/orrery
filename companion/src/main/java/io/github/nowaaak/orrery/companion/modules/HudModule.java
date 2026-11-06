package io.github.nowaaak.orrery.companion.modules;

import io.github.nowaaak.orrery.companion.hud.Canvas;
import java.util.function.Consumer;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.Font;

public abstract class HudModule extends Module {
    public record Frame(int width, int height, Consumer<Canvas> painter) {
    }

    private final float defaultX;
    private final float defaultY;

    protected HudModule(String id, boolean enabledByDefault, boolean oftenForbidden, float defaultX, float defaultY) {
        super(id, enabledByDefault, oftenForbidden);
        this.defaultX = defaultX;
        this.defaultY = defaultY;
    }

    public float defaultX() {
        return defaultX;
    }

    public float defaultY() {
        return defaultY;
    }

    public abstract Frame frame(Minecraft minecraft, Font font, boolean preview);
}

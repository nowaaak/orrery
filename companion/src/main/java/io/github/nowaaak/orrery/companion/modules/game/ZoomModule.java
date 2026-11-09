package io.github.nowaaak.orrery.companion.modules.game;

import io.github.nowaaak.orrery.companion.Keys;
import io.github.nowaaak.orrery.companion.Screens;
import io.github.nowaaak.orrery.companion.modules.Module;
import net.minecraft.client.Minecraft;

public class ZoomModule extends Module {
    private static final float DEFAULT_FACTOR = 0.25f;
    private static final float MIN_FACTOR = 0.1f;
    private static final float MAX_FACTOR = 0.8f;
    private static final float SCROLL_STEP = 1.25f;
    private static final float SPEED = 12f;

    private boolean zooming;
    private float target = DEFAULT_FACTOR;
    private float current = 1f;
    private long lastFrame;

    public ZoomModule() {
        super("zoom", false, false);
    }

    public void tick(Minecraft minecraft) {
        zooming = isActive() && minecraft.player != null && Screens.current(minecraft) == null && Keys.zoom.isDown();
        if (!zooming) {
            target = DEFAULT_FACTOR;
        }
    }

    public float fovFactor() {
        long now = System.currentTimeMillis();
        float delta = lastFrame == 0 ? 1f : Math.min(1f, (now - lastFrame) / 1000f * SPEED);
        lastFrame = now;
        float goal = zooming ? target : 1f;
        current += (goal - current) * delta;
        if (!zooming && current > 0.999f) {
            current = 1f;
        }
        return current;
    }

    public double sensitivity() {
        return zooming ? current : 1.0;
    }

    public boolean scroll(double amount) {
        if (!zooming || amount == 0) {
            return false;
        }
        float next = amount > 0 ? target / SCROLL_STEP : target * SCROLL_STEP;
        target = Math.max(MIN_FACTOR, Math.min(MAX_FACTOR, next));
        return true;
    }
}

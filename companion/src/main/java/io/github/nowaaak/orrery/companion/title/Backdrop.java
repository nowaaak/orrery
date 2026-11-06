package io.github.nowaaak.orrery.companion.title;

import io.github.nowaaak.orrery.companion.hud.Canvas;
import io.github.nowaaak.orrery.companion.ui.Theme;
import net.minecraft.resources.Identifier;

final class Backdrop {
    private static final Identifier IMAGE = Theme.id("textures/gui/title/background.png");
    private static final Identifier GLOW = Theme.id("textures/gui/title/glow.png");
    private static final int IMAGE_WIDTH = 1600;
    private static final int IMAGE_HEIGHT = 900;
    private static final int GLOW_SIZE = 256;
    private static final double TAU = Math.PI * 2;

    private Backdrop() {
    }

    static void draw(Canvas canvas, int width, int height, int contentRight, long millis) {
        double t = millis / 1000.0;
        canvas.fill(0, 0, width, height, Theme.BASE);
        float zoom = 1.06f + 0.03f * (float) Math.sin(t * TAU / 40);
        float scale = Math.max((float) width / IMAGE_WIDTH, (float) height / IMAGE_HEIGHT) * zoom;
        float x = (width - IMAGE_WIDTH * scale) / 2 + (float) Math.sin(t * TAU / 54) * width * 0.014f;
        float y = (height - IMAGE_HEIGHT * scale) / 2 + (float) Math.cos(t * TAU / 46) * height * 0.012f;
        canvas.texture(IMAGE, x, y, scale, IMAGE_WIDTH, IMAGE_HEIGHT, 0xFFFFFFFF);

        glow(canvas, width * (0.74f + 0.05f * (float) Math.sin(t * TAU / 23)), height * (0.36f + 0.07f * (float) Math.cos(t * TAU / 29)),
                height * 1.15f, 0x30666AF9);
        glow(canvas, width * (0.9f + 0.04f * (float) Math.cos(t * TAU / 31)), height * (0.84f + 0.05f * (float) Math.sin(t * TAU / 26)),
                height * 0.95f, 0x268B5CF6);

        canvas.horizontalFade(0, contentRight + width / 5, 0, height, Theme.withAlpha(Theme.BASE, 0xC0));
        canvas.verticalGradient(0, height - height / 3, width, height, Theme.withAlpha(Theme.BASE, 0), Theme.withAlpha(Theme.BASE, 0x99));
    }

    static void glow(Canvas canvas, float centerX, float centerY, float size, int argb) {
        canvas.texture(GLOW, centerX - size / 2, centerY - size / 2, size / GLOW_SIZE, GLOW_SIZE, GLOW_SIZE, argb);
    }
}

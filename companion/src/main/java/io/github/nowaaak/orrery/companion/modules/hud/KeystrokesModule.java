package io.github.nowaaak.orrery.companion.modules.hud;

import io.github.nowaaak.orrery.companion.hud.Canvas;
import io.github.nowaaak.orrery.companion.modules.ClickTracker;
import io.github.nowaaak.orrery.companion.modules.HudModule;
import io.github.nowaaak.orrery.companion.ui.Theme;
import java.util.Locale;
import net.minecraft.client.KeyMapping;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.Font;
import net.minecraft.client.resources.language.I18n;

public class KeystrokesModule extends HudModule {
    private static final int KEY = 20;
    private static final int GAP = 2;
    private static final int MOUSE = 20;
    private static final int SPACE = 10;
    private static final int WIDTH = KEY * 3 + GAP * 2;
    private static final int HEIGHT = KEY * 2 + MOUSE + SPACE + GAP * 3;
    private static final int IDLE = 0x70000000;
    private static final int PRESSED = 0xD8FFFFFF;
    private static final int TEXT = 0xFFFFFFFF;
    private static final int TEXT_PRESSED = 0xFF101014;
    private static final float SPEED = 18f;

    private final float[] glow = new float[7];
    private long lastFrame;

    public KeystrokesModule() {
        super("keystrokes", false, false, 0.01f, 0.55f);
    }

    @Override
    public Frame frame(Minecraft minecraft, Font font, boolean preview) {
        if (!preview && minecraft.player == null) {
            return null;
        }
        return new Frame(WIDTH, HEIGHT, canvas -> draw(canvas, font, minecraft));
    }

    private void draw(Canvas canvas, Font font, Minecraft minecraft) {
        KeyMapping[] keys = {
            minecraft.options.keyUp,
            minecraft.options.keyLeft,
            minecraft.options.keyDown,
            minecraft.options.keyRight,
            minecraft.options.keyAttack,
            minecraft.options.keyUse,
            minecraft.options.keyJump
        };
        animate(keys);
        int row = KEY + GAP;
        key(canvas, font, row, 0, KEY, KEY, label(font, keys[0]), null, 0);
        key(canvas, font, 0, row, KEY, KEY, label(font, keys[1]), null, 1);
        key(canvas, font, row, row, KEY, KEY, label(font, keys[2]), null, 2);
        key(canvas, font, row * 2, row, KEY, KEY, label(font, keys[3]), null, 3);
        int half = (WIDTH - GAP) / 2;
        int mouseY = row * 2;
        key(canvas, font, 0, mouseY, half, MOUSE, I18n.get("orrery-companion.hud.lmb"), I18n.get("orrery-companion.hud.cps_short", ClickTracker.left()), 4);
        key(canvas, font, half + GAP, mouseY, WIDTH - half - GAP, MOUSE, I18n.get("orrery-companion.hud.rmb"),
                I18n.get("orrery-companion.hud.cps_short", ClickTracker.right()), 5);
        int spaceY = mouseY + MOUSE + GAP;
        float jump = glow[6];
        canvas.fill(0, spaceY, WIDTH, spaceY + SPACE, Theme.mix(IDLE, PRESSED, jump));
        canvas.fill(WIDTH / 2 - 10, spaceY + SPACE / 2, WIDTH / 2 + 10, spaceY + SPACE / 2 + 1, Theme.mix(TEXT, TEXT_PRESSED, jump));
    }

    private void animate(KeyMapping[] keys) {
        long now = System.currentTimeMillis();
        float delta = lastFrame == 0 ? 1f : Math.min(1f, (now - lastFrame) / 1000f * SPEED);
        lastFrame = now;
        for (int i = 0; i < keys.length; i++) {
            float target = keys[i].isDown() ? 1f : 0f;
            glow[i] += (target - glow[i]) * delta;
        }
    }

    private void key(Canvas canvas, Font font, int x, int y, int width, int height, String label, String detail, int index) {
        float pressed = glow[index];
        canvas.fill(x, y, x + width, y + height, Theme.mix(IDLE, PRESSED, pressed));
        int color = Theme.mix(TEXT, TEXT_PRESSED, pressed);
        boolean shadow = pressed < 0.5f;
        if (detail == null) {
            canvas.text(font, label, x + (width - font.width(label) + 1) / 2, y + (height - 7) / 2, color, shadow);
            return;
        }
        canvas.text(font, label, x + (width - font.width(label) + 1) / 2, y + 3, color, shadow);
        float small = 0.6f;
        canvas.push(x + (width - font.width(detail) * small) / 2, y + 12.5f, small);
        canvas.text(font, detail, 0, 0, color, shadow);
        canvas.pop();
    }

    private static String label(Font font, KeyMapping key) {
        String text = key.getTranslatedKeyMessage().getString().toUpperCase(Locale.ROOT);
        while (text.length() > 1 && font.width(text) > KEY - 4) {
            text = text.substring(0, text.length() - 1);
        }
        return text;
    }
}

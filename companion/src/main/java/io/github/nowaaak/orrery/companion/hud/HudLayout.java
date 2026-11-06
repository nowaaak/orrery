package io.github.nowaaak.orrery.companion.hud;

import io.github.nowaaak.orrery.companion.modules.HudModule;
import io.github.nowaaak.orrery.companion.modules.ModuleSettings;

public final class HudLayout {
    private HudLayout() {
    }

    public record Box(HudModule module, HudModule.Frame frame, int x, int y, int width, int height, float scale) {
        public boolean contains(double pointX, double pointY) {
            return pointX >= x && pointX < x + width && pointY >= y && pointY < y + height;
        }
    }

    public static Box box(HudModule module, HudModule.Frame frame, int screenWidth, int screenHeight) {
        ModuleSettings settings = ModuleSettings.get(module);
        int width = Math.round(frame.width() * settings.scale);
        int height = Math.round(frame.height() * settings.scale);
        int x = clamp(Math.round(settings.x * screenWidth), screenWidth - width);
        int y = clamp(Math.round(settings.y * screenHeight), screenHeight - height);
        return new Box(module, frame, x, y, width, height, settings.scale);
    }

    public static void draw(Canvas canvas, Box box) {
        canvas.push(box.x(), box.y(), box.scale());
        box.frame().painter().accept(canvas);
        canvas.pop();
    }

    private static int clamp(int value, int max) {
        return Math.max(0, Math.min(Math.max(0, max), value));
    }
}

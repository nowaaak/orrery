package io.github.nowaaak.orrery.companion.modules.game;

import com.mojang.blaze3d.platform.Window;
import io.github.nowaaak.orrery.companion.ModuleOptionsScreen;
import io.github.nowaaak.orrery.companion.hud.Canvas;
import io.github.nowaaak.orrery.companion.modules.Module;
import java.util.ArrayList;
import java.util.List;
import net.minecraft.client.Minecraft;
import net.minecraft.network.chat.Component;

public class CrosshairModule extends Module {
    private static final int CROSS = 0;
    private static final int DOT = 1;
    private static final int CROSS_DOT = 2;
    private static final int CIRCLE = 3;
    private static final int T_SHAPE = 4;
    private static final int[] COLORS = {0xFFFFFFFF, 0xFF8B8FFF, 0xFF4ADE80, 0xFFF87171, 0xFF67E8F9, 0xFFFDE047};
    private static final String[] COLOR_NAMES = {"white", "indigo", "green", "red", "cyan", "yellow"};
    private static final String[] STYLE_NAMES = {"cross", "dot", "cross_dot", "circle", "t"};
    private static final int OUTLINE = 0xFF000000;
    private static final float PREVIEW_SCALE = 2f;

    private final Option style = option("style", CROSS, range(0, STYLE_NAMES.length - 1, 1),
            value -> Component.translatable("orrery-companion.module.crosshair.style." + STYLE_NAMES[value]));
    private final Option size = option("size", 5, range(2, 14, 1), ModuleOptionsScreen::number);
    private final Option gap = option("gap", 2, range(0, 8, 1), ModuleOptionsScreen::number);
    private final Option thickness = option("thickness", 1, range(1, 3, 1), ModuleOptionsScreen::number);
    private final Option color = option("color", 0, range(0, COLORS.length - 1, 1),
            value -> Component.translatable("orrery-companion.module.crosshair.color." + COLOR_NAMES[value]));
    private final Option outline = option("outline", 1, List.of(0, 1),
            value -> Component.translatable(value == 1 ? "options.on" : "options.off"));

    public CrosshairModule() {
        super("crosshair", false, false);
    }

    @Override
    public List<Option> options() {
        return List.of(style, size, gap, thickness, color, outline);
    }

    @Override
    public boolean hasPreview() {
        return true;
    }

    @Override
    public void drawPreview(Canvas canvas, int centerX, int centerY) {
        int scale = guiScale();
        canvas.push(centerX, centerY, PREVIEW_SCALE / scale);
        draw(canvas, 0, 0, scale);
        canvas.pop();
    }

    public boolean replacesVanilla() {
        return isActive() && Minecraft.getInstance().player != null;
    }

    public void drawInGame(Canvas canvas) {
        Minecraft minecraft = Minecraft.getInstance();
        if (!minecraft.options.getCameraType().isFirstPerson() || minecraft.player == null || minecraft.player.isSpectator()) {
            return;
        }
        Window window = minecraft.getWindow();
        int scale = guiScale();
        canvas.push(0, 0, 1f / scale);
        draw(canvas, window.getWidth() / 2, window.getHeight() / 2, scale);
        canvas.pop();
    }

    private static int guiScale() {
        return Math.max(1, (int) Math.round(Minecraft.getInstance().getWindow().getGuiScale()));
    }

    private void draw(Canvas canvas, int centerX, int centerY, int scale) {
        int shape = option(style);
        int length = option(size) * scale;
        int space = option(gap) * scale;
        int line = option(thickness) * scale;
        int argb = COLORS[option(color)];
        boolean outlined = option(outline) == 1;
        int border = Math.max(1, scale / 2);
        int left = centerX - line / 2;
        int top = centerY - line / 2;

        if (shape == CIRCLE) {
            int box = length * 2 + line;
            if (outlined) {
                canvas.physicalShape(left - length - border, top - length - border, box + border * 2, box + border * 2,
                        box / 2f + border, line + border * 2, OUTLINE);
            }
            canvas.physicalShape(left - length, top - length, box, box, box / 2f, line, argb);
            if (space == 0) {
                List<int[]> center = List.of(new int[] {left, top, left + line, top + line});
                paint(canvas, center, argb, outlined, border);
            }
            return;
        }

        List<int[]> parts = new ArrayList<>();
        if (shape == DOT || shape == CROSS_DOT) {
            parts.add(new int[] {left - border, top - border, left + line + border, top + line + border});
        }
        if (shape != DOT) {
            parts.add(new int[] {left - space - length, top, left - space, top + line});
            parts.add(new int[] {left + line + space, top, left + line + space + length, top + line});
            parts.add(new int[] {left, top + line + space, left + line, top + line + space + length});
            if (shape != T_SHAPE) {
                parts.add(new int[] {left, top - space - length, left + line, top - space});
            }
        }
        paint(canvas, parts, argb, outlined, border);
    }

    private static void paint(Canvas canvas, List<int[]> parts, int argb, boolean outlined, int border) {
        if (outlined) {
            for (int[] part : parts) {
                canvas.fill(part[0] - border, part[1] - border, part[2] + border, part[3] + border, OUTLINE);
            }
        }
        for (int[] part : parts) {
            canvas.fill(part[0], part[1], part[2], part[3], argb);
        }
    }
}

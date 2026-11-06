package io.github.nowaaak.orrery.companion.modules;

import java.util.List;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.Font;

public abstract class TextHudModule extends HudModule {
    public static final int PADDING = 3;
    public static final int BACKGROUND = 0x70000000;
    public static final int TEXT = 0xFFFFFFFF;

    protected TextHudModule(String id, boolean enabledByDefault, boolean oftenForbidden, float defaultX, float defaultY) {
        super(id, enabledByDefault, oftenForbidden, defaultX, defaultY);
    }

    public abstract List<String> lines(Minecraft minecraft);

    @Override
    public Frame frame(Minecraft minecraft, Font font, boolean preview) {
        List<String> lines = minecraft.player == null ? List.of() : lines(minecraft);
        if (lines.isEmpty()) {
            if (!preview) {
                return null;
            }
            lines = List.of(name().getString());
        }
        return textFrame(font, lines, null);
    }

    public static Frame textFrame(Font font, List<String> lines, List<Integer> colors) {
        int lineHeight = font.lineHeight + 1;
        int textWidth = 0;
        for (String line : lines) {
            textWidth = Math.max(textWidth, font.width(line));
        }
        int width = textWidth + PADDING * 2;
        int height = lines.size() * lineHeight - 2 + PADDING * 2;
        return new Frame(width, height, canvas -> {
            canvas.fill(0, 0, width, height, BACKGROUND);
            for (int i = 0; i < lines.size(); i++) {
                int color = colors == null ? TEXT : colors.get(i);
                canvas.text(font, lines.get(i), PADDING, PADDING + i * lineHeight, color);
            }
        });
    }
}

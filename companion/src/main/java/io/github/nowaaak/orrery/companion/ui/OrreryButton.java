package io.github.nowaaak.orrery.companion.ui;

import io.github.nowaaak.orrery.companion.hud.Canvas;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.Font;
import net.minecraft.client.gui.components.AbstractButton;
import net.minecraft.client.gui.narration.NarrationElementOutput;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.Identifier;
//? if >=26.1 {
import net.minecraft.client.gui.GuiGraphicsExtractor;
//?} else {
/*import net.minecraft.client.gui.GuiGraphics;
*///?}
//? if >=1.21.11 {
import net.minecraft.client.input.InputWithModifiers;
import net.minecraft.util.Util;
//?} else {
/*import net.minecraft.Util;
*///?}

public class OrreryButton extends AbstractButton {
    public enum Style {
        PRIMARY,
        TILE,
        ACTION,
        LINK,
        ROW
    }

    public static final Identifier ICON_PLAY = Theme.id("textures/gui/icons/play.png");
    public static final Identifier ICON_CUBE = Theme.id("textures/gui/icons/cube.png");
    public static final Identifier ICON_GLOBE = Theme.id("textures/gui/icons/globe.png");
    public static final Identifier ICON_SLIDERS = Theme.id("textures/gui/icons/sliders.png");
    public static final Identifier ICON_ORBIT = Theme.id("textures/gui/icons/orbit.png");
    public static final Identifier ICON_POWER = Theme.id("textures/gui/icons/power.png");
    public static final Identifier ICON_CHEVRON = Theme.id("textures/gui/icons/chevron.png");
    public static final Identifier ICON_ARROW = Theme.id("textures/gui/icons/arrow.png");
    public static final int ICON_TEXTURE = 48;

    private static final int CAP_HEIGHT = 7;
    private static final float HOVER_SPEED = 10f;

    private final Style style;
    private final Runnable action;
    private Identifier icon;
    private Component caption;
    private Identifier trailing;
    private float hover;
    private long lastFrame;

    public OrreryButton(int x, int y, int width, int height, Style style, Component label, Runnable action) {
        super(x, y, width, height, label);
        this.style = style;
        this.action = action;
    }

    public OrreryButton icon(Identifier value) {
        icon = value;
        return this;
    }

    public OrreryButton caption(Component value) {
        caption = value;
        return this;
    }

    public OrreryButton trailing(Identifier value) {
        trailing = value;
        return this;
    }

    private void animate() {
        long now = Util.getMillis();
        float delta = lastFrame == 0 ? 0f : Math.min(0.1f, (now - lastFrame) / 1000f);
        lastFrame = now;
        float target = active && isHoveredOrFocused() ? 1f : 0f;
        hover += (target - hover) * Math.min(1f, delta * HOVER_SPEED);
    }

    private void draw(Canvas canvas) {
        animate();
        Font font = Minecraft.getInstance().font;
        int x = getX();
        int y = getY();
        if (style == Style.TILE) {
            y -= Math.round(hover);
        }
        if (isFocused() && active && Minecraft.getInstance().getLastInputType().isKeyboard()) {
            canvas.roundedBorder(x - 2, y - 2, width + 4, height + 4, Theme.RADIUS + 2, Theme.fade(Theme.RING));
        }
        switch (style) {
            case PRIMARY -> drawPrimary(canvas, font, x, y);
            case TILE -> drawTile(canvas, font, x, y);
            case ACTION -> drawAction(canvas, font, x, y);
            case LINK -> drawLink(canvas, font, x, y);
            case ROW -> drawRow(canvas, font, x, y);
        }
    }

    private void drawPrimary(Canvas canvas, Font font, int x, int y) {
        canvas.roundedRect(x, y, width, height, Theme.RADIUS, Theme.fade(Theme.mix(Theme.PRIMARY, Theme.PRIMARY_HOVER, hover)));
        canvas.roundedBorder(x, y, width, height, Theme.RADIUS, Theme.fade(0x33FFFFFF));
        int textX = x + 14;
        if (icon != null) {
            canvas.roundedRect(x + 10, y + (height - 24) / 2f, 24, 24, 12, Theme.fade(0x2EFFFFFF));
            canvas.texture(icon, x + 16, y + (height - 12) / 2f, 12f / ICON_TEXTURE, ICON_TEXTURE, ICON_TEXTURE, Theme.fade(0xFFFFFFFF));
            textX = x + 44;
        }
        drawLabels(canvas, font, textX, y, 0xFFFFFFFF, 0xC8FFFFFF);
        if (trailing != null) {
            float arrowX = x + width - 24 + hover * 3;
            canvas.texture(trailing, arrowX, y + (height - 10) / 2f, 10f / ICON_TEXTURE, ICON_TEXTURE, ICON_TEXTURE,
                    Theme.fade(Theme.withAlpha(0xFFFFFF, Math.round(150 + 105 * hover))));
        }
    }

    private void drawTile(Canvas canvas, Font font, int x, int y) {
        canvas.roundedRect(x, y, width, height, Theme.RADIUS, Theme.fade(Theme.mix(Theme.GLASS, Theme.GLASS_HOVER, hover)));
        canvas.roundedBorder(x, y, width, height, Theme.RADIUS, Theme.fade(Theme.mix(Theme.GLASS_BORDER, Theme.GLASS_BORDER_HOVER, hover)));
        boolean compact = height < 52;
        int badgeX = x + 10;
        int badgeY = compact ? y + (height - 20) / 2 : y + 10;
        if (icon != null) {
            canvas.roundedRect(badgeX, badgeY, 20, 20, 5, Theme.fade(Theme.mix(0x33666AF9, 0x59666AF9, hover)));
            canvas.texture(icon, badgeX + 5, badgeY + 5, 10f / ICON_TEXTURE, ICON_TEXTURE, ICON_TEXTURE, Theme.fade(0xFFE4E6FF));
        }
        int labelX = compact && icon != null ? badgeX + 28 : x + 11;
        int labelWidth = x + width - 8 - labelX;
        int labelY = compact ? y + (height - (caption == null ? CAP_HEIGHT : CAP_HEIGHT * 2 + 5)) / 2 : y + height - (caption == null ? 18 : 27);
        canvas.text(font, fit(font, getMessage(), labelWidth, true), labelX, labelY, Theme.fade(active ? Theme.FOREGROUND : Theme.MUTED));
        if (caption != null) {
            canvas.text(font, fit(font, caption, labelWidth, false), labelX, labelY + (compact ? CAP_HEIGHT + 5 : 12), Theme.fade(Theme.MUTED));
        }
    }

    private void drawAction(Canvas canvas, Font font, int x, int y) {
        canvas.roundedRect(x, y, width, height, Theme.RADIUS, Theme.fade(Theme.mix(Theme.GLASS, Theme.GLASS_HOVER, hover)));
        canvas.roundedBorder(x, y, width, height, Theme.RADIUS, Theme.fade(Theme.mix(Theme.GLASS_BORDER, Theme.GLASS_BORDER_HOVER, hover)));
        Component label = Theme.ui(getMessage());
        int labelWidth = font.width(label);
        int iconSpace = icon == null ? 0 : 15;
        int startX = x + (width - labelWidth - iconSpace) / 2;
        int color = Theme.fade(Theme.mix(0xFFCFCFD8, Theme.FOREGROUND, hover));
        if (icon != null) {
            canvas.texture(icon, startX, y + (height - 10) / 2f, 10f / ICON_TEXTURE, ICON_TEXTURE, ICON_TEXTURE, color);
        }
        canvas.text(font, label, startX + iconSpace, y + (height - CAP_HEIGHT) / 2, color);
    }

    private void drawLink(Canvas canvas, Font font, int x, int y) {
        Component label = Theme.ui(getMessage());
        int color = Theme.fade(Theme.mix(Theme.MUTED, Theme.FOREGROUND, hover));
        canvas.text(font, label, x, y + (height - CAP_HEIGHT) / 2, color);
        if (trailing != null) {
            canvas.texture(trailing, x + font.width(label) + 4, y + (height - 8) / 2f + 1, 8f / ICON_TEXTURE, ICON_TEXTURE, ICON_TEXTURE, color);
        }
    }

    private void drawRow(Canvas canvas, Font font, int x, int y) {
        canvas.roundedRect(x, y, width, height, Theme.RADIUS - 2, Theme.fade(Theme.withAlpha(0xFFFFFF, Math.round(20 * hover))));
        int color = Theme.fade(active ? Theme.mix(0xFFCFCFD8, Theme.FOREGROUND, hover) : Theme.SUBTLE);
        canvas.text(font, Theme.ui(getMessage()), x + 10, y + (height - CAP_HEIGHT) / 2, color);
    }

    private void drawLabels(Canvas canvas, Font font, int textX, int y, int titleColor, int captionColor) {
        int maxWidth = getX() + width - (trailing == null ? 12 : 32) - textX;
        if (caption == null) {
            canvas.text(font, fit(font, getMessage(), maxWidth, true), textX, y + (height - CAP_HEIGHT) / 2, Theme.fade(titleColor));
            return;
        }
        int top = y + (height - CAP_HEIGHT * 2 - 5) / 2;
        canvas.text(font, fit(font, getMessage(), maxWidth, true), textX, top, Theme.fade(titleColor));
        canvas.text(font, fit(font, caption, maxWidth, false), textX, top + CAP_HEIGHT + 5, Theme.fade(captionColor));
    }

    private static Component fit(Font font, Component text, int maxWidth, boolean bold) {
        Component styled = bold ? Theme.bold(text) : Theme.ui(text);
        if (font.width(styled) <= maxWidth) {
            return styled;
        }
        String raw = text.getString();
        for (int end = raw.length() - 1; end > 0; end--) {
            String candidate = raw.substring(0, end).stripTrailing() + "…";
            Component trimmed = bold ? Theme.bold(candidate) : Theme.ui(candidate);
            if (font.width(trimmed) <= maxWidth) {
                return trimmed;
            }
        }
        return bold ? Theme.bold("…") : Theme.ui("…");
    }

    @Override
    protected void updateWidgetNarration(NarrationElementOutput output) {
        defaultButtonNarrationText(output);
    }

    //? if >=26.1 {
    @Override
    protected void extractContents(GuiGraphicsExtractor graphics, int mouseX, int mouseY, float delta) {
        draw(new Canvas(graphics));
    }
    //?} else if >=1.21.11 {
    /*@Override
    protected void renderContents(GuiGraphics graphics, int mouseX, int mouseY, float delta) {
        draw(new Canvas(graphics));
    }
    *///?} else {
    /*@Override
    protected void renderWidget(GuiGraphics graphics, int mouseX, int mouseY, float delta) {
        draw(new Canvas(graphics));
    }
    *///?}

    //? if >=1.21.11 {
    @Override
    public void onPress(InputWithModifiers input) {
        action.run();
    }
    //?} else {
    /*@Override
    public void onPress() {
        action.run();
    }
    *///?}
}

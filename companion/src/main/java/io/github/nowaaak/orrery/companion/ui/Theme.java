package io.github.nowaaak.orrery.companion.ui;

import io.github.nowaaak.orrery.companion.OrreryCompanion;
import net.minecraft.network.chat.Component;
import net.minecraft.network.chat.MutableComponent;
import net.minecraft.resources.Identifier;
//? if >=1.21.11 {
import net.minecraft.network.chat.FontDescription;
//?}

public final class Theme {
    public static final int BASE = 0xFF0B0B12;
    public static final int FOREGROUND = 0xFFF5F5F7;
    public static final int MUTED = 0xFF9A9AA6;
    public static final int SUBTLE = 0xFF6E6E7A;
    public static final int PRIMARY = 0xFF666AF9;
    public static final int PRIMARY_HOVER = 0xFF7A7EFF;
    public static final int RING = 0xFF8792FF;
    public static final int GLASS = 0x9E14141E;
    public static final int GLASS_HOVER = 0xC21C1C2A;
    public static final int GLASS_BORDER = 0x1FFFFFFF;
    public static final int GLASS_BORDER_HOVER = 0x40FFFFFF;
    public static final int ROW_HOVER = 0x14FFFFFF;
    public static final int ONLINE = 0xFF4ADE80;
    public static final float RADIUS = 6f;

    private static float opacity = 1f;

    //? if >=1.21.11 {
    private static final FontDescription FONT = new FontDescription.Resource(id("ui"));
    private static final FontDescription FONT_BOLD = new FontDescription.Resource(id("ui_bold"));
    //?} else {
    /*private static final Identifier FONT = id("ui");
    private static final Identifier FONT_BOLD = id("ui_bold");
    *///?}

    private Theme() {
    }

    public static Identifier id(String path) {
        //? if >=1.21.11 {
        return Identifier.fromNamespaceAndPath(OrreryCompanion.ID, path);
        //?} else {
        /*return new Identifier(OrreryCompanion.ID, path);
        *///?}
    }

    public static MutableComponent ui(Component text) {
        return text.copy().withStyle(style -> style.withFont(FONT));
    }

    public static MutableComponent ui(String text) {
        return ui(Component.literal(text));
    }

    public static MutableComponent bold(Component text) {
        return text.copy().withStyle(style -> style.withFont(FONT_BOLD));
    }

    public static MutableComponent bold(String text) {
        return bold(Component.literal(text));
    }

    public static float opacity() {
        return opacity;
    }

    public static void setOpacity(float value) {
        opacity = Math.max(0f, Math.min(1f, value));
    }

    public static int fade(int argb) {
        int alpha = Math.round((argb >>> 24) * opacity);
        return (alpha << 24) | (argb & 0x00FFFFFF);
    }

    public static int withAlpha(int argb, int alpha) {
        return (alpha << 24) | (argb & 0x00FFFFFF);
    }

    public static int mix(int from, int to, float t) {
        float clamped = Math.max(0f, Math.min(1f, t));
        int result = 0;
        for (int shift = 0; shift <= 24; shift += 8) {
            int a = (from >>> shift) & 0xFF;
            int b = (to >>> shift) & 0xFF;
            result |= Math.round(a + (b - a) * clamped) << shift;
        }
        return result;
    }
}

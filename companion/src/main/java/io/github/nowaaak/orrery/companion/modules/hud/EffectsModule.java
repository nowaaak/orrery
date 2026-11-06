package io.github.nowaaak.orrery.companion.modules.hud;

import io.github.nowaaak.orrery.companion.modules.HudModule;
import io.github.nowaaak.orrery.companion.modules.TextHudModule;
import io.github.nowaaak.orrery.companion.ui.Theme;
import java.util.ArrayList;
import java.util.List;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.Font;
import net.minecraft.world.effect.MobEffect;
import net.minecraft.world.effect.MobEffectInstance;

public class EffectsModule extends HudModule {
    private static final String[] LEVELS = {"", " II", " III", " IV", " V", " VI", " VII", " VIII", " IX", " X"};
    private static final int TICKS_PER_SECOND = 20;

    public EffectsModule() {
        super("effects", false, false, 0.8f, 0.25f);
    }

    @Override
    public Frame frame(Minecraft minecraft, Font font, boolean preview) {
        List<String> lines = new ArrayList<>();
        List<Integer> colors = new ArrayList<>();
        if (minecraft.player != null) {
            for (MobEffectInstance instance : minecraft.player.getActiveEffects()) {
                //? if >=1.21.11 {
                MobEffect effect = instance.getEffect().value();
                //?} else {
                /*MobEffect effect = instance.getEffect();
                *///?}
                lines.add(effect.getDisplayName().getString() + level(instance.getAmplifier()) + "  " + duration(instance));
                colors.add(Theme.mix(0xFF000000 | effect.getColor(), 0xFFFFFFFF, 0.45f));
            }
        }
        if (lines.isEmpty()) {
            if (!preview) {
                return null;
            }
            lines.add(name().getString() + "  1:23");
            colors.add(0xFFB9D9FF);
        }
        return TextHudModule.textFrame(font, lines, colors);
    }

    private static String level(int amplifier) {
        return amplifier < LEVELS.length ? LEVELS[amplifier] : " " + (amplifier + 1);
    }

    private static String duration(MobEffectInstance instance) {
        if (instance.isInfiniteDuration()) {
            return "∞";
        }
        int seconds = instance.getDuration() / TICKS_PER_SECOND;
        return String.format("%d:%02d", seconds / 60, seconds % 60);
    }
}

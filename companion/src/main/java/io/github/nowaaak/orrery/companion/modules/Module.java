package io.github.nowaaak.orrery.companion.modules;

import io.github.nowaaak.orrery.companion.ModuleOptionsScreen;
import io.github.nowaaak.orrery.companion.Policy;
import io.github.nowaaak.orrery.companion.hud.Canvas;
import java.util.List;
import java.util.function.IntFunction;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;

public abstract class Module {
    public record Option(String key, Component label, List<Integer> values, IntFunction<Component> display, int fallback) {
    }

    private final String id;
    private final boolean enabledByDefault;
    private final boolean oftenForbidden;

    protected Module(String id, boolean enabledByDefault, boolean oftenForbidden) {
        this.id = id;
        this.enabledByDefault = enabledByDefault;
        this.oftenForbidden = oftenForbidden;
    }

    public String id() {
        return id;
    }

    public Component name() {
        return Component.translatable("orrery-companion.module." + id);
    }

    public boolean enabledByDefault() {
        return enabledByDefault;
    }

    public boolean oftenForbidden() {
        return oftenForbidden;
    }

    public boolean isEnabled() {
        return ModuleSettings.get(this).enabled;
    }

    public boolean isAllowed() {
        return Policy.allows(id);
    }

    public boolean isActive() {
        return isEnabled() && isAllowed();
    }

    public List<Option> options() {
        return List.of();
    }

    public int option(Option option) {
        int value = ModuleSettings.get(this).option(option.key(), option.fallback());
        return option.values().contains(value) ? value : option.fallback();
    }

    public boolean hasSettings() {
        return !options().isEmpty();
    }

    public Screen settingsScreen(Screen parent) {
        return new ModuleOptionsScreen(parent, this);
    }

    public boolean hasPreview() {
        return false;
    }

    public void drawPreview(Canvas canvas, int centerX, int centerY) {
    }

    protected Option option(String key, int fallback, List<Integer> values, IntFunction<Component> display) {
        return new Option(key, Component.translatable("orrery-companion.module." + id + "." + key), values, display, fallback);
    }

    protected static List<Integer> range(int from, int to, int step) {
        return java.util.stream.IntStream.iterate(from, value -> value <= to, value -> value + step).boxed().toList();
    }
}

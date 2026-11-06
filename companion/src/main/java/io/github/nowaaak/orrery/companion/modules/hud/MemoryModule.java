package io.github.nowaaak.orrery.companion.modules.hud;

import io.github.nowaaak.orrery.companion.modules.TextHudModule;
import java.util.List;
import net.minecraft.client.Minecraft;
import net.minecraft.client.resources.language.I18n;

public class MemoryModule extends TextHudModule {
    private static final long MEGABYTE = 1024L * 1024L;

    public MemoryModule() {
        super("memory", false, false, 0.01f, 0.25f);
    }

    @Override
    public List<String> lines(Minecraft minecraft) {
        Runtime runtime = Runtime.getRuntime();
        long used = runtime.totalMemory() - runtime.freeMemory();
        long max = runtime.maxMemory();
        long percent = max > 0 ? used * 100 / max : 0;
        return List.of(I18n.get("orrery-companion.hud.memory", percent, used / MEGABYTE, max / MEGABYTE));
    }
}

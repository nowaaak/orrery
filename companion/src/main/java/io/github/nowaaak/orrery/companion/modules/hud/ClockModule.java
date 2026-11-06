package io.github.nowaaak.orrery.companion.modules.hud;

import io.github.nowaaak.orrery.companion.modules.TextHudModule;
import java.time.LocalTime;
import java.time.format.DateTimeFormatter;
import java.util.List;
import net.minecraft.client.Minecraft;

public class ClockModule extends TextHudModule {
    private static final DateTimeFormatter FORMAT = DateTimeFormatter.ofPattern("HH:mm");

    public ClockModule() {
        super("clock", false, false, 0.01f, 0.2f);
    }

    @Override
    public List<String> lines(Minecraft minecraft) {
        return List.of(LocalTime.now().format(FORMAT));
    }
}

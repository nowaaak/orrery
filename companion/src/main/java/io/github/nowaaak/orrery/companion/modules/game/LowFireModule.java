package io.github.nowaaak.orrery.companion.modules.game;

import io.github.nowaaak.orrery.companion.ModuleOptionsScreen;
import io.github.nowaaak.orrery.companion.modules.Module;
import java.util.List;

public class LowFireModule extends Module {
    private final Option offset = option("offset", 30, range(10, 60, 5), ModuleOptionsScreen::hundredths);

    public LowFireModule() {
        super("low_fire", false, false);
    }

    @Override
    public List<Option> options() {
        return List.of(offset);
    }

    public float offset() {
        return isActive() ? option(offset) / 100f : 0f;
    }
}

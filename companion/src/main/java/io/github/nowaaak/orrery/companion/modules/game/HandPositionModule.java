package io.github.nowaaak.orrery.companion.modules.game;

import io.github.nowaaak.orrery.companion.ModuleOptionsScreen;
import io.github.nowaaak.orrery.companion.modules.Module;
import java.util.List;

public class HandPositionModule extends Module {
    private final Option x = option("x", 0, range(-30, 30, 5), ModuleOptionsScreen::hundredths);
    private final Option y = option("y", 0, range(-30, 30, 5), ModuleOptionsScreen::hundredths);
    private final Option z = option("z", 0, range(-30, 30, 5), ModuleOptionsScreen::hundredths);

    public HandPositionModule() {
        super("hand_position", false, false);
    }

    @Override
    public List<Option> options() {
        return List.of(x, y, z);
    }

    public float x() {
        return isActive() ? option(x) / 100f : 0f;
    }

    public float y() {
        return isActive() ? option(y) / 100f : 0f;
    }

    public float z() {
        return isActive() ? option(z) / 100f : 0f;
    }
}

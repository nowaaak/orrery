package io.github.nowaaak.orrery.companion.modules.hud;

import io.github.nowaaak.orrery.companion.modules.ClickTracker;
import io.github.nowaaak.orrery.companion.modules.TextHudModule;
import java.util.List;
import net.minecraft.client.Minecraft;
import net.minecraft.client.resources.language.I18n;

public class CpsModule extends TextHudModule {
    public CpsModule() {
        super("cps", false, false, 0.01f, 0.36f);
    }

    @Override
    public List<String> lines(Minecraft minecraft) {
        return List.of(I18n.get("orrery-companion.hud.cps", ClickTracker.left(), ClickTracker.right()));
    }
}

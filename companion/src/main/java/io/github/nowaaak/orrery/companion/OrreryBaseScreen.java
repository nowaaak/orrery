package io.github.nowaaak.orrery.companion;

import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;
//? if <1.21.11 {
/*import net.minecraft.client.gui.GuiGraphics;
*///?}

public abstract class OrreryBaseScreen extends Screen {
    protected OrreryBaseScreen(Component title) {
        super(title);
    }

    //? if <1.21.11 {
    /*@Override
    public void render(GuiGraphics graphics, int mouseX, int mouseY, float delta) {
        renderBackground(graphics);
        super.render(graphics, mouseX, mouseY, delta);
    }
    *///?}
}

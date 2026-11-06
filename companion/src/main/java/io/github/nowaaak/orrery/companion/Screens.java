package io.github.nowaaak.orrery.companion;

import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.screens.Screen;

public final class Screens {
    private Screens() {
    }

    public static Screen current(Minecraft minecraft) {
        //? if >=26.1 {
        return minecraft.gui.screen();
        //?} else {
        /*return minecraft.screen;
        *///?}
    }

    public static void open(Minecraft minecraft, Screen screen) {
        //? if >=26.1 {
        minecraft.gui.setScreen(screen);
        //?} else {
        /*minecraft.setScreen(screen);
        *///?}
    }
}

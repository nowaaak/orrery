package io.github.nowaaak.orrery.companion.mixin;

import io.github.nowaaak.orrery.companion.title.OrreryTitleScreen;
import net.minecraft.client.gui.screens.Screen;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.ModifyVariable;
//? if >=26.1 {
import net.minecraft.client.gui.Gui;
//?} else {
/*import net.minecraft.client.Minecraft;
*///?}

//? if >=26.1 {
@Mixin(Gui.class)
//?} else {
/*@Mixin(Minecraft.class)
*///?}
public abstract class TitleScreenSwapMixin {
    @ModifyVariable(method = "setScreen", at = @At("HEAD"), argsOnly = true)
    private Screen orrery$swapTitleScreen(Screen screen) {
        return OrreryTitleScreen.replace(screen);
    }
}

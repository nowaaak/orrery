package io.github.nowaaak.orrery.companion.mixin;

import net.minecraft.client.gui.Gui;
import org.spongepowered.asm.mixin.Mixin;
//? if <1.21.11 {
/*import io.github.nowaaak.orrery.companion.hud.Canvas;
import io.github.nowaaak.orrery.companion.modules.Modules;
import net.minecraft.client.gui.GuiGraphics;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
*///?}

@Mixin(Gui.class)
public abstract class CrosshairMixin {
    //? if <1.21.11 {
    /*@Inject(method = "renderCrosshair", at = @At("HEAD"), cancellable = true)
    private void orrery$crosshair(GuiGraphics graphics, CallbackInfo info) {
        if (Modules.CROSSHAIR.replacesVanilla()) {
            Modules.CROSSHAIR.drawInGame(new Canvas(graphics));
            info.cancel();
        }
    }
    *///?}
}

package io.github.nowaaak.orrery.companion.mixin;

import io.github.nowaaak.orrery.companion.modules.ClickTracker;
import io.github.nowaaak.orrery.companion.modules.Modules;
import net.minecraft.client.MouseHandler;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
//? if >=1.21.11 {
import net.minecraft.client.input.MouseButtonInfo;
//?}

@Mixin(MouseHandler.class)
public abstract class MouseHandlerMixin {
    private static final int PRESS = 1;

    //? if >=1.21.11 {
    @Inject(method = "onButton", at = @At("HEAD"))
    private void orrery$countClick(long window, MouseButtonInfo button, int action, CallbackInfo info) {
        if (action == PRESS) {
            ClickTracker.press(button.button());
        }
    }
    //?} else {
    /*@Inject(method = "onPress", at = @At("HEAD"))
    private void orrery$countClick(long window, int button, int action, int modifiers, CallbackInfo info) {
        if (action == PRESS) {
            ClickTracker.press(button);
        }
    }
    *///?}

    @Inject(method = "onScroll", at = @At("HEAD"), cancellable = true)
    private void orrery$zoomScroll(long window, double horizontal, double vertical, CallbackInfo info) {
        if (Modules.ZOOM.scroll(vertical)) {
            info.cancel();
        }
    }
}

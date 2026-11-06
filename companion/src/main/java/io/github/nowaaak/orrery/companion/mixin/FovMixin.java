package io.github.nowaaak.orrery.companion.mixin;

import io.github.nowaaak.orrery.companion.modules.Modules;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;
//? if >=26.1 {
import net.minecraft.client.Camera;
//?} else {
/*import net.minecraft.client.Camera;
import net.minecraft.client.renderer.GameRenderer;
*///?}

//? if >=26.1 {
@Mixin(Camera.class)
//?} else {
/*@Mixin(GameRenderer.class)
*///?}
public abstract class FovMixin {
    //? if >=26.1 {
    @Inject(method = "calculateFov", at = @At("RETURN"), cancellable = true)
    private void orrery$zoom(float partialTick, CallbackInfoReturnable<Float> info) {
        info.setReturnValue(info.getReturnValueF() * Modules.ZOOM.fovFactor());
    }
    //?} else if >=1.21.11 {
    /*@Inject(method = "getFov", at = @At("RETURN"), cancellable = true)
    private void orrery$zoom(Camera camera, float partialTick, boolean useFovSetting, CallbackInfoReturnable<Float> info) {
        if (useFovSetting) {
            info.setReturnValue(info.getReturnValueF() * Modules.ZOOM.fovFactor());
        }
    }
    *///?} else {
    /*@Inject(method = "getFov", at = @At("RETURN"), cancellable = true)
    private void orrery$zoom(Camera camera, float partialTick, boolean useFovSetting, CallbackInfoReturnable<Double> info) {
        if (useFovSetting) {
            info.setReturnValue(info.getReturnValueD() * Modules.ZOOM.fovFactor());
        }
    }
    *///?}
}

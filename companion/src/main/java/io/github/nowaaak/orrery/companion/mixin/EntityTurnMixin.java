package io.github.nowaaak.orrery.companion.mixin;

import io.github.nowaaak.orrery.companion.modules.Modules;
import net.minecraft.client.player.LocalPlayer;
import net.minecraft.world.entity.Entity;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.ModifyVariable;

@Mixin(Entity.class)
public abstract class EntityTurnMixin {
    @ModifyVariable(method = "turn", at = @At("HEAD"), argsOnly = true, ordinal = 0)
    private double orrery$zoomYaw(double yawDelta) {
        return (Object) this instanceof LocalPlayer ? yawDelta * Modules.ZOOM.sensitivity() : yawDelta;
    }

    @ModifyVariable(method = "turn", at = @At("HEAD"), argsOnly = true, ordinal = 1)
    private double orrery$zoomPitch(double pitchDelta) {
        return (Object) this instanceof LocalPlayer ? pitchDelta * Modules.ZOOM.sensitivity() : pitchDelta;
    }
}

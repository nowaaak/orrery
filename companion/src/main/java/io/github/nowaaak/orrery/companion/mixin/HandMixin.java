package io.github.nowaaak.orrery.companion.mixin;

import com.mojang.blaze3d.vertex.PoseStack;
import io.github.nowaaak.orrery.companion.modules.Modules;
import net.minecraft.world.entity.HumanoidArm;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
//? if >=26.1 {
import net.minecraft.client.renderer.FirstPersonHandsAndItemsRenderer;
//?} else {
/*import net.minecraft.client.renderer.ItemInHandRenderer;
*///?}

//? if >=26.1 {
@Mixin(FirstPersonHandsAndItemsRenderer.class)
//?} else {
/*@Mixin(ItemInHandRenderer.class)
*///?}
public abstract class HandMixin {
    @Inject(method = "applyItemArmTransform", at = @At("RETURN"))
    private void orrery$handPosition(PoseStack pose, HumanoidArm arm, float progress, CallbackInfo info) {
        float x = Modules.HAND_POSITION.x();
        float y = Modules.HAND_POSITION.y();
        float z = Modules.HAND_POSITION.z();
        if (x != 0f || y != 0f || z != 0f) {
            pose.translate(arm == HumanoidArm.RIGHT ? x : -x, y, z);
        }
    }
}

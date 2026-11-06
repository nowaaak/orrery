package io.github.nowaaak.orrery.companion.mixin;

import io.github.nowaaak.orrery.companion.emotes.Emotes;
import net.minecraft.client.Minecraft;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
//? if >=1.21.11 {
import net.minecraft.client.model.player.PlayerModel;
import net.minecraft.client.renderer.entity.state.AvatarRenderState;
//?} else {
/*import net.minecraft.client.model.PlayerModel;
import net.minecraft.world.entity.LivingEntity;
*///?}

@Mixin(PlayerModel.class)
public abstract class EmoteMixin {
    //? if >=1.21.11 {
    @Inject(method = "setupAnim(Lnet/minecraft/client/renderer/entity/state/AvatarRenderState;)V", at = @At("RETURN"))
    private void orrery$emote(AvatarRenderState state, CallbackInfo info) {
        Minecraft minecraft = Minecraft.getInstance();
        if (Emotes.current() == null || minecraft.player == null || state.id != minecraft.player.getId()) {
            return;
        }
        PlayerModel model = (PlayerModel) (Object) this;
        Emotes.pose(model.head, model.rightArm, model.leftArm);
    }
    //?} else {
    /*@Inject(method = "setupAnim(Lnet/minecraft/world/entity/LivingEntity;FFFFF)V", at = @At("RETURN"))
    private void orrery$emote(LivingEntity entity, float limbSwing, float limbSwingAmount, float ageInTicks, float headYaw,
            float headPitch, CallbackInfo info) {
        if (Emotes.current() == null || entity != Minecraft.getInstance().player) {
            return;
        }
        PlayerModel<?> model = (PlayerModel<?>) (Object) this;
        Emotes.pose(model.head, model.rightArm, model.leftArm);
        model.hat.copyFrom(model.head);
        model.rightSleeve.copyFrom(model.rightArm);
        model.leftSleeve.copyFrom(model.leftArm);
    }
    *///?}
}

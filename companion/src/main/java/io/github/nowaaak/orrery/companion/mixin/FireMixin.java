package io.github.nowaaak.orrery.companion.mixin;

import com.mojang.blaze3d.vertex.PoseStack;
import io.github.nowaaak.orrery.companion.modules.Modules;
import net.minecraft.client.renderer.ScreenEffectRenderer;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Unique;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
//? if >=26.1 {
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.texture.TextureAtlasSprite;
//?} else if >=1.21.11 {
/*import net.minecraft.client.renderer.MultiBufferSource;
import net.minecraft.client.renderer.texture.TextureAtlasSprite;
*///?} else {
/*import net.minecraft.client.Minecraft;
*///?}

@Mixin(ScreenEffectRenderer.class)
public abstract class FireMixin {
    @Unique
    private static float orrery$applied;

    //? if >=26.1 {
    @Inject(method = "submitFire", at = @At("HEAD"))
    private static void orrery$lowerFire(PoseStack pose, SubmitNodeCollector collector, TextureAtlasSprite sprite, CallbackInfo info) {
        orrery$lower(pose);
    }

    @Inject(method = "submitFire", at = @At("RETURN"))
    private static void orrery$restoreFire(PoseStack pose, SubmitNodeCollector collector, TextureAtlasSprite sprite, CallbackInfo info) {
        orrery$restore(pose);
    }
    //?} else if >=1.21.11 {
    /*@Inject(method = "renderFire", at = @At("HEAD"))
    private static void orrery$lowerFire(PoseStack pose, MultiBufferSource buffers, TextureAtlasSprite sprite, CallbackInfo info) {
        orrery$lower(pose);
    }

    @Inject(method = "renderFire", at = @At("RETURN"))
    private static void orrery$restoreFire(PoseStack pose, MultiBufferSource buffers, TextureAtlasSprite sprite, CallbackInfo info) {
        orrery$restore(pose);
    }
    *///?} else {
    /*@Inject(method = "renderFire", at = @At("HEAD"))
    private static void orrery$lowerFire(Minecraft minecraft, PoseStack pose, CallbackInfo info) {
        orrery$lower(pose);
    }

    @Inject(method = "renderFire", at = @At("RETURN"))
    private static void orrery$restoreFire(Minecraft minecraft, PoseStack pose, CallbackInfo info) {
        orrery$restore(pose);
    }
    *///?}

    @Unique
    private static void orrery$lower(PoseStack pose) {
        orrery$applied = Modules.LOW_FIRE.offset();
        if (orrery$applied != 0f) {
            pose.translate(0f, -orrery$applied, 0f);
        }
    }

    @Unique
    private static void orrery$restore(PoseStack pose) {
        if (orrery$applied != 0f) {
            pose.translate(0f, orrery$applied, 0f);
            orrery$applied = 0f;
        }
    }
}

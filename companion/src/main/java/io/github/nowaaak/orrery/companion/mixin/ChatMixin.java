package io.github.nowaaak.orrery.companion.mixin;

import io.github.nowaaak.orrery.companion.ScreenshotNotice;
import io.github.nowaaak.orrery.companion.modules.Modules;
import java.util.List;
import net.minecraft.client.gui.components.ChatComponent;
import net.minecraft.network.chat.Component;
import org.spongepowered.asm.mixin.Final;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Constant;
import org.spongepowered.asm.mixin.injection.ModifyConstant;
import org.spongepowered.asm.mixin.injection.ModifyVariable;
//? if >=26.1 {
import net.minecraft.client.multiplayer.chat.GuiMessage;
//?} else {
/*import net.minecraft.client.GuiMessage;
*///?}

@Mixin(ChatComponent.class)
public abstract class ChatMixin {
    @Shadow
    @Final
    private List<GuiMessage> allMessages;

    @Shadow
    public abstract void rescaleChat();

    //? if >=26.1 {
    @ModifyVariable(method = "addMessage(Lnet/minecraft/network/chat/Component;Lnet/minecraft/network/chat/MessageSignature;Lnet/minecraft/client/multiplayer/chat/GuiMessageSource;Lnet/minecraft/client/multiplayer/chat/GuiMessageTag;)V",
            at = @At("HEAD"), argsOnly = true)
    //?} else {
    /*@ModifyVariable(method = "addMessage(Lnet/minecraft/network/chat/Component;Lnet/minecraft/network/chat/MessageSignature;Lnet/minecraft/client/GuiMessageTag;)V",
            at = @At("HEAD"), argsOnly = true)
    *///?}
    private Component orrery$decorate(Component message) {
        ScreenshotNotice.check(message);
        String newest = allMessages.isEmpty() ? null : allMessages.get(0).content().getString();
        return Modules.CHAT.decorate(message, newest, () -> {
            allMessages.remove(0);
            rescaleChat();
        });
    }

    //? if >=1.21.11 {
    @ModifyConstant(method = {"addMessageToDisplayQueue", "addMessageToQueue"}, constant = @Constant(intValue = 100))
    //?} else {
    /*@ModifyConstant(method = "addMessage(Lnet/minecraft/network/chat/Component;Lnet/minecraft/network/chat/MessageSignature;ILnet/minecraft/client/GuiMessageTag;Z)V",
            constant = @Constant(intValue = 100))
    *///?}
    private int orrery$history(int limit) {
        return Modules.CHAT.historyLimit(limit);
    }
}

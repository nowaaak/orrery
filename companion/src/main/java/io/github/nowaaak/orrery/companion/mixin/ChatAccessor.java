package io.github.nowaaak.orrery.companion.mixin;

import java.util.List;
import net.minecraft.client.gui.components.ChatComponent;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.gen.Accessor;
//? if >=26.1 {
import net.minecraft.client.multiplayer.chat.GuiMessage;
//?} else {
/*import net.minecraft.client.GuiMessage;
*///?}

@Mixin(ChatComponent.class)
public interface ChatAccessor {
    @Accessor("allMessages")
    List<GuiMessage> orrery$messages();
}

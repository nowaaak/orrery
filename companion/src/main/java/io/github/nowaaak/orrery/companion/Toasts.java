package io.github.nowaaak.orrery.companion;

import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.components.toasts.SystemToast;
import net.minecraft.network.chat.Component;

public final class Toasts {
    private Toasts() {
    }

    public static void show(Minecraft minecraft, String title, String body) {
        Component heading = Component.literal(title);
        Component message = body.isEmpty() ? null : Component.literal(body);
        //? if >=26.1 {
        SystemToast.add(minecraft.gui.toastManager(), SystemToast.SystemToastId.PERIODIC_NOTIFICATION, heading, message);
        //?} else if >=1.21.11 {
        /*SystemToast.add(minecraft.getToastManager(), SystemToast.SystemToastId.PERIODIC_NOTIFICATION, heading, message);
        *///?} else {
        /*SystemToast.add(minecraft.getToasts(), SystemToast.SystemToastIds.PERIODIC_NOTIFICATION, heading, message);
        *///?}
    }
}

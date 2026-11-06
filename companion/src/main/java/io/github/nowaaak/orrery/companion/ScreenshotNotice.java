package io.github.nowaaak.orrery.companion;

import com.google.gson.JsonObject;
import net.minecraft.network.chat.Component;
import net.minecraft.network.chat.contents.TranslatableContents;

public final class ScreenshotNotice {
    private static final String SUCCESS_KEY = "screenshot.success";

    private ScreenshotNotice() {
    }

    public static void check(Component message) {
        Bridge bridge = OrreryCompanion.bridge();
        if (bridge == null || !bridge.isConnected()) {
            return;
        }
        if (!(message.getContents() instanceof TranslatableContents contents) || !contents.getKey().equals(SUCCESS_KEY)) {
            return;
        }
        Object[] args = contents.getArgs();
        if (args.length == 0) {
            return;
        }
        String file = args[0] instanceof Component name ? name.getString() : String.valueOf(args[0]);
        JsonObject notice = new JsonObject();
        notice.addProperty("type", "screenshot");
        notice.addProperty("file", file);
        bridge.send(notice);
    }
}

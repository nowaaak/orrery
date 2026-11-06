package io.github.nowaaak.orrery.companion.modules.game;

import io.github.nowaaak.orrery.companion.ModuleOptionsScreen;
import io.github.nowaaak.orrery.companion.modules.Module;
import java.time.LocalTime;
import java.time.format.DateTimeFormatter;
import java.util.List;
import net.minecraft.ChatFormatting;
import net.minecraft.network.chat.Component;
import net.minecraft.network.chat.MutableComponent;

public class ChatModule extends Module {
    private static final DateTimeFormatter TIME = DateTimeFormatter.ofPattern("HH:mm");

    private final Option timestamps = option("timestamps", 1, List.of(0, 1), ChatModule::onOff);
    private final Option stacking = option("stacking", 1, List.of(0, 1), ChatModule::onOff);
    private final Option history = option("history", 500, List.of(100, 200, 500, 1000), ModuleOptionsScreen::number);

    private String lastPlain;
    private String lastDecorated;
    private int count;

    public ChatModule() {
        super("chat", false, false);
    }

    @Override
    public List<Option> options() {
        return List.of(timestamps, stacking, history);
    }

    public Component decorate(Component message, String newest, Runnable removeNewest) {
        if (!isActive()) {
            lastPlain = null;
            lastDecorated = null;
            return message;
        }
        String plain = message.getString();
        if (option(stacking) == 1 && plain.equals(lastPlain) && lastDecorated != null && lastDecorated.equals(newest)) {
            count++;
            removeNewest.run();
        } else {
            count = 1;
            lastPlain = plain;
        }
        MutableComponent result = Component.empty();
        if (option(timestamps) == 1) {
            result.append(Component.literal("[" + LocalTime.now().format(TIME) + "] ").withStyle(ChatFormatting.DARK_GRAY));
        }
        result.append(message);
        if (count > 1) {
            result.append(Component.literal(" ×" + count).withStyle(ChatFormatting.GRAY));
        }
        lastDecorated = result.getString();
        return result;
    }

    public int historyLimit(int vanilla) {
        return isActive() ? Math.max(vanilla, option(history)) : vanilla;
    }

    private static Component onOff(int value) {
        return Component.translatable(value == 1 ? "options.on" : "options.off");
    }
}

package io.github.nowaaak.orrery.companion.modules;

import com.mojang.blaze3d.platform.InputConstants;
import io.github.nowaaak.orrery.companion.Screens;
import java.util.ArrayDeque;
import java.util.Deque;
import net.minecraft.client.Minecraft;

public final class ClickTracker {
    private static final long WINDOW_MILLIS = 1000;
    private static final Deque<Long> LEFT = new ArrayDeque<>();
    private static final Deque<Long> RIGHT = new ArrayDeque<>();

    private ClickTracker() {
    }

    public static void press(int button) {
        if (Screens.current(Minecraft.getInstance()) != null) {
            return;
        }
        long now = System.currentTimeMillis();
        if (button == InputConstants.MOUSE_BUTTON_LEFT) {
            LEFT.addLast(now);
        } else if (button == InputConstants.MOUSE_BUTTON_RIGHT) {
            RIGHT.addLast(now);
        }
    }

    public static int left() {
        return count(LEFT);
    }

    public static int right() {
        return count(RIGHT);
    }

    private static int count(Deque<Long> clicks) {
        long cutoff = System.currentTimeMillis() - WINDOW_MILLIS;
        while (!clicks.isEmpty() && clicks.peekFirst() < cutoff) {
            clicks.removeFirst();
        }
        return clicks.size();
    }
}

package io.github.nowaaak.orrery.companion;

import com.mojang.blaze3d.platform.InputConstants;
import io.github.nowaaak.orrery.companion.hud.Canvas;
import io.github.nowaaak.orrery.companion.mixin.ChatAccessor;
import java.util.ArrayList;
import java.util.List;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.components.ChatComponent;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.resources.language.I18n;
import net.minecraft.network.chat.CommonComponents;
import net.minecraft.network.chat.Component;
//? if >=26.1 {
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.multiplayer.chat.GuiMessage;
//?} else {
/*import net.minecraft.client.GuiMessage;
import net.minecraft.client.gui.GuiGraphics;
*///?}
//? if >=1.21.11 {
import net.minecraft.client.input.MouseButtonEvent;
//?}

public class ChatHistoryScreen extends OrreryBaseScreen {
    private static final int LINE = 12;
    private static final int SIDE = 20;
    private static final int TOP = 32;
    private static final int BOTTOM = 50;
    private static final int SCROLL_STEP = 3;
    private static final long COPIED_MILLIS = 1500;
    private static final int HOVER = 0x22FFFFFF;
    private static final int TEXT = 0xFFE0E0E0;
    private static final int MUTED = 0xFF9A9AA6;
    private static final int COPIED = 0xFF6EE7A0;

    private final Screen parent;
    private final List<String> lines = new ArrayList<>();
    private int scroll;
    private int copiedIndex = -1;
    private long copiedAt;

    public ChatHistoryScreen(Screen parent) {
        super(Component.translatable("orrery-companion.chat_history.title"));
        this.parent = parent;
    }

    public static List<String> messages(Minecraft minecraft) {
        //? if >=26.1 {
        ChatComponent chat = minecraft.gui.hud.getChat();
        //?} else {
        /*ChatComponent chat = minecraft.gui.getChat();
        *///?}
        List<String> result = new ArrayList<>();
        List<GuiMessage> messages = ((ChatAccessor) chat).orrery$messages();
        for (int i = messages.size() - 1; i >= 0; i--) {
            result.add(messages.get(i).content().getString());
        }
        return result;
    }

    @Override
    protected void init() {
        lines.clear();
        lines.addAll(messages(minecraft));
        addRenderableWidget(Button.builder(CommonComponents.GUI_DONE, button -> onClose())
                .bounds(width / 2 - 100, height - 28, 200, 20)
                .build());
    }

    private int visibleRows() {
        return Math.max(1, (height - TOP - BOTTOM) / LINE);
    }

    private int firstIndex() {
        return Math.max(0, lines.size() - visibleRows() - scroll);
    }

    private int indexAt(double x, double y) {
        if (x < SIDE || x >= width - SIDE || y < TOP || y >= TOP + visibleRows() * LINE) {
            return -1;
        }
        int index = firstIndex() + (int) ((y - TOP) / LINE);
        return index < lines.size() ? index : -1;
    }

    private boolean press(double x, double y, int button) {
        int index = button == InputConstants.MOUSE_BUTTON_LEFT ? indexAt(x, y) : -1;
        if (index < 0) {
            return false;
        }
        minecraft.keyboardHandler.setClipboard(lines.get(index));
        copiedIndex = index;
        copiedAt = System.currentTimeMillis();
        return true;
    }

    private boolean scroll(double amount) {
        if (amount == 0) {
            return false;
        }
        int maxScroll = Math.max(0, lines.size() - visibleRows());
        scroll = Math.max(0, Math.min(maxScroll, scroll + (amount > 0 ? SCROLL_STEP : -SCROLL_STEP)));
        return true;
    }

    private void draw(Canvas canvas, int mouseX, int mouseY) {
        canvas.text(font, title.getString(), (width - font.width(title)) / 2, 12, TEXT);
        if (lines.isEmpty()) {
            String empty = I18n.get("orrery-companion.chat_history.empty");
            canvas.text(font, empty, (width - font.width(empty)) / 2, height / 2 - 4, MUTED);
            return;
        }
        String hint = I18n.get("orrery-companion.chat_history.hint");
        canvas.text(font, hint, (width - font.width(hint)) / 2, height - BOTTOM + 6, MUTED);
        int hovered = indexAt(mouseX, mouseY);
        boolean showCopied = copiedIndex >= 0 && System.currentTimeMillis() - copiedAt < COPIED_MILLIS;
        String copiedLabel = I18n.get("orrery-companion.chat_history.copied");
        int first = firstIndex();
        int rows = Math.min(visibleRows(), lines.size() - first);
        for (int row = 0; row < rows; row++) {
            int index = first + row;
            int y = TOP + row * LINE;
            if (index == hovered) {
                canvas.fill(SIDE, y, width - SIDE, y + LINE, HOVER);
            }
            boolean copied = showCopied && index == copiedIndex;
            int reserved = copied ? font.width(copiedLabel) + 8 : 0;
            String text = font.plainSubstrByWidth(lines.get(index), width - SIDE * 2 - 8 - reserved);
            canvas.text(font, text, SIDE + 4, y + 2, TEXT);
            if (copied) {
                canvas.text(font, copiedLabel, width - SIDE - 4 - font.width(copiedLabel), y + 2, COPIED);
            }
        }
    }

    @Override
    public void onClose() {
        Screens.open(minecraft, parent);
    }

    //? if >=26.1 {
    @Override
    public void extractRenderState(GuiGraphicsExtractor graphics, int mouseX, int mouseY, float delta) {
        super.extractRenderState(graphics, mouseX, mouseY, delta);
        draw(new Canvas(graphics), mouseX, mouseY);
    }
    //?} else {
    /*@Override
    public void render(GuiGraphics graphics, int mouseX, int mouseY, float delta) {
        super.render(graphics, mouseX, mouseY, delta);
        draw(new Canvas(graphics), mouseX, mouseY);
    }
    *///?}

    //? if >=1.21.11 {
    @Override
    public boolean mouseClicked(MouseButtonEvent event, boolean doubleClick) {
        return press(event.x(), event.y(), event.button()) || super.mouseClicked(event, doubleClick);
    }

    @Override
    public boolean mouseScrolled(double x, double y, double horizontal, double vertical) {
        return scroll(vertical) || super.mouseScrolled(x, y, horizontal, vertical);
    }
    //?} else {
    /*@Override
    public boolean mouseClicked(double x, double y, int button) {
        return press(x, y, button) || super.mouseClicked(x, y, button);
    }

    @Override
    public boolean mouseScrolled(double x, double y, double amount) {
        return scroll(amount) || super.mouseScrolled(x, y, amount);
    }
    *///?}
}

package io.github.nowaaak.orrery.companion.emotes;

import io.github.nowaaak.orrery.companion.OrreryBaseScreen;
import io.github.nowaaak.orrery.companion.OrreryCompanion;
import io.github.nowaaak.orrery.companion.Screens;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.components.StringWidget;
import net.minecraft.network.chat.Component;

public class EmoteScreen extends OrreryBaseScreen {
    private static final int BUTTON_WIDTH = 98;
    private static final int GAP = 4;
    private static final int ROW = 24;

    public EmoteScreen() {
        super(Component.translatable(OrreryCompanion.ID + ".emotes.title"));
    }

    @Override
    protected void init() {
        Emote[] emotes = Emote.values();
        int rows = (emotes.length + 1) / 2;
        int top = (height - (rows * ROW + 20)) / 2;
        addRenderableWidget(new StringWidget(width / 2 - 100, top, 200, 12, title, font));
        int y = top + 20;
        for (int index = 0; index < emotes.length; index++) {
            Emote emote = emotes[index];
            int x = width / 2 - BUTTON_WIDTH - GAP / 2 + (index % 2) * (BUTTON_WIDTH + GAP);
            addRenderableWidget(Button.builder(Component.translatable(OrreryCompanion.ID + ".emotes." + emote.key()), button -> {
                Screens.open(minecraft, null);
                Emotes.play(minecraft, emote);
            }).bounds(x, y + (index / 2) * ROW, BUTTON_WIDTH, 20).build());
        }
    }

    @Override
    public boolean isPauseScreen() {
        return false;
    }
}

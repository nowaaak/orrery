package io.github.nowaaak.orrery.companion;

import io.github.nowaaak.orrery.companion.hud.HudEditorScreen;
import io.github.nowaaak.orrery.companion.title.OrreryTitleScreen;
import net.minecraft.client.gui.screens.TitleScreen;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.components.CycleButton;
import net.minecraft.client.gui.components.StringWidget;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.CommonComponents;
import net.minecraft.network.chat.Component;

public class OrreryScreen extends OrreryBaseScreen {
    private static final int WIDTH = 300;
    private static final int LINE = 14;
    private final Screen parent;

    public OrreryScreen(Screen parent) {
        super(Component.translatable("orrery-companion.menu.title"));
        this.parent = parent;
    }

    @Override
    protected void init() {
        int x = width / 2 - WIDTH / 2;
        int y = Math.max(12, (height - (LINE * 6 + 24 * 3 + 20)) / 2);
        line(x, y, title);
        y += LINE * 2;
        line(x, y, Component.translatable("orrery-companion.menu.version", OrreryCompanion.VERSION, OrreryCompanion.MINECRAFT));
        y += LINE;
        Bridge bridge = OrreryCompanion.bridge();
        line(x, y, Component.translatable(bridge == null
                ? "orrery-companion.menu.bridge.none"
                : bridge.isConnected() ? "orrery-companion.menu.bridge.connected" : "orrery-companion.menu.bridge.waiting"));
        y += LINE;
        line(x, y, policyLine());
        y += LINE * 2;
        addRenderableWidget(Button.builder(Component.translatable("orrery-companion.menu.modules"), button -> Screens.open(minecraft, new ModulesScreen(this)))
                .bounds(width / 2 - 100, y, 98, 20)
                .build());
        addRenderableWidget(Button.builder(Component.translatable("orrery-companion.modules.edit_hud"), button -> Screens.open(minecraft, new HudEditorScreen(this)))
                .bounds(width / 2 + 2, y, 98, 20)
                .build());
        y += 24;
        addRenderableWidget(CycleButton.onOffBuilder(Settings.orreryTitleScreen())
                .create(width / 2 - 100, y, 200, 20, Component.translatable("orrery-companion.menu.orrery_title"),
                        (button, enabled) -> Settings.setOrreryTitleScreen(enabled)));
        y += 24;
        addRenderableWidget(CycleButton.onOffBuilder(Settings.titleScreen())
                .create(width / 2 - 100, y, 200, 20, Component.translatable("orrery-companion.menu.title_screen"),
                        (button, enabled) -> Settings.setTitleScreen(enabled)));
        y += 24;
        addRenderableWidget(Button.builder(Component.translatable("orrery-companion.chat_history.title"), button -> Screens.open(minecraft, new ChatHistoryScreen(this)))
                .bounds(width / 2 - 100, y, 98, 20)
                .build());
        addRenderableWidget(Button.builder(CommonComponents.GUI_DONE, button -> onClose())
                .bounds(width / 2 + 2, y, 98, 20)
                .build());
    }

    private Component policyLine() {
        boolean onServer = minecraft.level != null && minecraft.getSingleplayerServer() == null;
        if (!onServer) {
            return Component.translatable("orrery-companion.menu.policy.local");
        }
        return Policy.disabled().isEmpty()
                ? Component.translatable("orrery-companion.menu.policy.none")
                : Component.translatable("orrery-companion.menu.policy.disabled", String.join(", ", Policy.disabled()));
    }

    private void line(int x, int y, Component text) {
        addRenderableWidget(new StringWidget(x, y, WIDTH, LINE, text, font));
    }

    @Override
    public void onClose() {
        boolean fromTitle = parent instanceof TitleScreen || parent instanceof OrreryTitleScreen;
        Screens.open(minecraft, fromTitle ? new TitleScreen() : parent);
    }
}

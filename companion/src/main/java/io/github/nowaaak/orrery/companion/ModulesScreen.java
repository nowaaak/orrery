package io.github.nowaaak.orrery.companion;

import io.github.nowaaak.orrery.companion.hud.HudEditorScreen;
import io.github.nowaaak.orrery.companion.modules.Module;
import io.github.nowaaak.orrery.companion.modules.ModuleSettings;
import io.github.nowaaak.orrery.companion.modules.Modules;
import java.util.List;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.components.CycleButton;
import net.minecraft.client.gui.components.StringWidget;
import net.minecraft.client.gui.components.Tooltip;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.CommonComponents;
import net.minecraft.network.chat.Component;

public class ModulesScreen extends OrreryBaseScreen {
    private static final int COLUMN_WIDTH = 150;
    private static final int COLUMN_GAP = 10;
    private static final int ROW = 24;
    private final Screen parent;

    public ModulesScreen(Screen parent) {
        super(Component.translatable("orrery-companion.modules.title"));
        this.parent = parent;
    }

    @Override
    protected void init() {
        List<Module> modules = Modules.all();
        int columns = width >= COLUMN_WIDTH * 3 + COLUMN_GAP * 2 + 40 ? 3 : 2;
        int columnWidth = Math.min(COLUMN_WIDTH, (width - 40 - COLUMN_GAP * (columns - 1)) / columns);
        int rows = (modules.size() + columns - 1) / columns;
        int gridWidth = columnWidth * columns + COLUMN_GAP * (columns - 1);
        int left = (width - gridWidth) / 2;
        int top = Math.max(32, (height - rows * ROW - ROW - 20) / 2);
        int titleWidth = font.width(title);
        addRenderableWidget(new StringWidget(width / 2 - titleWidth / 2, top - 22, titleWidth, font.lineHeight, title, font));
        for (int i = 0; i < modules.size(); i++) {
            Module module = modules.get(i);
            int x = left + (i % columns) * (columnWidth + COLUMN_GAP);
            int y = top + (i / columns) * ROW;
            boolean configurable = module.hasSettings();
            int toggleWidth = configurable ? columnWidth - 22 : columnWidth;
            CycleButton<Boolean> button = CycleButton.onOffBuilder(module.isEnabled())
                    .create(x, y, toggleWidth, 20, module.name(), (cycle, enabled) -> ModuleSettings.setEnabled(module, enabled));
            if (configurable) {
                Button settings = Button.builder(Component.literal("…"), pressed -> Screens.open(minecraft, module.settingsScreen(this)))
                        .bounds(x + columnWidth - 20, y, 20, 20)
                        .build();
                settings.setTooltip(Tooltip.create(Component.translatable("orrery-companion.modules.settings")));
                addRenderableWidget(settings);
            }
            if (!module.isAllowed()) {
                button.active = false;
                button.setTooltip(Tooltip.create(Component.translatable("orrery-companion.modules.blocked")));
            } else if (module.oftenForbidden()) {
                button.setTooltip(Tooltip.create(Component.translatable("orrery-companion.modules.often_forbidden")));
            }
            addRenderableWidget(button);
        }
        int bottom = top + rows * ROW + ROW / 2;
        addRenderableWidget(Button.builder(Component.translatable("orrery-companion.modules.edit_hud"), button -> Screens.open(minecraft, new HudEditorScreen(this)))
                .bounds(width / 2 - 155, bottom, 150, 20)
                .build());
        addRenderableWidget(Button.builder(CommonComponents.GUI_DONE, button -> onClose())
                .bounds(width / 2 + 5, bottom, 150, 20)
                .build());
    }

    @Override
    public void onClose() {
        Screens.open(minecraft, parent);
    }
}

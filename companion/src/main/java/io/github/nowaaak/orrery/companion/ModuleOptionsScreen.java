package io.github.nowaaak.orrery.companion;

import io.github.nowaaak.orrery.companion.hud.Canvas;
import io.github.nowaaak.orrery.companion.modules.Module;
import io.github.nowaaak.orrery.companion.modules.ModuleSettings;
import java.util.List;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.components.CycleButton;
import net.minecraft.client.gui.components.StringWidget;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.CommonComponents;
import net.minecraft.network.chat.Component;
//? if >=26.1 {
import net.minecraft.client.gui.GuiGraphicsExtractor;
//?} else {
/*import net.minecraft.client.gui.GuiGraphics;
*///?}

public class ModuleOptionsScreen extends OrreryBaseScreen {
    private static final int BUTTON_WIDTH = 200;
    private static final int ROW = 24;
    private static final int PREVIEW = 70;
    private static final int COLUMN_GAP = 10;
    private final Screen parent;
    private final Module module;
    private int previewY;

    public ModuleOptionsScreen(Screen parent, Module module) {
        super(module.name());
        this.parent = parent;
        this.module = module;
    }

    @Override
    protected void init() {
        List<Module.Option> options = module.options();
        int previewHeight = module.hasPreview() ? PREVIEW : 0;
        int columns = 22 + previewHeight + options.size() * ROW + ROW + 4 > height - 16 ? 2 : 1;
        int columnWidth = columns == 1 ? BUTTON_WIDTH : (BUTTON_WIDTH * 3 / 2 - COLUMN_GAP) / 2;
        int rows = (options.size() + columns - 1) / columns;
        int total = 22 + previewHeight + rows * ROW + ROW + 4;
        int top = Math.max(8, (height - total) / 2);
        int titleWidth = font.width(title);
        addRenderableWidget(new StringWidget(width / 2 - titleWidth / 2, top, titleWidth, font.lineHeight, title, font));
        previewY = top + 22 + previewHeight / 2;
        int y = top + 22 + previewHeight;
        int gridWidth = columnWidth * columns + COLUMN_GAP * (columns - 1);
        int left = width / 2 - gridWidth / 2;
        for (int i = 0; i < options.size(); i++) {
            addRenderableWidget(cycle(options.get(i), left + (i % columns) * (columnWidth + COLUMN_GAP), y + (i / columns) * ROW, columnWidth));
        }
        addRenderableWidget(Button.builder(CommonComponents.GUI_DONE, button -> onClose())
                .bounds(width / 2 - BUTTON_WIDTH / 2, y + rows * ROW + 4, BUTTON_WIDTH, 20)
                .build());
    }

    private CycleButton<Integer> cycle(Module.Option option, int x, int y, int buttonWidth) {
        int current = module.option(option);
        //? if >=1.21.11 {
        return CycleButton.builder((Integer value) -> option.display().apply(value), current)
                .withValues(option.values())
                .create(x, y, buttonWidth, 20, option.label(), (button, value) -> ModuleSettings.setOption(module, option.key(), value));
        //?} else {
        /*return CycleButton.builder((Integer value) -> option.display().apply(value))
                .withValues(option.values())
                .withInitialValue(current)
                .create(x, y, buttonWidth, 20, option.label(), (button, value) -> ModuleSettings.setOption(module, option.key(), value));
        *///?}
    }

    private void drawPreview(Canvas canvas) {
        if (module.hasPreview()) {
            canvas.fill(width / 2 - PREVIEW / 2, previewY - PREVIEW / 2 + 4, width / 2 + PREVIEW / 2, previewY + PREVIEW / 2 - 4, 0x60000000);
            module.drawPreview(canvas, width / 2, previewY);
        }
    }

    @Override
    public void onClose() {
        Screens.open(minecraft, parent);
    }

    public static Component number(int value) {
        return Component.literal(Integer.toString(value));
    }

    public static Component hundredths(int value) {
        return Component.literal(String.format(java.util.Locale.ROOT, "%.2f", value / 100f));
    }

    //? if >=26.1 {
    @Override
    public void extractRenderState(GuiGraphicsExtractor graphics, int mouseX, int mouseY, float delta) {
        super.extractRenderState(graphics, mouseX, mouseY, delta);
        drawPreview(new Canvas(graphics));
    }
    //?} else {
    /*@Override
    public void render(GuiGraphics graphics, int mouseX, int mouseY, float delta) {
        super.render(graphics, mouseX, mouseY, delta);
        drawPreview(new Canvas(graphics));
    }
    *///?}
}

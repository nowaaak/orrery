package io.github.nowaaak.orrery.companion;

import io.github.nowaaak.orrery.companion.modules.ModuleSettings;
import io.github.nowaaak.orrery.companion.modules.Modules;
import io.github.nowaaak.orrery.companion.modules.game.Waypoints;
import io.github.nowaaak.orrery.companion.modules.game.WaypointsModule;
import java.util.List;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.components.CycleButton;
import net.minecraft.client.gui.components.EditBox;
import net.minecraft.client.gui.components.StringWidget;
import net.minecraft.client.gui.components.Tooltip;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.CommonComponents;
import net.minecraft.network.chat.Component;

public class WaypointsScreen extends OrreryBaseScreen {
    private static final int WIDTH = 300;
    private static final int ROW = 22;
    private static final int MAX_NAME = 32;
    private final Screen parent;
    private EditBox name;

    public WaypointsScreen(Screen parent) {
        super(Component.translatable("orrery-companion.module.waypoints"));
        this.parent = parent;
    }

    @Override
    protected void init() {
        int left = width / 2 - WIDTH / 2;
        int y = 12;
        int titleWidth = font.width(title);
        addRenderableWidget(new StringWidget(width / 2 - titleWidth / 2, y, titleWidth, font.lineHeight, title, font));
        y += 18;
        boolean inWorld = WaypointsModule.available(minecraft);
        String unavailable = minecraft.player != null && minecraft.level != null
                ? "orrery-companion.waypoints.reduced"
                : "orrery-companion.waypoints.need_world";
        String previous = name == null ? "" : name.getValue();
        name = new EditBox(font, left, y, WIDTH - 104, 20, Component.translatable("orrery-companion.waypoints.name"));
        name.setMaxLength(MAX_NAME);
        name.setValue(previous);
        name.active = inWorld;
        addRenderableWidget(name);
        Button add = addRenderableWidget(Button.builder(Component.translatable("orrery-companion.waypoints.add"), button -> addHere())
                .bounds(left + WIDTH - 100, y, 100, 20)
                .build());
        add.active = inWorld;
        if (!inWorld) {
            add.setTooltip(Tooltip.create(Component.translatable(unavailable)));
        }
        y += 24;
        addRenderableWidget(CycleButton.onOffBuilder(Modules.WAYPOINTS.option(Modules.WAYPOINTS.deathOption()) == 1)
                .create(left, y, WIDTH, 20, Component.translatable("orrery-companion.module.waypoints.death"),
                        (button, enabled) -> ModuleSettings.setOption(Modules.WAYPOINTS, "death", enabled ? 1 : 0)));
        y += 28;

        List<Waypoints.Waypoint> waypoints = inWorld ? Modules.WAYPOINTS.list(minecraft) : List.of();
        int rows = Math.max(0, (height - y - 36) / ROW);
        if (waypoints.isEmpty()) {
            Component empty = Component.translatable(inWorld ? "orrery-companion.waypoints.empty" : unavailable);
            int emptyWidth = font.width(empty);
            addRenderableWidget(new StringWidget(width / 2 - emptyWidth / 2, y + 4, emptyWidth, font.lineHeight, empty, font));
        }
        for (int i = 0; i < Math.min(rows, waypoints.size()); i++) {
            Waypoints.Waypoint waypoint = waypoints.get(i);
            Component label = Component.translatable("orrery-companion.waypoints.row", waypoint.name(), waypoint.x(), waypoint.y(), waypoint.z(),
                    shortDimension(waypoint.dimension()));
            addRenderableWidget(new StringWidget(left, y + i * ROW + 6, WIDTH - 28, font.lineHeight, label, font));
            Button delete = addRenderableWidget(Button.builder(Component.literal("×"), button -> {
                Modules.WAYPOINTS.remove(minecraft, waypoint);
                rebuildWidgets();
            }).bounds(left + WIDTH - 20, y + i * ROW, 20, 20).build());
            delete.setTooltip(Tooltip.create(Component.translatable("orrery-companion.waypoints.delete")));
        }
        if (waypoints.size() > rows && rows > 0) {
            Component more = Component.translatable("orrery-companion.waypoints.more", waypoints.size() - rows);
            int moreWidth = font.width(more);
            addRenderableWidget(new StringWidget(width / 2 - moreWidth / 2, height - 44, moreWidth, font.lineHeight, more, font));
        }
        addRenderableWidget(Button.builder(CommonComponents.GUI_DONE, button -> onClose())
                .bounds(width / 2 - 100, height - 28, 200, 20)
                .build());
    }

    private void addHere() {
        String value = name.getValue().isBlank()
                ? net.minecraft.client.resources.language.I18n.get("orrery-companion.waypoints.default", Modules.WAYPOINTS.list(minecraft).size() + 1)
                : name.getValue();
        Modules.WAYPOINTS.add(minecraft, value);
        name.setValue("");
        rebuildWidgets();
    }

    private static String shortDimension(String dimension) {
        return switch (dimension) {
            case "minecraft:overworld" -> "Overworld";
            case "minecraft:the_nether" -> "Nether";
            case "minecraft:the_end" -> "End";
            default -> dimension;
        };
    }

    @Override
    public void onClose() {
        Screens.open(minecraft, parent);
    }
}

package io.github.nowaaak.orrery.companion;

import java.util.List;
import net.fabricmc.fabric.api.client.screen.v1.ScreenEvents;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.components.AbstractWidget;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.components.StringWidget;
import net.minecraft.client.gui.components.Tooltip;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.gui.screens.TitleScreen;
import net.minecraft.network.chat.Component;
import net.minecraft.network.chat.contents.TranslatableContents;
//? if >=1.21.11 {
import net.minecraft.client.gui.components.PlayerSkinWidget;
//?}

public final class TitleScreenExtras {
    private static final int GAP = 4;
    private static final int ICON_SIZE = 20;
    private static final int LOGO_BOTTOM = 76;
    private static final int SKIN_WIDTH = 60;
    private static final int SKIN_HEIGHT = 90;

    private TitleScreenExtras() {
    }

    public static void register() {
        ScreenEvents.AFTER_INIT.register((minecraft, screen, width, height) -> {
            if (screen instanceof TitleScreen) {
                decorate(minecraft, screen);
            }
        });
    }

    private static void decorate(Minecraft minecraft, Screen screen) {
        List<AbstractWidget> widgets = widgets(screen);
        AbstractWidget singleplayer = widgets.stream().filter(widget -> hasKey(widget, "menu.singleplayer")).findFirst().orElse(null);
        if (singleplayer == null) {
            return;
        }
        int x = singleplayer.getX();
        int y = singleplayer.getY();
        int rowHeight = singleplayer.getHeight() + GAP;
        Component menuTitle = Component.translatable("orrery-companion.menu.title");
        widgets.add(Button.builder(Component.literal("O"), button -> Screens.open(minecraft, new OrreryScreen(screen)))
                .bounds(x - ICON_SIZE - GAP, y, ICON_SIZE, ICON_SIZE)
                .tooltip(Tooltip.create(menuTitle))
                .build());
        if (!Settings.titleScreen()) {
            return;
        }

        LastPlayed last = LastPlayed.get();
        int continueY = y - rowHeight;
        if (last != null && continueY >= LOGO_BOTTOM && last.isAvailable(minecraft)) {
            String tooltipKey = last.isWorld() ? "orrery-companion.title.continue.world" : "orrery-companion.title.continue.server";
            widgets.add(Button.builder(Component.translatable("orrery-companion.title.continue", last.name()), button -> last.open(minecraft, screen))
                    .bounds(x, continueY, singleplayer.getWidth(), singleplayer.getHeight())
                    .tooltip(Tooltip.create(Component.translatable(tooltipKey, last.name(), last.target())))
                    .build());
        }

        int areaWidth = x - ICON_SIZE - GAP * 3;
        if (areaWidth >= SKIN_WIDTH + GAP * 2) {
            addAccount(minecraft, widgets, areaWidth, y - rowHeight);
        }
    }

    private static void addAccount(Minecraft minecraft, List<AbstractWidget> widgets, int areaWidth, int top) {
        int center = GAP + areaWidth / 2;
        int nameY = top;
        //? if >=1.21.11 {
        PlayerSkinWidget skin = new PlayerSkinWidget(SKIN_WIDTH, SKIN_HEIGHT, minecraft.getEntityModels(),
                minecraft.getSkinManager().createLookup(minecraft.getGameProfile(), false));
        skin.setX(center - SKIN_WIDTH / 2);
        skin.setY(top);
        widgets.add(skin);
        nameY = top + SKIN_HEIGHT + GAP;
        //?}
        Component name = Component.literal(minecraft.getUser().getName());
        int nameWidth = Math.min(minecraft.font.width(name), areaWidth);
        widgets.add(new StringWidget(center - nameWidth / 2, nameY, nameWidth, minecraft.font.lineHeight, name, minecraft.font));
    }

    private static boolean hasKey(AbstractWidget widget, String key) {
        return widget.getMessage().getContents() instanceof TranslatableContents contents && contents.getKey().equals(key);
    }

    private static List<AbstractWidget> widgets(Screen screen) {
        //? if >=26.1 {
        return net.fabricmc.fabric.api.client.screen.v1.Screens.getWidgets(screen);
        //?} else {
        /*return net.fabricmc.fabric.api.client.screen.v1.Screens.getButtons(screen);
        *///?}
    }
}

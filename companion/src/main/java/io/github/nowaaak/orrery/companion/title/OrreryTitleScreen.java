package io.github.nowaaak.orrery.companion.title;

import com.mojang.realmsclient.RealmsMainScreen;
import io.github.nowaaak.orrery.companion.LastPlayed;
import io.github.nowaaak.orrery.companion.OrreryCompanion;
import io.github.nowaaak.orrery.companion.OrreryScreen;
import io.github.nowaaak.orrery.companion.Screens;
import io.github.nowaaak.orrery.companion.Settings;
import io.github.nowaaak.orrery.companion.hud.Canvas;
import io.github.nowaaak.orrery.companion.ui.OrreryButton;
import io.github.nowaaak.orrery.companion.ui.Theme;
import java.time.LocalTime;
import net.fabricmc.loader.api.FabricLoader;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.components.Tooltip;
import net.minecraft.client.gui.screens.CreditsAndAttributionScreen;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.gui.screens.TitleScreen;
import net.minecraft.client.gui.screens.multiplayer.JoinMultiplayerScreen;
import net.minecraft.client.gui.screens.multiplayer.SafetyScreen;
import net.minecraft.client.gui.screens.worldselection.SelectWorldScreen;
import net.minecraft.network.chat.Component;
//? if >=26.1 {
import net.minecraft.client.gui.GuiGraphicsExtractor;
//?} else {
/*import net.minecraft.client.gui.GuiGraphics;
*///?}
//? if >=1.21.11 {
import net.minecraft.client.gui.components.PlayerSkinWidget;
import net.minecraft.client.gui.screens.options.AccessibilityOptionsScreen;
import net.minecraft.client.gui.screens.options.LanguageSelectScreen;
import net.minecraft.client.gui.screens.options.OptionsScreen;
import net.minecraft.util.Util;
//?} else {
/*import net.minecraft.Util;
import net.minecraft.client.gui.screens.AccessibilityOptionsScreen;
import net.minecraft.client.gui.screens.LanguageSelectScreen;
import net.minecraft.client.gui.screens.OptionsScreen;
*///?}

public class OrreryTitleScreen extends Screen {
    private static final int LINK_HEIGHT = 14;
    private static final int ROW_HEIGHT = 20;
    private static final int PANEL_WIDTH = 170;
    private static final int COMPACT_BELOW = 360;
    private static final int SKIN_WIDTH = 80;
    private static final int SKIN_HEIGHT = 124;
    private static final long FADE_MILLIS = 450;
    //? if >=1.21.11 {
    private static final Component COPYRIGHT = Component.translatable("title.credits");
    //?} else {
    /*private static final Component COPYRIGHT = Component.literal("Copyright Mojang AB. Do not distribute!");
    *///?}

    private static boolean classicRequested;

    private final long openedAt = Util.getMillis();
    private boolean moreOpen;
    private int margin;
    private int columnWidth;
    private int headingY;
    private int gap;
    private int headingHeight;
    private float headingScale;
    private int panelX;
    private int panelWidth;
    private int panelTop = -1;
    private int panelBottom;
    private int skinCenterX = -1;
    private int skinCenterY;

    public OrreryTitleScreen() {
        super(Component.translatable("orrery-companion.menu.title"));
    }

    public static Screen replace(Screen screen) {
        if (screen == null || screen.getClass() != TitleScreen.class) {
            return screen;
        }
        if (classicRequested) {
            classicRequested = false;
            return screen;
        }
        if (!Settings.orreryTitleScreen() || Minecraft.getInstance().isDemo()) {
            return screen;
        }
        return new OrreryTitleScreen();
    }

    private static void openClassic(Minecraft minecraft) {
        classicRequested = true;
        Screens.open(minecraft, new TitleScreen());
    }

    @Override
    protected void init() {
        boolean compact = height < COMPACT_BELOW;
        gap = compact ? 6 : 8;
        headingHeight = compact ? 32 : 40;
        headingScale = compact ? 1.7f : 2.2f;
        int primaryHeight = compact ? 36 : 44;
        int tileHeight = compact ? 40 : 60;
        int actionHeight = compact ? 22 : 24;
        int headingGap = compact ? 10 : 14;
        int actionGap = compact ? 8 : 12;
        margin = compact ? Math.max(16, Math.min(32, width / 24)) : Math.max(20, Math.min(56, width / 14));
        columnWidth = Math.max(220, Math.min(300, width * 34 / 100));
        columnWidth = Math.min(columnWidth, width - margin * 2);
        int x = margin;
        LastPlayed last = LastPlayed.get();
        boolean hasContinue = last != null && last.isAvailable(minecraft);
        int blockHeight = headingHeight + headingGap + (hasContinue ? primaryHeight + gap : 0) + tileHeight + gap + actionHeight + actionGap + LINK_HEIGHT;
        int top = margin + 24;
        int bottom = height - margin / 2 - LINK_HEIGHT - 8;
        headingY = Math.max(top, top + (bottom - top - blockHeight) / 2);
        int y = headingY + headingHeight + headingGap;

        if (hasContinue) {
            Component caption = Component.translatable(last.isWorld() ? "orrery-companion.title.continue_world" : "orrery-companion.title.continue_server", last.name());
            addRenderableWidget(new OrreryButton(x, y, columnWidth, primaryHeight, OrreryButton.Style.PRIMARY,
                    Component.translatable("orrery-companion.title.continue_label"), () -> last.open(minecraft, this))
                    .icon(OrreryButton.ICON_PLAY).caption(caption).trailing(OrreryButton.ICON_ARROW));
            y += primaryHeight + gap;
        }
        int half = (columnWidth - gap) / 2;
        addRenderableWidget(new OrreryButton(x, y, half, tileHeight, OrreryButton.Style.TILE, Component.translatable("menu.singleplayer"),
                () -> Screens.open(minecraft, new SelectWorldScreen(this)))
                .icon(OrreryButton.ICON_CUBE).caption(Component.translatable("orrery-companion.title.singleplayer_caption")));
        OrreryButton multiplayer = addRenderableWidget(new OrreryButton(x + half + gap, y, columnWidth - half - gap, tileHeight,
                OrreryButton.Style.TILE, Component.translatable("menu.multiplayer"), this::openMultiplayer)
                .icon(OrreryButton.ICON_GLOBE).caption(Component.translatable("orrery-companion.title.multiplayer_caption")));
        multiplayer.active = minecraft.allowsMultiplayer();
        if (!multiplayer.active) {
            multiplayer.setTooltip(Tooltip.create(Component.translatable("title.multiplayer.disabled")));
        }
        y += tileHeight + gap;

        int third = (columnWidth - gap * 2) / 3;
        addRenderableWidget(new OrreryButton(x, y, third, actionHeight, OrreryButton.Style.ACTION, Component.translatable("orrery-companion.title.options"),
                () -> Screens.open(minecraft, new OptionsScreen(this, minecraft.options))).icon(OrreryButton.ICON_SLIDERS));
        addRenderableWidget(new OrreryButton(x + third + gap, y, third, actionHeight, OrreryButton.Style.ACTION,
                Component.translatable("orrery-companion.menu.title"), () -> Screens.open(minecraft, new OrreryScreen(this))).icon(OrreryButton.ICON_ORBIT));
        addRenderableWidget(new OrreryButton(x + (third + gap) * 2, y, columnWidth - (third + gap) * 2, actionHeight, OrreryButton.Style.ACTION,
                Component.translatable("orrery-companion.title.quit"), minecraft::stop).icon(OrreryButton.ICON_POWER));
        y += actionHeight + actionGap;

        Component moreLabel = Component.translatable(moreOpen ? "orrery-companion.title.less" : "orrery-companion.title.more");
        addRenderableWidget(new OrreryButton(x + 2, y, font.width(Theme.ui(moreLabel)) + 16, LINK_HEIGHT, OrreryButton.Style.LINK, moreLabel, this::toggleMore)
                .trailing(OrreryButton.ICON_CHEVRON));
        int linkBottom = y + LINK_HEIGHT;

        panelTop = -1;
        if (moreOpen) {
            boolean modMenu = FabricLoader.getInstance().isModLoaded("modmenu");
            int panelHeight = (modMenu ? 5 : 4) * ROW_HEIGHT + 8;
            panelWidth = PANEL_WIDTH;
            panelX = margin + columnWidth + 12;
            panelBottom = linkBottom;
            if (panelX + panelWidth > width - margin) {
                panelX = margin;
                panelWidth = columnWidth;
                panelBottom = y - 6;
            }
            panelTop = Math.max(margin, panelBottom - panelHeight);
            int rowX = panelX + 4;
            int rowWidth = panelWidth - 8;
            int rowY = panelTop + 4;
            OrreryButton realms = addRenderableWidget(row(rowX, rowY, rowWidth, Component.translatable("menu.online"),
                    () -> Screens.open(minecraft, new RealmsMainScreen(this))));
            realms.active = minecraft.allowsRealms();
            rowY += ROW_HEIGHT;
            addRenderableWidget(row(rowX, rowY, rowWidth, Component.translatable("options.language"),
                    () -> Screens.open(minecraft, new LanguageSelectScreen(this, minecraft.options, minecraft.getLanguageManager()))));
            rowY += ROW_HEIGHT;
            addRenderableWidget(row(rowX, rowY, rowWidth, Component.translatable("options.accessibility"),
                    () -> Screens.open(minecraft, new AccessibilityOptionsScreen(this, minecraft.options))));
            rowY += ROW_HEIGHT;
            if (modMenu) {
                addRenderableWidget(row(rowX, rowY, rowWidth, Component.translatable("orrery-companion.title.mods"), this::openModMenu));
                rowY += ROW_HEIGHT;
            }
            addRenderableWidget(row(rowX, rowY, rowWidth, Component.translatable("orrery-companion.title.classic"), () -> openClassic(minecraft)));
        }

        int copyrightWidth = font.width(Theme.ui(COPYRIGHT));
        addRenderableWidget(new OrreryButton(width - margin - copyrightWidth, height - margin / 2 - LINK_HEIGHT, copyrightWidth, LINK_HEIGHT,
                OrreryButton.Style.LINK, COPYRIGHT, () -> Screens.open(minecraft, new CreditsAndAttributionScreen(this))));

        skinCenterX = -1;
        //? if >=1.21.11 {
        int skinX = width - margin - SKIN_WIDTH - width / 12;
        boolean panelOverlaps = moreOpen && panelX + panelWidth > skinX - 20;
        if (!panelOverlaps && skinX > x + columnWidth + 80 && height > SKIN_HEIGHT + margin * 3) {
            PlayerSkinWidget skin = new PlayerSkinWidget(SKIN_WIDTH, SKIN_HEIGHT, minecraft.getEntityModels(),
                    minecraft.getSkinManager().createLookup(minecraft.getGameProfile(), false));
            skin.setX(skinX);
            skin.setY((height - SKIN_HEIGHT) / 2 + 6);
            addRenderableWidget(skin);
            skinCenterX = skinX + SKIN_WIDTH / 2;
            skinCenterY = skin.getY() + SKIN_HEIGHT / 2;
        }
        //?}
    }

    private OrreryButton row(int x, int y, int width, Component label, Runnable action) {
        return new OrreryButton(x, y, width, ROW_HEIGHT, OrreryButton.Style.ROW, label, action);
    }

    private void toggleMore() {
        moreOpen = !moreOpen;
        rebuildWidgets();
    }

    private void openMultiplayer() {
        Screens.open(minecraft, minecraft.options.skipMultiplayerWarning ? new JoinMultiplayerScreen(this) : new SafetyScreen(this));
    }

    private void openModMenu() {
        try {
            Object screen = Class.forName("com.terraformersmc.modmenu.api.ModMenuApi")
                    .getMethod("createModsScreen", Screen.class)
                    .invoke(null, this);
            Screens.open(minecraft, (Screen) screen);
        } catch (ReflectiveOperationException | ClassCastException e) {
            OrreryCompanion.LOGGER.warn("Could not open the Mod Menu screen", e);
        }
    }

    private float appear() {
        float t = Math.min(1f, (Util.getMillis() - openedAt) / (float) FADE_MILLIS);
        return 1f - (1f - t) * (1f - t) * (1f - t);
    }

    private void drawBackground(Canvas canvas) {
        Backdrop.draw(canvas, width, height, margin + columnWidth, Util.getMillis());
        if (skinCenterX >= 0) {
            Backdrop.glow(canvas, skinCenterX, skinCenterY + 10, SKIN_HEIGHT * 1.9f, 0x40666AF9);
        }
        float previous = Theme.opacity();
        Theme.setOpacity(appear());
        if (panelTop >= 0) {
            canvas.roundedRect(panelX, panelTop, panelWidth, panelBottom - panelTop, Theme.RADIUS, Theme.fade(0xE614141E));
            canvas.roundedBorder(panelX, panelTop, panelWidth, panelBottom - panelTop, Theme.RADIUS, Theme.fade(Theme.GLASS_BORDER));
        }
        Theme.setOpacity(previous);
    }

    private void drawForeground(Canvas canvas) {
        canvas.texture(OrreryButton.ICON_ORBIT, margin, margin, 14f / OrreryButton.ICON_TEXTURE, OrreryButton.ICON_TEXTURE, OrreryButton.ICON_TEXTURE,
                Theme.fade(0xFF8B8FFF));
        canvas.text(font, Theme.bold("Orrery"), margin + 20, margin + 4, Theme.fade(Theme.FOREGROUND));

        Component greeting = Component.translatable(greetingKey(), minecraft.getUser().getName());
        int greetingWidth = font.width(Theme.bold(greeting));
        float scale = Math.min(headingScale, columnWidth / (float) Math.max(1, greetingWidth));
        canvas.push(margin, headingY, scale);
        canvas.text(font, Theme.bold(greeting), 0, 0, Theme.fade(Theme.FOREGROUND));
        canvas.pop();
        canvas.text(font, Theme.ui(Component.translatable("orrery-companion.title.edition", OrreryCompanion.MINECRAFT)), margin,
                headingY + Math.round(9 * scale) + 8, Theme.fade(Theme.MUTED));

        canvas.text(font, Theme.ui("Orrery Companion " + OrreryCompanion.VERSION), margin, height - margin / 2 - LINK_HEIGHT + 4,
                Theme.fade(Theme.SUBTLE));
    }

    private static String greetingKey() {
        int hour = LocalTime.now().getHour();
        if (hour >= 5 && hour < 11) {
            return "orrery-companion.title.greeting.morning";
        }
        if (hour >= 11 && hour < 18) {
            return "orrery-companion.title.greeting.day";
        }
        if (hour >= 18 && hour < 23) {
            return "orrery-companion.title.greeting.evening";
        }
        return "orrery-companion.title.greeting.night";
    }

    @Override
    public boolean shouldCloseOnEsc() {
        return false;
    }

    @Override
    public boolean isPauseScreen() {
        return false;
    }

    //? if >=26.1 {
    @Override
    public void extractBackground(GuiGraphicsExtractor graphics, int mouseX, int mouseY, float delta) {
        drawBackground(new Canvas(graphics));
    }

    @Override
    public void extractRenderState(GuiGraphicsExtractor graphics, int mouseX, int mouseY, float delta) {
        Theme.setOpacity(appear());
        super.extractRenderState(graphics, mouseX, mouseY, delta);
        drawForeground(new Canvas(graphics));
        Theme.setOpacity(1f);
    }
    //?} else if >=1.21.11 {
    /*@Override
    public void renderBackground(GuiGraphics graphics, int mouseX, int mouseY, float delta) {
        drawBackground(new Canvas(graphics));
    }

    @Override
    public void render(GuiGraphics graphics, int mouseX, int mouseY, float delta) {
        Theme.setOpacity(appear());
        super.render(graphics, mouseX, mouseY, delta);
        drawForeground(new Canvas(graphics));
        Theme.setOpacity(1f);
    }
    *///?} else {
    /*@Override
    public void render(GuiGraphics graphics, int mouseX, int mouseY, float delta) {
        drawBackground(new Canvas(graphics));
        Theme.setOpacity(appear());
        super.render(graphics, mouseX, mouseY, delta);
        drawForeground(new Canvas(graphics));
        Theme.setOpacity(1f);
    }
    *///?}
}

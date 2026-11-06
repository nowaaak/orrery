package io.github.nowaaak.orrery.companion.hud;

import com.mojang.blaze3d.platform.InputConstants;
import io.github.nowaaak.orrery.companion.Screens;
import io.github.nowaaak.orrery.companion.modules.HudModule;
import io.github.nowaaak.orrery.companion.modules.ModuleSettings;
import io.github.nowaaak.orrery.companion.modules.Modules;
import java.util.ArrayList;
import java.util.List;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.resources.language.I18n;
import net.minecraft.network.chat.CommonComponents;
import net.minecraft.network.chat.Component;
//? if >=26.1 {
import net.minecraft.client.gui.GuiGraphicsExtractor;
//?} else {
/*import net.minecraft.client.gui.GuiGraphics;
*///?}
//? if >=1.21.11 {
import net.minecraft.client.input.MouseButtonEvent;
//?}

public class HudEditorScreen extends Screen {
    private static final int SNAP_DISTANCE = 4;
    private static final float SCALE_STEP = 0.1f;
    private static final int DIM = 0x30000000;
    private static final int OUTLINE = 0xA0FFFFFF;
    private static final int OUTLINE_ACTIVE = 0xFFFFD24A;
    private static final int GUIDE = 0xC04AA8FF;
    private static final int HINT = 0xFFE0E0E0;

    private final Screen parent;
    private HudModule dragging;
    private double grabX;
    private double grabY;
    private int guideX = -1;
    private int guideY = -1;

    public HudEditorScreen(Screen parent) {
        super(Component.translatable("orrery-companion.hud_editor.title"));
        this.parent = parent;
    }

    @Override
    protected void init() {
        addRenderableWidget(Button.builder(CommonComponents.GUI_DONE, button -> onClose())
                .bounds(width / 2 - 50, height - 28, 100, 20)
                .build());
    }

    private List<HudLayout.Box> boxes() {
        List<HudLayout.Box> boxes = new ArrayList<>();
        for (HudModule module : Modules.hud()) {
            if (module.isEnabled()) {
                HudModule.Frame frame = module.frame(minecraft, font, true);
                if (frame != null) {
                    boxes.add(HudLayout.box(module, frame, width, height));
                }
            }
        }
        return boxes;
    }

    private HudLayout.Box boxAt(double x, double y) {
        List<HudLayout.Box> boxes = boxes();
        for (int i = boxes.size() - 1; i >= 0; i--) {
            if (boxes.get(i).contains(x, y)) {
                return boxes.get(i);
            }
        }
        return null;
    }

    private boolean press(double x, double y, int button) {
        HudLayout.Box box = button == InputConstants.MOUSE_BUTTON_LEFT && getChildAt(x, y).isEmpty() ? boxAt(x, y) : null;
        if (box == null) {
            return false;
        }
        dragging = box.module();
        grabX = x - box.x();
        grabY = y - box.y();
        return true;
    }

    private boolean drag(double x, double y) {
        if (dragging == null) {
            return false;
        }
        HudLayout.Box current = null;
        List<HudLayout.Box> others = new ArrayList<>();
        for (HudLayout.Box box : boxes()) {
            if (box.module() == dragging) {
                current = box;
            } else {
                others.add(box);
            }
        }
        if (current == null) {
            return false;
        }
        int left = snapX((int) Math.round(x - grabX), current.width(), others);
        int top = snapY((int) Math.round(y - grabY), current.height(), others);
        ModuleSettings settings = ModuleSettings.get(dragging);
        settings.x = clamp01((float) left / width);
        settings.y = clamp01((float) top / height);
        return true;
    }

    private boolean release() {
        boolean wasDragging = dragging != null;
        dragging = null;
        guideX = -1;
        guideY = -1;
        return wasDragging;
    }

    private boolean scroll(double x, double y, double amount) {
        HudLayout.Box box = boxAt(x, y);
        if (box == null || amount == 0) {
            return false;
        }
        ModuleSettings settings = ModuleSettings.get(box.module());
        settings.scale = ModuleSettings.clampScale(Math.round((settings.scale + Math.signum(amount) * SCALE_STEP) * 10f) / 10f);
        return true;
    }

    private int snapX(int left, int boxWidth, List<HudLayout.Box> others) {
        List<int[]> candidates = new ArrayList<>();
        candidates.add(new int[] {0, 0});
        candidates.add(new int[] {width - boxWidth, width - 1});
        candidates.add(new int[] {(width - boxWidth) / 2, width / 2});
        for (HudLayout.Box other : others) {
            candidates.add(new int[] {other.x(), other.x()});
            candidates.add(new int[] {other.x() + other.width() - boxWidth, other.x() + other.width() - 1});
            candidates.add(new int[] {other.x() + other.width(), other.x() + other.width()});
            candidates.add(new int[] {other.x() - boxWidth, other.x() - 1});
        }
        int[] best = nearest(left, candidates);
        guideX = best == null ? -1 : best[1];
        return best == null ? left : best[0];
    }

    private int snapY(int top, int boxHeight, List<HudLayout.Box> others) {
        List<int[]> candidates = new ArrayList<>();
        candidates.add(new int[] {0, 0});
        candidates.add(new int[] {height - boxHeight, height - 1});
        candidates.add(new int[] {(height - boxHeight) / 2, height / 2});
        for (HudLayout.Box other : others) {
            candidates.add(new int[] {other.y(), other.y()});
            candidates.add(new int[] {other.y() + other.height() - boxHeight, other.y() + other.height() - 1});
            candidates.add(new int[] {other.y() + other.height(), other.y() + other.height()});
            candidates.add(new int[] {other.y() - boxHeight, other.y() - 1});
        }
        int[] best = nearest(top, candidates);
        guideY = best == null ? -1 : best[1];
        return best == null ? top : best[0];
    }

    private static int[] nearest(int value, List<int[]> candidates) {
        int[] best = null;
        for (int[] candidate : candidates) {
            int distance = Math.abs(candidate[0] - value);
            if (distance <= SNAP_DISTANCE && (best == null || distance < Math.abs(best[0] - value))) {
                best = candidate;
            }
        }
        return best;
    }

    private static float clamp01(float value) {
        return Math.max(0f, Math.min(1f, value));
    }

    private void draw(Canvas canvas, int mouseX, int mouseY) {
        canvas.fill(0, 0, width, height, DIM);
        List<HudLayout.Box> boxes = boxes();
        HudLayout.Box hovered = dragging == null ? boxAt(mouseX, mouseY) : null;
        for (HudLayout.Box box : boxes) {
            HudLayout.draw(canvas, box);
            boolean active = box.module() == dragging || box == hovered;
            canvas.outline(box.x() - 1, box.y() - 1, box.width() + 2, box.height() + 2, active ? OUTLINE_ACTIVE : OUTLINE);
        }
        if (dragging != null && guideX >= 0) {
            canvas.fill(guideX, 0, guideX + 1, height, GUIDE);
        }
        if (dragging != null && guideY >= 0) {
            canvas.fill(0, guideY, width, guideY + 1, GUIDE);
        }
        String hint = I18n.get(boxes.isEmpty() ? "orrery-companion.hud_editor.empty" : "orrery-companion.hud_editor.hint");
        canvas.text(font, hint, (width - font.width(hint)) / 2, height - 42, HINT);
    }

    @Override
    public void mouseMoved(double x, double y) {
        drag(x, y);
        super.mouseMoved(x, y);
    }

    @Override
    public void onClose() {
        ModuleSettings.save();
        Screens.open(minecraft, parent);
    }

    //? if >=26.1 {
    @Override
    public void extractRenderState(GuiGraphicsExtractor graphics, int mouseX, int mouseY, float delta) {
        draw(new Canvas(graphics), mouseX, mouseY);
        super.extractRenderState(graphics, mouseX, mouseY, delta);
    }

    @Override
    public void extractBackground(GuiGraphicsExtractor graphics, int mouseX, int mouseY, float delta) {
    }
    //?} else if >=1.21.11 {
    /*@Override
    public void render(GuiGraphics graphics, int mouseX, int mouseY, float delta) {
        draw(new Canvas(graphics), mouseX, mouseY);
        super.render(graphics, mouseX, mouseY, delta);
    }

    @Override
    public void renderBackground(GuiGraphics graphics, int mouseX, int mouseY, float delta) {
    }
    *///?} else {
    /*@Override
    public void render(GuiGraphics graphics, int mouseX, int mouseY, float delta) {
        draw(new Canvas(graphics), mouseX, mouseY);
        super.render(graphics, mouseX, mouseY, delta);
    }
    *///?}

    //? if >=1.21.11 {
    @Override
    public boolean mouseClicked(MouseButtonEvent event, boolean doubleClick) {
        return press(event.x(), event.y(), event.button()) || super.mouseClicked(event, doubleClick);
    }

    @Override
    public boolean mouseDragged(MouseButtonEvent event, double deltaX, double deltaY) {
        return drag(event.x(), event.y()) || super.mouseDragged(event, deltaX, deltaY);
    }

    @Override
    public boolean mouseReleased(MouseButtonEvent event) {
        return release() || super.mouseReleased(event);
    }

    @Override
    public boolean mouseScrolled(double x, double y, double horizontal, double vertical) {
        return scroll(x, y, vertical) || super.mouseScrolled(x, y, horizontal, vertical);
    }
    //?} else {
    /*@Override
    public boolean mouseClicked(double x, double y, int button) {
        return press(x, y, button) || super.mouseClicked(x, y, button);
    }

    @Override
    public boolean mouseDragged(double x, double y, int button, double deltaX, double deltaY) {
        return drag(x, y) || super.mouseDragged(x, y, button, deltaX, deltaY);
    }

    @Override
    public boolean mouseReleased(double x, double y, int button) {
        return release() || super.mouseReleased(x, y, button);
    }

    @Override
    public boolean mouseScrolled(double x, double y, double amount) {
        return scroll(x, y, amount) || super.mouseScrolled(x, y, amount);
    }
    *///?}
}

package io.github.nowaaak.orrery.companion.hud;

import io.github.nowaaak.orrery.companion.ui.RoundedShapes;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.Font;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.Identifier;
import net.minecraft.world.item.ItemStack;
//? if >=26.1 {
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.renderer.RenderPipelines;
//?} else if >=1.21.11 {
/*import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.renderer.RenderPipelines;
*///?} else {
/*import com.mojang.blaze3d.systems.RenderSystem;
import net.minecraft.client.gui.GuiGraphics;
*///?}

public final class Canvas {
    //? if >=26.1 {
    private final GuiGraphicsExtractor graphics;

    public Canvas(GuiGraphicsExtractor graphics) {
        this.graphics = graphics;
    }
    //?} else {
    /*private final GuiGraphics graphics;

    public Canvas(GuiGraphics graphics) {
        this.graphics = graphics;
    }
    *///?}

    public int width() {
        return graphics.guiWidth();
    }

    public int height() {
        return graphics.guiHeight();
    }

    public void fill(int x0, int y0, int x1, int y1, int argb) {
        graphics.fill(x0, y0, x1, y1, argb);
    }

    public void verticalGradient(int x0, int y0, int x1, int y1, int top, int bottom) {
        graphics.fillGradient(x0, y0, x1, y1, top, bottom);
    }

    public void outline(int x, int y, int width, int height, int argb) {
        fill(x, y, x + width, y + 1, argb);
        fill(x, y + height - 1, x + width, y + height, argb);
        fill(x, y + 1, x + 1, y + height - 1, argb);
        fill(x + width - 1, y + 1, x + width, y + height - 1, argb);
    }

    public void text(Font font, String text, int x, int y, int argb) {
        text(font, text, x, y, argb, true);
    }

    public void text(Font font, String text, int x, int y, int argb, boolean shadow) {
        //? if >=26.1 {
        graphics.text(font, text, x, y, argb, shadow);
        //?} else {
        /*graphics.drawString(font, text, x, y, argb, shadow);
        *///?}
    }

    public void item(ItemStack stack, int x, int y) {
        //? if >=26.1 {
        graphics.item(stack, x, y);
        //?} else {
        /*graphics.renderItem(stack, x, y);
        *///?}
    }

    public void text(Font font, Component text, int x, int y, int argb) {
        if ((argb >>> 24) < 8) {
            return;
        }
        //? if >=26.1 {
        graphics.text(font, text, x, y, argb, false);
        //?} else {
        /*graphics.drawString(font, text, x, y, argb, false);
        *///?}
    }

    public void roundedRect(float x, float y, float width, float height, float radius, int argb) {
        shape(x, y, width, height, radius, 0, argb);
    }

    public void roundedBorder(float x, float y, float width, float height, float radius, int argb) {
        shape(x, y, width, height, radius, 1, argb);
    }

    private void shape(float x, float y, float width, float height, float radius, int border, int argb) {
        int alpha = argb >>> 24;
        if (alpha == 0) {
            return;
        }
        double scale = Minecraft.getInstance().getWindow().getGuiScale();
        int x0 = (int) Math.round(x * scale);
        int y0 = (int) Math.round(y * scale);
        int w = (int) Math.round((x + width) * scale) - x0;
        int h = (int) Math.round((y + height) * scale) - y0;
        int r = (int) Math.min(Math.round(radius * scale), Math.min(w, h) / 2);
        push(0, 0, (float) (1 / scale));
        physicalShape(x0, y0, w, h, r, border, argb);
        pop();
    }

    public void physicalShape(int x0, int y0, int width, int height, float radius, int border, int argb) {
        int alpha = argb >>> 24;
        int rgb = argb & 0x00FFFFFF;
        for (RoundedShapes.Run run : RoundedShapes.runs(width, height, Math.min(radius, Math.min(width, height) / 2f), border)) {
            int runAlpha = Math.round(alpha * run.coverage());
            if (runAlpha > 0) {
                fill(x0 + run.x0(), y0 + run.y0(), x0 + run.x1(), y0 + run.y1(), (runAlpha << 24) | rgb);
            }
        }
    }

    public void horizontalFade(int x0, int x1, int y0, int y1, int argb) {
        int steps = Math.max(1, x1 - x0);
        int alpha = argb >>> 24;
        for (int i = 0; i < steps; i++) {
            float t = (float) i / steps;
            int stepAlpha = Math.round(alpha * (1f - t * t * (3f - 2f * t)));
            fill(x0 + i, y0, x0 + i + 1, y1, (stepAlpha << 24) | (argb & 0x00FFFFFF));
        }
    }

    public void texture(Identifier texture, float x, float y, float scale, int textureWidth, int textureHeight, int argb) {
        if ((argb >>> 24) == 0) {
            return;
        }
        push(x, y, scale);
        //? if >=1.21.11 {
        graphics.blit(RenderPipelines.GUI_TEXTURED, texture, 0, 0, 0f, 0f, textureWidth, textureHeight, textureWidth, textureHeight, argb);
        //?} else {
        /*RenderSystem.enableBlend();
        RenderSystem.setShaderColor(((argb >> 16) & 0xFF) / 255f, ((argb >> 8) & 0xFF) / 255f, (argb & 0xFF) / 255f, (argb >>> 24) / 255f);
        graphics.blit(texture, 0, 0, textureWidth, textureHeight, 0f, 0f, textureWidth, textureHeight, textureWidth, textureHeight);
        RenderSystem.setShaderColor(1f, 1f, 1f, 1f);
        *///?}
        pop();
    }

    public void push(float x, float y, float scale) {
        //? if >=1.21.11 {
        graphics.pose().pushMatrix();
        graphics.pose().translate(x, y);
        graphics.pose().scale(scale, scale);
        //?} else {
        /*graphics.pose().pushPose();
        graphics.pose().translate(x, y, 0);
        graphics.pose().scale(scale, scale, 1);
        *///?}
    }

    public void pop() {
        //? if >=1.21.11 {
        graphics.pose().popMatrix();
        //?} else {
        /*graphics.pose().popPose();
        *///?}
    }
}

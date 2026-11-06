package io.github.nowaaak.orrery.companion.hud;

import io.github.nowaaak.orrery.companion.OrreryCompanion;
import io.github.nowaaak.orrery.companion.Screens;
import io.github.nowaaak.orrery.companion.modules.HudModule;
import io.github.nowaaak.orrery.companion.modules.Modules;
import net.minecraft.client.Minecraft;
//? if >=1.21.11 {
import net.fabricmc.fabric.api.client.rendering.v1.hud.HudElementRegistry;
import net.fabricmc.fabric.api.client.rendering.v1.hud.VanillaHudElements;
import net.minecraft.resources.Identifier;
//?} else {
/*import net.fabricmc.fabric.api.client.rendering.v1.HudRenderCallback;
*///?}

public final class HudRenderer {
    private HudRenderer() {
    }

    public static void register() {
        //? if >=26.1 {
        HudElementRegistry.addLast(Identifier.fromNamespaceAndPath(OrreryCompanion.ID, "hud"), (graphics, delta) -> render(new Canvas(graphics)));
        HudElementRegistry.replaceElement(VanillaHudElements.CROSSHAIR, original -> (graphics, delta) -> {
            if (Modules.CROSSHAIR.replacesVanilla()) {
                Modules.CROSSHAIR.drawInGame(new Canvas(graphics));
            } else {
                original.extractRenderState(graphics, delta);
            }
        });
        //?} else if >=1.21.11 {
        /*HudElementRegistry.addLast(Identifier.fromNamespaceAndPath(OrreryCompanion.ID, "hud"), (graphics, delta) -> render(new Canvas(graphics)));
        HudElementRegistry.replaceElement(VanillaHudElements.CROSSHAIR, original -> (graphics, delta) -> {
            if (Modules.CROSSHAIR.replacesVanilla()) {
                Modules.CROSSHAIR.drawInGame(new Canvas(graphics));
            } else {
                original.render(graphics, delta);
            }
        });
        *///?} else {
        /*HudRenderCallback.EVENT.register((graphics, delta) -> render(new Canvas(graphics)));
        *///?}
    }

    private static void render(Canvas canvas) {
        Minecraft minecraft = Minecraft.getInstance();
        if (minecraft.player == null || isHidden(minecraft) || Screens.current(minecraft) instanceof HudEditorScreen) {
            return;
        }
        Modules.WAYPOINTS.drawCompass(canvas, minecraft);
        for (HudModule module : Modules.hud()) {
            if (!module.isActive()) {
                continue;
            }
            HudModule.Frame frame = module.frame(minecraft, minecraft.font, false);
            if (frame != null) {
                HudLayout.draw(canvas, HudLayout.box(module, frame, canvas.width(), canvas.height()));
            }
        }
    }

    private static boolean isHidden(Minecraft minecraft) {
        //? if >=26.1 {
        return minecraft.gui.hud.isHidden() || minecraft.debugEntries.isOverlayVisible();
        //?} else if >=1.21.11 {
        /*return minecraft.options.hideGui || minecraft.debugEntries.isOverlayVisible();
        *///?} else {
        /*return minecraft.options.hideGui || minecraft.options.renderDebug;
        *///?}
    }
}

package io.github.nowaaak.orrery.companion;

import com.mojang.blaze3d.platform.InputConstants;
import net.minecraft.client.KeyMapping;
//? if >=26.1 {
import net.fabricmc.fabric.api.client.keymapping.v1.KeyMappingHelper;
import net.minecraft.resources.Identifier;
//?} else if >=1.21.11 {
/*import net.fabricmc.fabric.api.client.keybinding.v1.KeyBindingHelper;
import net.minecraft.resources.Identifier;
*///?} else {
/*import net.fabricmc.fabric.api.client.keybinding.v1.KeyBindingHelper;
*///?}

public final class Keys {
    public static KeyMapping menu;
    public static KeyMapping zoom;
    public static KeyMapping waypoint;
    public static KeyMapping emote;

    //? if >=1.21.11 {
    private static KeyMapping.Category category;
    //?}

    private Keys() {
    }

    public static void register() {
        //? if >=1.21.11 {
        category = KeyMapping.Category.register(Identifier.fromNamespaceAndPath(OrreryCompanion.ID, "main"));
        //?}
        menu = register("menu", InputConstants.KEY_RSHIFT);
        zoom = register("zoom", InputConstants.KEY_C);
        waypoint = register("waypoint", InputConstants.KEY_B);
        emote = register("emote", InputConstants.KEY_N);
    }

    private static KeyMapping register(String name, int key) {
        String translation = "key." + OrreryCompanion.ID + "." + name;
        //? if >=26.1 {
        return KeyMappingHelper.registerKeyMapping(new KeyMapping(translation, key, category));
        //?} else if >=1.21.11 {
        /*return KeyBindingHelper.registerKeyBinding(new KeyMapping(translation, key, category));
        *///?} else {
        /*return KeyBindingHelper.registerKeyBinding(new KeyMapping(translation, key, "key.category.orrery-companion.main"));
        *///?}
    }
}

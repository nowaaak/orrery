package io.github.nowaaak.orrery.companion.modules.hud;

import io.github.nowaaak.orrery.companion.modules.HudModule;
import io.github.nowaaak.orrery.companion.modules.TextHudModule;
import java.util.ArrayList;
import java.util.List;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.Font;
import net.minecraft.client.player.LocalPlayer;
import net.minecraft.world.entity.EquipmentSlot;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.Items;

public class DurabilityModule extends HudModule {
    private static final EquipmentSlot[] SLOTS = {
        EquipmentSlot.HEAD, EquipmentSlot.CHEST, EquipmentSlot.LEGS, EquipmentSlot.FEET, EquipmentSlot.MAINHAND, EquipmentSlot.OFFHAND
    };
    private static final int ICON = 16;
    private static final int ROW = 17;
    private static final int GAP = 3;
    private static final int PADDING = TextHudModule.PADDING;

    public DurabilityModule() {
        super("durability", false, false, 0.9f, 0.55f);
    }

    private record Entry(ItemStack stack, String text, int color) {
    }

    @Override
    public Frame frame(Minecraft minecraft, Font font, boolean preview) {
        List<Entry> entries = entries(minecraft.player);
        if (entries.isEmpty()) {
            if (!preview) {
                return null;
            }
            entries = List.of(
                    new Entry(new ItemStack(Items.DIAMOND_HELMET), "363", color(1f)),
                    new Entry(new ItemStack(Items.DIAMOND_CHESTPLATE), "201", color(0.38f)),
                    new Entry(new ItemStack(Items.DIAMOND_SWORD), "42", color(0.03f)));
        }
        int textWidth = 0;
        for (Entry entry : entries) {
            textWidth = Math.max(textWidth, font.width(entry.text()));
        }
        int width = PADDING * 2 + ICON + GAP + textWidth;
        int height = PADDING * 2 + entries.size() * ROW - 1;
        List<Entry> rows = entries;
        return new Frame(width, height, canvas -> {
            canvas.fill(0, 0, width, height, TextHudModule.BACKGROUND);
            for (int i = 0; i < rows.size(); i++) {
                Entry entry = rows.get(i);
                int y = PADDING + i * ROW;
                canvas.item(entry.stack(), PADDING, y);
                canvas.text(font, entry.text(), PADDING + ICON + GAP, y + 4, entry.color());
            }
        });
    }

    private static List<Entry> entries(LocalPlayer player) {
        List<Entry> entries = new ArrayList<>();
        if (player == null) {
            return entries;
        }
        for (EquipmentSlot slot : SLOTS) {
            ItemStack stack = player.getItemBySlot(slot);
            if (stack.isEmpty() || !stack.isDamageableItem() || stack.getMaxDamage() <= 0) {
                continue;
            }
            int remaining = stack.getMaxDamage() - stack.getDamageValue();
            entries.add(new Entry(stack, Integer.toString(remaining), color(remaining / (float) stack.getMaxDamage())));
        }
        return entries;
    }

    private static int color(float fraction) {
        if (fraction > 0.5f) {
            return 0xFF6EE7A0;
        }
        if (fraction > 0.2f) {
            return 0xFFFCD34D;
        }
        return 0xFFF87171;
    }
}

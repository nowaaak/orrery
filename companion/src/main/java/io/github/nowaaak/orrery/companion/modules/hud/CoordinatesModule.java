package io.github.nowaaak.orrery.companion.modules.hud;

import io.github.nowaaak.orrery.companion.modules.TextHudModule;
import java.util.List;
import net.minecraft.client.Minecraft;
import net.minecraft.client.player.LocalPlayer;
import net.minecraft.client.resources.language.I18n;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;

public class CoordinatesModule extends TextHudModule {
    public CoordinatesModule() {
        super("coordinates", false, false, 0.01f, 0.06f);
    }

    @Override
    public List<String> lines(Minecraft minecraft) {
        LocalPlayer player = minecraft.player;
        if (player == null || minecraft.level == null || minecraft.showOnlyReducedInfo()) {
            return List.of();
        }
        BlockPos pos = player.blockPosition();
        Direction facing = player.getDirection();
        return List.of(
                I18n.get("orrery-companion.hud.coordinates", pos.getX(), pos.getY(), pos.getZ()),
                I18n.get("orrery-companion.hud.biome", biome(minecraft, pos)),
                I18n.get("orrery-companion.hud.facing", I18n.get("orrery-companion.direction." + facing.getName()), axis(facing)));
    }

    private static String biome(Minecraft minecraft, BlockPos pos) {
        return minecraft.level.getBiome(pos).unwrapKey()
                //? if >=1.21.11 {
                .map(key -> key.identifier())
                //?} else {
                /*.map(key -> key.location())
                *///?}
                .map(id -> I18n.get("biome." + id.getNamespace() + "." + id.getPath()))
                .orElse("?");
    }

    private static String axis(Direction facing) {
        return switch (facing) {
            case NORTH -> "-Z";
            case SOUTH -> "+Z";
            case WEST -> "-X";
            case EAST -> "+X";
            default -> "";
        };
    }
}

package io.github.nowaaak.orrery.companion.emotes;

import net.minecraft.client.CameraType;
import net.minecraft.client.Minecraft;
import net.minecraft.client.model.geom.ModelPart;
import net.minecraft.world.phys.Vec3;

public final class Emotes {
    private static final double CANCEL_DISTANCE_SQUARED = 0.04;

    private static Emote current;
    private static long startedAt;
    private static Vec3 startPosition;
    private static CameraType restoreCamera;

    private Emotes() {
    }

    public static void play(Minecraft minecraft, Emote emote) {
        if (minecraft.player == null) {
            return;
        }
        current = emote;
        startedAt = System.nanoTime();
        startPosition = minecraft.player.position();
        if (minecraft.options.getCameraType().isFirstPerson()) {
            restoreCamera = minecraft.options.getCameraType();
            minecraft.options.setCameraType(CameraType.THIRD_PERSON_FRONT);
        }
    }

    public static void stop(Minecraft minecraft) {
        current = null;
        if (restoreCamera != null && minecraft.options.getCameraType() == CameraType.THIRD_PERSON_FRONT) {
            minecraft.options.setCameraType(restoreCamera);
        }
        restoreCamera = null;
    }

    private static float elapsed() {
        return (System.nanoTime() - startedAt) / 1_000_000_000f;
    }

    public static void tick(Minecraft minecraft) {
        if (current == null) {
            return;
        }
        boolean moved = minecraft.player == null || minecraft.player.position().distanceToSqr(startPosition) > CANCEL_DISTANCE_SQUARED;
        if (moved || elapsed() >= current.seconds()) {
            stop(minecraft);
        }
    }

    public static Emote current() {
        return current;
    }

    public static void pose(ModelPart head, ModelPart rightArm, ModelPart leftArm) {
        Emote emote = current;
        if (emote != null) {
            emote.pose(head, rightArm, leftArm, Math.min(elapsed(), emote.seconds()));
        }
    }
}

package io.github.nowaaak.orrery.companion.emotes;

import net.minecraft.client.model.geom.ModelPart;

public enum Emote {
    WAVE("wave", 3f),
    CHEER("cheer", 3f),
    CLAP("clap", 3f),
    DANCE("dance", 6f),
    POINT("point", 3f);

    private final String key;
    private final float seconds;

    Emote(String key, float seconds) {
        this.key = key;
        this.seconds = seconds;
    }

    public String key() {
        return key;
    }

    public float seconds() {
        return seconds;
    }

    private static float sin(float value) {
        return (float) Math.sin(value);
    }

    public void pose(ModelPart head, ModelPart rightArm, ModelPart leftArm, float time) {
        float fade = Math.min(1f, Math.min(time, seconds - time) * 4f);
        switch (this) {
            case WAVE -> {
                set(rightArm, -0.2f, 0f, 2.55f + sin(time * 10f) * 0.3f, fade);
            }
            case CHEER -> {
                float lift = sin(time * 8f) * 0.15f;
                set(rightArm, -0.2f, 0f, 2.7f + lift, fade);
                set(leftArm, -0.2f, 0f, -2.7f - lift, fade);
            }
            case CLAP -> {
                float clap = Math.abs(sin(time * 9f)) * 0.3f;
                set(rightArm, -1.35f, -0.25f - clap, 0f, fade);
                set(leftArm, -1.35f, 0.25f + clap, 0f, fade);
            }
            case DANCE -> {
                float swing = sin(time * 6f) * 1.1f;
                set(rightArm, -1.4f + swing, 0f, 0f, fade);
                set(leftArm, -1.4f - swing, 0f, 0f, fade);
                head.xRot += sin(time * 12f) * 0.15f * fade;
            }
            case POINT -> {
                set(rightArm, -1.55f, -0.05f, 0f, fade);
            }
        }
    }

    private static void set(ModelPart part, float x, float y, float z, float fade) {
        part.xRot += (x - part.xRot) * fade;
        part.yRot += (y - part.yRot) * fade;
        part.zRot += (z - part.zRot) * fade;
    }
}

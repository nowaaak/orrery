vec3 waterFogColor() {
    return vec3(0.02, 0.09, 0.12) * (skyAmbientColor() + directLightColor() * 0.2 + 0.05);
}

vec3 applyFog(vec3 color, vec3 viewPos, bool border) {
    float dist = length(viewPos);
    vec3 dir = viewToWorldDir(viewPos);

    if (isEyeInWater == 1) {
        return mix(waterFogColor(), color, exp(-dist * 0.07));
    }
    if (isEyeInWater == 2) {
        return mix(vec3(0.9, 0.25, 0.03), color, exp(-dist * 1.0));
    }
    if (isEyeInWater == 3) {
        return mix(vec3(0.7, 0.8, 0.9), color, exp(-dist * 0.8));
    }

    vec3 tint;
    float density;
    if (hasSkylight) {
        tint = skyColor(safeNormalize(vec3(dir.x, max(dir.y, 0.0), dir.z)));
        float outside = smoothstep(0.0, 0.5, float(eyeBrightnessSmooth.y) / 240.0);
        tint = mix(vec3(0.010, 0.012, 0.020), tint, outside);
        density = mix(0.0009, 0.006, rainStrength) * (1.0 + duskAmount(sunDirection().y) * 1.5);
    } else if (hasCeiling) {
        tint = toLinear(fogColor) * 0.9;
        density = 0.010;
    } else {
        tint = vec3(0.025, 0.018, 0.040);
        density = 0.004;
    }

    float amount = 1.0 - exp(-dist * density * FOG_DENSITY);
    if (border) {
        amount = max(amount, smoothstep(far * 0.72, far, dist));
    }
    color = mix(color, tint, amount);
    return color * (1.0 - blindness * smoothstep(2.0, 6.0, dist));
}

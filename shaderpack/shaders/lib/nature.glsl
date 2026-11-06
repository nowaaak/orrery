vec3 waveOffset(vec3 worldPos, int blockId, bool topVertex) {
#ifdef WAVING_PLANTS
    float time = frameTimeCounter;
    float strength = 1.0 + rainStrength * 1.5;
    if (blockId == 10003) {
        vec3 sway = vec3(
            sin(time * 1.7 + worldPos.x * 0.9 + worldPos.y * 0.5),
            sin(time * 1.3 + worldPos.z * 1.1) * 0.5,
            sin(time * 1.9 + worldPos.z * 0.8 + worldPos.x * 0.3)
        );
        return sway * 0.035 * strength;
    }
    if (blockId == 10004 && topVertex) {
        float gust = sin(time * 2.1 + worldPos.x * 0.7 + worldPos.z * 0.6) + 0.5 * sin(time * 3.3 + worldPos.x * 1.3);
        return vec3(gust, 0.0, gust * 0.6 + cos(time * 1.9 + worldPos.z) * 0.4) * 0.07 * strength;
    }
#endif
    return vec3(0.0);
}

float waveHeight(vec2 pos, float time) {
    float height = smoothNoise(pos * 0.035 + vec2(time * 0.010, time * 0.006)) * 0.55;
    height += smoothNoise(pos * 0.080 - vec2(time * 0.014, -time * 0.009)) * 0.30;
    height += smoothNoise(pos * 0.190 + vec2(time * 0.020, time * 0.017)) * 0.15;
    return height;
}

vec3 waveNormal(vec3 worldPos, float dist) {
#ifdef WATER_WAVES
    const float delta = 0.1;
    float strength = 1.0 / (1.0 + dist * 0.06);
    float time = frameTimeCounter;
    float center = waveHeight(worldPos.xz, time);
    float dx = (center - waveHeight(worldPos.xz + vec2(delta, 0.0), time)) / delta;
    float dz = (center - waveHeight(worldPos.xz + vec2(0.0, delta), time)) / delta;
    return normalize(vec3(dx * strength, 1.0, dz * strength));
#else
    return vec3(0.0, 1.0, 0.0);
#endif
}

float causticPattern(vec3 worldPos) {
    float time = frameTimeCounter * 0.02;
    vec2 pos = worldPos.xz + worldPos.y * 0.3;
    float a = smoothNoise(pos * 0.045 + vec2(time, time * 0.6));
    float b = smoothNoise(pos * 0.052 - vec2(time * 0.7, -time));
    float lines = 1.0 - abs(a - b) * 6.0;
    return pow(clamp(lines, 0.0, 1.0), 5.0);
}

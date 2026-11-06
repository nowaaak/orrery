const vec3 BLOCK_LIGHT = vec3(1.0, 0.60, 0.30);

vec3 directLightColor() {
    if (!hasSkylight) {
        return vec3(0.0);
    }
    float height = sunDirection().y;
    vec3 sunColor = mix(vec3(1.0, 0.50, 0.24) * 1.6, vec3(1.0, 0.93, 0.84) * 3.2, smoothstep(0.02, 0.45, height));
    vec3 moonColor = vec3(0.42, 0.52, 0.95) * 0.30;
    vec3 color = height > 0.0
        ? sunColor * smoothstep(0.0, 0.08, height)
        : moonColor * smoothstep(0.0, 0.08, -height);
    return color * (1.0 - rainStrength * 0.8);
}

vec3 skyAmbientColor() {
    if (hasCeiling && !hasSkylight) {
        return toLinear(fogColor) * 0.6 + vec3(0.12, 0.06, 0.04);
    }
    if (!hasSkylight) {
        return vec3(0.20, 0.16, 0.30);
    }
    float height = sunDirection().y;
    vec3 color = mix(vec3(0.045, 0.055, 0.110), vec3(0.42, 0.52, 0.72), daylight(height));
    color = mix(color, vec3(0.42, 0.30, 0.36), duskAmount(height) * 0.5);
    return mix(color, vec3(luminance(color)) * 0.7, rainStrength * 0.6);
}

vec3 lightAmount(vec2 light, float direct, float ao) {
    float sky = hasSkylight ? light.y * light.y : 1.0;
    float block = light.x * light.x * light.x * 2.2 + light.x * 0.06;
    vec3 result = (skyAmbientColor() * sky + vec3(0.018, 0.018, 0.026)) * ao + nightVision * 0.5;
    result += BLOCK_LIGHT * block * mix(1.0, ao, 0.5);
    result += directLightColor() * direct * smoothstep(0.05, 0.35, light.y);
    return result * (1.0 - darknessFactor * 0.5);
}

vec3 surfaceLight(vec2 light, float direct) {
    return lightAmount(light, direct, 1.0);
}

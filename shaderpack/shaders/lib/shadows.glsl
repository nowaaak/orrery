const float SHADOW_DISTORTION = 0.85;

float shadowDistortFactor(vec2 pos) {
    return length(pos) * SHADOW_DISTORTION + (1.0 - SHADOW_DISTORTION);
}

vec3 distortShadow(vec3 pos) {
    return vec3(pos.xy / shadowDistortFactor(pos.xy), pos.z * 0.25);
}

vec3 playerToShadow(vec3 playerPos) {
    vec3 shadowPos = (shadowProjection * (shadowModelView * vec4(playerPos, 1.0))).xyz;
    return distortShadow(shadowPos) * 0.5 + 0.5;
}

#ifdef SHADOW_LOOKUP
bool insideShadowMap(vec3 shadowPos) {
    return all(greaterThan(shadowPos, vec3(0.0))) && all(lessThan(shadowPos, vec3(1.0)));
}

float shadowAmount(vec3 playerPos, vec3 worldNormal, float noise) {
    float dist = length(playerPos);
    vec3 shadowPos = playerToShadow(playerPos + worldNormal * (0.04 + dist * 0.004));
    if (!insideShadowMap(shadowPos)) {
        return 1.0;
    }
    shadowPos.z -= 0.0001;

    float radius = SHADOW_SOFTNESS * 1.6 / float(shadowMapResolution);
    float angle = noise * TAU;
    float sum = 0.0;
    for (int i = 0; i < SHADOW_SAMPLES; i++) {
        float r = sqrt((float(i) + 0.5) / float(SHADOW_SAMPLES));
        float a = float(i) * 2.39996 + angle;
        sum += texture(shadowtex1, vec3(shadowPos.xy + vec2(cos(a), sin(a)) * r * radius, shadowPos.z));
    }
    float fade = smoothstep(shadowDistance * 0.8, shadowDistance, length(playerPos.xz));
    return mix(sum / float(SHADOW_SAMPLES), 1.0, fade);
}

float shadowSample(vec3 playerPos) {
    vec3 shadowPos = playerToShadow(playerPos);
    if (!insideShadowMap(shadowPos)) {
        return 1.0;
    }
    return texture(shadowtex1, vec3(shadowPos.xy, shadowPos.z - 0.0002));
}
#endif

vec3 shadeSurface(vec3 albedo, vec3 viewPos, vec3 worldNormal, vec2 light, int material, float emission, float ao, float noise) {
    vec3 playerPos = viewToPlayer(viewPos);
    vec3 rayDir = viewToWorldDir(viewPos);
    vec3 lightDir = lightDirection();
    bool foliage = material == MAT_LEAVES || material == MAT_PLANT;

    float lambert = dot(worldNormal, lightDir);
    float direct = material == MAT_LEAVES ? max(lambert, 0.0) * 0.6 + 0.4
        : material == MAT_PLANT ? 0.8
        : max(lambert, 0.0);

    float shadow = 1.0;
#ifdef SHADOW_LOOKUP
    if (direct > 0.0 && hasSkylight) {
        shadow = shadowAmount(playerPos, material == MAT_PLANT ? vec3(0.0, 1.0, 0.0) : worldNormal, noise);
    }
#endif
    if (isEyeInWater == 1) {
        shadow *= 0.5 + causticPattern(playerPos + cameraPosition) * 1.5;
    }

    float wet = 0.0;
    if (material == MAT_SOLID && hasSkylight) {
        wet = wetness * smoothstep(0.85, 0.95, light.y) * smoothstep(0.7, 0.9, worldNormal.y);
        albedo *= 1.0 - wet * 0.35;
    }

    vec3 color = albedo * lightAmount(light, direct * shadow, ao);

    if (foliage) {
        float through = pow(max(dot(rayDir, lightDir), 0.0), 4.0);
        color += albedo * directLightColor() * through * shadow * 0.9 * smoothstep(0.05, 0.35, light.y);
    }
    if (wet > 0.0) {
        vec3 reflected = reflect(rayDir, worldNormal);
        float fresnel = 0.04 + 0.96 * pow(1.0 - clamp(dot(-rayDir, worldNormal), 0.0, 1.0), 5.0);
        color += skyColor(reflected) * fresnel * wet * 0.6;
    }
    color += albedo * emission;
    return color;
}

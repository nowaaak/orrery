vec3 skyColor(vec3 dir) {
    vec3 sun = sunDirection();
    float day = daylight(sun.y);
    float dusk = duskAmount(sun.y);
    float horizon = pow(1.0 - max(dir.y, 0.0), 4.0);

    vec3 zenith = mix(vec3(0.004, 0.005, 0.016), vec3(0.14, 0.32, 0.78), day);
    zenith = mix(zenith, vec3(0.12, 0.11, 0.30), dusk * 0.55);

    vec3 horizonColor = mix(vec3(0.020, 0.020, 0.050), vec3(0.52, 0.70, 0.96), day);
    float towardSun = dot(safeNormalize(vec3(dir.x, 0.0, dir.z)), safeNormalize(vec3(sun.x, 0.0, sun.z)));
    vec3 duskColor = mix(vec3(0.55, 0.22, 0.32), vec3(1.0, 0.45, 0.16), towardSun * 0.5 + 0.5);
    horizonColor = mix(horizonColor, duskColor, dusk * 0.85);

    vec3 color = mix(zenith, horizonColor, horizon);
    color *= mix(1.0, 0.3, smoothstep(0.0, 0.4, -dir.y));

    float cosSun = max(dot(dir, sun), 0.0);
    color += vec3(1.0, 0.55, 0.25) * pow(cosSun, 6.0) * (dusk * 0.9 + day * 0.15);
    color += vec3(1.0, 0.92, 0.80) * pow(cosSun, 48.0) * 0.5 * max(day, dusk);
    float cosMoon = max(dot(dir, -sun), 0.0);
    color += vec3(0.30, 0.36, 0.55) * pow(cosMoon, 24.0) * 0.06 * (1.0 - day);

    vec3 overcast = vec3(luminance(color)) * vec3(0.82, 0.88, 1.0) * 0.7;
    return mix(color, overcast, rainStrength * 0.85);
}

vec3 celestialCoords(vec3 dir) {
    vec3 sun = sunDirection();
    float tilt = radians(sunPathRotation);
    vec3 axis = vec3(0.0, sin(tilt), cos(tilt));
    axis = safeNormalize(axis - sun * dot(axis, sun));
    vec3 side = cross(axis, sun);
    return vec3(dot(dir, sun), dot(dir, side), dot(dir, axis));
}

vec3 starField(vec3 coords, float pixelAngle, float density) {
    const float scale = 120.0;
    vec3 p = coords * scale;
    vec3 cell = floor(p);
    if (hashCell(cell) > 0.055 * density) {
        return vec3(0.0);
    }
    vec3 center = cell + 0.3 + 0.4 * vec3(hashCell(cell + 11.0), hashCell(cell + 23.0), hashCell(cell + 37.0));
    float size = 0.07 + 0.16 * pow(hashCell(cell + 5.0), 8.0);
    float radius = max(size, pixelAngle * scale);
    float glow = (1.0 - smoothstep(0.0, radius, length(p - center))) * (size * size) / (radius * radius);
    float twinkle = 0.7 + 0.3 * sin(frameTimeCounter * (1.0 + 3.0 * hashCell(cell + 3.0)) + hashCell(cell + 7.0) * TAU);
    vec3 tint = mix(vec3(0.65, 0.78, 1.0), vec3(1.0, 0.82, 0.62), hashCell(cell + 13.0));
    return tint * glow * twinkle * 4.0;
}

vec3 nebulaBand(vec3 coords, float strength) {
    const vec3 bandNormal = vec3(0.31, 0.22, 0.92);
    float band = exp(-pow(dot(coords, bandNormal), 2.0) / 0.05);
    float clouds = valueNoise3D(coords * 4.0) * 0.55 + valueNoise3D(coords * 9.0) * 0.3 + valueNoise3D(coords * 21.0) * 0.15;
    float dust = smoothstep(0.35, 0.75, valueNoise3D(coords * 6.0 + 13.0));
    vec3 tint = mix(vec3(0.42, 0.20, 0.85), vec3(0.12, 0.55, 0.90), valueNoise3D(coords * 2.5 + 7.0));
    float glow = band * smoothstep(0.3, 0.85, clouds) * (1.0 - dust * 0.7);
    return (tint * glow + vec3(0.5, 0.45, 0.7) * band * 0.04) * strength;
}

vec3 auroraCurtain(vec3 dir, float noise) {
    if (dir.y < 0.04) {
        return vec3(0.0);
    }
    vec3 color = vec3(0.0);
    float time = frameTimeCounter * 0.03;
    for (int i = 0; i < 10; i++) {
        float layer = (float(i) + noise) / 10.0;
        vec2 plane = dir.xz / dir.y * (1.0 + layer * 0.35);
        float wave = valueNoise3D(vec3(plane * 0.6 + vec2(time, -time * 0.5), time * 2.0));
        float curtain = pow(1.0 - abs(wave * 2.0 - 1.0), 12.0);
        curtain *= smoothstep(0.25, 0.6, valueNoise3D(vec3(plane * 0.15, time)));
        color += mix(vec3(0.10, 1.0, 0.55), vec3(0.55, 0.22, 1.0), layer) * curtain * (1.0 - layer);
    }
    return color * smoothstep(0.04, 0.35, dir.y) * 0.035;
}

vec3 nightSky(vec3 dir, float pixelAngle, float noise) {
    vec3 sun = sunDirection();
    float night = (1.0 - daylight(sun.y)) * (1.0 - rainStrength);
    if (night < 0.001) {
        return vec3(0.0);
    }
    vec3 coords = celestialCoords(dir);
    float aboveHorizon = smoothstep(-0.05, 0.25, dir.y);
    vec3 color = vec3(0.0);
#ifdef STARS
    color += starField(coords, pixelAngle, STAR_DENSITY);
#endif
#ifdef NEBULA
    color += nebulaBand(coords, 0.06);
#endif
#ifdef AURORA
    color += auroraCurtain(dir, noise);
#endif
    return color * night * aboveHorizon;
}

vec3 endSky(vec3 dir, float pixelAngle) {
    vec3 coords = dir;
    vec3 color = vec3(0.010, 0.005, 0.020) + vec3(0.04, 0.02, 0.07) * pow(1.0 - abs(dir.y), 3.0);
    color += nebulaBand(coords, 0.35);
    color += starField(coords, pixelAngle, 2.5) * 0.8;
    return color;
}

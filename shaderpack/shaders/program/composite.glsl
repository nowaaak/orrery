#include "/lib/settings.glsl"
#include "/lib/common.glsl"
#include "/lib/sky.glsl"
#include "/lib/lighting.glsl"

#ifdef VSH
out vec2 texcoord;

void main() {
    texcoord = gl_MultiTexCoord0.xy;
    gl_Position = ftransform();
}
#endif

#ifdef FSH
uniform sampler2D colortex0;
uniform sampler2D depthtex0;
#ifdef SHADOWS
uniform sampler2DShadow shadowtex1;
#define SHADOW_LOOKUP
#endif

#include "/lib/shadows.glsl"

in vec2 texcoord;

/* RENDERTARGETS: 0 */
layout(location = 0) out vec4 outColor;

#if CLOUD_STEPS > 0
float cloudBottom() {
    return (cloudHeight > 1.0 ? cloudHeight : 192.0) + 8.0;
}

float cloudDensity(vec3 pos, float bottom, float top) {
    float height = (pos.y - bottom) / (top - bottom);
    if (height < 0.0 || height > 1.0) {
        return 0.0;
    }
    float time = frameTimeCounter;
    vec3 drift = pos + vec3(time * 4.0, 0.0, time * 1.5);
    float coverage = smoothNoise(drift.xz / 3600.0);
    float shape = valueNoise3D(drift / 42.0) * 0.55 + valueNoise3D(drift / 17.0) * 0.30 + valueNoise3D(drift / 6.5) * 0.15;
    float profile = smoothstep(0.0, 0.18, height) * (1.0 - smoothstep(0.45, 1.0, height));
    float cover = CLOUD_COVERAGE + rainStrength * 0.25;
    float density = (coverage * 0.7 + shape * 0.7) * profile - (1.15 - cover);
    return max(density, 0.0) * 3.5;
}

vec4 volumetricClouds(vec3 dir, float sceneDist, float noise) {
    float bottom = cloudBottom();
    float top = bottom + 110.0;
    if (abs(dir.y) < 1e-4) {
        return vec4(0.0, 0.0, 0.0, 1.0);
    }
    float t0 = (bottom - eyeAltitude) / dir.y;
    float t1 = (top - eyeAltitude) / dir.y;
    float rayEnter = max(min(t0, t1), 0.0);
    float rayExit = min(max(t0, t1), 4000.0);
    rayExit = min(rayExit, sceneDist);
    if (rayExit <= rayEnter) {
        return vec4(0.0, 0.0, 0.0, 1.0);
    }
    rayExit = min(rayExit, rayEnter + 900.0);

    vec3 eye = vec3(cameraPosition.x, eyeAltitude, cameraPosition.z);
    vec3 lightDir = lightDirection();
    float cosTheta = dot(dir, lightDir);
    float phase = mix(phaseHG(cosTheta, 0.45), phaseHG(cosTheta, -0.2), 0.4) * 4.0 * PI;
    vec3 sunColor = directLightColor() * 0.8;
    vec3 ambient = skyAmbientColor() * 1.1;

    float stepLength = (rayExit - rayEnter) / float(CLOUD_STEPS);
    float transmittance = 1.0;
    vec3 light = vec3(0.0);
    for (int i = 0; i < CLOUD_STEPS; i++) {
        vec3 pos = eye + dir * (rayEnter + (float(i) + noise) * stepLength);
        float density = cloudDensity(pos, bottom, top);
        if (density < 0.002) {
            continue;
        }
        float depthToLight = 0.0;
        for (int j = 0; j < 4; j++) {
            depthToLight += cloudDensity(pos + lightDir * (float(j) + 0.5) * 14.0, bottom, top) * 14.0;
        }
        float height = (pos.y - bottom) / (top - bottom);
        float beer = exp(-depthToLight * 0.035);
        float powder = 1.0 - exp(-depthToLight * 0.07);
        vec3 scattered = sunColor * beer * phase * mix(0.6, 1.0, powder) + ambient * mix(0.45, 1.0, height);
        float stepTransmittance = exp(-density * stepLength * 0.03);
        light += transmittance * scattered * (1.0 - stepTransmittance);
        transmittance *= stepTransmittance;
        if (transmittance < 0.02) {
            break;
        }
    }
    light *= mix(0.6, 1.0, daylight(sunDirection().y));
    float haze = 1.0 - exp(-rayEnter * 0.0007);
    light = mix(light, skyColor(dir) * (1.0 - transmittance), haze);
    float fade = 1.0 - smoothstep(2500.0, 4000.0, rayEnter);
    return vec4(light * fade, mix(1.0, transmittance, fade));
}
#endif

#if VL_STEPS > 0
vec3 volumetricLight(vec3 color, vec3 dir, float sceneDist, float noise) {
#ifdef SHADOW_LOOKUP
    float maxDist = min(sceneDist, 96.0);
    float stepLength = maxDist / float(VL_STEPS);
    vec3 origin = gbufferModelViewInverse[3].xyz;
    float lit = 0.0;
    for (int i = 0; i < VL_STEPS; i++) {
        vec3 pos = origin + dir * (float(i) + noise) * stepLength;
        float altitude = pos.y + cameraPosition.y;
        float density = exp(-max(altitude - 62.0, 0.0) / 35.0);
        lit += shadowSample(pos) * density * stepLength;
    }
    float cosTheta = dot(dir, lightDirection());
    float phase = phaseHG(cosTheta, 0.6) * 0.6 + 0.4 / (4.0 * PI);
    float dawn = duskAmount(sunDirection().y);
    float density = (0.0012 + rainStrength * 0.006 + dawn * 0.004) * VL_DENSITY;
    vec3 tint = isEyeInWater == 1 ? vec3(0.2, 0.7, 0.9) * 4.0 : vec3(1.0);
    return color + directLightColor() * tint * phase * lit * density;
#else
    return color;
#endif
}
#endif

void main() {
    vec3 color = texture(colortex0, texcoord).rgb;
    float depth = texture(depthtex0, texcoord).r;
    vec3 viewPos = screenToView(vec3(texcoord, depth));
    vec3 dir = viewToWorldDir(viewPos);
    float sceneDist = depth >= 1.0 ? 1e6 : length(viewPos);
    float noise = temporalNoise(gl_FragCoord.xy);

#if CLOUD_STEPS > 0
    if (hasSkylight && isEyeInWater == 0) {
        vec4 clouds = volumetricClouds(dir, sceneDist, noise);
        color = color * clouds.a + clouds.rgb;
    }
#endif
#if VL_STEPS > 0
    if (hasSkylight) {
        color = volumetricLight(color, dir, sceneDist, noise);
    }
#endif
    outColor = vec4(color, 1.0);
}
#endif

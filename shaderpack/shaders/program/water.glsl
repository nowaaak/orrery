#include "/lib/settings.glsl"
#include "/lib/common.glsl"
#include "/lib/sky.glsl"
#include "/lib/lighting.glsl"
#include "/lib/fog.glsl"
#include "/lib/nature.glsl"

#ifdef VSH
in vec4 mc_Entity;

out vec2 texcoord;
out vec2 lightCoord;
out vec4 vertexColor;
out vec3 viewPos;
out vec3 worldNormal;
flat out int blockId;

void main() {
    texcoord = (gl_TextureMatrix[0] * gl_MultiTexCoord0).xy;
    lightCoord = clamp(((gl_TextureMatrix[1] * gl_MultiTexCoord1).xy - 1.0 / 32.0) * 16.0 / 15.0, 0.0, 1.0);
    vertexColor = gl_Color;
    viewPos = (gl_ModelViewMatrix * gl_Vertex).xyz;
    worldNormal = mat3(gbufferModelViewInverse) * (gl_NormalMatrix * gl_Normal);
    blockId = int(mc_Entity.x + 0.5);
    gl_Position = jitter(gl_ProjectionMatrix * vec4(viewPos, 1.0));
}
#endif

#ifdef FSH
uniform sampler2D gtexture;
uniform sampler2D colortex4;
uniform sampler2D depthtex1;
#ifdef SHADOWS
uniform sampler2DShadow shadowtex1;
#define SHADOW_LOOKUP
#endif

#include "/lib/shadows.glsl"
#include "/lib/shading.glsl"

in vec2 texcoord;
in vec2 lightCoord;
in vec4 vertexColor;
in vec3 viewPos;
in vec3 worldNormal;
flat in int blockId;

/* RENDERTARGETS: 0 */
layout(location = 0) out vec4 outColor;

vec3 screenReflection(vec3 startView, vec3 viewNormal, vec3 fallback, float noise) {
#if SSR_STEPS > 0
    vec3 dir = reflect(normalize(startView), viewNormal);
    float stepLength = 0.6;
    vec3 previous = startView;
    vec3 pos = startView + dir * stepLength * noise;
    for (int i = 0; i < SSR_STEPS; i++) {
        previous = pos;
        pos += dir * stepLength;
        stepLength *= 1.32;
        if (pos.z > -0.1) {
            break;
        }
        vec3 screen = viewToScreen(pos);
        if (any(lessThan(screen, vec3(0.0))) || any(greaterThan(screen, vec3(1.0)))) {
            break;
        }
        float sceneDepth = texture(depthtex1, screen.xy).r;
        if (screen.z <= sceneDepth || sceneDepth >= 1.0) {
            continue;
        }
        vec3 sceneView = screenToView(vec3(screen.xy, sceneDepth));
        if (abs(sceneView.z - pos.z) > stepLength * 2.0) {
            continue;
        }
        vec3 low = previous;
        vec3 high = pos;
        for (int j = 0; j < 4; j++) {
            vec3 mid = (low + high) * 0.5;
            vec3 midScreen = viewToScreen(mid);
            if (midScreen.z > texture(depthtex1, midScreen.xy).r) {
                high = mid;
            } else {
                low = mid;
            }
        }
        vec2 hitUV = viewToScreen(high).xy;
        vec2 edge = smoothstep(vec2(0.0), vec2(0.08), hitUV) * (1.0 - smoothstep(vec2(0.92), vec2(1.0), hitUV));
        return mix(fallback, texture(colortex4, hitUV).rgb, edge.x * edge.y);
    }
#endif
    return fallback;
}

vec4 shadeWater(vec3 geometryNormal, float detail, float noise) {
    float dist = length(viewPos);
    vec3 playerPos = viewToPlayer(viewPos);
    vec3 normal = geometryNormal.y > 0.5 ? waveNormal(playerPos + cameraPosition, dist) : geometryNormal;
    vec3 rayDir = viewToWorldDir(viewPos);
    vec2 uv = gl_FragCoord.xy / vec2(viewWidth, viewHeight);

    float shadow = 1.0;
#ifdef SHADOW_LOOKUP
    shadow = shadowAmount(playerPos, geometryNormal, noise);
#endif

    vec2 refractedUV = uv + normal.xz * 0.02 / (1.0 + dist * 0.08);
    if (texture(depthtex1, refractedUV).r < gl_FragCoord.z) {
        refractedUV = uv;
    }
    float behindDepth = texture(depthtex1, refractedUV).r;
    vec3 behindView = screenToView(vec3(refractedUV, behindDepth));
    float thickness = behindDepth >= 1.0 ? 64.0 : max(length(behindView) - dist, 0.0);
    vec3 scene = texture(colortex4, refractedUV).rgb;

    vec3 tint = toLinear(vertexColor.rgb);
    if (behindDepth < 1.0 && isEyeInWater == 0) {
        vec3 groundPos = viewToPlayer(behindView) + cameraPosition;
        scene *= 1.0 + causticPattern(groundPos) * 2.0 * shadow * exp(-thickness * 0.2) * daylight(sunDirection().y);
    }

    vec3 absorption = (1.0 - tint) * 0.32 + 0.03;
    vec3 transmittance = exp(-absorption * thickness);
    vec3 body = tint * 0.09 * (0.45 + 0.9 * detail) * surfaceLight(lightCoord, max(dot(normal, lightDirection()), 0.0) * shadow * 0.3);
    vec3 transmitted = scene * transmittance + body * (1.0 - transmittance);

    if (isEyeInWater == 1) {
        return vec4(transmitted, 1.0);
    }

    float fresnel = 0.02 + 0.98 * pow(1.0 - clamp(dot(-rayDir, normal), 0.0, 1.0), 5.0);
    vec3 reflectedDir = reflect(rayDir, normal);
    vec3 skyReflection = hasSkylight ? skyColor(vec3(reflectedDir.x, abs(reflectedDir.y), reflectedDir.z)) : toLinear(fogColor);
    skyReflection *= mix(0.15, 1.0, lightCoord.y);
    vec3 reflection = screenReflection(viewPos, mat3(gbufferModelView) * normal, skyReflection, noise);

    float highlight = pow(max(dot(reflectedDir, lightDirection()), 0.0), 1200.0);
    vec3 specular = min(directLightColor() * highlight * shadow * 2.5, vec3(4.0));
    return vec4(mix(transmitted, reflection, fresnel) + specular, 1.0);
}

void main() {
    vec4 albedo = texture(gtexture, texcoord) * vertexColor;
    vec3 normal = safeNormalize(worldNormal) * (gl_FrontFacing ? 1.0 : -1.0);
    float noise = temporalNoise(gl_FragCoord.xy);

    if (blockId == 10001) {
        float detail = luminance(toLinear(albedo.rgb)) / max(luminance(toLinear(vertexColor.rgb)), 1e-3);
        vec3 water = applyFog(shadeWater(normal, detail, noise).rgb, viewPos, true);
        bool invalid = any(isnan(water)) || any(isinf(water));
        outColor = vec4(invalid ? texture(colortex4, gl_FragCoord.xy / vec2(viewWidth, viewHeight)).rgb : water, 1.0);
        return;
    }
    if (albedo.a < 0.01) {
        discard;
    }
    vec3 color = shadeSurface(toLinear(albedo.rgb), viewPos, normal, lightCoord, MAT_SOLID, 0.0, 1.0, noise);
    outColor = vec4(applyFog(color, viewPos, true), albedo.a);
}
#endif

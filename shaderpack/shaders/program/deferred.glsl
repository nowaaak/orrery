#include "/lib/settings.glsl"
#include "/lib/common.glsl"
#include "/lib/sky.glsl"
#include "/lib/lighting.glsl"
#include "/lib/fog.glsl"
#include "/lib/nature.glsl"

#ifdef VSH
out vec2 texcoord;

void main() {
    texcoord = gl_MultiTexCoord0.xy;
    gl_Position = ftransform();
}
#endif

#ifdef FSH
uniform sampler2D colortex0;
uniform sampler2D colortex1;
uniform sampler2D colortex2;
uniform sampler2D depthtex0;
#ifdef SHADOWS
uniform sampler2DShadow shadowtex1;
#define SHADOW_LOOKUP
#endif

#include "/lib/shadows.glsl"
#include "/lib/shading.glsl"

in vec2 texcoord;

/* RENDERTARGETS: 0,4 */
layout(location = 0) out vec4 outColor;
layout(location = 1) out vec4 outScene;

float ambientOcclusion(vec3 viewPos, vec3 viewNormal, float noise) {
#if SSAO_SAMPLES > 0
    const float radius = 0.7;
    vec3 tangent = normalize(cross(viewNormal, abs(viewNormal.y) < 0.9 ? vec3(0.0, 1.0, 0.0) : vec3(1.0, 0.0, 0.0)));
    vec3 bitangent = cross(viewNormal, tangent);
    float occlusion = 0.0;
    for (int i = 0; i < SSAO_SAMPLES; i++) {
        float fraction = (float(i) + noise) / float(SSAO_SAMPLES);
        float angle = float(i) * 2.39996 + noise * TAU;
        float r = sqrt(fraction);
        vec3 dir = (tangent * cos(angle) + bitangent * sin(angle)) * r + viewNormal * sqrt(max(1.0 - fraction, 0.0));
        vec3 samplePos = viewPos + dir * radius * mix(0.2, 1.0, fraction);
        vec3 screen = viewToScreen(samplePos);
        if (any(lessThan(screen.xy, vec2(0.0))) || any(greaterThan(screen.xy, vec2(1.0)))) {
            continue;
        }
        vec3 sceneView = screenToView(vec3(screen.xy, texture(depthtex0, screen.xy).r));
        float diff = sceneView.z - samplePos.z;
        occlusion += step(0.03, diff) * (1.0 - smoothstep(radius, radius * 2.5, diff));
    }
    return clamp(1.0 - occlusion / float(SSAO_SAMPLES) * SSAO_STRENGTH, 0.0, 1.0);
#else
    return 1.0;
#endif
}

void main() {
    vec4 scene = texture(colortex0, texcoord);
    float depth = texture(depthtex0, texcoord).r;
    int material = int(texture(colortex2, texcoord).r * 255.0 + 0.5);
    if (depth >= 1.0 || material == MAT_NONE || material == MAT_PRELIT) {
        outColor = scene;
        outScene = scene;
        return;
    }

    vec4 data = texture(colortex1, texcoord);
    vec3 worldNormal = decodeNormal(data.xy);
    vec3 viewPos = screenToView(vec3(texcoord, depth));
    float noise = temporalNoise(gl_FragCoord.xy);
    float ao = ambientOcclusion(viewPos, mat3(gbufferModelView) * worldNormal, noise);
    float emission = texture(colortex2, texcoord).b * 8.0;

    vec3 color = shadeSurface(scene.rgb, viewPos, worldNormal, data.zw, material, emission, ao, noise);
    color = applyFog(color, viewPos, true);
    outColor = vec4(color, 1.0);
    outScene = outColor;
}
#endif

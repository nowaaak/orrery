#include "/lib/settings.glsl"
#include "/lib/common.glsl"
#include "/lib/sky.glsl"
#include "/lib/lighting.glsl"
#include "/lib/fog.glsl"
#include "/lib/nature.glsl"

#ifdef VSH
out vec2 texcoord;
out vec2 lightCoord;
out vec4 vertexColor;
out vec3 viewPos;
out vec3 worldNormal;

void main() {
    texcoord = (gl_TextureMatrix[0] * gl_MultiTexCoord0).xy;
    lightCoord = clamp(((gl_TextureMatrix[1] * gl_MultiTexCoord1).xy - 1.0 / 32.0) * 16.0 / 15.0, 0.0, 1.0);
    vertexColor = gl_Color;
    viewPos = (gl_ModelViewMatrix * gl_Vertex).xyz;
    worldNormal = mat3(gbufferModelViewInverse) * (gl_NormalMatrix * gl_Normal);
    gl_Position = jitter(gl_ProjectionMatrix * vec4(viewPos, 1.0));
}
#endif

#ifdef FSH
uniform sampler2D gtexture;
#ifdef SHADOWS
#ifndef NO_SHADOWS
uniform sampler2DShadow shadowtex1;
#define SHADOW_LOOKUP
#endif
#endif

#include "/lib/shadows.glsl"
#include "/lib/shading.glsl"

in vec2 texcoord;
in vec2 lightCoord;
in vec4 vertexColor;
in vec3 viewPos;
in vec3 worldNormal;

/* RENDERTARGETS: 0,2 */
layout(location = 0) out vec4 outColor;
layout(location = 1) out vec4 outMaterial;

void main() {
    vec4 albedo = texture(gtexture, texcoord) * vertexColor;
    if (albedo.a < 0.1) {
        discard;
    }
    vec3 normal = safeNormalize(worldNormal) * (gl_FrontFacing ? 1.0 : -1.0);
    vec3 color = shadeSurface(toLinear(albedo.rgb), viewPos, normal, lightCoord, MAT_ENTITY, 0.0, 1.0, temporalNoise(gl_FragCoord.xy));
    outColor = vec4(applyFog(color, viewPos, true), albedo.a);
    outMaterial = vec4(1.0, 0.0, 0.0, 1.0);
}
#endif

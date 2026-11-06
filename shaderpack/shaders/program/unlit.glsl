#include "/lib/settings.glsl"
#include "/lib/common.glsl"
#include "/lib/sky.glsl"
#include "/lib/lighting.glsl"
#include "/lib/fog.glsl"

#ifdef VSH
out vec2 texcoord;
out vec2 lightCoord;
out vec4 vertexColor;
out vec3 viewPos;

void main() {
    texcoord = (gl_TextureMatrix[0] * gl_MultiTexCoord0).xy;
    lightCoord = clamp(((gl_TextureMatrix[1] * gl_MultiTexCoord1).xy - 1.0 / 32.0) * 16.0 / 15.0, 0.0, 1.0);
    vertexColor = gl_Color;
    viewPos = (gl_ModelViewMatrix * gl_Vertex).xyz;
    gl_Position = jitter(gl_ProjectionMatrix * vec4(viewPos, 1.0));
}
#endif

#ifdef FSH
uniform sampler2D gtexture;

in vec2 texcoord;
in vec2 lightCoord;
in vec4 vertexColor;
in vec3 viewPos;

/* RENDERTARGETS: 0 */
layout(location = 0) out vec4 outColor;

void main() {
#ifdef MODE_BASIC
    vec4 color = vertexColor;
#else
    vec4 color = texture(gtexture, texcoord) * vertexColor;
#endif

#ifdef MODE_DAMAGE
    outColor = color;
#else
    if (color.a < 0.01) {
        discard;
    }
    vec3 rgb = toLinear(color.rgb);
#if defined MODE_PARTICLE
    rgb = applyFog(rgb * surfaceLight(lightCoord, 0.6), viewPos, true);
#elif defined MODE_WEATHER
    rgb = applyFog(rgb * surfaceLight(lightCoord, 0.3), viewPos, true);
    color.a *= 0.6;
#elif defined MODE_EMISSIVE
    rgb *= 2.5;
#elif defined MODE_BASIC
    rgb = applyFog(rgb, viewPos, true);
#endif
    outColor = vec4(rgb, color.a);
#endif
}
#endif

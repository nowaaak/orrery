#include "/lib/settings.glsl"
#include "/lib/common.glsl"
#include "/lib/sky.glsl"
#include "/lib/lighting.glsl"
#include "/lib/fog.glsl"

#ifdef VSH
out vec2 texcoord;
out vec4 vertexColor;

void main() {
    texcoord = (gl_TextureMatrix[0] * gl_MultiTexCoord0).xy;
    vertexColor = gl_Color;
    gl_Position = jitter(ftransform());
}
#endif

#ifdef FSH
#ifdef SKY_TEXTURED
uniform sampler2D gtexture;
#endif

in vec2 texcoord;
in vec4 vertexColor;

/* RENDERTARGETS: 0 */
layout(location = 0) out vec4 outColor;

void main() {
#ifdef SKY_TEXTURED
    if (!hasSkylight && !hasCeiling) {
        vec3 viewDir = safeNormalize(screenToView(vec3(gl_FragCoord.xy / vec2(viewWidth, viewHeight), 1.0)));
        vec3 dir = mat3(gbufferModelViewInverse) * viewDir;
        outColor = vec4(endSky(dir, 2.0 / (gbufferProjection[1][1] * viewHeight)), 1.0);
        return;
    }
    vec4 color = texture(gtexture, texcoord) * vertexColor;
    vec3 rgb = toLinear(color.rgb);
    if (renderStage == MC_RENDER_STAGE_SUN) {
        rgb *= 5.0 * (1.0 - rainStrength);
    } else if (renderStage == MC_RENDER_STAGE_MOON) {
        rgb *= 1.6 * (1.0 - rainStrength);
    }
    outColor = vec4(rgb, color.a);
#else
    if (renderStage == MC_RENDER_STAGE_STARS || renderStage == MC_RENDER_STAGE_SUNSET) {
        discard;
    }
    vec3 viewDir = safeNormalize(screenToView(vec3(gl_FragCoord.xy / vec2(viewWidth, viewHeight), 1.0)));
    vec3 dir = mat3(gbufferModelViewInverse) * viewDir;
    vec3 color;
    if (hasSkylight) {
        color = skyColor(dir) + nightSky(dir, 2.0 / (gbufferProjection[1][1] * viewHeight), temporalNoise(gl_FragCoord.xy));
    } else {
        color = toLinear(fogColor);
    }
    if (isEyeInWater != 0) {
        color = applyFog(color, viewDir * far, false);
    }
    outColor = vec4(color * (1.0 - blindness), 1.0);
#endif
}
#endif

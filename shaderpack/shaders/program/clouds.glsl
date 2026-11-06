#include "/lib/settings.glsl"
#include "/lib/common.glsl"
#include "/lib/sky.glsl"
#include "/lib/lighting.glsl"

#ifdef VSH
out vec2 texcoord;
out vec4 vertexColor;
out vec3 viewPos;
out vec3 viewNormal;

void main() {
    texcoord = (gl_TextureMatrix[0] * gl_MultiTexCoord0).xy;
    vertexColor = gl_Color;
    viewPos = (gl_ModelViewMatrix * gl_Vertex).xyz;
    viewNormal = gl_NormalMatrix * gl_Normal;
    gl_Position = jitter(gl_ProjectionMatrix * vec4(viewPos, 1.0));
}
#endif

#ifdef FSH
uniform sampler2D gtexture;

in vec2 texcoord;
in vec4 vertexColor;
in vec3 viewPos;
in vec3 viewNormal;

/* RENDERTARGETS: 0 */
layout(location = 0) out vec4 outColor;

void main() {
    vec4 albedo = texture(gtexture, texcoord) * vertexColor;
    if (albedo.a < 0.01) {
        discard;
    }
    vec3 normal = length(viewNormal) > 1e-3 ? normalize(viewNormal) : normalize(gbufferModelView[1].xyz);
    float lambert = max(dot(normal, normalize(shadowLightPosition)), 0.0);
    vec3 color = vec3(0.95) * (skyAmbientColor() * 1.3 + directLightColor() * (0.35 + 0.4 * lambert));

    float dist = length(viewPos);
    float haze = 1.0 - exp(-dist * 0.0008 * (1.0 + rainStrength * 3.0));
    color = mix(color, skyColor(viewToWorldDir(viewPos)), haze);
    outColor = vec4(color, albedo.a * 0.8);
}
#endif

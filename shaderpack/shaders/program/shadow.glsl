#include "/lib/settings.glsl"
#include "/lib/common.glsl"
#include "/lib/nature.glsl"
#include "/lib/shadows.glsl"

#ifdef VSH
in vec4 mc_Entity;
in vec4 mc_midTexCoord;

out vec2 texcoord;
out vec4 vertexColor;

void main() {
    texcoord = (gl_TextureMatrix[0] * gl_MultiTexCoord0).xy;
    vertexColor = gl_Color;
    vec4 shadowView = gl_ModelViewMatrix * gl_Vertex;
    vec3 playerPos = (shadowModelViewInverse * shadowView).xyz;
    int blockId = int(mc_Entity.x + 0.5);
    playerPos += waveOffset(playerPos + cameraPosition, blockId, gl_MultiTexCoord0.t < mc_midTexCoord.t);
    gl_Position = gl_ProjectionMatrix * (shadowModelView * vec4(playerPos, 1.0));
    gl_Position.xyz = distortShadow(gl_Position.xyz);
}
#endif

#ifdef FSH
uniform sampler2D gtexture;

in vec2 texcoord;
in vec4 vertexColor;

/* RENDERTARGETS: 0 */
layout(location = 0) out vec4 outColor;

void main() {
    vec4 color = texture(gtexture, texcoord) * vertexColor;
    if (color.a < 0.1) {
        discard;
    }
    outColor = color;
}
#endif

#include "/lib/settings.glsl"
#include "/lib/common.glsl"
#include "/lib/nature.glsl"

#ifdef VSH
in vec4 mc_Entity;
in vec4 mc_midTexCoord;

out vec2 texcoord;
out vec2 lightCoord;
out vec4 vertexColor;
out vec3 worldNormal;
flat out int blockId;

void main() {
    texcoord = (gl_TextureMatrix[0] * gl_MultiTexCoord0).xy;
    lightCoord = clamp(((gl_TextureMatrix[1] * gl_MultiTexCoord1).xy - 1.0 / 32.0) * 16.0 / 15.0, 0.0, 1.0);
    vertexColor = gl_Color;
    worldNormal = mat3(gbufferModelViewInverse) * (gl_NormalMatrix * gl_Normal);
    blockId = int(mc_Entity.x + 0.5);

    vec3 viewPos = (gl_ModelViewMatrix * gl_Vertex).xyz;
#ifdef TERRAIN
    vec3 playerPos = viewToPlayer(viewPos);
    playerPos += waveOffset(playerPos + cameraPosition, blockId, gl_MultiTexCoord0.t < mc_midTexCoord.t);
    viewPos = (gbufferModelView * vec4(playerPos, 1.0)).xyz;
#endif
    gl_Position = jitter(gl_ProjectionMatrix * vec4(viewPos, 1.0));
}
#endif

#ifdef FSH
uniform sampler2D gtexture;

in vec2 texcoord;
in vec2 lightCoord;
in vec4 vertexColor;
in vec3 worldNormal;
flat in int blockId;

/* RENDERTARGETS: 0,1,2 */
layout(location = 0) out vec4 outAlbedo;
layout(location = 1) out vec4 outData;
layout(location = 2) out vec4 outMaterial;

void main() {
    vec4 albedo = texture(gtexture, texcoord) * vertexColor;
    if (albedo.a < 0.1) {
        discard;
    }
#ifdef ENTITY
    albedo.rgb = mix(albedo.rgb, entityColor.rgb, entityColor.a);
    int material = MAT_ENTITY;
#else
    int material = blockId == 10002 ? MAT_EMISSIVE
        : blockId == 10003 ? MAT_LEAVES
        : blockId == 10004 ? MAT_PLANT
        : MAT_SOLID;
#endif
    vec3 baseColor = toLinear(albedo.rgb);
    float emission = material == MAT_EMISSIVE ? pow(luminance(baseColor), 1.5) * 4.0 + 0.5 : 0.0;
    vec3 normal = safeNormalize(worldNormal) * (gl_FrontFacing ? 1.0 : -1.0);

    outAlbedo = vec4(baseColor, albedo.a);
    outData = vec4(encodeNormal(normal), lightCoord);
    outMaterial = vec4(float(material) / 255.0, 0.0, emission / 8.0, 1.0);
}
#endif

uniform mat4 gbufferModelView;
uniform mat4 gbufferModelViewInverse;
uniform mat4 gbufferProjection;
uniform mat4 gbufferProjectionInverse;
uniform mat4 gbufferPreviousModelView;
uniform mat4 gbufferPreviousProjection;
uniform mat4 shadowModelView;
uniform mat4 shadowModelViewInverse;
uniform mat4 shadowProjection;
uniform vec3 cameraPosition;
uniform vec3 previousCameraPosition;
uniform vec3 sunPosition;
uniform vec3 shadowLightPosition;
uniform vec3 fogColor;
uniform vec4 entityColor;
uniform float frameTimeCounter;
uniform float rainStrength;
uniform float wetness;
uniform float nightVision;
uniform float blindness;
uniform float darknessFactor;
uniform float viewWidth;
uniform float viewHeight;
uniform float far;
uniform float eyeAltitude;
uniform float cloudHeight;
uniform int frameCounter;
uniform int isEyeInWater;
uniform int renderStage;
uniform ivec2 eyeBrightnessSmooth;
uniform bool hasSkylight;
uniform bool hasCeiling;
uniform sampler2D orreryNoise;

const float TAU = 6.28318531;
const float PI = 3.14159265;
const int MAT_NONE = 0;
const int MAT_SOLID = 1;
const int MAT_LEAVES = 2;
const int MAT_PLANT = 3;
const int MAT_EMISSIVE = 4;
const int MAT_ENTITY = 5;
const int MAT_PRELIT = 255;

vec3 toLinear(vec3 color) {
    return pow(max(color, 0.0), vec3(2.2));
}

vec3 toDisplay(vec3 color) {
    return pow(max(color, 0.0), vec3(1.0 / 2.2));
}

float luminance(vec3 color) {
    return dot(color, vec3(0.2126, 0.7152, 0.0722));
}

vec3 safeNormalize(vec3 v) {
    float len = length(v);
    return len > 1e-5 ? v / len : vec3(0.0, 0.0, 1.0);
}

vec3 viewToPlayer(vec3 viewPos) {
    return mat3(gbufferModelViewInverse) * viewPos + gbufferModelViewInverse[3].xyz;
}

vec3 viewToWorldDir(vec3 viewPos) {
    return safeNormalize(mat3(gbufferModelViewInverse) * viewPos);
}

vec3 screenToView(vec3 screenPos) {
    vec4 pos = gbufferProjectionInverse * vec4(screenPos * 2.0 - 1.0, 1.0);
    return pos.xyz / pos.w;
}

vec3 viewToScreen(vec3 viewPos) {
    vec4 clip = gbufferProjection * vec4(viewPos, 1.0);
    return clip.xyz / clip.w * 0.5 + 0.5;
}

vec3 sunDirection() {
    return safeNormalize(mat3(gbufferModelViewInverse) * sunPosition);
}

vec3 lightDirection() {
    return safeNormalize(mat3(gbufferModelViewInverse) * shadowLightPosition);
}

float daylight(float sunHeight) {
    return smoothstep(-0.12, 0.18, sunHeight);
}

float duskAmount(float sunHeight) {
    return exp(-sunHeight * sunHeight / 0.03);
}

uint mixBits(uint x) {
    x ^= x >> 16u;
    x *= 0x7feb352du;
    x ^= x >> 15u;
    x *= 0x846ca68bu;
    x ^= x >> 16u;
    return x;
}

float hashCell(vec3 cell) {
    uvec3 q = uvec3(ivec3(floor(cell)));
    return float(mixBits(q.x ^ mixBits(q.y ^ mixBits(q.z)))) / 4294967295.0;
}

float valueNoise3D(vec3 p) {
    vec3 cell = floor(p);
    vec3 f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    vec2 uv = cell.xy + vec2(37.0, 17.0) * cell.z + f.xy;
    vec2 rg = texture(orreryNoise, (uv + 0.5) / 128.0).rg;
    return mix(rg.r, rg.g, f.z);
}

float smoothNoise(vec2 p) {
    return texture(orreryNoise, p).b;
}

float gradientNoise(vec2 pixel) {
    return fract(52.9829189 * fract(dot(pixel, vec2(0.06711056, 0.00583715))));
}

float temporalNoise(vec2 pixel) {
    return gradientNoise(pixel + 5.588238 * float(frameCounter % 64));
}

float phaseHG(float cosTheta, float g) {
    float g2 = g * g;
    return (1.0 - g2) / (4.0 * PI * pow(1.0 + g2 - 2.0 * g * cosTheta, 1.5));
}

vec2 encodeNormal(vec3 n) {
    n /= abs(n.x) + abs(n.y) + abs(n.z);
    vec2 signs = vec2(n.x >= 0.0 ? 1.0 : -1.0, n.y >= 0.0 ? 1.0 : -1.0);
    vec2 e = n.z >= 0.0 ? n.xy : (1.0 - abs(n.yx)) * signs;
    return e * 0.5 + 0.5;
}

vec3 decodeNormal(vec2 e) {
    e = e * 2.0 - 1.0;
    vec3 n = vec3(e, 1.0 - abs(e.x) - abs(e.y));
    float t = max(-n.z, 0.0);
    n.x += n.x >= 0.0 ? -t : t;
    n.y += n.y >= 0.0 ? -t : t;
    return normalize(n);
}

vec2 taaOffset() {
    const vec2 offsets[8] = vec2[8](
        vec2(0.125, -0.375), vec2(-0.125, 0.375), vec2(0.625, 0.125), vec2(0.375, -0.625),
        vec2(-0.625, 0.625), vec2(-0.875, -0.125), vec2(0.375, 0.875), vec2(0.875, -0.875)
    );
    return offsets[frameCounter % 8] / vec2(viewWidth, viewHeight);
}

vec4 jitter(vec4 clipPos) {
#ifdef TAA
    clipPos.xy += taaOffset() * clipPos.w;
#endif
    return clipPos;
}

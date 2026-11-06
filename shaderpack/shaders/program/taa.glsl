#include "/lib/settings.glsl"
#include "/lib/common.glsl"

#ifdef VSH
out vec2 texcoord;

void main() {
    texcoord = gl_MultiTexCoord0.xy;
    gl_Position = ftransform();
}
#endif

#ifdef FSH
uniform sampler2D colortex0;
uniform sampler2D colortex5;
uniform sampler2D depthtex0;

/*
const bool colortex5Clear = false;
*/

in vec2 texcoord;

/* RENDERTARGETS: 0,5 */
layout(location = 0) out vec4 outColor;
layout(location = 1) out vec4 outHistory;

vec2 previousUV(float depth) {
    if (depth < 0.56) {
        return texcoord;
    }
    vec3 playerPos = viewToPlayer(screenToView(vec3(texcoord, depth)));
    vec3 previousPlayer = playerPos + cameraPosition - previousCameraPosition;
    vec4 clip = gbufferPreviousProjection * (gbufferPreviousModelView * vec4(previousPlayer, 1.0));
    return clip.xy / clip.w * 0.5 + 0.5;
}

vec3 sanitize(vec3 color) {
    return any(isnan(color)) || any(isinf(color)) ? vec3(0.0) : min(color, vec3(64.0));
}

void main() {
    vec3 current = sanitize(texture(colortex0, texcoord).rgb);
#ifdef TAA
    vec2 pixel = 1.0 / vec2(viewWidth, viewHeight);
    vec3 low = current;
    vec3 high = current;
    for (int x = -1; x <= 1; x++) {
        for (int y = -1; y <= 1; y++) {
            vec3 neighbor = sanitize(texture(colortex0, texcoord + vec2(x, y) * pixel).rgb);
            low = min(low, neighbor);
            high = max(high, neighbor);
        }
    }
    vec2 previous = previousUV(texture(depthtex0, texcoord).r);
    vec3 history = clamp(sanitize(texture(colortex5, previous).rgb), low, high);
    bool onScreen = all(greaterThan(previous, vec2(0.0))) && all(lessThan(previous, vec2(1.0)));
    float motion = length((texcoord - previous) * vec2(viewWidth, viewHeight));
    float blend = onScreen ? 0.9 - min(motion * 0.02, 0.2) : 0.0;
    vec3 result = mix(current, history, blend);
    if (any(isnan(result)) || any(isinf(result))) {
        result = current;
    }
#else
    vec3 result = current;
#endif
    outColor = vec4(result, 1.0);
    outHistory = vec4(result, 1.0);
}
#endif

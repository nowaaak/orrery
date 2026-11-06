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

/*
const int colortex0Format = RGBA16F;
const int colortex1Format = RGBA16;
const int colortex2Format = RGBA8;
const int colortex4Format = RGBA16F;
const int colortex5Format = RGBA16F;
const vec4 colortex1ClearColor = vec4(0.0, 0.0, 0.0, 0.0);
const vec4 colortex2ClearColor = vec4(0.0, 0.0, 0.0, 0.0);
const bool colortex0MipmapEnabled = true;
*/

in vec2 texcoord;

layout(location = 0) out vec4 outColor;

vec3 tonemap(vec3 x) {
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0);
}

vec3 bloom() {
    vec2 viewSize = vec2(viewWidth, viewHeight);
    vec3 sum = vec3(0.0);
    float total = 0.0;
    for (int lod = 2; lod <= 6; lod++) {
        vec2 offset = exp2(float(lod)) / viewSize;
        vec3 tap = textureLod(colortex0, texcoord + offset * vec2(0.5, 0.5), float(lod)).rgb
            + textureLod(colortex0, texcoord + offset * vec2(-0.5, 0.5), float(lod)).rgb
            + textureLod(colortex0, texcoord + offset * vec2(0.5, -0.5), float(lod)).rgb
            + textureLod(colortex0, texcoord + offset * vec2(-0.5, -0.5), float(lod)).rgb;
        float weight = 1.0 / float(lod - 1);
        sum += tap * 0.25 * weight;
        total += weight;
    }
    return sum / total;
}

void main() {
    vec2 pixel = 1.0 / vec2(viewWidth, viewHeight);
    vec3 color = textureLod(colortex0, texcoord, 0.0).rgb;
#ifdef TAA
    vec3 neighbors = textureLod(colortex0, texcoord + vec2(pixel.x, 0.0), 0.0).rgb
        + textureLod(colortex0, texcoord - vec2(pixel.x, 0.0), 0.0).rgb
        + textureLod(colortex0, texcoord + vec2(0.0, pixel.y), 0.0).rgb
        + textureLod(colortex0, texcoord - vec2(0.0, pixel.y), 0.0).rgb;
    color = max(color + (color - neighbors * 0.25) * 0.35, 0.0);
#endif
    color = mix(color, bloom(), BLOOM_STRENGTH);

    float exposure = EXPOSURE * 0.8;
    if (hasSkylight) {
        exposure *= mix(1.9, 1.0, smoothstep(0.0, 0.6, float(eyeBrightnessSmooth.y) / 240.0));
    }
    color = tonemap(color * exposure);

    float gray = luminance(color);
    color = max(mix(vec3(gray), color, SATURATION), 0.0);
    color = mix(color, color * vec3(0.94, 0.95, 1.06), (1.0 - gray) * 0.35);

    vec2 centered = texcoord - 0.5;
    color *= 1.0 - VIGNETTE * dot(centered, centered) * 2.0;

    color = toDisplay(color) + (gradientNoise(gl_FragCoord.xy) - 0.5) / 255.0;
    outColor = vec4(color, 1.0);
}
#endif

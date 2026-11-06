#define SHADOWS
#define SHADOW_SOFTNESS 1.0 // [0.5 1.0 1.5 2.0 3.0]
#define SHADOW_SAMPLES 6 // [4 6 8 12 16]
#define SSAO_SAMPLES 4 // [0 4 8 12]
#define SSAO_STRENGTH 0.9 // [0.5 0.7 0.9 1.1]
#define WAVING_PLANTS

#define CLOUD_STEPS 0 // [0 16 24 40]
#define CLOUD_COVERAGE 0.5 // [0.3 0.4 0.5 0.6 0.7]
#define VL_STEPS 0 // [0 8 12 16]
#define VL_DENSITY 1.0 // [0.5 1.0 1.5 2.0 3.0]
#define FOG_DENSITY 1.0 // [0.0 0.5 0.75 1.0 1.5 2.0]

#define STARS
#define STAR_DENSITY 1.0 // [0.5 0.75 1.0 1.5 2.0]
#define NEBULA
#define AURORA

#define WATER_WAVES
#define SSR_STEPS 10 // [0 10 20 32]

#define TAA
#define BLOOM_STRENGTH 0.08 // [0.0 0.04 0.08 0.12 0.16]
#define EXPOSURE 1.0 // [0.6 0.7 0.8 0.9 1.0 1.1 1.2 1.3 1.4]
#define SATURATION 0.95 // [0.6 0.8 0.9 0.95 1.0 1.05 1.1 1.2 1.4]
#define VIGNETTE 0.25 // [0.0 0.1 0.25 0.4]

const int shadowMapResolution = 1536; // [1024 1536 2048 3072 4096]
const float shadowDistance = 96.0; // [64.0 96.0 128.0 160.0 192.0 256.0]
const float sunPathRotation = -25.0; // [-40.0 -30.0 -25.0 -20.0 -10.0 0.0 10.0 20.0 30.0]
const float shadowDistanceRenderMul = 1.0;
const bool shadowHardwareFiltering = true;
const bool shadowHardwareFiltering1 = true;

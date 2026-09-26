// Depth only, from a light's point of view. No fragment stage: the depth buffer
// is the whole output. See specs/0015-shadows.md and 0021-spot-lights.md.
const MAX_SPOT_LIGHTS: u32 = 4u;

struct SpotLight {
    position_range: vec4<f32>,
    direction_cos_outer: vec4<f32>,
    color_intensity: vec4<f32>,
    cos_inner: vec4<f32>,
    view_projection: mat4x4<f32>,
};

struct PointLight {
    position_range: vec4<f32>,
    color_intensity: vec4<f32>,
};

struct Uniforms {
    view_projection: mat4x4<f32>,
    camera_position: vec4<f32>,
    light_direction: vec4<f32>,
    light_color: vec4<f32>,
    ambient: vec4<f32>,
    light_view_projection: mat4x4<f32>,
    point_light_count: vec4<u32>,
    point_lights: array<PointLight, 8>,
    spot_light_count: vec4<u32>,
    spot_lights: array<SpotLight, MAX_SPOT_LIGHTS>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

// Which light this pass is filling the map for. Anything below MAX_SPOT_LIGHTS
// is that spot; anything at or above it is the sun. One pipeline fills every
// map, and this is the only thing that differs between the passes.
//
// A whole bind group for one number, because the alternative is immediate data,
// and that is a device capability this engine would then require everywhere.
struct Which {
    index: vec4<u32>,
};

@group(1) @binding(0) var<uniform> which: Which;

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(3) model_0: vec4<f32>,
    @location(4) model_1: vec4<f32>,
    @location(5) model_2: vec4<f32>,
    @location(6) model_3: vec4<f32>,
) -> @builtin(position) vec4<f32> {
    let model = mat4x4<f32>(model_0, model_1, model_2, model_3);
    let world = model * vec4<f32>(position, 1.0);

    if which.index.x >= MAX_SPOT_LIGHTS {
        return uniforms.light_view_projection * world;
    }

    return uniforms.spot_lights[which.index.x].view_projection * world;
}

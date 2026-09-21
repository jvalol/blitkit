// Depth only, from the light's point of view. No fragment stage: the depth
// buffer is the whole output. See specs/0015-shadows.md.
struct Uniforms {
    view_projection: mat4x4<f32>,
    camera_position: vec4<f32>,
    light_direction: vec4<f32>,
    light_color: vec4<f32>,
    ambient: vec4<f32>,
    light_view_projection: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(3) model_0: vec4<f32>,
    @location(4) model_1: vec4<f32>,
    @location(5) model_2: vec4<f32>,
    @location(6) model_3: vec4<f32>,
) -> @builtin(position) vec4<f32> {
    let model = mat4x4<f32>(model_0, model_1, model_2, model_3);
    return uniforms.light_view_projection * model * vec4<f32>(position, 1.0);
}

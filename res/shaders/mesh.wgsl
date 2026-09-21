struct Camera {
    view_projection: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

// Per vertex: the mesh. Per instance: where it goes and what color it is.
@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) model_0: vec4<f32>,
    @location(4) model_1: vec4<f32>,
    @location(5) model_2: vec4<f32>,
    @location(6) model_3: vec4<f32>,
    @location(7) normal_0: vec3<f32>,
    @location(8) normal_1: vec3<f32>,
    @location(9) normal_2: vec3<f32>,
    @location(10) color: vec4<f32>,
) -> VertexOutput {
    let model = mat4x4<f32>(model_0, model_1, model_2, model_3);
    let normal_matrix = mat3x3<f32>(normal_0, normal_1, normal_2);

    var out: VertexOutput;
    out.clip_position = camera.view_projection * model * vec4<f32>(position, 1.0);
    out.color = color;
    out.normal = normalize(normal_matrix * normal);
    out.uv = uv;
    return out;
}

// Flat color for now. Lighting is spec 0012 and textures are spec 0011; the
// normal is carried through so neither has to change this stage.
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}

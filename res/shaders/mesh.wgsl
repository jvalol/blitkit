// The camera and the one light, written once per frame.
struct Uniforms {
    view_projection: mat4x4<f32>,
    // xyz is the camera, w is unused padding
    camera_position: vec4<f32>,
    // xyz is the direction the light travels, w is unused padding
    light_direction: vec4<f32>,
    // rgb is the light's color, a is its intensity
    light_color: vec4<f32>,
    // rgb fills the side facing away, a is unused padding
    ambient: vec4<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

// White by default, so an untextured mesh multiplies by one, per spec 0011.
@group(1) @binding(0) var surface_texture: texture_2d<f32>;
@group(1) @binding(1) var surface_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) world_position: vec3<f32>,
    @location(4) shininess: f32,
};

// Per vertex: the mesh. Per instance: where it goes, what color it is, and how
// tight its highlight is.
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
    @location(11) shininess: f32,
) -> VertexOutput {
    let model = mat4x4<f32>(model_0, model_1, model_2, model_3);
    let normal_matrix = mat3x3<f32>(normal_0, normal_1, normal_2);
    let world = model * vec4<f32>(position, 1.0);

    var out: VertexOutput;
    out.clip_position = uniforms.view_projection * world;
    out.color = color;
    out.normal = normalize(normal_matrix * normal);
    out.uv = uv;
    out.world_position = world.xyz;
    out.shininess = shininess;
    return out;
}

// Blinn-Phong, matching Light::shade in src/lighting.rs. Keep the two in step.
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(in.normal);
    let to_light = normalize(-uniforms.light_direction.xyz);
    let to_viewer = normalize(uniforms.camera_position.xyz - in.world_position);

    let lambert = max(dot(normal, to_light), 0.0);
    let light = uniforms.light_color.rgb * uniforms.light_color.a;
    let diffuse = light * lambert;

    var specular = vec3<f32>(0.0);
    if lambert > 0.0 {
        let half_vector = normalize(to_light + to_viewer);
        specular = light * pow(max(dot(normal, half_vector), 0.0), max(in.shininess, 1.0));
    }

    // the instance color tints what is sampled rather than replacing it
    let sampled = textureSample(surface_texture, surface_sampler, in.uv);
    let base = in.color * sampled;

    let shaded = base.rgb * (uniforms.ambient.rgb + diffuse) + specular;
    return vec4<f32>(shaded, base.a);
}

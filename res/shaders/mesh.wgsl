// How many lamps fit. Matches lighting::MAX_POINT_LIGHTS. Keep the two in step.
const MAX_POINT_LIGHTS: u32 = 8u;

// One lamp: two vec4s exactly, because a uniform block aligns every array
// element to sixteen bytes. Matches GpuPointLight in renderer/mod.rs.
struct PointLight {
    // xyz is where it is, w is how far it reaches
    position_range: vec4<f32>,
    // rgb is the color, a is the intensity
    color_intensity: vec4<f32>,
};

// The camera, the sun and the lamps, written once per frame.
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
    // world to the light's clip space, for the shadow map
    light_view_projection: mat4x4<f32>,
    // x is how many of the lamps below are real, the rest is padding
    point_light_count: vec4<u32>,
    point_lights: array<PointLight, MAX_POINT_LIGHTS>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

// White by default, so an untextured mesh multiplies by one, per spec 0011.
@group(1) @binding(0) var surface_texture: texture_2d<f32>;
@group(1) @binding(1) var surface_sampler: sampler;

// What the light can see, per spec 0015.
@group(2) @binding(0) var shadow_map: texture_depth_2d;
@group(2) @binding(1) var shadow_sampler: sampler_comparison;

const MIN_BIAS: f32 = 0.0005;
const MAX_BIAS: f32 = 0.004;

// Matches shadow::is_lit and shadow::bias in Rust. Keep the two in step.
fn shadow_factor(world_position: vec3<f32>, normal: vec3<f32>, to_light: vec3<f32>) -> f32 {
    let clip = uniforms.light_view_projection * vec4<f32>(world_position, 1.0);
    if clip.w <= 0.0 {
        return 1.0;
    }

    let ndc = clip.xyz / clip.w;
    // outside the map is lit, not dark: wrong in the forgiving direction
    if abs(ndc.x) > 1.0 || abs(ndc.y) > 1.0 || ndc.z < 0.0 || ndc.z > 1.0 {
        return 1.0;
    }

    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, ndc.y * -0.5 + 0.5);
    let facing = clamp(dot(normal, to_light), 0.0, 1.0);
    let bias = MIN_BIAS + MAX_BIAS * (1.0 - facing);

    // nine samples in a small square, so edges are soft rather than stepped
    let texel = 1.0 / f32(textureDimensions(shadow_map).x);
    var lit = 0.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel;
            lit += textureSampleCompare(shadow_map, shadow_sampler, uv + offset, ndc.z - bias);
        }
    }

    return lit / 9.0;
}

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

// What one lamp adds, matching PointLight::shade in src/lighting.rs. Keep the
// two in step.
//
// The falloff is (1 - d/range) squared rather than inverse square: it reaches
// exactly nothing at the range, and it does not go to infinity at the lamp. See
// spec 0020. No ambient here, because the sun owns that.
fn point_light(
    lamp: PointLight,
    world_position: vec3<f32>,
    normal: vec3<f32>,
    to_viewer: vec3<f32>,
    base: vec3<f32>,
    shininess: f32,
) -> vec3<f32> {
    let range = lamp.position_range.w;
    if range <= 0.0 {
        return vec3<f32>(0.0);
    }

    let offset = lamp.position_range.xyz - world_position;
    let distance = length(offset);
    let left = 1.0 - clamp(distance / range, 0.0, 1.0);
    let faded = left * left;
    if faded <= 0.0 {
        return vec3<f32>(0.0);
    }

    let to_light = normalize(offset);
    let lambert = max(dot(normal, to_light), 0.0);
    if lambert <= 0.0 {
        return vec3<f32>(0.0);
    }

    let light = lamp.color_intensity.rgb * lamp.color_intensity.a * faded;
    let half_vector = normalize(to_light + to_viewer);
    let specular = light * pow(max(dot(normal, half_vector), 0.0), max(shininess, 1.0));

    return base * light * lambert + specular;
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

    // shadow dims what the light contributes, never the ambient fill
    let lit = shadow_factor(in.world_position, normal, to_light);

    // the instance color tints what is sampled rather than replacing it
    let sampled = textureSample(surface_texture, surface_sampler, in.uv);
    let base = in.color * sampled;

    var shaded = base.rgb * (uniforms.ambient.rgb + diffuse * lit) + specular * lit;

    // the lamps on top of the sun. They cast no shadow, so `lit` does not
    // touch them: a lamp inside a shadow still lights what is next to it.
    let lamps = min(uniforms.point_light_count.x, MAX_POINT_LIGHTS);
    for (var index = 0u; index < lamps; index++) {
        shaded += point_light(
            uniforms.point_lights[index],
            in.world_position,
            normal,
            to_viewer,
            base.rgb,
            in.shininess,
        );
    }

    return vec4<f32>(shaded, base.a);
}

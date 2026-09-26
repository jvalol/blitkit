// How many lamps and spots fit. Matches lighting::MAX_POINT_LIGHTS and
// MAX_SPOT_LIGHTS. Keep the three in step.
const MAX_POINT_LIGHTS: u32 = 8u;
const MAX_SPOT_LIGHTS: u32 = 4u;

// One lamp: two vec4s exactly, because a uniform block aligns every array
// element to sixteen bytes. Matches GpuPointLight in renderer/mod.rs.
struct PointLight {
    // xyz is where it is, w is how far it reaches
    position_range: vec4<f32>,
    // rgb is the color, a is the intensity
    color_intensity: vec4<f32>,
};

// One spot: three vec4s of parameters and the matrix onto its shadow layer.
// Matches GpuSpotLight in renderer/mod.rs.
struct SpotLight {
    // xyz is where it is, w is how far it reaches
    position_range: vec4<f32>,
    // xyz is the way it points, w is the cosine of the outer angle
    direction_cos_outer: vec4<f32>,
    // rgb is the color, a is the intensity
    color_intensity: vec4<f32>,
    // x is the cosine of the inner angle, the rest is padding
    cos_inner: vec4<f32>,
    view_projection: mat4x4<f32>,
};

// The camera, the sun, the lamps and the spots, written once per frame.
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
    // x is how many of the spots below are real
    spot_light_count: vec4<u32>,
    spot_lights: array<SpotLight, MAX_SPOT_LIGHTS>,
    // x is how many lamps cast, y and z are which lamps those are. A table of
    // indices rather than a flag on each lamp, because the slot in this table is
    // what says which six layers a lamp owns. See spec 0022.
    point_shadow: vec4<u32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

// White by default, so an untextured mesh multiplies by one, per spec 0011.
@group(1) @binding(0) var surface_texture: texture_2d<f32>;
@group(1) @binding(1) var surface_sampler: sampler;

// What the light can see, per spec 0015.
@group(2) @binding(0) var shadow_map: texture_depth_2d;
@group(2) @binding(1) var shadow_sampler: sampler_comparison;
// one layer per spot, per spec 0021
@group(2) @binding(2) var spot_shadow_maps: texture_depth_2d_array;
// six layers per casting lamp, per spec 0022
@group(2) @binding(3) var point_shadow_maps: texture_depth_2d_array;

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

// How much of a spot reaches a point, from its own shadow layer. The same
// comparison the sun gets, against a different map. Outside the map is lit, not
// dark: wrong in the forgiving direction, per spec 0021.
fn spot_shadow(
    spot: SpotLight,
    layer: u32,
    world_position: vec3<f32>,
    normal: vec3<f32>,
    to_light: vec3<f32>,
) -> f32 {
    let clip = spot.view_projection * vec4<f32>(world_position, 1.0);
    if clip.w <= 0.0 {
        return 1.0;
    }

    let ndc = clip.xyz / clip.w;
    if abs(ndc.x) > 1.0 || abs(ndc.y) > 1.0 || ndc.z < 0.0 || ndc.z > 1.0 {
        return 1.0;
    }

    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, ndc.y * -0.5 + 0.5);
    let facing = clamp(dot(normal, to_light), 0.0, 1.0);
    let bias = MIN_BIAS + MAX_BIAS * (1.0 - facing);

    let texel = 1.0 / f32(textureDimensions(spot_shadow_maps).x);
    var lit = 0.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel;
            lit += textureSampleCompare(
                spot_shadow_maps,
                shadow_sampler,
                uv + offset,
                layer,
                ndc.z - bias,
            );
        }
    }

    return lit / 9.0;
}

// What one spot adds, matching SpotLight::shade in src/lighting.rs. Keep the
// two in step. Unlike a lamp, a spot casts, so its own shadow dims it.
fn spot_light(
    spot: SpotLight,
    layer: u32,
    world_position: vec3<f32>,
    normal: vec3<f32>,
    to_viewer: vec3<f32>,
    base: vec3<f32>,
    shininess: f32,
) -> vec3<f32> {
    let range = spot.position_range.w;
    if range <= 0.0 {
        return vec3<f32>(0.0);
    }

    let offset = spot.position_range.xyz - world_position;
    let distance = length(offset);
    let left = 1.0 - clamp(distance / range, 0.0, 1.0);
    let faded = left * left;
    if faded <= 0.0 {
        return vec3<f32>(0.0);
    }

    // the cone, smooth between the inner and outer angles
    let facing = normalize(spot.direction_cos_outer.xyz);
    let toward = normalize(-offset);
    let cos_outer = spot.direction_cos_outer.w;
    let cos_inner = spot.cos_inner.x;
    let width = cos_inner - cos_outer;
    let cosine = dot(facing, toward);

    var cone = 0.0;
    if width <= 0.0 {
        cone = select(0.0, 1.0, cosine >= cos_inner);
    } else {
        let along = clamp((cosine - cos_outer) / width, 0.0, 1.0);
        cone = along * along * (3.0 - 2.0 * along);
    }
    if cone <= 0.0 {
        return vec3<f32>(0.0);
    }

    let to_light = normalize(offset);
    let lambert = max(dot(normal, to_light), 0.0);
    if lambert <= 0.0 {
        return vec3<f32>(0.0);
    }

    let reaching = spot_shadow(spot, layer, world_position, normal, to_light);
    if reaching <= 0.0 {
        return vec3<f32>(0.0);
    }

    let light = spot.color_intensity.rgb * spot.color_intensity.a * faded * cone * reaching;
    let half_vector = normalize(to_light + to_viewer);
    let specular = light * pow(max(dot(normal, half_vector), 0.0), max(shininess, 1.0));

    return base * light * lambert + specular;
}

// How many lamps may cast, and how much a lamp's distance comparison forgives.
// Matches lighting::MAX_SHADOWING_POINT_LIGHTS, shadow::POINT_MIN_BIAS and
// shadow::POINT_BIAS_PER_UNIT. Keep them in step.
const MAX_SHADOWING_POINT_LIGHTS: u32 = 2u;
const POINT_MIN_BIAS: f32 = 0.02;
const POINT_BIAS_PER_UNIT: f32 = 0.01;

// A face's three directions, written out rather than derived, because a shader
// has no business doing cross products for a constant. Across, up, and the way
// it looks, in the layer order +x, -x, +y, -y, +z, -z.
//
// Matches shadow::face_basis in Rust, which derives these the way `look_at`
// does and has a test that the two agree. Keep them in step.
fn point_face_right(face: u32) -> vec3<f32> {
    switch face {
        case 0u: { return vec3<f32>(0.0, 0.0, 1.0); }
        case 1u: { return vec3<f32>(0.0, 0.0, -1.0); }
        case 2u: { return vec3<f32>(1.0, 0.0, 0.0); }
        case 3u: { return vec3<f32>(1.0, 0.0, 0.0); }
        case 4u: { return vec3<f32>(-1.0, 0.0, 0.0); }
        default: { return vec3<f32>(1.0, 0.0, 0.0); }
    }
}

fn point_face_up(face: u32) -> vec3<f32> {
    switch face {
        case 2u: { return vec3<f32>(0.0, 0.0, 1.0); }
        case 3u: { return vec3<f32>(0.0, 0.0, -1.0); }
        default: { return vec3<f32>(0.0, 1.0, 0.0); }
    }
}

fn point_face_forward(face: u32) -> vec3<f32> {
    switch face {
        case 0u: { return vec3<f32>(1.0, 0.0, 0.0); }
        case 1u: { return vec3<f32>(-1.0, 0.0, 0.0); }
        case 2u: { return vec3<f32>(0.0, 1.0, 0.0); }
        case 3u: { return vec3<f32>(0.0, -1.0, 0.0); }
        case 4u: { return vec3<f32>(0.0, 0.0, 1.0); }
        default: { return vec3<f32>(0.0, 0.0, -1.0); }
    }
}

// Which face a direction leaves through, and where on it. The largest component
// picks the side of the box it reaches first; dividing the other two by it is
// the same division the projection would have done.
//
// Matches shadow::face_and_uv in Rust. Keep the two in step.
fn point_face_and_uv(direction: vec3<f32>) -> vec3<f32> {
    let size = abs(direction);

    var axis = 2u;
    var along = direction.z;
    if size.x >= size.y && size.x >= size.z {
        axis = 0u;
        along = direction.x;
    } else if size.y >= size.z {
        axis = 1u;
        along = direction.y;
    }

    let face = axis * 2u + select(0u, 1u, along < 0.0);
    let ahead = dot(point_face_forward(face), direction);

    let u = 0.5 + 0.5 * dot(point_face_right(face), direction) / ahead;
    let v = 0.5 - 0.5 * dot(point_face_up(face), direction) / ahead;

    return vec3<f32>(f32(face), u, v);
}

// How much of a lamp reaches a point, out of its six layers. Distance against
// distance, both as a fraction of the lamp's range.
//
// Matches shadow::point_is_lit and shadow::point_bias in Rust. Keep them in
// step.
fn point_shadow(lamp: PointLight, slot: u32, world_position: vec3<f32>) -> f32 {
    let range = lamp.position_range.w;
    let away = world_position - lamp.position_range.xyz;
    let distance = length(away);
    // a lamp that does not reach this far is not shadowing it either
    if range <= 0.0 || distance > range || distance <= 0.0 {
        return 1.0;
    }

    let found = point_face_and_uv(away);
    let layer = slot * 6u + u32(found.x);
    let uv = found.yz;

    let bias = (POINT_MIN_BIAS + POINT_BIAS_PER_UNIT * range) / range;
    let fraction = distance / range;

    // nine samples, the same soft edge the sun and the spots get. Across a face
    // edge the sampler clamps rather than carrying on into the next face, which
    // is the one place the six faces show.
    let texel = 1.0 / f32(textureDimensions(point_shadow_maps).x);
    var lit = 0.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel;
            lit += textureSampleCompare(
                point_shadow_maps,
                shadow_sampler,
                uv + offset,
                layer,
                fraction - bias,
            );
        }
    }

    return lit / 9.0;
}

// Which of a lamp's six layers to look in, or none at all. A search of two,
// because at most two lamps cast. See spec 0022.
fn point_shadow_slot(index: u32) -> i32 {
    let casting = min(uniforms.point_shadow.x, MAX_SHADOWING_POINT_LIGHTS);
    if casting > 0u && index == uniforms.point_shadow.y {
        return 0;
    }
    if casting > 1u && index == uniforms.point_shadow.z {
        return 1;
    }

    return -1;
}

// What one lamp adds, matching PointLight::shade in src/lighting.rs. Keep the
// two in step.
//
// The falloff is (1 - d/range) squared rather than inverse square: it reaches
// exactly nothing at the range, and it does not go to infinity at the lamp. See
// spec 0020. No ambient here, because the sun owns that.
fn point_light(
    lamp: PointLight,
    reaching: f32,
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

    if reaching <= 0.0 {
        return vec3<f32>(0.0);
    }

    let light = lamp.color_intensity.rgb * lamp.color_intensity.a * faded * reaching;
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

    // the lamps on top of the sun. The sun's own `lit` does not touch them: a
    // lamp inside a shadow still lights what is next to it. A lamp that asked
    // to cast carries its own shadow instead, per spec 0022.
    let lamps = min(uniforms.point_light_count.x, MAX_POINT_LIGHTS);
    for (var index = 0u; index < lamps; index++) {
        let lamp = uniforms.point_lights[index];
        let slot = point_shadow_slot(index);
        var reaching = 1.0;
        if slot >= 0 {
            reaching = point_shadow(lamp, u32(slot), in.world_position);
        }

        shaded += point_light(
            lamp,
            reaching,
            in.world_position,
            normal,
            to_viewer,
            base.rgb,
            in.shininess,
        );
    }

    // and the spots, which do cast, so each carries its own shadow
    let spots = min(uniforms.spot_light_count.x, MAX_SPOT_LIGHTS);
    for (var index = 0u; index < spots; index++) {
        shaded += spot_light(
            uniforms.spot_lights[index],
            index,
            in.world_position,
            normal,
            to_viewer,
            base.rgb,
            in.shininess,
        );
    }

    return vec4<f32>(shaded, base.a);
}

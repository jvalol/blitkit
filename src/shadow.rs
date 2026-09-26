//! Shadow mapping from the one directional light.
//!
//! See `specs/0015-shadows.md`. The matrix, the bias and the comparison rule
//! live here as plain maths so they can be checked without a GPU. The same
//! comparison runs in `mesh.wgsl`, and the two are kept in step by hand.

use crate::collision::Aabb;
use glam::{Mat4, Vec3, Vec4Swizzles};

/// How wide the shadow map is, in pixels. Bigger is sharper and slower.
pub const MAP_SIZE: u32 = 2048;

pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// The shadow a surface facing the light gets, before its angle is considered.
pub const MIN_BIAS: f32 = 0.0005;
/// The most bias a surface turned edge-on to the light gets. Too little stripes
/// lit surfaces, too much lifts a shadow off what casts it.
pub const MAX_BIAS: f32 = 0.004;

/// What the light's view covers when a game has not said otherwise.
pub fn default_bounds() -> Aabb {
    Aabb::from_center_size(Vec3::ZERO, Vec3::splat(40.0))
}

/// One matrix taking a world position into the light's clip space, fitted so
/// `bounds` fills the map and nothing outside it is recorded.
///
/// The light has no position, being directional, so the view is placed far
/// enough back to hold the whole box in front of it.
pub fn light_view_projection(direction: Vec3, bounds: &Aabb) -> Mat4 {
    let direction = direction.normalize_or_zero();
    let direction = if direction.length_squared() < 0.5 {
        Vec3::NEG_Y
    } else {
        direction
    };

    let center = bounds.center();
    let radius = (bounds.size().length() * 0.5).max(1e-3);
    // straight down would make the usual up vector useless
    let up = if direction.dot(Vec3::Y).abs() > 0.99 {
        Vec3::Z
    } else {
        Vec3::Y
    };

    let eye = center - direction * radius * 2.0;
    let view = glam::camera::rh::view::look_at_mat4(eye, center, up);

    // fit the box exactly, in the light's own space
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for corner in corners(bounds) {
        let in_light = (view * corner.extend(1.0)).xyz();
        min = min.min(in_light);
        max = max.max(in_light);
    }

    // the light looks down -z, so what is in front has negative z
    let near = (-max.z).max(0.0);
    let far = -min.z;

    glam::camera::rh::proj::directx::orthographic(min.x, max.x, min.y, max.y, near, far) * view
}

/// How wide a spot's shadow map is. Smaller than the sun's, because a cone
/// covers less ground than a sunrise does, and there can be four of them.
/// See spec 0021.
pub const SPOT_MAP_SIZE: u32 = 1024;

/// How close to a spot the map starts. Anything nearer than this is not
/// recorded, which keeps the depth range from being spent on the first
/// centimetre in front of the bulb.
pub const SPOT_NEAR: f32 = 0.1;

/// One matrix taking a world position into a spot's clip space.
///
/// A cone has a direction and an angle, so this is an ordinary perspective
/// view: the field of view is the whole cone, widened a little so the soft edge
/// is inside the map rather than clipped by it. Unlike the sun's, this one has
/// a place to look from.
pub fn spot_view_projection(position: Vec3, direction: Vec3, outer: f32, range: f32) -> Mat4 {
    let direction = direction.normalize_or_zero();
    let direction = if direction.length_squared() < 0.5 {
        Vec3::NEG_Y
    } else {
        direction
    };

    // straight down would make the usual up vector useless, the same way the
    // sun's does
    let up = if direction.dot(Vec3::Y).abs() > 0.99 {
        Vec3::Z
    } else {
        Vec3::Y
    };

    let view = glam::camera::rh::view::look_at_mat4(position, position + direction, up);

    // twice the half angle is the whole cone, and a tenth more so the edge of
    // the light is not sitting on the edge of the map
    let fov = (outer * 2.2).clamp(0.05, std::f32::consts::PI * 0.98);
    let far = range.max(SPOT_NEAR * 2.0);

    glam::camera::rh::proj::directx::perspective(fov, 1.0, SPOT_NEAR, far) * view
}

fn corners(bounds: &Aabb) -> [Vec3; 8] {
    let (min, max) = (bounds.min, bounds.max);
    [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(min.x, max.y, max.z),
        Vec3::new(max.x, max.y, max.z),
    ]
}

/// Where a world position lands on the shadow map: texture coordinates and the
/// depth to compare, or `None` when it falls outside the map.
///
/// Outside means lit, not dark: wrong in the forgiving direction.
pub fn map_position(light_view_projection: Mat4, world: Vec3) -> Option<(f32, f32, f32)> {
    let clip = light_view_projection * world.extend(1.0);
    if clip.w <= 0.0 {
        return None;
    }

    let ndc = clip.xyz() / clip.w;
    if ndc.x < -1.0 || ndc.x > 1.0 || ndc.y < -1.0 || ndc.y > 1.0 || ndc.z < 0.0 || ndc.z > 1.0 {
        return None;
    }

    // clip space is y up, texture coordinates are y down
    Some((ndc.x * 0.5 + 0.5, ndc.y * -0.5 + 0.5, ndc.z))
}

/// How much to forgive a depth comparison, given how square-on the surface is
/// to the light. An edge-on surface records depths its own fragments fail
/// against, which stripes it with false shadow.
pub fn bias(normal: Vec3, to_light: Vec3) -> f32 {
    let facing = normal.normalize_or_zero().dot(to_light.normalize_or_zero());
    MIN_BIAS + MAX_BIAS * (1.0 - facing.clamp(0.0, 1.0))
}

/// The comparison itself: is this fragment nearer the light than whatever the
/// map recorded there?
pub fn is_lit(recorded_depth: f32, fragment_depth: f32, bias: f32) -> bool {
    fragment_depth - bias <= recorded_depth
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_spot_projection_covers_its_cone() {
        let at = Vec3::new(0.0, 4.0, 0.0);
        let outer = 30f32.to_radians();
        let range = 12.0;
        let matrix = spot_view_projection(at, -Vec3::Y, outer, range);

        // straight down the middle, partway along, lands in the map
        let middle = at - Vec3::Y * 6.0;
        assert!(map_position(matrix, middle).is_some(), "its own axis is off the map");

        // and so does the rim of the cone, which is what the widened field of
        // view is for: the soft edge has to be recorded, not clipped
        let reach = 6.0 * outer.tan();
        for (x, z) in [(reach, 0.0), (-reach, 0.0), (0.0, reach), (0.0, -reach)] {
            let rim = at + Vec3::new(x, -6.0, z);
            assert!(
                map_position(matrix, rim).is_some(),
                "the rim at {:?} fell off the map",
                rim
            );
        }

        // behind the spot is not in its map at all
        assert!(map_position(matrix, at + Vec3::Y * 3.0).is_none());
    }

    #[test]
    fn a_spot_projection_ends_at_its_range() {
        let at = Vec3::Y * 4.0;
        let matrix = spot_view_projection(at, -Vec3::Y, 30f32.to_radians(), 10.0);

        assert!(map_position(matrix, at - Vec3::Y * 9.0).is_some());
        // past the range there is nothing recorded, so nothing shadows
        assert!(map_position(matrix, at - Vec3::Y * 11.0).is_none());
    }

    #[test]
    fn a_spot_pointed_straight_down_still_has_an_up() {
        // the usual up vector is useless when the cone points along it, the
        // same trap the sun's matrix has
        let matrix = spot_view_projection(Vec3::Y * 3.0, -Vec3::Y, 30f32.to_radians(), 8.0);
        let below = map_position(matrix, Vec3::ZERO);

        assert!(below.is_some(), "straight down produced nothing");
        let (u, v, _) = below.expect("straight down is on the map");
        assert!((u - 0.5).abs() < 1e-3 && (v - 0.5).abs() < 1e-3, "off centre at {} {}", u, v);
    }

    #[test]
    fn a_spot_aimed_nowhere_falls_back_rather_than_producing_nonsense() {
        let matrix = spot_view_projection(Vec3::Y * 3.0, Vec3::ZERO, 30f32.to_radians(), 8.0);

        // aimed down, the same fallback the sun uses
        assert!(map_position(matrix, Vec3::ZERO).is_some());
        assert!(matrix.to_cols_array().iter().all(|v| v.is_finite()));
    }
    use glam::vec3;

    fn light() -> Vec3 {
        vec3(-0.3, -1.0, -0.4).normalize()
    }

    #[test]
    fn the_bounds_fit_in_the_light_view() {
        let bounds = default_bounds();
        let matrix = light_view_projection(light(), &bounds);

        for corner in corners(&bounds) {
            let clip = matrix * corner.extend(1.0);
            let ndc = clip.xyz() / clip.w;

            assert!(ndc.x.abs() <= 1.0 + 1e-4, "x {} for {:?}", ndc.x, corner);
            assert!(ndc.y.abs() <= 1.0 + 1e-4, "y {} for {:?}", ndc.y, corner);
            assert!(
                (-1e-4..=1.0 + 1e-4).contains(&ndc.z),
                "z {} for {:?}",
                ndc.z,
                corner
            );
        }
    }

    #[test]
    fn light_space_depth_matches_wgpu() {
        let bounds = Aabb::from_center_size(Vec3::ZERO, Vec3::splat(10.0));
        // straight down, so depth follows height
        let matrix = light_view_projection(Vec3::NEG_Y, &bounds);

        let high = map_position(matrix, vec3(0.0, 4.0, 0.0)).expect("inside the map");
        let low = map_position(matrix, vec3(0.0, -4.0, 0.0)).expect("inside the map");

        // nearer the light is a smaller depth, and both are within 0 to 1
        assert!(high.2 < low.2, "high {} low {}", high.2, low.2);
        assert!((0.0..=1.0).contains(&high.2));
        assert!((0.0..=1.0).contains(&low.2));
    }

    #[test]
    fn nearer_than_the_map_is_lit() {
        // the map says something is at 0.5; this fragment is in front of it
        assert!(is_lit(0.5, 0.4, MIN_BIAS));
    }

    #[test]
    fn further_than_the_map_is_shadowed() {
        assert!(!is_lit(0.5, 0.6, MIN_BIAS));
        // and the bias does not forgive a difference that large
        assert!(!is_lit(0.5, 0.6, MAX_BIAS));
    }

    #[test]
    fn the_bias_forgives_a_surface_against_itself() {
        // the same depth, give or take the wobble that causes acne
        assert!(is_lit(0.5, 0.5 + MIN_BIAS * 0.5, MIN_BIAS));
    }

    #[test]
    fn outside_the_map_is_lit() {
        let bounds = Aabb::from_center_size(Vec3::ZERO, Vec3::splat(10.0));
        let matrix = light_view_projection(Vec3::NEG_Y, &bounds);

        // well outside the box the map covers
        assert!(map_position(matrix, vec3(500.0, 0.0, 0.0)).is_none());
        assert!(map_position(matrix, vec3(0.0, 0.0, -500.0)).is_none());
        // and inside it is on the map
        assert!(map_position(matrix, Vec3::ZERO).is_some());
    }

    #[test]
    fn the_bias_grows_with_the_angle() {
        let to_light = Vec3::Y;

        let square_on = bias(Vec3::Y, to_light);
        let tilted = bias(vec3(1.0, 1.0, 0.0).normalize(), to_light);
        let edge_on = bias(Vec3::X, to_light);

        assert!(square_on < tilted, "{} {}", square_on, tilted);
        assert!(tilted < edge_on, "{} {}", tilted, edge_on);
        assert!((square_on - MIN_BIAS).abs() < 1e-6);
        assert!((edge_on - (MIN_BIAS + MAX_BIAS)).abs() < 1e-6);
    }

    #[test]
    fn moving_the_light_moves_its_view() {
        let bounds = default_bounds();
        let from_above = light_view_projection(Vec3::NEG_Y, &bounds);
        let from_the_side = light_view_projection(vec3(-1.0, -0.2, 0.0).normalize(), &bounds);

        assert_ne!(from_above, from_the_side);

        // a tall post's shadow lands somewhere different
        let top = vec3(0.0, 5.0, 0.0);
        let overhead = map_position(from_above, top).expect("inside");
        let sideways = map_position(from_the_side, top).expect("inside");
        assert!((overhead.0 - sideways.0).abs() > 0.01 || (overhead.1 - sideways.1).abs() > 0.01);
    }
}

//! The sun, the lamps, and the shading they produce.
//!
//! See `specs/0012-lighting.md`. The maths here also runs in `mesh.wgsl`, on the
//! GPU. This copy exists so it can be checked without one; the two are kept in
//! step by hand, which is the gap the spec's hand checks cover.

use glam::Vec3;

/// A light with no position, only a direction, like the sun.
#[derive(Debug, Copy, Clone)]
pub struct Light {
    /// The direction the light travels, so a light overhead points down.
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    /// Fills the side facing away, so it is not pure black.
    pub ambient: Vec3,
}

impl Light {
    pub fn new() -> Self {
        Self {
            // from above and a little to one side, so a cube's faces differ
            direction: Vec3::new(-0.3, -1.0, -0.4).normalize(),
            color: Vec3::ONE,
            intensity: 1.0,
            ambient: Vec3::splat(0.15),
        }
    }

    /// Blinn-Phong: ambient, plus diffuse from the angle to the light, plus a
    /// highlight from the half vector between the light and the viewer.
    ///
    /// `normal` and `to_viewer` are unit vectors in world space, `base` is the
    /// surface color, and `shininess` is how tight the highlight is.
    pub fn shade(&self, normal: Vec3, to_viewer: Vec3, base: Vec3, shininess: f32) -> Vec3 {
        let to_light = -self.direction.normalize_or_zero();
        let normal = normal.normalize_or_zero();
        let lambert = normal.dot(to_light).max(0.0);

        let light = self.color * self.intensity;
        let diffuse = light * lambert;

        let specular = if lambert > 0.0 {
            let half = (to_light + to_viewer.normalize_or_zero()).normalize_or_zero();
            light * normal.dot(half).max(0.0).powf(shininess.max(1.0))
        } else {
            Vec3::ZERO
        };

        base * (self.ambient + diffuse) + specular
    }
}

impl Default for Light {
    fn default() -> Self {
        Self::new()
    }
}

/// How many lamps fit in the block the shader reads. See spec 0020.
pub const MAX_POINT_LIGHTS: usize = 8;

/// A light with a place, and a distance past which it stops. Unlike the sun it
/// casts no shadow, because there is one shadow map and it belongs to the sun.
#[derive(Debug, Copy, Clone)]
pub struct PointLight {
    pub position: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    /// How far it reaches. Past this it contributes nothing at all.
    pub range: f32,
}

impl PointLight {
    pub fn new(position: Vec3, color: Vec3, intensity: f32, range: f32) -> Self {
        Self {
            position,
            color,
            intensity,
            range,
        }
    }

    /// How much of the light is left at `distance`: one where it sits, nothing
    /// at its range, and smooth between.
    ///
    /// Not inverse square. That never reaches zero, so every light would have
    /// to be weighed against every surface, and it goes to infinity where the
    /// light is, so anything touching a lamp turns white. The square here is
    /// what keeps the middle from reading as a flat disc. See spec 0020.
    pub fn falloff(&self, distance: f32) -> f32 {
        if self.range <= 0.0 {
            return 0.0;
        }

        let left = 1.0 - (distance / self.range).clamp(0.0, 1.0);

        left * left
    }

    /// What this light adds at a point on a surface. Diffuse and highlight, the
    /// same Blinn-Phong the sun uses, faded by distance. No ambient: the sun
    /// owns that, and eight lamps each adding their own would wash the scene
    /// out.
    ///
    /// `base` tints the diffuse and not the highlight, the same way
    /// [`Light::shade`] does it, because a highlight is the light reflected
    /// rather than the surface lit.
    ///
    /// Matches the loop in `mesh.wgsl`. Keep the two in step.
    pub fn shade(
        &self,
        at: Vec3,
        normal: Vec3,
        to_viewer: Vec3,
        base: Vec3,
        shininess: f32,
    ) -> Vec3 {
        let offset = self.position - at;
        let distance = offset.length();
        let faded = self.falloff(distance);
        if faded <= 0.0 {
            return Vec3::ZERO;
        }

        let to_light = offset.normalize_or_zero();
        let normal = normal.normalize_or_zero();
        let lambert = normal.dot(to_light).max(0.0);
        if lambert <= 0.0 {
            return Vec3::ZERO;
        }

        let light = self.color * self.intensity * faded;
        let half = (to_light + to_viewer.normalize_or_zero()).normalize_or_zero();
        let specular = light * normal.dot(half).max(0.0).powf(shininess.max(1.0));

        base * light * lambert + specular
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHITE: Vec3 = Vec3::ONE;

    /// A lamp one unit above the origin, reaching four.
    fn lamp() -> PointLight {
        PointLight::new(Vec3::Y, Vec3::ONE, 1.0, 4.0)
    }

    #[test]
    fn a_point_light_is_full_strength_at_its_own_position() {
        assert!((lamp().falloff(0.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn a_point_light_stops_at_its_range() {
        let lamp = lamp();

        assert!(lamp.falloff(lamp.range).abs() < 1e-6);
        assert_eq!(lamp.falloff(lamp.range + 1.0), 0.0);
        assert_eq!(lamp.falloff(1_000.0), 0.0);
    }

    #[test]
    fn a_point_light_fades_the_whole_way() {
        let lamp = lamp();
        let mut last = f32::INFINITY;

        for step in 0..=40 {
            let distance = lamp.range * step as f32 / 40.0;
            let now = lamp.falloff(distance);

            assert!(now <= last + 1e-6, "it brightened at {:.2}", distance);
            assert!((0.0..=1.0).contains(&now));
            last = now;
        }
        // and it is a curve rather than a straight line, which is what stops
        // the lit patch reading as a flat disc
        assert!(lamp.falloff(lamp.range * 0.5) < 0.5 - 0.05);
    }

    #[test]
    fn a_light_with_no_range_lights_nothing() {
        let dead = PointLight::new(Vec3::ZERO, Vec3::ONE, 1.0, 0.0);

        assert_eq!(dead.falloff(0.0), 0.0);
        assert_eq!(dead.shade(Vec3::ZERO, Vec3::Y, Vec3::Y, WHITE, 32.0), Vec3::ZERO);
    }

    #[test]
    fn a_point_light_does_not_light_the_back_of_a_surface() {
        // a floor at the origin facing down, with the lamp above it
        let away = lamp().shade(Vec3::ZERO, -Vec3::Y, -Vec3::Y, WHITE, 32.0);

        assert_eq!(away, Vec3::ZERO);
        // and the same surface turned over is lit
        let toward = lamp().shade(Vec3::ZERO, Vec3::Y, Vec3::Y, WHITE, 32.0);
        assert!(toward.length() > 0.0);
    }

    #[test]
    fn a_point_light_carries_its_color() {
        let red = PointLight::new(Vec3::Y, Vec3::new(1.0, 0.0, 0.0), 1.0, 4.0);
        let lit = red.shade(Vec3::ZERO, Vec3::Y, Vec3::Y, WHITE, 32.0);

        assert!(lit.x > 0.0);
        assert!(lit.y.abs() < 1e-6, "red light turned up green");
        assert!(lit.z.abs() < 1e-6, "red light turned up blue");

        // and twice the intensity is brighter
        let brighter = PointLight::new(Vec3::Y, Vec3::new(1.0, 0.0, 0.0), 2.0, 4.0);
        assert!(brighter.shade(Vec3::ZERO, Vec3::Y, Vec3::Y, WHITE, 32.0).x > lit.x);
    }

    #[test]
    fn point_lights_add_together() {
        let red = PointLight::new(Vec3::Y, Vec3::new(1.0, 0.0, 0.0), 1.0, 4.0);
        let blue = PointLight::new(Vec3::Y, Vec3::new(0.0, 0.0, 1.0), 1.0, 4.0);

        let both = red.shade(Vec3::ZERO, Vec3::Y, Vec3::Y, WHITE, 32.0)
            + blue.shade(Vec3::ZERO, Vec3::Y, Vec3::Y, WHITE, 32.0);

        assert!(both.x > 0.0 && both.z > 0.0, "one of them won outright");
    }

    #[test]
    fn no_point_lights_is_the_old_shading() {
        // nothing added is nothing changed, which is what keeps every game that
        // never asks for a lamp looking the way it looked
        let sun = Light::new();
        let none: &[PointLight] = &[];

        let from_sun = sun.shade(Vec3::Y, Vec3::Y, WHITE, 32.0);
        let from_lamps = none.iter().fold(Vec3::ZERO, |total, lamp| {
            total + lamp.shade(Vec3::ZERO, Vec3::Y, Vec3::Y, WHITE, 32.0)
        });

        assert_eq!(from_lamps, Vec3::ZERO);
        assert_eq!(from_sun + from_lamps, from_sun);
    }

    fn white() -> Vec3 {
        Vec3::ONE
    }

    #[test]
    fn facing_the_light_is_brighter() {
        let light = Light::new();
        let to_light = -light.direction;
        let away = light.direction;

        let lit = light.shade(to_light, to_light, white(), 32.0);
        let dark = light.shade(away, to_light, white(), 32.0);

        assert!(lit.x > dark.x, "lit {:?} dark {:?}", lit, dark);
    }

    #[test]
    fn the_dark_side_is_ambient() {
        let light = Light::new();
        // facing directly away, so no diffuse and no highlight
        let shaded = light.shade(light.direction, -light.direction, white(), 32.0);

        assert!((shaded - light.ambient).length() < 1e-5, "{:?}", shaded);
    }

    #[test]
    fn intensity_scales_the_light() {
        let mut light = Light::new();
        let normal = -light.direction;

        let dim = light.shade(normal, normal, white(), 32.0);
        light.intensity = 2.0;
        let bright = light.shade(normal, normal, white(), 32.0);

        assert!(bright.x > dim.x);
        // ambient is not scaled by intensity, only the light itself
        light.intensity = 0.0;
        let unlit = light.shade(normal, normal, white(), 32.0);
        assert!((unlit - light.ambient).length() < 1e-5, "{:?}", unlit);
    }

    #[test]
    fn the_default_light_is_overhead() {
        let light = Light::new();

        // it travels downward, which means it comes from above
        assert!(light.direction.y < 0.0);
        assert!((light.direction.length() - 1.0).abs() < 1e-5);
        assert_eq!(light.color, Vec3::ONE);
    }

    #[test]
    fn a_tighter_highlight_is_smaller() {
        let light = Light::new();
        let normal = -light.direction;

        let broad = light.shade(normal, Vec3::Y, white(), 4.0);
        let tight = light.shade(normal, Vec3::Y, white(), 128.0);

        // same diffuse, so any difference is the highlight
        assert!(broad.x >= tight.x);
    }
}

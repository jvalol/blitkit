//! One directional light, and the shading it produces.
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

#[cfg(test)]
mod tests {
    use super::*;

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

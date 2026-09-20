/// A rectangle in physical pixels: `position` is its center, measured from the
/// window's top-left corner with y pointing down, and `size` is its full width and height.
///
/// `color` is linear RGBA in the 0 to 1 range, and defaults to opaque white.
#[derive(Debug, Copy, Clone)]
pub struct Quad {
    pub position: cgmath::Vector2<f32>,
    pub size: cgmath::Vector2<f32>,
    pub color: cgmath::Vector4<f32>,
}

/// Opaque white, what a quad is drawn in unless it says otherwise.
pub const WHITE: cgmath::Vector4<f32> = cgmath::Vector4 {
    x: 1.0,
    y: 1.0,
    z: 1.0,
    w: 1.0,
};

impl Quad {
    pub fn new(position: cgmath::Vector2<f32>, size: cgmath::Vector2<f32>) -> Quad {
        Quad {
            position,
            size,
            color: WHITE,
        }
    }

    pub fn colored(
        position: cgmath::Vector2<f32>,
        size: cgmath::Vector2<f32>,
        color: cgmath::Vector4<f32>,
    ) -> Quad {
        Quad {
            position,
            size,
            color,
        }
    }
}

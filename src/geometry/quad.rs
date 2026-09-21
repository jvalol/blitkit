/// A rectangle in physical pixels: `position` is its center, measured from the
/// window's top-left corner with y pointing down, and `size` is its full width and height.
///
/// `color` is linear RGBA in the 0 to 1 range, and defaults to opaque white.
#[derive(Debug, Copy, Clone)]
pub struct Quad {
    pub position: glam::Vec2,
    pub size: glam::Vec2,
    pub color: glam::Vec4,
}

/// Opaque white, what a quad is drawn in unless it says otherwise.
pub const WHITE: glam::Vec4 = glam::Vec4::ONE;

impl Quad {
    pub fn new(position: glam::Vec2, size: glam::Vec2) -> Quad {
        Quad {
            position,
            size,
            color: WHITE,
        }
    }

    pub fn colored(position: glam::Vec2, size: glam::Vec2, color: glam::Vec4) -> Quad {
        Quad {
            position,
            size,
            color,
        }
    }
}

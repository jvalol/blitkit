pub const UNBOUNDED_F32: f32 = f32::INFINITY;

#[derive(Debug, Clone)]
pub struct RenderText {
    pub position: cgmath::Vector2<f32>,
    pub bounds: cgmath::Vector2<f32>,
    pub color: cgmath::Vector4<f32>,
    pub text: String,
    pub size: f32,
    pub focused: bool,
    pub centered: bool,
}

impl Default for RenderText {
    fn default() -> Self {
        Self {
            position: (0.0, 0.0).into(),
            bounds: (UNBOUNDED_F32, UNBOUNDED_F32).into(),
            color: (1.0, 1.0, 1.0, 1.0).into(),
            text: String::new(),
            size: 16.0,
            focused: false,
            centered: false,
        }
    }
}

pub struct TextRenderer {
    pub render_texts: Vec<RenderText>,
}

impl Default for TextRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl TextRenderer {
    pub fn new() -> Self {
        Self {
            render_texts: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.render_texts.clear();
    }

    pub fn push_render_text(&mut self, text: RenderText) {
        self.render_texts.push(text);
    }
}

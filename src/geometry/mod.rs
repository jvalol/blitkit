pub mod quad;
pub mod vertex;
use quad::Quad;

pub struct Geometry {
    vertex_data: Vec<vertex::Vertex>,
    index_data: Vec<u32>,
    pub num_quads: u32,
}

impl Geometry {
    pub fn new() -> Self {
        Self {
            vertex_data: Vec::new(),
            index_data: Vec::new(),
            num_quads: 0,
        }
    }

    pub fn reset(&mut self) {
        self.vertex_data.clear();
        self.index_data.clear();
        self.num_quads = 0;
    }

    pub fn push_quad(&mut self, quad: &Quad) {
        let min_x = quad.position.x - quad.size.x * 0.5;
        let min_y = quad.position.y - quad.size.y * 0.5;
        let max_x = quad.position.x + quad.size.x * 0.5;
        let max_y = quad.position.y + quad.size.y * 0.5;

        self.vertex_data.extend(&[
            vertex::Vertex {
                position: (min_x, min_y).into(),
            },
            vertex::Vertex {
                position: (max_x, min_y).into(),
            },
            vertex::Vertex {
                position: (max_x, max_y).into(),
            },
            vertex::Vertex {
                position: (min_x, max_y).into(),
            },
        ]);
        self.index_data.extend(&[
            self.num_quads * 4 + 0,
            self.num_quads * 4 + 1,
            self.num_quads * 4 + 2,
            self.num_quads * 4 + 0,
            self.num_quads * 4 + 2,
            self.num_quads * 4 + 3,
        ]);
        self.num_quads += 1;
    }

    pub(crate) fn vertex_data(&self) -> &[vertex::Vertex] {
        &self.vertex_data
    }

    pub(crate) fn index_data(&self) -> &[u32] {
        &self.index_data
    }
}

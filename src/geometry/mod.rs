pub mod quad;
pub mod vertex;
use quad::Quad;

pub struct Geometry {
    vertex_data: Vec<vertex::Vertex>,
    index_data: Vec<u32>,
    pub num_quads: u32,
}

impl Default for Geometry {
    fn default() -> Self {
        Self::new()
    }
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
                color: quad.color,
            },
            vertex::Vertex {
                position: (max_x, min_y).into(),
                color: quad.color,
            },
            vertex::Vertex {
                position: (max_x, max_y).into(),
                color: quad.color,
            },
            vertex::Vertex {
                position: (min_x, max_y).into(),
                color: quad.color,
            },
        ]);
        self.index_data.extend(&[
            (self.num_quads * 4),
            self.num_quads * 4 + 1,
            self.num_quads * 4 + 2,
            (self.num_quads * 4),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn corners(geometry: &Geometry) -> Vec<(f32, f32)> {
        geometry
            .vertex_data()
            .iter()
            .map(|vertex| (vertex.position.x, vertex.position.y))
            .collect()
    }

    #[test]
    fn push_quad_makes_corner_vertices() {
        let mut geometry = Geometry::new();
        geometry.push_quad(&Quad::new((100.0, 50.0).into(), (20.0, 10.0).into()));

        assert_eq!(
            corners(&geometry),
            vec![(90.0, 45.0), (110.0, 45.0), (110.0, 55.0), (90.0, 55.0)]
        );
        assert_eq!(geometry.num_quads, 1);
    }

    #[test]
    fn push_quad_carries_its_color_to_every_vertex() {
        let mut geometry = Geometry::new();
        let red = cgmath::Vector4::new(1.0, 0.0, 0.0, 1.0);
        geometry.push_quad(&Quad::colored((0.0, 0.0).into(), (2.0, 2.0).into(), red));

        assert!(geometry
            .vertex_data()
            .iter()
            .all(|vertex| vertex.color == red));
    }

    #[test]
    fn quads_default_to_white() {
        let mut geometry = Geometry::new();
        geometry.push_quad(&Quad::new((0.0, 0.0).into(), (2.0, 2.0).into()));

        assert!(geometry
            .vertex_data()
            .iter()
            .all(|vertex| vertex.color == quad::WHITE));
    }

    #[test]
    fn push_quad_makes_two_triangles() {
        let mut geometry = Geometry::new();
        geometry.push_quad(&Quad::new((0.0, 0.0).into(), (2.0, 2.0).into()));

        assert_eq!(geometry.index_data(), &[0, 1, 2, 0, 2, 3]);
    }

    #[test]
    fn second_quad_indices_are_offset() {
        let mut geometry = Geometry::new();
        geometry.push_quad(&Quad::new((0.0, 0.0).into(), (2.0, 2.0).into()));
        geometry.push_quad(&Quad::new((8.0, 8.0).into(), (2.0, 2.0).into()));

        assert_eq!(geometry.vertex_data().len(), 8);
        assert_eq!(geometry.index_data()[6..], [4, 5, 6, 4, 6, 7]);
        assert_eq!(geometry.num_quads, 2);
    }

    #[test]
    fn reset_clears_geometry() {
        let mut geometry = Geometry::new();
        geometry.push_quad(&Quad::new((0.0, 0.0).into(), (2.0, 2.0).into()));
        geometry.reset();

        assert!(geometry.vertex_data().is_empty());
        assert!(geometry.index_data().is_empty());
        assert_eq!(geometry.num_quads, 0);
    }
}

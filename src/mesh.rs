//! 3D geometry: what a mesh is made of, the shapes that ship with the engine,
//! and the transform that places one in the world.
//!
//! See `specs/0010-meshes.md`. Everything here is plain data, so it can be built
//! and checked without a GPU. Uploading it is the renderer's job.

use glam::{Mat3, Mat4, Quat, Vec3};

/// A corner of a mesh. Arrays rather than glam types, because this goes
/// straight to the GPU and `Vec3` would pad it, per spec 0007.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}

impl Vertex {
    pub const SIZE: wgpu::BufferAddress = std::mem::size_of::<Self>() as wgpu::BufferAddress;
    pub const DESC: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: Self::SIZE,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2],
    };

    pub fn new(position: [f32; 3], normal: [f32; 3], uv: [f32; 2]) -> Self {
        Self {
            position,
            normal,
            uv,
        }
    }
}

/// A mesh before it reaches the GPU.
#[derive(Debug, Clone, Default)]
pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl MeshData {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u32>) -> Self {
        Self { vertices, indices }
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// A unit cube centered on the origin, with a normal per face rather than
    /// per corner, so its edges stay sharp. That means four vertices per face
    /// instead of eight shared corners.
    pub fn cube() -> Self {
        let faces: [([f32; 3], [f32; 3], [f32; 3]); 6] = [
            // normal, then the two axes spanning the face
            ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
            ([0.0, 0.0, -1.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
            ([1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]),
            ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
            ([0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0]),
            ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
        ];

        let mut vertices = Vec::with_capacity(24);
        let mut indices = Vec::with_capacity(36);

        for (normal, right, up) in faces {
            let normal = Vec3::from(normal);
            let right = Vec3::from(right);
            let up = Vec3::from(up);
            let center = normal * 0.5;
            let base = vertices.len() as u32;

            for (corner, uv) in [
                (-right - up, [0.0, 1.0]),
                (right - up, [1.0, 1.0]),
                (right + up, [1.0, 0.0]),
                (-right + up, [0.0, 0.0]),
            ] {
                let position = center + corner * 0.5;
                vertices.push(Vertex::new(position.to_array(), normal.to_array(), uv));
            }

            // counter-clockwise seen from outside, which is what the back face
            // culling in spec 0009 expects
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }

        Self::new(vertices, indices)
    }

    /// A unit square on the xz plane, facing up. Handy as a floor.
    pub fn plane() -> Self {
        let normal = [0.0, 1.0, 0.0];
        let vertices = vec![
            Vertex::new([-0.5, 0.0, 0.5], normal, [0.0, 1.0]),
            Vertex::new([0.5, 0.0, 0.5], normal, [1.0, 1.0]),
            Vertex::new([0.5, 0.0, -0.5], normal, [1.0, 0.0]),
            Vertex::new([-0.5, 0.0, -0.5], normal, [0.0, 0.0]),
        ];

        Self::new(vertices, vec![0, 1, 2, 0, 2, 3])
    }

    /// Loads the first model out of a Wavefront OBJ file. Missing normals are
    /// filled in from the face they belong to; missing texture coordinates are
    /// zero.
    pub fn from_obj(bytes: &[u8]) -> Result<Self, tobj::LoadError> {
        let mut reader = std::io::BufReader::new(bytes);
        let (models, _) = tobj::load_obj_buf(
            &mut reader,
            &tobj::LoadOptions {
                triangulate: true,
                single_index: true,
                ..Default::default()
            },
            // no .mtl files: materials are spec 0011's problem
            |_| Ok((Vec::new(), Default::default())),
        )?;

        let model = models.into_iter().next().ok_or(tobj::LoadError::InvalidObjectName)?;
        let mesh = model.mesh;
        let count = mesh.positions.len() / 3;

        let mut vertices = Vec::with_capacity(count);
        for i in 0..count {
            let position = [
                mesh.positions[i * 3],
                mesh.positions[i * 3 + 1],
                mesh.positions[i * 3 + 2],
            ];
            let normal = if mesh.normals.len() >= (i + 1) * 3 {
                [
                    mesh.normals[i * 3],
                    mesh.normals[i * 3 + 1],
                    mesh.normals[i * 3 + 2],
                ]
            } else {
                [0.0, 0.0, 0.0]
            };
            let uv = if mesh.texcoords.len() >= (i + 1) * 2 {
                [mesh.texcoords[i * 2], mesh.texcoords[i * 2 + 1]]
            } else {
                [0.0, 0.0]
            };
            vertices.push(Vertex::new(position, normal, uv));
        }

        let mut data = Self::new(vertices, mesh.indices);
        if mesh.normals.is_empty() {
            data.compute_normals();
        }

        Ok(data)
    }

    /// Gives every vertex the normal of the faces it belongs to, averaged.
    pub fn compute_normals(&mut self) {
        let mut normals = vec![Vec3::ZERO; self.vertices.len()];

        for triangle in self.indices.chunks_exact(3) {
            let [a, b, c] = [
                triangle[0] as usize,
                triangle[1] as usize,
                triangle[2] as usize,
            ];
            let edge1 = Vec3::from(self.vertices[b].position) - Vec3::from(self.vertices[a].position);
            let edge2 = Vec3::from(self.vertices[c].position) - Vec3::from(self.vertices[a].position);
            let face = edge1.cross(edge2);

            for index in [a, b, c] {
                normals[index] += face;
            }
        }

        for (vertex, normal) in self.vertices.iter_mut().zip(normals) {
            vertex.normal = normal.normalize_or_zero().to_array();
        }
    }
}

/// Where a mesh sits in the world.
#[derive(Debug, Copy, Clone)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub fn new() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }

    pub fn at(position: Vec3) -> Self {
        Self {
            position,
            ..Self::new()
        }
    }

    pub fn with_rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn with_scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }

    /// Scale, then rotate, then move: the order that keeps scaling along the
    /// object's own axes.
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }

    /// Normals need the inverse transpose, or a squashed object lights as if it
    /// were not squashed.
    pub fn normal_matrix(&self) -> Mat3 {
        Mat3::from_mat4(self.matrix()).inverse().transpose()
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertex_has_position_normal_and_uv() {
        let vertex = Vertex::new([1.0, 2.0, 3.0], [0.0, 1.0, 0.0], [0.5, 0.25]);

        assert_eq!(vertex.position, [1.0, 2.0, 3.0]);
        assert_eq!(vertex.normal, [0.0, 1.0, 0.0]);
        assert_eq!(vertex.uv, [0.5, 0.25]);

        // the same padding trap as the 2D vertex, per spec 0007
        assert_eq!(std::mem::size_of::<Vertex>(), 32);
        assert_eq!(std::mem::offset_of!(Vertex, position), 0);
        assert_eq!(std::mem::offset_of!(Vertex, normal), 12);
        assert_eq!(std::mem::offset_of!(Vertex, uv), 24);

        let attributes = Vertex::DESC.attributes;
        assert_eq!(attributes[1].offset, 12);
        assert_eq!(attributes[2].offset, 24);
        assert_eq!(Vertex::DESC.array_stride, Vertex::SIZE);
    }

    #[test]
    fn the_cube_is_a_cube() {
        let cube = MeshData::cube();

        // four vertices per face, so the faces can have their own normals
        assert_eq!(cube.vertices.len(), 24);
        assert_eq!(cube.triangle_count(), 12);

        let mut corners: Vec<[i32; 3]> = cube
            .vertices
            .iter()
            .map(|v| [
                (v.position[0] * 2.0) as i32,
                (v.position[1] * 2.0) as i32,
                (v.position[2] * 2.0) as i32,
            ])
            .collect();
        corners.sort_unstable();
        corners.dedup();
        assert_eq!(corners.len(), 8, "a cube has eight corners");

        let mut normals: Vec<[i32; 3]> = cube
            .vertices
            .iter()
            .map(|v| [v.normal[0] as i32, v.normal[1] as i32, v.normal[2] as i32])
            .collect();
        normals.sort_unstable();
        normals.dedup();
        assert_eq!(normals.len(), 6, "one normal per face");

        // every normal points away from the middle, so the outside faces out
        for vertex in cube.vertices.iter() {
            let position = Vec3::from(vertex.position);
            let normal = Vec3::from(vertex.normal);
            assert!(position.dot(normal) > 0.0);
        }
    }

    #[test]
    fn the_plane_faces_up() {
        let plane = MeshData::plane();

        assert_eq!(plane.triangle_count(), 2);
        assert!(plane.vertices.iter().all(|v| v.normal == [0.0, 1.0, 0.0]));
        assert!(plane.vertices.iter().all(|v| v.position[1] == 0.0));
    }

    #[test]
    fn transform_applies_scale_rotation_then_position() {
        let transform = Transform::at(Vec3::new(10.0, 0.0, 0.0))
            .with_scale(Vec3::splat(2.0))
            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2));

        let moved = transform.matrix() * Vec3::new(1.0, 0.0, 0.0).extend(1.0);

        // scaled to (2,0,0), turned a quarter turn to (0,2,0), then moved
        assert!((moved.x - 10.0).abs() < 1e-5, "x was {}", moved.x);
        assert!((moved.y - 2.0).abs() < 1e-5, "y was {}", moved.y);
    }

    #[test]
    fn normal_matrix_survives_non_uniform_scale() {
        // squashed flat in y, which would tilt a normal if the model matrix
        // were used directly
        let transform = Transform::new().with_scale(Vec3::new(1.0, 0.1, 1.0));
        let slope = Vec3::new(0.0, 1.0, 1.0).normalize();

        let wrong = (Mat3::from_mat4(transform.matrix()) * slope).normalize();
        let right = (transform.normal_matrix() * slope).normalize();

        assert!(right.y > wrong.y, "the normal should stay steep, not flatten");
        // and a straight up normal is unchanged either way
        let up = transform.normal_matrix() * Vec3::Y;
        assert!((up.normalize() - Vec3::Y).length() < 1e-5);
    }

    #[test]
    fn loads_an_obj_file() {
        // one square, two triangles, with normals and texture coordinates
        let obj = b"v -1 0 1\nv 1 0 1\nv 1 0 -1\nv -1 0 -1\n\
vn 0 1 0\n\
vt 0 0\nvt 1 0\nvt 1 1\nvt 0 1\n\
f 1/1/1 2/2/1 3/3/1\nf 1/1/1 3/3/1 4/4/1\n";

        let mesh = MeshData::from_obj(obj).expect("the obj is valid");

        assert_eq!(mesh.triangle_count(), 2);
        assert!(mesh.vertices.iter().all(|v| v.normal == [0.0, 1.0, 0.0]));
        assert!(mesh.vertices.iter().all(|v| v.position[1] == 0.0));
    }

    #[test]
    fn computes_missing_normals() {
        let obj = b"v -1 0 1\nv 1 0 1\nv 1 0 -1\nf 1 2 3\n";

        let mesh = MeshData::from_obj(obj).expect("the obj is valid");

        // the face lies flat, so its normal points straight up or down
        for vertex in mesh.vertices.iter() {
            assert!((vertex.normal[1].abs() - 1.0).abs() < 1e-5, "{:?}", vertex);
        }
    }
}

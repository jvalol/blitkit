//! What a game wants drawn in 3D this frame.
//!
//! See `specs/0010-meshes.md`. A game pushes a mesh and a transform for each
//! thing it wants drawn, the same shape as `Geometry::push_quad`, and the
//! renderer batches everything sharing a mesh into one instanced draw.

use crate::mesh::Transform;
use glam::Vec4;
use std::collections::HashMap;

/// A mesh that has been uploaded to the GPU. Handing out a handle rather than
/// the buffers keeps a game from holding wgpu types, per the engine's
/// conventions.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MeshId(pub(crate) usize);

/// One copy of a mesh: where it is, and what color it is drawn in.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Instance {
    pub model: [[f32; 4]; 4],
    /// Three columns of the normal matrix. Vertex attributes pack tightly, so
    /// these are three floats each and not padded out to four.
    pub normal: [[f32; 3]; 3],
    pub color: [f32; 4],
}

unsafe impl bytemuck::Pod for Instance {}
unsafe impl bytemuck::Zeroable for Instance {}

impl Instance {
    pub const SIZE: wgpu::BufferAddress = std::mem::size_of::<Self>() as wgpu::BufferAddress;
    pub const DESC: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: Self::SIZE,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &wgpu::vertex_attr_array![
            3 => Float32x4,
            4 => Float32x4,
            5 => Float32x4,
            6 => Float32x4,
            7 => Float32x3,
            8 => Float32x3,
            9 => Float32x3,
            10 => Float32x4
        ],
    };

    pub fn new(transform: &Transform, color: Vec4) -> Self {
        let normal = transform.normal_matrix();

        Self {
            model: transform.matrix().to_cols_array_2d(),
            normal: [
                normal.x_axis.to_array(),
                normal.y_axis.to_array(),
                normal.z_axis.to_array(),
            ],
            color: color.to_array(),
        }
    }
}

/// The 3D half of a frame, emptied and refilled like `Geometry`.
#[derive(Debug, Default)]
pub struct Scene {
    instances: HashMap<MeshId, Vec<Instance>>,
}

impl Scene {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        // keeps the allocations, drops the contents
        for instances in self.instances.values_mut() {
            instances.clear();
        }
    }

    /// Draws `mesh` at `transform`, in white.
    pub fn push(&mut self, mesh: MeshId, transform: &Transform) {
        self.push_colored(mesh, transform, crate::geometry::quad::WHITE);
    }

    pub fn push_colored(&mut self, mesh: MeshId, transform: &Transform, color: Vec4) {
        self.instances
            .entry(mesh)
            .or_default()
            .push(Instance::new(transform, color));
    }

    /// Every mesh with something to draw, and its instances. Sorted, so a frame
    /// draws in the same order each time.
    pub(crate) fn batches(&self) -> Vec<(MeshId, &[Instance])> {
        let mut batches: Vec<(MeshId, &[Instance])> = self
            .instances
            .iter()
            .filter(|(_, instances)| !instances.is_empty())
            .map(|(mesh, instances)| (*mesh, instances.as_slice()))
            .collect();
        batches.sort_by_key(|(mesh, _)| *mesh);
        batches
    }

    pub fn is_empty(&self) -> bool {
        self.instances.values().all(|instances| instances.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn repeated_meshes_are_instanced() {
        let cube = MeshId(0);
        let plane = MeshId(1);
        let mut scene = Scene::new();

        scene.push(cube, &Transform::at(Vec3::ZERO));
        scene.push(cube, &Transform::at(Vec3::X));
        scene.push(plane, &Transform::new());

        let batches = scene.batches();

        // one draw per mesh, not one per thing drawn
        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].0, cube);
        assert_eq!(batches[0].1.len(), 2);
        assert_eq!(batches[1].1.len(), 1);
    }

    #[test]
    fn an_instance_carries_its_transform_and_color() {
        let transform = Transform::at(Vec3::new(1.0, 2.0, 3.0));
        let instance = Instance::new(&transform, Vec4::new(1.0, 0.0, 0.0, 1.0));

        // translation lives in the last column
        assert_eq!(instance.model[3][0], 1.0);
        assert_eq!(instance.model[3][1], 2.0);
        assert_eq!(instance.model[3][2], 3.0);
        assert_eq!(instance.color, [1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn the_instance_layout_matches_the_struct() {
        // a 4x4 matrix, three tightly packed columns, then a color
        assert_eq!(std::mem::size_of::<Instance>(), 64 + 36 + 16);
        assert_eq!(std::mem::offset_of!(Instance, normal), 64);
        assert_eq!(std::mem::offset_of!(Instance, color), 100);

        let attributes = Instance::DESC.attributes;
        assert_eq!(attributes[4].offset, 64);
        assert_eq!(attributes[7].offset, 100);
        assert_eq!(Instance::DESC.array_stride, Instance::SIZE);
    }

    #[test]
    fn resetting_empties_the_scene() {
        let mut scene = Scene::new();
        scene.push(MeshId(0), &Transform::new());
        assert!(!scene.is_empty());

        scene.reset();

        assert!(scene.is_empty());
        assert!(scene.batches().is_empty());
    }
}

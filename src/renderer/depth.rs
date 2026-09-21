//! The depth buffer, and the pipeline settings that go with it.
//!
//! See `specs/0009-depth.md`. The values here are plain data so they can be
//! checked without a GPU; the texture itself needs a device.

/// Depth only, 32 bit float. No stencil, because nothing here uses one.
pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// The depth buffer is the size of the surface, and is rebuilt whenever that
/// changes. A stale one would test this frame's fragments against last size's
/// depths.
pub fn descriptor(width: u32, height: u32) -> wgpu::TextureDescriptor<'static> {
    wgpu::TextureDescriptor {
        label: Some("Depth Texture"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    }
}

/// What the 3D pipeline declares: nearer fragments win, and they write their
/// depth so later ones are tested against them.
pub fn state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: FORMAT,
        depth_write_enabled: Some(true),
        depth_compare: Some(wgpu::CompareFunction::LessEqual),
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    }
}

/// 3D geometry culls its back faces, counter-clockwise winding facing the
/// camera. Quads do not, per spec 0001, so they keep the default.
pub fn mesh_primitive_state() -> wgpu::PrimitiveState {
    wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleList,
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: Some(wgpu::Face::Back),
        ..Default::default()
    }
}

/// The depth buffer and its view, kept together so a resize replaces both.
pub struct DepthTexture {
    pub view: wgpu::TextureView,
    pub size: (u32, u32),
}

impl DepthTexture {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let descriptor = descriptor(width, height);
        let texture = device.create_texture(&descriptor);

        Self {
            view: texture.create_view(&wgpu::TextureViewDescriptor::default()),
            size: (descriptor.size.width, descriptor.size.height),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_texture_matches_the_surface() {
        let descriptor = descriptor(1280, 720);

        assert_eq!(descriptor.size.width, 1280);
        assert_eq!(descriptor.size.height, 720);
        assert_eq!(descriptor.format, FORMAT);
        assert!(descriptor
            .usage
            .contains(wgpu::TextureUsages::RENDER_ATTACHMENT));
    }

    #[test]
    fn resizing_recreates_the_depth_texture() {
        let before = descriptor(800, 600);
        let after = descriptor(1024, 768);

        assert_ne!(before.size, after.size);
        // a minimized window reports zero, which is not a legal texture size
        assert_eq!(descriptor(0, 0).size.width, 1);
        assert_eq!(descriptor(0, 0).size.height, 1);
    }

    #[test]
    fn the_3d_pipeline_tests_depth() {
        let state = state();

        assert_eq!(state.format, FORMAT);
        assert_eq!(state.depth_write_enabled, Some(true));
        assert_eq!(state.depth_compare, Some(wgpu::CompareFunction::LessEqual));
    }

    #[test]
    fn the_3d_pipeline_culls_back_faces() {
        let primitive = mesh_primitive_state();

        assert_eq!(primitive.cull_mode, Some(wgpu::Face::Back));
        assert_eq!(primitive.front_face, wgpu::FrontFace::Ccw);
    }
}

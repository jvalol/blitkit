pub mod depth;
pub mod render_text;
pub mod scene;

use crate::camera::Camera;
use crate::collision::Aabb;
use crate::geometry::vertex::*;
use crate::geometry::Geometry;
use crate::lighting::Light;
use crate::mesh::MeshData;
use crate::shadow;
use crate::texture::TextureData;
use render_text::*;
use scene::{Batch, Instance, MeshId, Scene, TextureId};

use std::sync::Arc;

use wgpu::util::DeviceExt;
use wgpu_text::glyph_brush::ab_glyph::FontRef;
use wgpu_text::glyph_brush::{HorizontalAlign, Layout, Section, Text};
use wgpu_text::{BrushBuilder, TextBrush};
use winit::window::Window;

const FONT_BYTES: &[u8] = include_bytes!("../../res/fonts/PressStart2P-Regular.ttf");

pub struct Renderer {
    /// Kept so a game can lock or hide the cursor, per spec 0013. The surface
    /// owns the window too, but does not hand it back.
    window: Arc<Window>,
    cursor_locked: bool,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    screen_buffer: wgpu::Buffer,
    screen_bind_group: wgpu::BindGroup,
    camera: Camera,
    camera_buffer: wgpu::Buffer,
    /// Bound by the 3D pipeline, which arrives with meshes in spec 0010.
    #[allow(dead_code)]
    camera_bind_group: wgpu::BindGroup,
    /// Kept so that pipeline can be built against the same layout.
    #[allow(dead_code)]
    camera_bind_group_layout: wgpu::BindGroupLayout,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    text_brush: TextBrush<FontRef<'static>>,
    /// Built here and resized with the surface. The pass that attaches it
    /// arrives with meshes in spec 0010; quads and text never use it.
    depth: depth::DepthTexture,
    mesh_pipeline: wgpu::RenderPipeline,
    meshes: Vec<GpuMesh>,
    textures: Vec<wgpu::BindGroup>,
    shadow_pipeline: wgpu::RenderPipeline,
    /// The far side of translucent geometry, then the near side.
    translucent_pipelines: [wgpu::RenderPipeline; 2],
    shadow_view: wgpu::TextureView,
    shadow_bind_group: wgpu::BindGroup,
    /// What the light's view covers. A game sets it to fit its world.
    scene_bounds: Aabb,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    /// Grown as needed, like the quad buffers.
    instance_buffer: wgpu::Buffer,
}

/// A mesh living on the GPU. Built once, drawn many times.
struct GpuMesh {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    index_count: u32,
}

impl Renderer {
    pub fn width(&self) -> f32 {
        self.config.width as f32
    }

    pub fn height(&self) -> f32 {
        self.config.height as f32
    }

    pub async fn new(window: Arc<Window>, instance_desc: wgpu::InstanceDescriptor) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(instance_desc);
        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Device"),
                ..Default::default()
            })
            .await
            .unwrap();

        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(|format| format.is_srgb())
            .unwrap_or(capabilities.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            format,
            present_mode: wgpu::PresentMode::Fifo,
            ..surface
                .get_default_config(&adapter, size.width.max(1), size.height.max(1))
                .unwrap()
        };
        surface.configure(&device, &config);

        let screen_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Screen Uniform"),
            contents: bytemuck::cast_slice(&screen_size(&config)),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let screen_bind_group_layout =
            uniform_bind_group_layout(&device, "Screen", wgpu::ShaderStages::VERTEX);
        let screen_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Screen Bind Group"),
            layout: &screen_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: screen_buffer.as_entire_binding(),
            }],
        });

        let mut camera = Camera::new();
        camera.set_viewport(config.width as f32, config.height as f32);
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Scene Uniform"),
            contents: bytemuck::cast_slice(&[SceneUniform::new(
                &camera,
                &Light::new(),
                &shadow::default_bounds(),
            )]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_bind_group_layout = uniform_bind_group_layout(
            &device,
            "Scene",
            wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
        );
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let pipeline = create_render_pipeline(&device, config.format, &screen_bind_group_layout);

        let vertex_buffer = create_buffer(&device, 0, wgpu::BufferUsages::VERTEX);
        let index_buffer = create_buffer(&device, 0, wgpu::BufferUsages::INDEX);

        let text_brush = BrushBuilder::using_font_bytes(FONT_BYTES).unwrap().build(
            &device,
            config.width,
            config.height,
            config.format,
        );

        let depth = depth::DepthTexture::new(&device, config.width, config.height);
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        // linear and repeating, with mips, per spec 0011
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Texture Sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        let shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Shadow Map"),
            size: wgpu::Extent3d {
                width: shadow::MAP_SIZE,
                height: shadow::MAP_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: shadow::FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let shadow_view = shadow_texture.create_view(&wgpu::TextureViewDescriptor::default());
        // a comparison sampler returns how much of the sample passed the depth
        // test rather than a depth, which is what makes the edges soft
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Shadow Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let shadow_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Shadow Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        count: None,
                    },
                ],
            });
        let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Shadow Bind Group"),
            layout: &shadow_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&shadow_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                },
            ],
        });
        let shadow_pipeline = create_shadow_pipeline(&device, &camera_bind_group_layout);
        let mesh_pipeline = create_mesh_pipeline(
            &device,
            config.format,
            &camera_bind_group_layout,
            &texture_bind_group_layout,
            &shadow_bind_group_layout,
            depth::mesh_primitive_state(),
            depth::state(),
        );
        // the same shader twice more, for the far side of a translucent shape
        // and then its near side, per spec 0018
        let translucent_pipelines = [wgpu::Face::Front, wgpu::Face::Back].map(|cull| {
            create_mesh_pipeline(
                &device,
                config.format,
                &camera_bind_group_layout,
                &texture_bind_group_layout,
                &shadow_bind_group_layout,
                depth::translucent_primitive_state(cull),
                depth::translucent_state(),
            )
        });
        let instance_buffer = create_buffer(&device, 0, wgpu::BufferUsages::VERTEX);

        let mut renderer = Self {
            window,
            cursor_locked: false,
            surface,
            device,
            queue,
            config,
            pipeline,
            screen_buffer,
            screen_bind_group,
            camera,
            camera_buffer,
            camera_bind_group,
            camera_bind_group_layout,
            vertex_buffer,
            index_buffer,
            text_brush,
            depth,
            mesh_pipeline,
            translucent_pipelines,
            meshes: Vec::new(),
            textures: Vec::new(),
            shadow_pipeline,
            shadow_view,
            shadow_bind_group,
            scene_bounds: shadow::default_bounds(),
            texture_bind_group_layout,
            sampler,
            instance_buffer,
        };

        // TextureId::WHITE, what an untextured mesh is drawn with
        renderer.add_texture(&TextureData::white());
        renderer
    }

    /// Uploads an image and its mip chain, and hands back the handle a game
    /// draws with.
    pub fn add_texture(&mut self, data: &TextureData) -> TextureId {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Texture"),
            size: wgpu::Extent3d {
                width: data.width(),
                height: data.height(),
                depth_or_array_layers: 1,
            },
            mip_level_count: data.mip_level_count(),
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: crate::texture::FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        for (level, mip) in data.levels.iter().enumerate() {
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: level as u32,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &mip.pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(mip.width * 4),
                    rows_per_image: Some(mip.height),
                },
                wgpu::Extent3d {
                    width: mip.width,
                    height: mip.height,
                    depth_or_array_layers: 1,
                },
            );
        }

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Bind Group"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        self.textures.push(bind_group);
        TextureId(self.textures.len() - 1)
    }

    /// Uploads a mesh once, and hands back the handle a game draws it with.
    pub fn add_mesh(&mut self, data: &MeshData) -> MeshId {
        // an empty mesh is a legal thing to build, a lattice of no lines for
        // one, and wgpu refuses to make a buffer with nothing in it. It gets
        // placeholder buffers and an index count of zero, which the passes skip.
        let vertices = mesh_buffer(
            &self.device,
            "Mesh Vertices",
            bytemuck::cast_slice(&data.vertices),
            wgpu::BufferUsages::VERTEX,
        );
        let indices = mesh_buffer(
            &self.device,
            "Mesh Indices",
            bytemuck::cast_slice(&data.indices),
            wgpu::BufferUsages::INDEX,
        );

        self.meshes.push(GpuMesh {
            vertices,
            indices,
            index_count: data.indices.len() as u32,
        });

        MeshId(self.meshes.len() - 1)
    }

    /// What the shadow map covers. A world bigger than this gets no shadows
    /// outside it, per spec 0015; smaller bounds give sharper shadows.
    pub fn set_scene_bounds(&mut self, bounds: Aabb) {
        self.scene_bounds = bounds;
    }

    pub fn scene_bounds(&self) -> Aabb {
        self.scene_bounds
    }

    /// Hides the pointer and pins it, so turning does not stop at the screen
    /// edge. If the platform refuses, this says so and leaves the cursor
    /// visible: raw mouse motion arrives either way, so looking around still
    /// works.
    pub fn set_cursor_locked(&mut self, locked: bool) -> bool {
        let mode = if locked {
            winit::window::CursorGrabMode::Locked
        } else {
            winit::window::CursorGrabMode::None
        };

        let grabbed = match self.window.set_cursor_grab(mode) {
            Ok(()) => true,
            Err(e) if locked => {
                // some platforms only confine the cursor, which is close enough
                match self
                    .window
                    .set_cursor_grab(winit::window::CursorGrabMode::Confined)
                {
                    Ok(()) => true,
                    Err(_) => {
                        log::warn!("could not lock the cursor: {}", e);
                        false
                    }
                }
            }
            Err(_) => false,
        };

        self.window.set_cursor_visible(!locked || !grabbed);
        self.cursor_locked = locked && grabbed;
        self.cursor_locked
    }

    pub fn cursor_locked(&self) -> bool {
        self.cursor_locked
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    /// Games move the camera through this. It takes effect on the next frame.
    pub fn set_camera(&mut self, camera: Camera) {
        self.camera = camera;
        self.camera
            .set_viewport(self.config.width as f32, self.config.height as f32);
    }

    pub(crate) fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        // A minimized window reports a zero size, which a surface cannot be configured with.
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
        self.queue.write_buffer(
            &self.screen_buffer,
            0,
            bytemuck::cast_slice(&screen_size(&self.config)),
        );
        self.depth = depth::DepthTexture::new(&self.device, self.config.width, self.config.height);
        self.camera.set_viewport(self.width(), self.height());
        self.text_brush
            .resize_view(self.width(), self.height(), &self.queue);
    }

    pub fn render(&mut self, scene: &Scene, geometry: &Geometry, text_renderer: &TextRenderer) {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                self.surface.configure(&self.device, &self.config);
                frame
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => return,
            other => {
                log::error!("Failed to acquire surface texture: {:?}", other);
                return;
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[SceneUniform::new(
                &self.camera,
                &scene.light,
                &self.scene_bounds,
            )]),
        );
        self.upload_geometry(geometry);

        let sections: Vec<Section> = text_renderer
            .render_texts
            .iter()
            .map(text_section)
            .collect();
        if let Err(e) = self.text_brush.queue(&self.device, &self.queue, sections) {
            log::error!("Failed to queue text: {}", e);
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Renderer Encoder"),
            });

        let (opaque, translucent) = self.upload_instances(scene);
        // the light sees translucent things as solid: a shadow map holds a
        // depth and has nowhere to put an alpha
        let batches: Vec<Batch> = opaque.iter().chain(translucent.iter()).copied().collect();

        // what the light can see, first of all. Skipped when nothing is drawn in
        // 3D: the instance buffer is empty then, and slicing an empty buffer is
        // a panic in wgpu.
        if !batches.is_empty() {
            let mut shadow_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Shadow Pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            shadow_pass.set_pipeline(&self.shadow_pipeline);
            shadow_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            shadow_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

            for (mesh_id, _, first, count) in batches.iter() {
                let mesh = &self.meshes[mesh_id.0];
                if mesh.index_count == 0 {
                    continue;
                }
                shadow_pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                shadow_pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                shadow_pass.draw_indexed(0..mesh.index_count, 0, *first..(*first + *count));
            }
        }

        // the world next, depth tested, then the interface painted on top
        {
            let mut mesh_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Mesh Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            // This pass clears the frame, so it runs even with nothing 3D in
            // it. Only the drawing waits on an instance buffer that exists.
            if !batches.is_empty() {
                mesh_pass.set_bind_group(0, &self.camera_bind_group, &[]);
                mesh_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

                // solid first, so it is in the depth buffer before anything is
                // blended over it. Then the far side of everything see-through,
                // then the near side, per spec 0018.
                let runs = [
                    (&self.mesh_pipeline, &opaque),
                    (&self.translucent_pipelines[0], &translucent),
                    (&self.translucent_pipelines[1], &translucent),
                ];

                for (pipeline, batches) in runs {
                    if batches.is_empty() {
                        continue;
                    }
                    mesh_pass.set_pipeline(pipeline);

                    for (mesh_id, texture_id, first, count) in batches.iter() {
                        let mesh = &self.meshes[mesh_id.0];
                        if mesh.index_count == 0 {
                            continue;
                        }
                        mesh_pass.set_bind_group(1, &self.textures[texture_id.0], &[]);
                        mesh_pass.set_bind_group(2, &self.shadow_bind_group, &[]);
                        mesh_pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                        mesh_pass
                            .set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                        mesh_pass.draw_indexed(0..mesh.index_count, 0, *first..(*first + *count));
                    }
                }
            }
        }

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            let num_indices = geometry.index_data().len() as u32;
            if num_indices != 0 {
                render_pass.set_pipeline(&self.pipeline);
                render_pass.set_bind_group(0, &self.screen_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..num_indices, 0, 0..1);
            }

            self.text_brush.draw(&mut render_pass);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        self.queue.present(frame);
    }

    /// Packs every instance into one buffer and says where each mesh's run
    /// starts, so each mesh is one instanced draw. The opaque runs come back
    /// separately from the translucent ones, because they are drawn in
    /// different passes, per spec 0018.
    fn upload_instances(&mut self, scene: &Scene) -> (Vec<Batch>, Vec<Batch>) {
        let mut instances: Vec<Instance> = Vec::new();
        let mut opaque = Vec::new();
        let mut translucent = Vec::new();

        for (mesh, texture, mesh_instances) in scene.batches() {
            if mesh.0 >= self.meshes.len() {
                log::warn!("a scene asked for mesh {:?}, which was never added", mesh);
                continue;
            }
            if texture.0 >= self.textures.len() {
                log::warn!(
                    "a scene asked for texture {:?}, which was never added",
                    texture
                );
                continue;
            }

            // one mesh can be drawn solid here and see-through there, so the
            // split is per instance and each half gets its own run
            for (list, wanted) in [(&mut opaque, false), (&mut translucent, true)] {
                let first = instances.len() as u32;
                instances.extend(
                    mesh_instances
                        .iter()
                        .filter(|instance| instance.is_translucent() == wanted),
                );

                let count = instances.len() as u32 - first;
                if count > 0 {
                    list.push((mesh, texture, first, count));
                }
            }
        }

        if instances.is_empty() {
            return (opaque, translucent);
        }

        let bytes: &[u8] = bytemuck::cast_slice(&instances);
        if bytes.len() as u64 > self.instance_buffer.size() {
            self.instance_buffer =
                create_buffer_init(&self.device, bytes, wgpu::BufferUsages::VERTEX);
        } else {
            self.queue.write_buffer(&self.instance_buffer, 0, bytes);
        }

        (opaque, translucent)
    }

    /// Writes this frame's quads into the vertex and index buffers, growing them when they are too small.
    fn upload_geometry(&mut self, geometry: &Geometry) {
        let vertices: &[u8] = bytemuck::cast_slice(geometry.vertex_data());
        let indices: &[u8] = bytemuck::cast_slice(geometry.index_data());

        if vertices.len() as u64 > self.vertex_buffer.size() {
            self.vertex_buffer =
                create_buffer_init(&self.device, vertices, wgpu::BufferUsages::VERTEX);
        } else if !vertices.is_empty() {
            self.queue.write_buffer(&self.vertex_buffer, 0, vertices);
        }

        if indices.len() as u64 > self.index_buffer.size() {
            self.index_buffer =
                create_buffer_init(&self.device, indices, wgpu::BufferUsages::INDEX);
        } else if !indices.is_empty() {
            self.queue.write_buffer(&self.index_buffer, 0, indices);
        }
    }
}

fn create_buffer(
    device: &wgpu::Device,
    size: wgpu::BufferAddress,
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: usage | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn create_buffer_init(
    device: &wgpu::Device,
    contents: &[u8],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents,
        usage: usage | wgpu::BufferUsages::COPY_DST,
    })
}

/// The camera and the light, as the GPU sees them. Every field is padded out
/// to four floats because that is how a uniform block is laid out.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct SceneUniform {
    view_projection: [f32; 16],
    camera_position: [f32; 4],
    light_direction: [f32; 4],
    /// rgb is the color, a is the intensity.
    light_color: [f32; 4],
    ambient: [f32; 4],
    /// World to the light's clip space, for the shadow map.
    light_view_projection: [f32; 16],
}

unsafe impl bytemuck::Pod for SceneUniform {}
unsafe impl bytemuck::Zeroable for SceneUniform {}

impl SceneUniform {
    fn new(camera: &Camera, light: &Light, bounds: &Aabb) -> Self {
        Self {
            view_projection: camera.view_projection().to_cols_array(),
            camera_position: camera.position.extend(0.0).to_array(),
            light_direction: light.direction.normalize_or_zero().extend(0.0).to_array(),
            light_color: light.color.extend(light.intensity).to_array(),
            ambient: light.ambient.extend(0.0).to_array(),
            light_view_projection: shadow::light_view_projection(light.direction, bounds)
                .to_cols_array(),
        }
    }
}

/// The 3D pipeline: mesh vertices, one instance per copy, the camera uniform,
/// depth testing and back face culling from spec 0009.
/// Depth only, from the light. No fragment stage and no color target.
fn create_shadow_pipeline(
    device: &wgpu::Device,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::include_wgsl!("../../res/shaders/shadow.wgsl"));

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Shadow Pipeline Layout"),
        bind_group_layouts: &[Some(camera_bind_group_layout)],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Shadow Pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[Some(crate::mesh::Vertex::DESC), Some(Instance::DESC)],
            compilation_options: Default::default(),
        },
        fragment: None,
        // front faces are culled instead of back ones, which pushes the
        // recorded depth to the far side of a wall and hides most acne
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Front),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: shadow::FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

fn create_mesh_pipeline(
    device: &wgpu::Device,
    color_format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    texture_bind_group_layout: &wgpu::BindGroupLayout,
    shadow_bind_group_layout: &wgpu::BindGroupLayout,
    primitive: wgpu::PrimitiveState,
    depth_stencil: wgpu::DepthStencilState,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::include_wgsl!("../../res/shaders/mesh.wgsl"));

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Mesh Pipeline Layout"),
        bind_group_layouts: &[
            Some(camera_bind_group_layout),
            Some(texture_bind_group_layout),
            Some(shadow_bind_group_layout),
        ],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Mesh Pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[Some(crate::mesh::Vertex::DESC), Some(Instance::DESC)],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: color_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive,
        depth_stencil: Some(depth_stencil),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

/// A vertex or index buffer for a mesh. Empty contents get four zero bytes
/// instead, because `create_buffer_init` panics on nothing at all and an empty
/// mesh is something a game is allowed to hand over.
fn mesh_buffer(
    device: &wgpu::Device,
    label: &str,
    contents: &[u8],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: if contents.is_empty() {
            &[0; 4]
        } else {
            contents
        },
        usage,
    })
}

/// Quads are drawn in the order they are pushed, with no depth testing, per
/// spec 0001. The 3D pipeline is the one that uses `depth::state()`.
const QUAD_DEPTH_STENCIL: Option<wgpu::DepthStencilState> = None;

/// A uniform buffer binding, which is the shape both the screen size and the
/// scene use. `visibility` matters: the scene uniform carries the light, so the
/// fragment stage reads it too, and a vertex-only layout fails validation.
fn uniform_bind_group_layout(
    device: &wgpu::Device,
    label: &str,
    visibility: wgpu::ShaderStages,
) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(&format!("{} Bind Group Layout", label)),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

/// The screen size uniform the quad shader converts pixel positions with.
fn screen_size(config: &wgpu::SurfaceConfiguration) -> [f32; 4] {
    [config.width as f32, config.height as f32, 0.0, 0.0]
}

fn create_render_pipeline(
    device: &wgpu::Device,
    color_format: wgpu::TextureFormat,
    screen_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::include_wgsl!("../../res/shaders/quad.wgsl"));

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Pipeline Layout"),
        bind_group_layouts: &[Some(screen_bind_group_layout)],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[Some(Vertex::DESC)],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: color_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: QUAD_DEPTH_STENCIL,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

fn text_section(text: &RenderText) -> Section<'_> {
    let layout = Layout::default().h_align(if text.centered {
        HorizontalAlign::Center
    } else {
        HorizontalAlign::Left
    });

    Section::default()
        .with_screen_position((text.position.x, text.position.y))
        .with_bounds((text.bounds.x, text.bounds.y))
        .with_layout(layout)
        .add_text(
            Text::new(&text.text)
                .with_color(text.color)
                .with_scale(if text.focused {
                    text.size + 8.0
                } else {
                    text.size
                }),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_2d_pipeline_ignores_depth() {
        // quads and text keep painting over each other in push order
        assert!(QUAD_DEPTH_STENCIL.is_none());
    }
}

#[cfg(test)]
mod uniform_tests {
    use super::*;

    #[test]
    fn the_scene_uniform_is_laid_out_for_the_gpu() {
        // a uniform block wants each field on a 16 byte boundary
        assert_eq!(std::mem::size_of::<SceneUniform>(), 64 + 16 * 4 + 64);
        assert_eq!(
            std::mem::offset_of!(SceneUniform, light_view_projection),
            128
        );
        assert_eq!(std::mem::offset_of!(SceneUniform, camera_position), 64);
        assert_eq!(std::mem::offset_of!(SceneUniform, light_direction), 80);
        assert_eq!(std::mem::offset_of!(SceneUniform, light_color), 96);
        assert_eq!(std::mem::offset_of!(SceneUniform, ambient), 112);
    }

    #[test]
    fn the_uniform_carries_the_light() {
        let mut light = Light::new();
        light.intensity = 0.5;
        let uniform = SceneUniform::new(&Camera::new(), &light, &shadow::default_bounds());

        assert_eq!(uniform.light_color[3], 0.5);
        // pointing down, as the default light comes from above
        assert!(uniform.light_direction[1] < 0.0);
    }
}

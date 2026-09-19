pub mod render_text;

use crate::geometry::vertex::*;
use crate::geometry::Geometry;
use render_text::*;

use std::sync::Arc;

use wgpu::util::DeviceExt;
use wgpu_text::glyph_brush::ab_glyph::FontRef;
use wgpu_text::glyph_brush::{HorizontalAlign, Layout, Section, Text};
use wgpu_text::{BrushBuilder, TextBrush};
use winit::window::Window;

const FONT_BYTES: &[u8] = include_bytes!("../../res/fonts/PressStart2P-Regular.ttf");

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    text_brush: TextBrush<FontRef<'static>>,
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
        let surface = instance.create_surface(window).unwrap();
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

        let pipeline = create_render_pipeline(&device, config.format);

        let vertex_buffer = create_buffer(&device, 0, wgpu::BufferUsages::VERTEX);
        let index_buffer = create_buffer(&device, 0, wgpu::BufferUsages::INDEX);

        let text_brush = BrushBuilder::using_font_bytes(FONT_BYTES).unwrap().build(
            &device,
            config.width,
            config.height,
            config.format,
        );

        Self {
            surface,
            device,
            queue,
            config,
            pipeline,
            vertex_buffer,
            index_buffer,
            text_brush,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        // A minimized window reports a zero size, which a surface cannot be configured with.
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
        self.text_brush
            .resize_view(self.width(), self.height(), &self.queue);
    }

    pub fn render(&mut self, geometry: &Geometry, text_renderer: &TextRenderer) {
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

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
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

fn create_render_pipeline(
    device: &wgpu::Device,
    color_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::include_wgsl!("../../res/shaders/quad.wgsl"));

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Pipeline Layout"),
        bind_group_layouts: &[],
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
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
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

pub mod geometry;
pub mod keyboard;
pub mod renderer;
pub mod sound;

use geometry::Geometry;
use renderer::render_text::*;
use renderer::*;
use sound::SoundSystem;

use std::sync::Arc;
use std::time::Instant;

use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::PhysicalKey,
    window::{Window, WindowId},
};

pub trait Game {
    fn initialize(
        &mut self,
        geometry: &mut Geometry,
        text_renderer: &mut TextRenderer,
        sound_system: &SoundSystem,
        window_size: (f32, f32),
    );
    /// `dt` is the time since the previous update in seconds, capped at `MAX_DELTA_TIME`.
    fn update(
        &mut self,
        dt: f32,
        geometry: &mut Geometry,
        text_renderer: &mut TextRenderer,
        sound_system: &SoundSystem,
    );
    fn process_keyboard(&mut self, input: keyboard::KeyboardInput);
    fn is_quitting(&self) -> bool;
    fn focus_changed(&mut self, focus: bool);
}

/// Longest frame time handed to `Game::update`. Below 20 fps the game slows down
/// instead of taking one large step.
pub const MAX_DELTA_TIME: f32 = 0.05;

pub fn start(title: &str, game: Box<dyn Game>) {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App {
        title: title.to_string(),
        game,
        running: None,
    };
    event_loop.run_app(&mut app).unwrap();
}

struct App {
    title: String,
    game: Box<dyn Game>,
    running: Option<Running>,
}

/// Everything that needs a window, which winit only hands out once the app has resumed.
struct Running {
    window: Arc<Window>,
    renderer: Renderer,
    geometry: Geometry,
    text_renderer: TextRenderer,
    sound_system: SoundSystem,
    last_update: Instant,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.running.is_some() {
            return;
        }

        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title(&self.title))
                .unwrap(),
        );
        let instance_desc = wgpu::InstanceDescriptor::new_with_display_handle(Box::new(
            event_loop.owned_display_handle(),
        ));
        let renderer = pollster::block_on(Renderer::new(window.clone(), instance_desc));
        let mut geometry = Geometry::new();
        let mut text_renderer = TextRenderer::new();
        let sound_system = SoundSystem::new();

        self.game.initialize(
            &mut geometry,
            &mut text_renderer,
            &sound_system,
            (renderer.width(), renderer.height()),
        );

        self.running = Some(Running {
            window,
            renderer,
            geometry,
            text_renderer,
            sound_system,
            last_update: Instant::now(),
        });
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let running = match self.running.as_mut() {
            Some(running) if running.window.id() == window_id => running,
            _ => return,
        };

        match event {
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - running.last_update)
                    .as_secs_f32()
                    .min(MAX_DELTA_TIME);
                running.last_update = now;

                self.game.update(
                    dt,
                    &mut running.geometry,
                    &mut running.text_renderer,
                    &running.sound_system,
                );
                running
                    .renderer
                    .render(&running.geometry, &running.text_renderer);
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key_code),
                        state,
                        repeat,
                        ..
                    },
                ..
            } => {
                if let Some(key) = keyboard::KeyboardKey::from_key_code(key_code) {
                    let keyboard_input = keyboard::KeyboardInput::new(key, &state, repeat);
                    self.game.process_keyboard(keyboard_input);
                }
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(physical_size) => {
                running.renderer.resize(physical_size);
            }
            WindowEvent::Focused(focused) => {
                self.game.focus_changed(focused);
            }
            _ => {}
        }

        if self.game.is_quitting() {
            event_loop.exit();
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(running) = self.running.as_ref() {
            running.window.request_redraw();
        }
    }
}

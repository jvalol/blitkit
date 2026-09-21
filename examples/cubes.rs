//! Three cubes and a floor, to see that 3D works: depth sorting, back face
//! culling, instancing, and a camera that moves.
//!
//! `cargo run --example cubes`
//!
//! The camera orbits on its own. Left and right turn it, up and down raise and
//! lower it, and escape quits.
//!
//! Drag with the left button to turn, scroll to move closer or further, and
//! press space to lock the cursor so turning never stops at the screen edge.

use blitzkit::camera::Camera;
use blitzkit::geometry::Geometry;
use blitzkit::keyboard::{KeyboardInput, KeyboardKey, KeyboardKeyState};
use blitzkit::mesh::{MeshData, Transform};
use blitzkit::mouse::{MouseButton, MouseInput};
use blitzkit::renderer::render_text::{RenderText, TextRenderer};
use blitzkit::renderer::scene::{MeshId, Scene, TextureId};
use blitzkit::renderer::Renderer;
use blitzkit::sound::SoundSystem;
use blitzkit::texture::TextureData;
use blitzkit::{start, Game};
use glam::{vec3, vec4, Quat, Vec3};

const CHECKER: &[u8] = include_bytes!("../res/textures/checker.png");

struct Cubes {
    cube: Option<MeshId>,
    floor: Option<MeshId>,
    checker: Option<TextureId>,
    /// Seconds since the example started, which drives the spin and the orbit.
    time: f32,
    angle: f32,
    height: f32,
    turning: f32,
    rising: f32,
    /// Set by dragging or by raw motion while the cursor is locked.
    dragging: bool,
    drag_turn: f32,
    drag_rise: f32,
    distance: f32,
    lock_cursor: Option<bool>,
    cursor: glam::Vec2,
    quitting: bool,
}

impl Cubes {
    fn new() -> Self {
        Self {
            cube: None,
            floor: None,
            checker: None,
            time: 0.0,
            angle: 0.0,
            height: 2.0,
            turning: 0.0,
            rising: 0.0,
            dragging: false,
            drag_turn: 0.0,
            drag_rise: 0.0,
            distance: 6.0,
            lock_cursor: None,
            cursor: glam::Vec2::ZERO,
            quitting: false,
        }
    }
}

impl Game for Cubes {
    fn load(&mut self, renderer: &mut Renderer) {
        self.cube = Some(renderer.add_mesh(&MeshData::cube()));
        self.floor = Some(renderer.add_mesh(&MeshData::plane()));
        self.checker = TextureData::from_bytes(CHECKER)
            .map(|data| renderer.add_texture(&data))
            .ok();
    }

    fn initialize(
        &mut self,
        _geometry: &mut Geometry,
        _text_renderer: &mut TextRenderer,
        _sound_system: &SoundSystem,
        _window_size: (f32, f32),
    ) {
    }

    fn update(
        &mut self,
        dt: f32,
        _geometry: &mut Geometry,
        text_renderer: &mut TextRenderer,
        _sound_system: &SoundSystem,
    ) {
        self.time += dt;
        self.angle += self.turning * dt + self.drag_turn;
        self.height = (self.height + self.rising * dt - self.drag_rise).clamp(0.2, 8.0);
        // dragging is a one-off nudge per event, not a rate
        self.drag_turn = 0.0;
        self.drag_rise = 0.0;

        // 2D on top of the world, to see the two passes do not fight
        text_renderer.reset();
        text_renderer.push_render_text(RenderText {
            position: glam::vec2(20.0, 20.0),
            text: String::from("arrows or drag to move, scroll to zoom, space locks the cursor"),
            size: 14.0,
            ..Default::default()
        });
        text_renderer.push_render_text(RenderText {
            position: glam::vec2(20.0, 44.0),
            text: format!("cursor {:.0}, {:.0}", self.cursor.x, self.cursor.y),
            size: 14.0,
            ..Default::default()
        });
    }

    fn draw(&mut self, scene: &mut Scene, camera: &mut Camera) {
        let (cube, floor) = match (self.cube, self.floor) {
            (Some(cube), Some(floor)) => (cube, floor),
            _ => return,
        };

        // a wide floor, textured if the image loaded
        let floor_transform =
            Transform::at(vec3(0.0, -0.5, 0.0)).with_scale(Vec3::splat(20.0));
        match self.checker {
            Some(checker) => scene.push_textured(
                floor,
                checker,
                &floor_transform,
                vec4(0.6, 0.6, 0.7, 1.0),
                8.0,
            ),
            None => scene.push_colored(floor, &floor_transform, vec4(0.15, 0.15, 0.2, 1.0)),
        }

        // one spinning in the middle, wearing the same image, tinted red, so a
        // texture and a tint together are visible
        let spinning =
            Transform::at(Vec3::ZERO).with_rotation(Quat::from_rotation_y(self.time));
        match self.checker {
            Some(checker) => {
                scene.push_textured(cube, checker, &spinning, vec4(0.9, 0.5, 0.5, 1.0), 32.0)
            }
            None => scene.push_colored(cube, &spinning, vec4(0.9, 0.3, 0.3, 1.0)),
        }

        // one near the camera and one far from it, pushed far first, so the
        // near one has to win on depth rather than on draw order
        scene.push_colored(
            cube,
            &Transform::at(vec3(-1.2, 0.0, -4.0)),
            vec4(0.3, 0.5, 0.9, 1.0),
        );
        scene.push_colored(
            cube,
            &Transform::at(vec3(-0.6, 0.0, 2.5)).with_scale(Vec3::splat(0.8)),
            vec4(0.9, 0.8, 0.2, 1.0),
        );

        // a shinier cube than the rest, to make the highlight obvious
        scene.push_material(
            cube,
            &Transform::at(vec3(1.6, 0.0, 0.0)).with_scale(Vec3::splat(0.7)),
            vec4(0.8, 0.8, 0.85, 1.0),
            128.0,
        );

        // the camera orbits whatever the player does, so every side shows
        let orbit = self.angle + self.time * 0.3;
        camera.position = vec3(
            orbit.sin() * self.distance,
            self.height,
            orbit.cos() * self.distance,
        );
        camera.target = Vec3::ZERO;
    }

    fn process_keyboard(&mut self, input: KeyboardInput) {
        let held = input.state == KeyboardKeyState::Pressed;

        match input.key {
            KeyboardKey::Left => self.turning = if held { -1.5 } else { 0.0 },
            KeyboardKey::Right => self.turning = if held { 1.5 } else { 0.0 },
            KeyboardKey::Up => self.rising = if held { 2.0 } else { 0.0 },
            KeyboardKey::Down => self.rising = if held { -2.0 } else { 0.0 },
            KeyboardKey::Escape => self.quitting = held,
            // applied next frame, when the renderer is reachable
            KeyboardKey::Space if held => {
                self.lock_cursor = Some(!matches!(self.lock_cursor, Some(true)));
            }
            _ => (),
        }
    }

    fn before_frame(&mut self, renderer: &mut Renderer) {
        if let Some(wanted) = self.lock_cursor {
            if wanted != renderer.cursor_locked() {
                let locked = renderer.set_cursor_locked(wanted);
                // the platform may refuse, so believe it rather than the ask
                self.lock_cursor = Some(locked);
            }
        }
    }

    fn process_mouse(&mut self, input: MouseInput) {
        if input.button == MouseButton::Left {
            self.dragging = input.is_pressed();
        }
    }

    fn cursor_moved(&mut self, position: glam::Vec2) {
        self.cursor = position;
    }

    fn mouse_motion(&mut self, delta: glam::Vec2) {
        // while dragging, or always once the cursor is locked
        if self.dragging || self.lock_cursor == Some(true) {
            self.drag_turn += delta.x * 0.005;
            self.drag_rise += delta.y * 0.01;
        }
    }

    fn mouse_wheel(&mut self, delta: glam::Vec2) {
        self.distance = (self.distance - delta.y * 0.05).clamp(2.0, 20.0);
    }

    fn is_quitting(&self) -> bool {
        self.quitting
    }

    fn focus_changed(&mut self, _focus: bool) {}
}

fn main() {
    start("blitzkit: cubes", Box::new(Cubes::new()));
}

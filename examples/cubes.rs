//! Three cubes and a floor, to see that 3D works: depth sorting, back face
//! culling, instancing, and a camera that moves.
//!
//! `cargo run --example cubes`
//!
//! The camera orbits on its own. Left and right turn it, up and down raise and
//! lower it, and escape quits.

use blitkit::camera::Camera;
use blitkit::geometry::Geometry;
use blitkit::keyboard::{KeyboardInput, KeyboardKey, KeyboardKeyState};
use blitkit::mesh::{MeshData, Transform};
use blitkit::renderer::render_text::{RenderText, TextRenderer};
use blitkit::renderer::scene::{MeshId, Scene};
use blitkit::renderer::Renderer;
use blitkit::sound::SoundSystem;
use blitkit::{start, Game};
use glam::{vec3, vec4, Quat, Vec3};

struct Cubes {
    cube: Option<MeshId>,
    floor: Option<MeshId>,
    /// Seconds since the example started, which drives the spin and the orbit.
    time: f32,
    angle: f32,
    height: f32,
    turning: f32,
    rising: f32,
    quitting: bool,
}

impl Cubes {
    fn new() -> Self {
        Self {
            cube: None,
            floor: None,
            time: 0.0,
            angle: 0.0,
            height: 2.0,
            turning: 0.0,
            rising: 0.0,
            quitting: false,
        }
    }
}

impl Game for Cubes {
    fn load(&mut self, renderer: &mut Renderer) {
        self.cube = Some(renderer.add_mesh(&MeshData::cube()));
        self.floor = Some(renderer.add_mesh(&MeshData::plane()));
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
        self.angle += self.turning * dt;
        self.height = (self.height + self.rising * dt).clamp(0.2, 8.0);

        // 2D on top of the world, to see the two passes do not fight
        text_renderer.reset();
        text_renderer.push_render_text(RenderText {
            position: glam::vec2(20.0, 20.0),
            text: String::from("arrows move the camera"),
            size: 20.0,
            ..Default::default()
        });
    }

    fn draw(&mut self, scene: &mut Scene, camera: &mut Camera) {
        let (cube, floor) = match (self.cube, self.floor) {
            (Some(cube), Some(floor)) => (cube, floor),
            _ => return,
        };

        // a wide, dim floor to catch the eye at a distance
        scene.push_colored(
            floor,
            &Transform::at(vec3(0.0, -0.5, 0.0)).with_scale(Vec3::splat(20.0)),
            vec4(0.15, 0.15, 0.2, 1.0),
        );

        // one spinning in the middle
        scene.push_colored(
            cube,
            &Transform::at(Vec3::ZERO).with_rotation(Quat::from_rotation_y(self.time)),
            vec4(0.9, 0.3, 0.3, 1.0),
        );

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

        // the camera orbits whatever the player does, so every side shows
        let orbit = self.angle + self.time * 0.3;
        camera.position = vec3(orbit.sin() * 6.0, self.height, orbit.cos() * 6.0);
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
            _ => (),
        }
    }

    fn is_quitting(&self) -> bool {
        self.quitting
    }

    fn focus_changed(&mut self, _focus: bool) {}
}

fn main() {
    start("blitkit: cubes", Box::new(Cubes::new()));
}

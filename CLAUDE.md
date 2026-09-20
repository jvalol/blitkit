# blitkit

A small 2D game engine over wgpu. A game implements the `Game` trait and calls
`start()`, and the engine owns the window, the event loop, rendering, keyboard
input, and sound. `pong` in the sibling directory is the game that exercises it.

## Build and test

Requires Rust 1.87 or newer (wgpu's MSRV).

```
cargo build
cargo test
cargo clippy
cargo fmt
```

## How work happens here

Behavior changes are spec driven:

1. **Write the spec first.** Copy `specs/TEMPLATE.md` to `specs/NNNN-short-name.md`
   and fill it in. Keep it to what the engine does, not how it does it.
2. **Make the acceptance criteria testable.** Each one names the test that proves
   it, or goes under "Verified by hand" when it needs a GPU or a window.
3. **Write the tests, then the code.** `cargo test` passes before a commit.
4. **Update the spec when behavior changes.** A spec that disagrees with the code
   is a bug in the spec.

Bug fixes, refactors, and dependency bumps don't need a new spec. They do need the
existing specs to stay true.

## Layout

- `src/lib.rs` — the `Game` trait, the winit event loop, frame timing.
- `src/renderer/` — wgpu setup, the quad pipeline, text through `wgpu_text`.
- `src/geometry/` — quads the game pushes each frame, and their vertices.
- `src/keyboard.rs` — winit `KeyCode` to the engine's own `KeyboardKey`.
- `src/sound.rs` — rodio playback, silent when no device opens.
- `res/` — the font and the quad shader, both compiled into the binary.
- `specs/` — what the engine promises.

## Conventions

- **Everything is in physical pixels**, origin top-left, y down. Quads, text
  positions, and the size handed to `initialize` and `resized` all use it.
- **Games never see wgpu or winit types.** Input arrives as `KeyboardInput`, sizes
  as `(f32, f32)`.
- **A missing device disables a feature, it doesn't panic.** Sound already works
  this way. Failing to get a GPU adapter is still fatal, since nothing can draw.
- Tests live next to the code in `#[cfg(test)] mod tests`, and none of them may
  need a GPU, a window, or an audio device.

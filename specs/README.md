# Specs

What the engine promises, one file per area. `TEMPLATE.md` is the starting point
for a new one, and `CLAUDE.md` in the repo root describes the flow.

Specs are numbered in the order they were written. The number is an identifier,
not a priority, and it never changes once a spec exists.

| Spec | Covers |
| --- | --- |
| [0001](0001-pixel-coordinates.md) | The coordinate system quads and text share |
| [0002](0002-frame-timing.md) | Frame timing and the delta time cap |
| [0003](0003-keyboard-input.md) | Keyboard events, key repeat, and unmapped keys |
| [0004](0004-sound-output.md) | Sound playback and what happens with no device |
| [0005](0005-window-lifecycle.md) | Window size, focus, resizing, and quitting |
| [0006](0006-quad-color.md) | Per-quad color |
| [0007](0007-math-types.md) | glam, and the coordinate conventions |
| [0008](0008-camera.md) | The camera and its matrices |
| [0009](0009-depth.md) | The depth buffer and back-face culling |
| [0010](0010-meshes.md) | 3D vertices, meshes, transforms, instancing |
| [0011](0011-textures.md) | Textures, samplers and mipmaps |
| [0012](0012-lighting.md) | One directional light, Blinn-Phong shading |
| [0013](0013-mouse-input.md) | Buttons, the cursor, raw motion, the wheel, cursor lock |

`cargo run --example cubes` draws what specs 0007 through 0012 built: lit,
textured, depth sorted, instanced geometry with a camera that moves.

## Drafted, not built

What a 3D game needs that the engine still lacks, in the order it would be done.

| Spec | Covers |
| --- | --- |
| [0014](0014-collision.md) | Boxes, spheres, rays, swept tests, move and slide |
| [0015](0015-shadows.md) | Shadow mapping from the one directional light |

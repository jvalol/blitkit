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

## Drafted, not built

The 3D work, in the order it would be done. Each one is a draft until it ships.

| Spec | Covers |
| --- | --- |
| [0011](0011-textures.md) | Textures and samplers |
| [0012](0012-lighting.md) | One directional light, Blinn-Phong shading |

`cargo run --example cubes` draws what 0007 through 0010 built. The two left
make it look like something.

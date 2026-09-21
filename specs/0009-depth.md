# 0009 Depth

**Status:** draft
**Date:** 2026-09-21

## Goal

Near things cover far things, whatever order they were drawn in.

## Behavior

The renderer keeps a depth texture the size of the surface, in `Depth32Float`.
The 3D pipeline tests and writes depth with `LessEqual`, so a fragment nearer the
camera wins. The depth buffer is cleared to 1, the far plane, at the start of
each frame.

The depth texture is recreated whenever the surface is reconfigured, and is
never left at the old size. A zero-sized window still skips rendering entirely,
per spec 0005.

**Back faces are culled** for 3D geometry: counter-clockwise winding faces the
camera, and the far side of a closed mesh is not drawn.

The 2D pipeline does not use depth. Quads and text draw in the order they are
pushed, as spec 0001 says, and are drawn after the 3D pass so a game's interface
sits on top of its world.

## Acceptance criteria

- The depth texture matches the surface size. — `renderer::tests::depth_texture_matches_the_surface`
- Resizing recreates it. — `renderer::tests::resizing_recreates_the_depth_texture`
- The 3D pipeline is configured to test and write depth. — `renderer::tests::the_3d_pipeline_tests_depth`
- The 2D pipeline is not. — `renderer::tests::the_2d_pipeline_ignores_depth`

### Verified by hand

- A far cube behind a near one stays behind it when pushed later. — run the 3D
  example with two overlapping cubes.
- Interface text stays on top of the world. — same example, with text drawn.

## Out of scope

Stencil buffers, depth pre-passes, transparency sorting, and order independent
transparency. Alpha blended 3D geometry will look wrong until sorted, and this
spec does not solve that.

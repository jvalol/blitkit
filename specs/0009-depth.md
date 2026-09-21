# 0009 Depth

**Status:** implemented
**Date:** 2026-09-21

## Goal

Near things cover far things, whatever order they were drawn in.

## Behavior

The renderer keeps a depth texture the size of the surface, in `Depth32Float`.
The 3D pipeline tests and writes depth with `LessEqual`, so a fragment nearer the
camera wins. The depth buffer is cleared to 1, the far plane, at the start of
each frame.

The texture, the depth state and the culling settings live in
`renderer::depth` as plain values, so they can be checked without a GPU. The
render pass that attaches the texture and clears it arrives with the 3D pipeline
in spec 0010; until then the depth buffer exists and resizes but nothing reads
it.

The depth texture is recreated whenever the surface is reconfigured, and is
never left at the old size. A zero-sized window still skips rendering entirely,
per spec 0005.

**Back faces are culled** for 3D geometry: counter-clockwise winding faces the
camera, and the far side of a closed mesh is not drawn.

The 2D pipeline does not use depth. Quads and text draw in the order they are
pushed, as spec 0001 says, and are drawn after the 3D pass so a game's interface
sits on top of its world.

## Acceptance criteria

- The depth texture matches the surface size. — `renderer::depth::tests::depth_texture_matches_the_surface`
- A different size gives a different texture, and a minimized window still gives a legal one. — `renderer::depth::tests::resizing_recreates_the_depth_texture`
- The 3D pipeline is configured to test and write depth. — `renderer::depth::tests::the_3d_pipeline_tests_depth`
- 3D geometry culls its back faces. — `renderer::depth::tests::the_3d_pipeline_culls_back_faces`
- The 2D pipeline declares no depth state. — `renderer::tests::the_2d_pipeline_ignores_depth`

These check the values handed to wgpu, not what the GPU does with them. That the
texture is actually rebuilt on resize is the renderer calling this code, which
the hand checks below cover once there is 3D geometry to look at.

### Verified by hand

- A far cube behind a near one stays behind it when pushed later. — run the 3D
  example with two overlapping cubes.
- Interface text stays on top of the world. — same example, with text drawn.

## Out of scope

Stencil buffers, depth pre-passes, transparency sorting, and order independent
transparency. Alpha blended 3D geometry will look wrong until sorted, and this
spec does not solve that.

# 0001 Pixel coordinates

**Status:** implemented
**Date:** 2026-09-19

## Goal

One coordinate system for everything a game places on screen, so a game author
never converts between two of them.

## Behavior

Positions and sizes are in physical pixels. The origin is the window's top-left
corner and y points down. This covers quads, text positions, the size passed to
`initialize` and `resized`, and `Renderer::width` and `height`.

A `Quad`'s `position` is its center and its `size` is its full width and height.
`Geometry::push_quad` turns one into four vertices and six indices, two triangles,
with indices offset by the quads already pushed. `Geometry::reset` empties it, and
a game rebuilds its geometry every frame.

The shader converts pixels to clip space with the window size, which the renderer
updates whenever the surface is reconfigured. A quad is drawn at the same pixel
position whatever the window's shape, so a square quad stays square.

Physical pixels are not logical points. On a 2x display a 32 pixel quad is half the
size it would be on a 1x display, and text sizes behave the same way. A game that
wants to scale with the display does it from the size given to `resized`.

## Acceptance criteria

- A quad becomes four vertices at its corners. — `geometry::tests::push_quad_makes_corner_vertices`
- A quad becomes six indices forming two triangles. — `geometry::tests::push_quad_makes_two_triangles`
- A second quad's indices are offset by four. — `geometry::tests::second_quad_indices_are_offset`
- `reset` clears vertices, indices, and the quad count. — `geometry::tests::reset_clears_geometry`

### Verified by hand

- A square quad stays square in a wide or tall window. — run snake and resize it.
- Quads and text line up in the same space. — run pong, where the win text is
  centered with the same numbers the quads use.

## Out of scope

Rotation, scaling, and textures. Color is spec 0006.

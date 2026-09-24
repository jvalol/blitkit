# 0018 Translucency

**Status:** implemented
**Date:** 2026-09-24

## Goal

Seeing through a thing, so that what is inside it shows.

## Behavior

A game asks for translucency by dimming the alpha of an instance's color and
nothing else. Any alpha short of one is translucent; a full alpha is solid. The
same mesh can be solid in one instance and see-through in the next, because the
decision is per instance rather than per mesh.

**Solid is drawn first**, all of it, so it is in the depth buffer before
anything is blended over it. Translucent geometry is drawn after, and is tested
against that depth, so a wall in front of a pane of glass still hides it.

**Translucent geometry does not write depth.** If it did, the near wall of a
shape would land in the depth buffer first and then hide the far wall, which is
exactly the thing you wanted to see through it.

**It is drawn twice**, because blending takes its inputs in the order they
arrive and nothing here sorts triangles. The first pass culls front faces, which
leaves the inside of a shape; the second culls back faces, which leaves the
outside. Far before near is what blending wants. For a shape that is roughly
convex this is right; for a spout that loops back over a handle it is an
approximation, and the wrong one shows as the further part looking too strong.

**The light sees it as solid.** A shadow map holds a depth and has nowhere to
put an alpha, so a pane of glass casts the same shadow a wall would. That is
wrong and it is deliberate: the alternative is a second shadow map, which is
more than a small engine needs.

## Acceptance criteria

- An instance is translucent when its alpha is short of one. — `renderer::scene::tests::an_instance_is_translucent_when_its_alpha_is_short`
- Translucent geometry tests depth but does not write it. — `renderer::depth::tests::translucent_geometry_tests_depth_without_writing_it`
- It is drawn from both sides, far before near. — `renderer::depth::tests::translucent_geometry_is_drawn_from_both_sides`

### Verified by hand

- `cargo run --example teapot`, then T: the far wall of the pot, its handle and
  the inside of its spout show through the near wall, and the floor shows
  through all of it.
- `cargo run --example klein`, then T: the neck is visible inside the body.
- A solid thing drawn in front of a translucent one still hides it.

## Out of scope

Sorting translucent triangles back to front, order independent transparency,
translucent shadows, and refraction.

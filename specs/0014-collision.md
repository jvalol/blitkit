# 0014 Collision

**Status:** implemented
**Date:** 2026-09-21

## Goal

Things can touch, block and be pointed at, without the engine becoming a physics
engine.

## Behavior

Three shapes, all in world space: `Aabb` (a box aligned to the axes, held as a
min and a max), `Sphere` (a center and a radius), and `Ray` (an origin and a
direction).

**Tests between them** answer whether two shapes overlap: box against box, sphere
against sphere, sphere against box, and whether a point is inside either.

**Ray casts** return how far along the ray the hit is, and the surface normal
there. A ray against a box, and a ray against a sphere. This is also how a game
turns a cursor position into the thing under it, per spec 0013.

**Swept tests** move a sphere along a line and report the first box it touches,
so something fast cannot pass through a wall between frames. A test that only
looks at where things ended up misses that.

The sweep grows the box by the sphere's radius and casts a ray at it, which
treats the sphere as square at the corners. A ball clipping the very corner of a
box stops a little early, which is the forgiving direction.

**Move and slide** is the one piece of movement the engine provides: given a
sphere, a velocity, a time step and a set of boxes, it returns where the sphere
ends up, sliding along surfaces rather than stopping dead at them. It repeats up
to four times per step, so a corner slides into a stop rather than jittering.

**`MeshData::bounds`** gives a mesh's `Aabb`, so a game can build colliders from
what it draws instead of typing numbers twice.

Everything here is pure maths on the CPU, with no GPU and no allocation in the
hot path.

## Acceptance criteria

- Boxes overlap, or touch exactly, or miss. — `collision::tests::boxes_overlap_or_miss`
- Spheres overlap or miss. — `collision::tests::spheres_overlap_or_miss`
- A sphere against a box catches face, edge and corner contact. — `collision::tests::a_sphere_meets_a_box`
- A point is inside a shape or outside it. — `collision::tests::points_are_inside_or_outside`
- A ray hits a box and reports the distance and the face's normal. — `collision::tests::a_ray_hits_a_box`
- A ray pointing away from a box misses. — `collision::tests::a_ray_pointing_away_misses`
- A ray hits a sphere at the near surface. — `collision::tests::a_ray_hits_a_sphere`
- A fast sphere is caught by the wall it would have passed through. — `collision::tests::a_swept_sphere_does_not_tunnel`
- A sphere driven into a wall slides along it rather than stopping. — `collision::tests::move_and_slide_slides`
- A sphere driven into a corner stops without jitter. — `collision::tests::move_and_slide_settles_in_a_corner`
- Movement through open space is unchanged. — `collision::tests::move_and_slide_leaves_open_space_alone`
- A mesh reports its bounds. — `mesh::tests::a_mesh_knows_its_bounds`

### Verified by hand

Run `cargo run --example rolling` in blitkit, which draws every collider exactly
where it collides, so a mismatch between what is seen and what is hit shows up.

- A ball driven straight into a wall stops against it, without sticking,
  shuddering or passing through.
- Driven diagonally, it slides along rather than stopping dead.
- Wedged into a corner, it settles quietly instead of vibrating.

## Out of scope

Rigid bodies, mass, restitution, friction, joints, rotation of colliders,
triangle-accurate collision against meshes, and any broad phase. A game with
thousands of colliders needs one, and this is not it.

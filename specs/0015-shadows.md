# 0015 Shadows

**Status:** implemented
**Date:** 2026-09-21

## Goal

You can tell where something is. A cube hovering and a cube resting look the
same without a shadow, which is the depth cue spec 0012's lighting cannot give.

## Behavior

The scene's one directional light casts shadows by shadow mapping. Each frame
the renderer draws the depth of every mesh from the light's point of view into a
depth texture, then the mesh shader compares each fragment against it: nearer to
the light than what the map recorded means lit, further means in shadow.

**The light's view** is orthographic, because a directional light has no
position. It is fitted to a scene bounds, an `Aabb` a game sets through the
renderer, defaulting to 40 units around the origin. The map covers that box and
nothing else, so a game with a larger world raises it and accepts softer
shadows, or lowers it for sharper ones.

**Outside the map is lit, not dark.** A fragment beyond the bounds, or behind
the light's near plane, gets no shadow rather than a black smear. Wrong in the
forgiving direction.

**Softening.** The map is sampled nine times in a small square and averaged, so
edges are a few pixels soft rather than stair-stepped.

**Bias.** A surface facing the light nearly edge-on records depths that its own
fragments fail against, which stripes it with false shadow. The comparison adds
a small offset that grows with that angle: 0.0005 square-on, rising to 0.0045.
Too little gives those stripes, too much lifts a shadow away from the thing
casting it. Both are visible, and the hand checks below are how they get caught.

The shadow pass also culls front faces rather than back ones, which pushes the
recorded depth to the far side of each wall and hides most acne before the bias
has to deal with it. That is why these numbers can stay small.

Everything drawn in 3D casts and receives. Quads and text do neither: they are a
separate pipeline drawn afterwards, per spec 0009.

## Acceptance criteria

- The light's matrix puts the whole scene bounds inside clip space. — `shadow::tests::the_bounds_fit_in_the_light_view`
- Depth in light space runs 0 to 1, matching wgpu. — `shadow::tests::light_space_depth_matches_wgpu`
- A point nearer the light than the recorded depth is lit. — `shadow::tests::nearer_than_the_map_is_lit`
- A point further away is in shadow. — `shadow::tests::further_than_the_map_is_shadowed`
- A point outside the map is lit rather than dark. — `shadow::tests::outside_the_map_is_lit`
- The bias grows as a surface turns edge-on to the light. — `shadow::tests::the_bias_grows_with_the_angle`
- Moving the light moves its matrix. — `shadow::tests::moving_the_light_moves_its_view`

The comparison itself runs in the shader. These test the same rules in Rust, the
way spec 0012's lighting is tested, so the two copies must be kept in step by
hand.

### Verified by hand

Run `cargo run --example rolling` or `cargo run --example cubes` in blitkit.
Rolling is the better test of a shadow staying under a moving thing; cubes is
the better test of resting against floating, since its cubes hover.

- Each cube casts a shadow on the floor, and the spinning one's shadow turns
  with it.
- A cube resting on the floor has its shadow touching it; a raised one has a
  shadow separated from it, which is the cue this spec exists for.
- The shadows move when the light direction changes.
- No stripes across lit surfaces, which would be too little bias, and no shadow
  detached from what casts it, which would be too much.

## Out of scope

Cascades, shadows from more than one light, point and spot light shadows,
contact hardening, and any transparency in the shadow pass. A game that needs a
world larger than one map's bounds needs cascades, and this is not them.

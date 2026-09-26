# 0021 Spot lights

**Status:** implemented
**Date:** 2026-09-26

## Goal

A light that can be moved and still casts a shadow, so a scene lit by
something other than the sun still has something underneath it.

## Behavior

Spec 0020 gave lamps a place and a reach, and said plainly they cast nothing.
With the sun switched off that is what it looks like: objects standing in
coloured light with nothing beneath them, which reads as broken rather than as
a limitation.

The reason is shape. The sun shines one way, so one flat projection covers the
whole scene and one depth texture holds it, which is what spec 0015 built. A
point light shines every way, and covering that takes six projections and a cube
map. Spec 0022 is that. This one takes the cheaper road.

**A spot light is a cone.** It has a place, a direction, a color, an intensity
and a range like a lamp, and two angles: an inner one it fills, and an outer one
it fades to nothing by. A cone has a direction, so it needs exactly one
projection and one depth map, which is the machinery already here.

**The edge is soft.** Between the inner and outer angle the light falls off
smoothly rather than stepping, because a hard circle of light on a floor is the
first thing that gives away a cheap spot light. Inside the inner angle it is at
full strength; outside the outer angle it is nothing at all.

**Distance falls off the same way lamps do**, `(1 - d/range)²`, for the reason
spec 0020 gives. A spot is a lamp with a direction, not a different kind of
thing.

**Up to four, and every one casts.** Each costs a full pass over the scene to
fill its map, which is why the cap is low and nowhere near the eight that lamps
get. The maps are a texture array so there is one binding rather than four, and
they are 1024 to the side rather than the sun's 2048, because a cone covers less
ground than a sunrise does.

**The sun keeps its own map.** It has a different size and a different kind of
projection, and mixing them would make both worse.

**Shadow acne is handled the way spec 0015 handles it**: sloped bias, and the
front faces culled while filling the map so the bias has a back face to hide
behind.

**Every layer is cleared each frame**, in use or not, so a spot switched off
cannot leave last frame's shadow sitting on its layer waiting to be sampled.

**Which light a shadow pass belongs to travels in its own small bind group**,
one per pass. The obvious way to pass one number is immediate data, which is
what push constants became, and that is a device capability the engine would
then require on every backend forever. A bind group holding a single integer is
uglier and asks nothing of the hardware.

**A spot outside its own map is lit, not dark.** Same as the sun: being wrong in
the forgiving direction leaves a scene looking flat rather than leaving a black
rectangle in the middle of it.

## Acceptance criteria

- A spot at full strength down the middle of its cone. — `lighting::tests::a_spot_is_full_strength_down_its_middle`
- Nothing outside the outer angle. — `lighting::tests::a_spot_stops_at_its_outer_angle`
- Smooth between the two angles, never stepping. — `lighting::tests::a_spot_edge_is_soft`
- An inner angle wider than the outer does not invert the cone. — `lighting::tests::a_spot_with_its_angles_backwards_still_behaves`
- Distance fades it the way a lamp fades. — `lighting::tests::a_spot_fades_with_distance_like_a_lamp`
- A surface facing away is not lit. — `lighting::tests::a_spot_does_not_light_the_back_of_a_surface`
- Its color and intensity reach the surface. — `lighting::tests::a_spot_carries_its_color`
- The projection covers the cone and no more. — `shadow::tests::a_spot_projection_covers_its_cone`
- The projection ends at the range, so nothing past it shadows. — `shadow::tests::a_spot_projection_ends_at_its_range`
- A spot pointed straight down still has an up to build its view from. — `shadow::tests::a_spot_pointed_straight_down_still_has_an_up`
- A spot aimed nowhere falls back rather than producing nonsense. — `shadow::tests::a_spot_aimed_nowhere_falls_back_rather_than_producing_nonsense`
- A spot aimed nowhere lights nothing. — `lighting::tests::a_spot_aimed_nowhere_lights_nothing`
- Spots are pushed per frame and cleared with the scene. — `renderer::scene::tests::spot_lights_are_cleared_with_the_scene`
- The fifth spot is dropped, not the first. — `renderer::scene::tests::only_the_first_four_spots_are_kept`
- Lamps and spots are counted apart. — `renderer::scene::tests::lamps_and_spots_are_counted_apart`
- The uniform matches the shader's block, field by field. — `renderer::uniform_tests::the_scene_uniform_is_laid_out_for_the_gpu`
- A spot is four vec4s and a matrix, so an array of them is not padded apart. — `renderer::uniform_tests::a_spot_is_laid_out_for_an_array`
- The spots a scene gives reach the uniform, and the rest stay dark. — `renderer::uniform_tests::the_uniform_carries_the_spots_it_is_given`

### Verified by hand

- `cargo run --release --example cubes`, press L until the sun is off. The
  lamps' shadows sweep round the floor as they orbit, and they are coloured by
  the light that cast them. This is the thing spec 0020 could not do.
- Two spots overlapping put two shadows under one cube, each the colour of the
  other light.
- The edge of a cone on the floor is soft, not a cut circle.

## Out of scope

Shadows from point lights, which is spec 0022. More than four spots. Light that
bounces, and any cone that is not round.

# 0012 Lighting

**Status:** draft
**Date:** 2026-09-21

## Goal

Shape you can see. An unlit cube is a hexagon.

## Behavior

The scene has one directional light: a direction, a color, and an intensity,
plus an ambient color that fills the shadowed side so it isn't black. Both go to
the GPU in the same uniform as the camera, once per frame.

Shading is Blinn-Phong: ambient, plus diffuse from the angle between the normal
and the light, plus a specular highlight from the half vector between the light
and the view. Material shininess is a number on the mesh.

Lighting happens in world space, using the normal matrix from spec 0010.

**Defaults** so a scene is lit without being told: a white light from above and
slightly to one side, at full intensity, with dim grey ambient.

Quads and text are unlit and unaffected. They are a separate pipeline.

## Acceptance criteria

- A surface facing the light is brighter than one facing away. — `lighting::tests::facing_the_light_is_brighter`
- A surface facing away gets ambient only. — `lighting::tests::the_dark_side_is_ambient`
- Intensity scales the diffuse contribution. — `lighting::tests::intensity_scales_the_light`
- The default light comes from above. — `lighting::tests::the_default_light_is_overhead`

The shading itself runs on the GPU, so these test the same math in Rust rather
than the shader. That gap is the point of the hand checks below.

### Verified by hand

- A lit cube reads as a cube, with visibly different faces. — run the 3D example.
- A specular highlight moves when the camera orbits. — same example.

## Out of scope

Point and spot lights, several lights at once, shadows, physically based
rendering, and ambient occlusion.

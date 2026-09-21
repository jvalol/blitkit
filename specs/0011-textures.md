# 0011 Textures

**Status:** implemented
**Date:** 2026-09-21

## Goal

Surfaces that look like something, rather than flat color.

## Behavior

A `Texture` is an image on the GPU with a sampler and a bind group, built from
PNG or JPEG bytes through the `image` crate. A game loads one and hands it to
the meshes that use it.

Images are uploaded as `Rgba8UnormSrgb`, so color values are treated as sRGB and
match the surface format the renderer already picks.

Sampling is linear with repeating wrap, and mipmaps are generated when a texture
is built. Without them, a textured floor seen at an angle shimmers. The chain is
built on the CPU by box filter, halving until both sides are one pixel, which
keeps it testable and avoids a second pipeline just to downsample.

A mesh drawn without a texture uses a one pixel white texture, `TextureId::WHITE`,
so there is one pipeline and one shader rather than two of each.

A draw call wears one texture, so the scene batches by mesh and texture together:
the same mesh in two textures is two draws.

**Color still applies.** A mesh has a tint that multiplies the sampled color,
which is how the engine's existing solid-color drawing survives into 3D.

## Acceptance criteria

- A texture is built from PNG bytes at the right size. — `texture::tests::loads_a_png`
- A texture is built from JPEG bytes. — `texture::tests::loads_a_jpeg`
- Images are uploaded as sRGB. — `texture::tests::uploads_as_srgb`
- Mipmaps are generated down to one pixel. — `texture::tests::builds_a_full_mip_chain`
- An untextured mesh gets the white fallback. — `texture::tests::untextured_meshes_get_white`
- Broken image bytes are an error, not a panic. — `texture::tests::rejects_bad_bytes`
- A non-square image halves until both sides are one. — `texture::tests::a_wide_image_keeps_halving_until_both_sides_are_one`
- One mesh in two textures is two batches. — `renderer::scene::tests::one_mesh_in_two_textures_is_two_batches`

These cover decoding and the mip chain, which happen on the CPU. How the GPU
samples is the hand check below.

### Verified by hand

Run `cargo run --example cubes` in blitkit.

- The floor's checkerboard shows its red corner patch once, the right way up,
  which would look wrong if the texture were mirrored or rotated.
- The spinning cube wears the same image tinted red, so the color multiplies
  what is sampled rather than replacing it.
- Distant floor squares do not shimmer as the camera orbits, which is the mip
  chain doing its job.

## Out of scope

Cube maps, normal and roughness maps, texture atlases, compressed formats, and
streaming.

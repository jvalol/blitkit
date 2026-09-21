# 0011 Textures

**Status:** draft
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
is built. Without them, a textured floor seen at an angle shimmers.

A mesh drawn without a texture uses a one pixel white texture, so there is one
pipeline and one shader rather than two of each.

**Color still applies.** A mesh has a tint that multiplies the sampled color,
which is how the engine's existing solid-color drawing survives into 3D.

## Acceptance criteria

- A texture is built from PNG bytes at the right size. — `texture::tests::loads_a_png`
- A texture is built from JPEG bytes. — `texture::tests::loads_a_jpeg`
- Images are uploaded as sRGB. — `texture::tests::uploads_as_srgb`
- Mipmaps are generated down to one pixel. — `texture::tests::builds_a_full_mip_chain`
- An untextured mesh gets the white fallback. — `texture::tests::untextured_meshes_get_white`
- Broken image bytes are an error, not a panic. — `texture::tests::rejects_bad_bytes`

### Verified by hand

- A textured cube shows its image the right way up, not mirrored. — run the 3D
  example with a lettered texture.

## Out of scope

Cube maps, normal and roughness maps, texture atlases, compressed formats, and
streaming.

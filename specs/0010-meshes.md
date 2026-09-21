# 0010 Meshes

**Status:** draft
**Date:** 2026-09-21

## Goal

Something to draw in 3D, placed where the game wants it.

## Behavior

A 3D vertex is a position, a normal, and a texture coordinate: `Vec3`, `Vec3`,
`Vec2`. That is a separate type from the 2D vertex, which stays as it is.

A `Mesh` owns a vertex buffer and an index buffer on the GPU, built once from
vertex and index data rather than rebuilt every frame the way quads are. Meshes
are long lived: a game builds one and draws it many times.

The engine ships a cube and a plane, so a game can see something without
supplying geometry.

**Drawing.** A game pushes a mesh and a transform for each thing it wants drawn
this frame, the same shape as `push_quad`. The transform is a `Mat4`, and a
`Transform` helper builds one from position, rotation and scale. Drawing the
same mesh many times is instanced, one draw call per mesh with the transforms in
an instance buffer.

Normals are transformed by the inverse transpose of the model matrix, so
non-uniform scaling does not bend the lighting.

**Loading from a file** is `Mesh::from_obj`, using `tobj`. Wavefront OBJ is
enough to get real models on screen, and it is one dependency rather than the
several glTF wants.

## Acceptance criteria

- A 3D vertex carries a position, a normal and a texture coordinate. — `mesh::tests::vertex_has_position_normal_and_uv`
- The built-in cube has 8 corners, 12 triangles, and normals per face. — `mesh::tests::the_cube_is_a_cube`
- A transform built from position, rotation and scale applies them in that order. — `mesh::tests::transform_applies_scale_rotation_then_position`
- The normal matrix is the inverse transpose of the model matrix. — `mesh::tests::normal_matrix_survives_non_uniform_scale`
- Pushing the same mesh twice makes one draw call with two instances. — `renderer::tests::repeated_meshes_are_instanced`
- An OBJ file loads into vertices and indices. — `mesh::tests::loads_an_obj_file`

### Verified by hand

- The cube looks like a cube from every angle. — run the 3D example and orbit it.

## Out of scope

Skeletal animation, glTF, materials per submesh, level of detail, and any scene
graph. A game keeps its own list of what to draw.

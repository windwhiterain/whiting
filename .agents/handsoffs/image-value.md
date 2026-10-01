# Handsoff: an image as a value a scene owns

Status: **implementation complete**, awaiting merge. Branch `image-value`, worktree
`.worktrees/image-value`, three commits on top of `3a6020a`.

Read [proposals/image-as-a-value.md](../proposals/image-as-a-value.md) for the design. This file
records what was built, what the build changed about the design, and what is still open.

## What landed

| commit | what |
| --- | --- |
| `1bbe290` | the sort: `graphics::image` as a schema marker, the `images` list, and the emitter lifting a named image into a texture |
| `a29ff43` | four tests proving the emitted pair, written before the gallery scene |
| `3836b26` | the `tiles` gallery scene, a render test on a real adapter, and the kit builder |

## What the build changed about the design

The proposal said the image lives in `custom_data`. It does, and it cannot **stay** there: custom
data is packed into the instance row as vertex attributes, so every field of it has to flatten
into four byte numbers and a name does not. So the compiler sorts by kind — the numbers are
packed, the image is lifted — and the kind comes from the shader's `custom_schema` rather than
from the value, which is what lets it decide before it reads anything an author wrote.

Two things the design did not predict, both found by the checks rather than by review:

1. **The binding point's number depends on the name.** A pass binds its resources in name order,
   so an image field called `image` lands *between* `depth_buffer` and `shadow_map`, and every
   number after it shifts. The first attempt declared the shader's bindings as though the image
   came last, and `check_bindings` refused it with
   `declares `image` at binding 3 as a texture, and this pass binds a sampler there.`
2. **The image binder has to be optional in the interface.** A material need not name an image,
   so a required binder makes every material of that shader carry one. That showed up as
   `at binders.image: field `image` is required` on the *shadow* pass, whose material names none.

## The scene that shows it

`tiles` is a floor whose surface detail comes from a sampled image. It generates its own pixels,
because nothing in this framework reads the filesystem: a shader carries its own source and an
image carries its own pixels, so a texture is something a scene compiler produces.

**The picture is nearly flat, and that is correct rather than a bug.** The floor is drawn at a low
angle, so the texture is minified past a texel per pixel and the level chain averages the pattern
away. Turning the camera up does not help; the honest fix is a smaller floor or a bigger pattern,
and neither is worth doing to a scene whose job is to prove the channel.

## What the tests hold

- `crates/scene/tests/image_channel.rs` (4): the name is not in the row and the numbers beside it
  still are; two materials naming one image upload it once; the pass binds the field's own name;
  a name no image carries is an error naming the material.
- `crates/device/tests/gallery_render.rs` (2): the tiled floor draws **more than one colour**,
  which is the only thing that can tell a sampled image from a declared one — a texture that never
  arrived still produces *a* picture. And every gallery scene still renders.
- All five scenes that existed before this render **byte for byte** as they did.

## What is still open

- **A screen material cannot name an image.** The same mechanism covers it and the post stage's
  binder set would grow the same way. Not built.
- **An image on a material that no stage draws** costs nothing, which is the behaviour of an
  unused mesh, and is not checked for.
- **The `tiles` frame is a poor advertisement.** See above: the sampling is proven by the test and
  not by the picture.

## Before merging

`main` is `3a6020a` plus two commits made while this branch was open (`30f257e` ignoring
`.worktrees/`, `692e273` adding the proposal). The branch touches `crates/graphics`,
`crates/scene` and `crates/device`, and does not overlap those two, so the merge is a
fast-forward or a clean merge either way. The proposal's status line and the align note still
have to be updated **on main**, since the proposal was committed there after this branch started.

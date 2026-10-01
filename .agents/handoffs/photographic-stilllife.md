# Still life by window light: a photograph, not a diagram

Worktree: `./.worktrees/stilllife`, branch `feature/stilllife`, on top of `main` at `b998f89`.

The deliverable is a gallery scene — `stilllife` — built to be looked at as a photograph rather
than as a proof that the pipeline runs, plus the record of what the framework could not say
while building it:
[proposals/photographic-stilllife.md](../proposals/photographic-stilllife.md).

```text
cargo run --release -p whiting-device --features wgpu --example render -- stilllife out/stilllife
```

**Build note.** This worktree shares `main`'s target directory, so the workspace crates
recompile in about twenty seconds and the wgpu dependency tree is not rebuilt:

```text
$env:CARGO_TARGET_DIR="C:\resource\whiting\target"
```

## Status

- [x] the lathe, and `light_with_radius`
- [x] the scene: window light as nine scattered point lights, six surface bodies, one post pass
      doing the whole camera
- [x] iterated to a picture: eighteen renders, each one change
- [x] the proposal, with every claim measured
- [x] the pass-count test entry the scene's arrival requires
- [x] `whiting-scene` clustered_lights and wgsl_validation, and `gallery_render` on the adapter
- [x] merged to `main`, worktree and branch deleted

## What was found, in one line each

1. **The lighting stage does not test depth and declares that it does.** An occluded surface is
   *added* to what is in front of it; two jugs 0.3 m apart on one sight line took the near one's
   belly from 66 to 201. The emitter declares `{write: false, compare: LessEqual}` on a pass
   whose only target is the colour target, and the device builds a depth-stencil state only
   from a depth *attachment*. Recorded, not built, by ruling.
2. **The cluster grid loses its lights on a wide depth range**, and the frame goes blocky at a
   cluster's size. A cluster of the last slice is the slab at the far plane; a light standing
   over the table is further from it than its own radius. The scene's workaround is tight depth
   planes.
3. **There is no environment**, and a still life is mostly what things reflect of one another.
   The scene writes the room as a function, and the reflection's radiance and the lights'
   irradiance share no scale.
4. **The falloff is a constant in absolute units**, so the scene is a hundred units across to
   have any gradient at all.
5. **No area light**: a window is nine point lights, which is nine steps in every penumbra, nine
   dim delta lobes in every highlight, and 88 of the frame's 105 passes.
6. **The ambient is a fixed hemisphere of sky.** Every surface body subtracts it and adds the
   room's own light, which is a scene reaching around the model.

## The picture, honestly

It reads as a still life: the light comes from off-frame left, the jug models and the bowl does
not, the wall's gradient does the compositional work, the near cloth is out of focus, and there
is grain. It is not a photograph, and the tells are all the same one: **nothing in the frame
has any texture that is not arithmetic** — the ceramics are uniformly smooth, the cloth's weave
is a regular mesh, and the wood's figure is a colour. Findings 1 and 3 are why the ceramics are
flat and why nothing has wear on it.

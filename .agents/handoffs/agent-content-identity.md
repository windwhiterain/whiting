# Content identity: pixel → authored path

Implementation handoff for [proposals/agent-content-identity.md](../proposals/agent-content-identity.md).
Read the proposal for the design; this file is for a session picking the work up.

## Where the work is

Branch `feature/agent-content-identity`, worktree `.worktrees/content-identity`. Two commits:

| commit | what |
| --- | --- |
| `a909cb5` | the measurement, red: `r32float` does not build a pipeline today |
| `afcc701` | the fix: an attachment's blend follows its own format |

**Nothing else has been started.** In particular the scene-declared `outputs` field does not exist
yet — the design in the proposal is not code.

## What is settled, so it is not re-argued

- **The id is allocated by the content, globally, with `0` reserved** for "not an authored
  element". It is not a face ordinal and not a per-mesh index. Nothing in the mountain crate
  changes its indexing.
- **`belt_id` is not in the image.** It is hop 3, a lookup into `FaceAttributes` the crate already
  holds. The image carries only what cannot be recovered elsewhere: `id`, `u`, `v`.
- **Three `f32` outputs, not one `vec4`.** An `rgba16float` is exact only to 2048; a `r32float` is
  exact to 2^24 and is the only way an id above 2048 survives. Measured: 4093 comes back 4093.
- **`(u, v)` is `FacetMesh::belt_uv`**, defined once, and a scene writes what it returns. `s` runs
  along a belt's top path and `t` crosses the strip. See the expander in `expand.rs` for what a
  belt is.
- **Every lighting stage writes the output**, and a stage that draws something not authored writes
  `0`. `check_outputs` and `render_outputs` already enforce both halves.
- **`mountain` is not asked to know about paths.** `Massif` has no naming and no grouping, and
  SPEC/mountain.md is human-editable only.
- **`face_index` is never a handle** and does not need to be in the image: `@builtin(vertex_index)`
  is free and `check_vertex_inputs` does not check builtins.

## Landed and verified

The blend change. `blend: Some(blend_of(state.blend))` became `blend: blend_for(format, blend)`,
where `blend_for` returns `None` for the three float32 formats.

Two findings that are not in the code's comments in a form that explains them:

- the backend tests `cs.blend.is_some()`, **not** whether the factors discard the destination — so
  a `Replace` state on `r32float` is refused exactly as `Alpha` is. This cost one red run;
- the three 32 bit float formats are the entire unblendable set, because the device requests
  `Features::empty()` and `float32-blendable` is therefore off. Same reason a float texture cannot
  be filtered, which the workspace already relies on.

Verified on the adapter: `whiting-device --features wgpu` green (50 tests), and SHA-256 of `dunes`,
`landscape`, `tidewater-camera0`, `tidewater-camera1` identical against the same four rendered
with `raster.rs` reverted. The revert was checked to have actually taken effect, because an
inconclusive before/after is worse than none.

## The next step, and its shape

**The scene-declared `outputs` field.** The rest cannot be built without it: a scene that cannot
declare an output has nowhere to write an id, so the content side has nothing to aim at.

Shape, agreed:

```ron
outputs: {
    authored_id: { location: 1, type: f32 },
    authored_u:   { location: 2, type: f32 },
    authored_v:   { location: 3, type: f32 },
}
```

- **The scene names a `location` and a `type`, never a `point`.** The compiler generates
  `output_<name>`, builds the texture from `TexelType::format`, sizes it to the frame, and appends
  it to `targets` of every lighting pass. A scene naming a binding point could name one that does
  not exist.
- **`StandardScene` currently writes the whole `outputs` map itself** —
  [`debug_outputs`](../../../crates/scene/src/standard/emit.rs#L3424-L3444) builds it from
  `present` plus the built-in channels, and `scene_schema.rs` has no `outputs` const among its 53.
  That function is where the scene's own declarations get merged in.
- **`add_targets`** ([emit.rs:3158](../../../crates/scene/src/standard/emit.rs#L3158)) is where the
  instance entries go.
- **The harness is already done.** `render.rs:724` calls
  `scene::declared_description(name, ty, location)` for any output that is not a built-in, and
  `marks_alpha` keeps a scene-declared output's padding from being read as a marker. Neither needs
  changing.

Two things to be careful about, both easy to get backwards:

- **The frame's `@location(0)` is taken.** Lighting groups write the colour target at 0, so the
  first declared output is location 1. `check_outputs` cross-checks a declaration's `location`
  against the pass's attachment order, so a wrong number is an error — but only once the point is
  actually in `targets`.
- **Adding a target to every lighting pass changes every lighting shader's fragment return.** A
  scene that declares the outputs and does not write them gets refused by name, which is the
  intent — but the gallery will need its lighting shaders updated, and that is a real edit, not a
  no-op.

## Files this work will own

- `crates/scene/src/standard/scene_schema.rs` — the `outputs` field
- `crates/scene/src/standard/emit.rs` — `debug_outputs`, `add_targets`, the lighting pass targets
- `crates/scene/src/standard/mod.rs` if a declaration type is wanted
- `crates/mountain/src/facet.rs` — `belt_uv`
- `crates/gallery/src/landscape.rs` and its lighting shader bodies — the content side
- `.agents/SPEC.align.md` — the scene-declared output shape
- a new `crates/gallery/tests/` file for `locate`, or an example beside `census.rs`

**Check `git status` before widening this set.** `render.rs` (the example) and `SPEC.align.md` are
where other work lands too, and the gbuffer handoff records a partial hand-apply into `main` that
broke the build by exactly this route.

## Commands

The machine has twenty cores and cargo defaults to all of them, so **every cargo command here needs
`-j 3`** explicitly; there is no persistent environment between tool calls.

```powershell
cargo test -j 3 -p whiting-device --features wgpu
cargo run  -j 3 -p whiting-device --features wgpu --example render -- landscape out/landscape
cargo run  -j 3 -p whiting-mountain --example census
```

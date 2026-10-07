# Content identity: pixel → authored path

Implementation handoff for [proposals/agent-content-identity.md](../proposals/agent-content-identity.md).
Read the proposal for the design; this file is for a session picking the work up.

## Where the work is

Branch `feature/agent-content-identity`, worktree `.worktrees/content-identity`.

| commit | what | state |
| --- | --- | --- |
| `a909cb5` | the measurement, red: `r32float` does not build a pipeline today | done |
| `afcc701` | an attachment's blend follows its own format | done |
| `fc30e92` | a scene may declare the images its own shaders write | done |
| `9cc7f06` | `Face::uv` and `FacetMesh::locate` | done |
| `72c5186` | SPEC.align for both | done |
| `3a6774a` | the lighting template can write beside the frame | done |

**The framework and the crate are finished and verified. The content side has not been started** —
`landscape` still declares no outputs, carries no id, and nothing calls `locate`. The whole
mechanism is in place underneath and unused, which is the state to be careful with: the next
session is wiring two ends together, not building anything new.

## What each landed commit established

- **`r32float` is a renderable colour attachment on this adapter and a `f32` fragment output
  survives the round trip exactly** — measured with 4093, a value `rgba16float` cannot represent.
  Both facts the design rests on, and the first is not in any specification.
- **The backend tests whether a blend state is *present*, not what its factors do.** `Replace` on
  `r32float` is refused exactly as `Alpha` is. An unblendable attachment gets `None`.
- **Every stage that draws geometry writes a declared output** — lighting and emission — and
  `check_outputs` enforces it. A scene that opts in and forgets a stage gets told which shader.
- **`Face::uv` is distance along the crest, not sample index**, and it is permuted with the
  winding. Both were wrong in the first draft.
- **The lighting template's two `@location` namespaces are different.** A varying's addresses an
  entry of the `Vertex` struct and continues from six; a fragment output's addresses a colour
  attachment and is the number the scene declared. They run in the same order, so deriving one
  from the other looks right and is not.

## What is settled, so it is not re-argued

- **The id is allocated by the content, globally, with `0` reserved** for "not an authored
  element". It is not a face ordinal and not a per-mesh index. Nothing in the mountain crate
  changes its indexing.
- **`belt_id` is not in the image.** It is hop 3, a lookup into `FaceAttributes` the crate already
  holds. The image carries only what cannot be recovered elsewhere: `id`, `u`, `v`.
- **Three `f32` outputs, not one `vec4`.** An `rgba16float` is exact only to 2048; a `r32float` is
  exact to 2^24 and is the only way an id above 2048 survives. Measured: 4093 comes back 4093.
- **`(u, v)` is `Face::uv`**, defined once, and a scene writes what the face carries. `s` runs
  along a belt's top path and `t` crosses the strip. See the expander in `expand.rs` for what a
  belt is, and `FacetMesh::locate` for the inverse.
- **Every lighting stage writes the output**, and a stage that draws something not authored writes
  `0`. `check_outputs` and `render_outputs` already enforce both halves.
- **`mountain` is not asked to know about paths.** `Massif` has no naming and no grouping, and
  SPEC/mountain.md is human-editable only.
- **`face_index` is never a handle** and does not need to be in the image: `@builtin(vertex_index)`
  is free and `check_vertex_inputs` does not check builtins.

## Landed and verified

Every commit above was checked additively, by rendering before and after on the adapter and
comparing sha256: `dunes`, `landscape`, `tidewater-camera0` and `tidewater-camera1` are byte for
byte the same across all six. `whiting-device --features wgpu` is green (50 tests);
`whiting-scene` and `whiting-gallery` are green.

The blend change's two findings, neither of which is obvious from the code:

- the backend tests `cs.blend.is_some()`, **not** whether the factors discard the destination — so
  a `Replace` state on `r32float` is refused exactly as `Alpha` is. This cost one red run;
- the three 32 bit float formats are the entire unblendable set, because the device requests
  `Features::empty()` and `float32-blendable` is therefore off. Same reason a float texture cannot
  be filtered, which the workspace already relies on.

The before/after renders were checked to have actually differed, by confirming the revert took
effect before rendering the baseline. An inconclusive before/after is worse than none, and the
first attempt at one would have been exactly that.

## The next step, and its shape

**The `landscape` scene, and `locate`.** The two ends are built and the wire between them is not.
Four pieces, in this order, because each is checkable before the next exists.

### 1. The plain is the first surprise

**Every lighting stage writes the outputs, and `landscape` has two.** The massif and the plain
are separate groups with separate shaders, and the plain draws over the massif's lower slopes. A
plain shader that does not write the three outputs is **refused by name** — which is the intent
working — but it means the plain's point data has to carry the same three fields, filled with
`0`.

`heightfield` builds the plain through `Shaded`, whose layout is fixed at
`normal, position, texture`. Either `Shaded` grows a field every caller must supply, or the plain
gets its own builder. **The second is smaller**: `Shaded` is used by most of the gallery and
widening it touches every scene, which is the additive property this whole change has held.

### 2. The id allocator, producing the table and the numbers together

One function walks the massif's belts in `BeltRef` order, assigns `1..N`, and writes the json
table beside the images. **It must be the only thing that assigns ids**, because the table and the
numbers in the pixels have to come from one walk — two walks would be two things that can
disagree, which is the failure the whole design is arranged to prevent.

`0` is the plain's, and means "not an authored element". The path strings are the content's own;
`Massif` is not asked to know them.

### 3. `faceted()` writes what `Face` already knows

`Face` now carries `uv` per corner, so `faceted()` writes `mesh.face.uv` straight into the point
data — **no derivation, no second definition of the parameter space**. The id is the one addition,
written at all three corners exactly as the normal is.

### 4. `locate(x, y)`

The composite. Reads the three declared outputs at one pixel, looks the id up in the content's
table, calls `FacetMesh::locate` for the belt, and prints one line. **It refuses to answer when
the manifest's hash for the frame and the hash stamped on the table disagree** — a stale table
answering confidently is worse than no answer.

## Files this work will own

Taken by the landed commits, and still owned by this branch:

- `crates/device/src/wgpu_renderer/raster.rs` — `blend_for`, `blendable`
- `crates/device/tests/r32float_attachment.rs` — the measurement, kept
- `crates/scene/src/standard/scene_schema.rs` — the `outputs` field
- `crates/scene/src/standard/emit.rs` — `SceneOutput`, `read_scene_outputs`, `add_targets`,
  `target_list`, `output_bindings`, `debug_outputs`
- `crates/scene/src/lib.rs` — `OutputLocations`, `UndeclaredTexelType`
- `crates/graphics/src/output.rs` — `TexelError::spelled`
- `crates/mountain/src/facet.rs`, `expand.rs`, `math.rs` — `Face::uv`, `locate`, `span`
- `crates/gallery/src/shaders.rs` — `Extras` and the six template holes
- `crates/scene/tests/clustered_lights.rs` — the stale `landscape` pin, now correct

Still free for the content side:

- `crates/gallery/src/landscape.rs` — the scene
- `crates/gallery/src/geometry.rs` — a plain builder that carries the three fields
- `crates/gallery/tests/` or an example beside `census.rs` — `locate`

**Check `git status` before widening this set.** `render.rs` (the example) and `SPEC.align.md` are
where other work lands too, and the gbuffer handoff records a partial hand-apply into `main` that
broke the build by exactly this route.

## Two traps found the hard way

## Two traps found the hard way

- **Line endings.** `crates/gallery/src/shaders.rs` checked out as CRLF and the `edit` tool's
  LF-matched strings silently did not match. `core.autocrlf` is `true` and the repo stores LF, so
  converting the working copy to LF is safe and commits back as LF — but a template edit inside a
  1400 line `format!` is done with a **byte-range-scoped** script that reports its match count,
  because the same `Vertex` struct appears in three templates and a whole-file replace edits all
  three.
- **`PowerShell` here-strings drop the leading newline**, so an anchor written as `@'` newline
  `text` `'@` is one character short of the file and matches nothing.

## Commands

The machine has twenty cores and cargo defaults to all of them, so **every cargo command here needs
`-j 3`** explicitly; there is no persistent environment between tool calls.

```powershell
cargo test -j 3 -p whiting-device --features wgpu
cargo run  -j 3 -p whiting-device --features wgpu --example render -- landscape out/landscape
cargo run  -j 3 -p whiting-mountain --example census
```

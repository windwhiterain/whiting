# Handoff: the framework the three bamboo worktrees found, without the scenes

Worktree: `./.worktrees/framework-merge`, branch `feature/framework-merge`, on top of the
checkpoint `7f5ff03`.

Three agents built the same brief — a bamboo courtyard in six frames — and the superior's
ruling was **framework and proposals only, no scenes**. So this lands what the three found
about the framework, and drops what they found about pictures.

## What landed

| from | what | where |
| --- | --- | --- |
| dsf | six geometry generators | `gallery::geometry` |
| dsf | `gallery_vertex_inputs_with` | `gallery::attributes` |
| dsf | `lighting_shader_with_schema`, `emission_shader_with_schema`, `placement_shaders_with_schema` | `gallery::shaders` |
| dsf | `Gallery::census`, `Gallery::bounds`, `--census` | `gallery::lib`, `device/examples/render.rs` |
| k3 | the sky fog defect | `proposals/sky-fog-distance.md` |
| — | `scene::standard::INSTANCES` and `TRANSLATION` | `scene::standard` |

## The generators

`culm`, `blade`, `double_blade`, `leaf_spray`, `tuft` and `masked_heightfield`. They are
additive and they compile against the tree as it stands, because `geometry.rs` had not moved
since the merge base — the two files were copied whole rather than merged.

`masked_heightfield` is the one that is not a plant. It carries a **surface mask** in the
normal's third component, on the argument that a shading normal always satisfies
`nx² + ny² + nz² = 1` and a terrain rolled out of a heightfield never has `nz = 1`, so the
component is free for a value that is not a normal. A body that knows its normal is a mask
rather than a normal reads both before anything reads either.

## The schema work, and why it is the whole of what was ported

dsf's three `_with_schema` builders were written against the old shader API. `shaders.rs`
has since been refactored into `Block<'text>` composition, so they were **re-expressed, not
merged**. What survived the rewrite is the shape, and the shape is small: each takes a
`custom_schema: Schema` where its sibling took `kit::gallery_custom_schema()`, and threads it
into two places — `gallery_vertex_inputs_with` for the locations, and `kit::shader` for the
row it is verified against. The `&str` parameters became `impl Into<Block<'text>>` because
that is what the surrounding functions now take.

Why it matters: a scene with more than a handful of surfaces has to put a field of its own
in the instance row, because the alternative is one shader and one pass per material. A
field in the row moves every field that sorts after it, and **nothing checks a vertex
layout** — so a list written from the gallery's schema reads a normal where a matrix is, and
the failure is a draw rather than an error.

## `census` and `bounds` are not dsf's

dsf's versions were written for six scenes that are not landing, and both answered
"`{name}` has no census of its own" for everything else. As merged they would be dead code
with six dead scene names in them. They are generalised instead: both take **any** name
[`Gallery::build`](crate) knows, so all ten existing scenes are reportable today.

Two things were dropped on the way. dsf's `bounds` collected placement labels into a `named`
vector it never read — dead, and it would have warned. And its `census` read the post stage's
`const … f32` lines out of the shader's WGSL source; that is kept, because those constants
are the frame's brightness and there is no field a caller can read instead, but it is now
stated as what it is rather than left looking like a scene feature.

`bounds` is of **translation only** — an origin box, not a geometry box. Folding in rotation
and scale would need each placement's matrix against each mesh's own extent, which is a
different and more expensive measurement. An origin box is what tells an author their
arithmetic was wrong.

## Two names the scene crate was missing

`INSTANCES` existed as a `pub const` in the private `scene_schema` module and was not in the
`pub use` list. `TRANSLATION` did not exist at all: `transform_schema` spelled
`"translation"` inline while its sibling `TRANSFORM` had a const beside it. Both are now
consts on the public surface, because the new code reads a document rather than writing one
and should not have to spell a field to reach it.

**Twelve more consts are in the same state** and were left alone: `FAR`, `FOCAL_LENGTH`,
`LIGHTS`, `MESHES`, `NAME`, `NEAR`, `POINT_SCHEMA_FIELD`, `POST_PROCESSES`, `PROJECTION`,
`ROLL`, `SCREEN_MATERIALS`, `SENSOR_WIDTH`, `TIME`. That is a systematic gap in one manual
list, not thirteen separate oversights, and it is worth one pass of its own.

## What was dropped, and why

- **All three scenes.** k3 and m3.1 both register the gallery name `bamboo`; dsf registers
  six `courtyard-*`. None lands.
- **m3.1's shadow bias bump** (`0.0006→0.0011`, `0.003→0.0040`) and the comment with it. It
  changes every scene's shadows, so it is a framework decision and not a scene one. The
  reasoning in that comment — that a page too coarse for the picture is a resolution limit
  and no depth bias hides it — is worth keeping and is recorded in the commit message.
- **dsf's `clustered_lights.rs` rows** for the six courtyard scenes.
- **dsf's six rendered PNGs.** `/out` is gitignored on `main`; dsf force-added 14 MB of them.

## State

- [x] geometry generators, additive, compiling
- [x] `gallery_vertex_inputs_with`
- [x] the three `_with_schema` builders, re-expressed against `Block`
- [x] `census`/`bounds` generalised, `--census` wired and in `usage`
- [x] the two missing scene field names
- [x] the sky fog proposal, marked unfixed and re-verified against `terraces.rs:1015`
- [x] `cargo test --workspace` green
- [x] `cargo test --workspace --features whiting-device/wgpu` green — 34 suites, 183 tests, 0
      failures, including the two adapter tests on a real device
- [x] `--census` run against `tiles`, `dunes`, `tidewater` and `terraces`

## Not verified here

No frame was rendered by this change, and it could not be: the only scenes that would exercise
a masked heightfield or a custom row are the three that are not landing, so there is nothing in
the tree to draw them with. The generators are dsf's own code and were render-proven in
`worktrees/dsf` — its handoff measures a grove of 205 culms, 636 leaf sprays and 410 single
leaves across six rendered frames — so they are not unexercised, but that evidence comes from a
branch that did not land, and this merge did not reproduce it.

What *was* verified here is the part that is new: `--census` and `--census`'s bounds run
against four real scenes, and they show what they were built to show. `tidewater` reads 213
depthing placements against 212 lit, which is the invariant `clustered_lights.rs` holds still
and which was previously only visible by reading a test's expected table.

One thing the census could not show: `exposure` is a declared scene field as of `7f5ff03`, but
no gallery scene names one yet, so every scene today takes the "field absent" path and the
field is printed by no run.

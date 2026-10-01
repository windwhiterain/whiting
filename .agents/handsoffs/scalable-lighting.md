# Scalable Lighting: Sparse Adaptive Shadow, Clustered Lights, Image Based Lighting

A five stage effort to let one scene hold thousands of lights without one pass per light
per material group, and to give materials an environment instead of a flat ambient term.

Worktree: `./.worktrees/lighting`, branch `feature/lighting`, based on `main` at `8eb58d8`.

## Why the current renderer cannot scale

`crates/scene/src/standard/emit.rs` builds one shadowing pass and one lighting pass per light
per group, so pass count is `O(lights * groups) * 2`. Each light also gets its own instance
texture entry named `{label}_{shadow_map}`, so shadow memory is `O(lights * 2048 * 2048 * 4)`.
One thousand lights is one thousand shadow maps. The clustered path below is what replaces this.

## This machine has a gpu

**An adapter is present and rendering works.** This is worth stating plainly because earlier
stages of this effort were written under the belief that it was not, and that belief cost real
verification. Confirm it before you take any statement here on trust:

```
cargo test -p whiting-device --features wgpu
cargo run -p whiting-device --features wgpu --example render -- terraces out/terraces.png
```

The second writes a 1024 square image of the terrace scene in about 1.1 seconds, thirty one
passes deep. `crates/device/tests/gallery_render.rs` and `crates/device/tests/texture_channel_render.rs`
both **pass**, and neither prints its `skipped: no gpu adapter is present` note. A pass with that
note printed is a skip that reports success, so read the note rather than the summary.

What this changes: a shader can be compiled by `wgpu` and executed, a `texture_depth_2d_array`
view can be created and rendered into, a storage buffer can be written by one pass and read by
another, and a frame can be looked at. The clustered lighting pass in the next stage is
therefore verifiable by rendering a scene and counting what it draws, not only by validating
WGSL and counting passes in a document.

**Run the `wgpu` feature when you run tests.** `crates/device/Cargo.toml` marks the render
tests `required-features = ["wgpu"]`, so `cargo test --workspace` on its own does not execute
them. A checkpoint was merged with six failing tests inside exactly that gap: the strict usage
check refused the word `uniform`, only the tests behind the feature could see it, and a plain
workspace run reported green. `cargo test --workspace` **and**
`cargo test --workspace --features whiting-device/wgpu` are both part of the gate from here.

## What this renderer is for

This is an offline renderer. It has no real time frame and no ray tracing, and it is to be
designed as the first without the second. Two consequences run through every stage below.

**Statelessness is correct, not a limitation to work around.** `render()` is a pure call and
stays one, because every image is independent and a result must depend only on the documents
that produced it. This is what rules out reservoir sampling with temporal accumulation and
any cross frame page cache, and it is why those mechanisms are replaced rather than deferred.

**Configuration is data, never a Rust constant.** A page size, a page count and a cluster
grid are schema fields on the scene or the graphics document, read at build time. A number in
the source is a value someone chose once; a number in a document is a value an author chooses
per scene. The project already has this rule in writing, in `.agents/SPEC.align.md`: a schema
can change at runtime without recompiling Rust, and a document a reader cannot tell apart is a
document an agent can edit with the same tools. `SHADOW_MAP_WIDTH` and `SHADOW_MAP_HEIGHT` at
`crates/scene/src/standard/pass_schemas.rs:143` and `:146` are `pub const` and violate it, and
they are the first thing stage 4 replaces. The precedent for doing it correctly is in the same
file: the comment at `pass_schemas.rs:151-157` states that the depth buffer's size is not a
constant because it follows the scene's `frame`.

## Budget the work must fit in

`WgpuRenderer` requests `wgpu::Limits::downlevel_defaults()`. From
`wgpu-types-30.0.1/src/limits.rs:557-571`:

| limit | value |
| --- | --- |
| `max_texture_dimension_2d` | 2048 |
| `max_texture_array_layers` | 256 |
| `max_storage_buffers_per_shader_stage` | 4 |
| `max_uniform_buffer_binding_size` | 16 KiB |
| `max_bind_groups` | 4 |
| `max_compute_invocations_per_workgroup` | 256 |
| `max_compute_workgroup_storage_size` | 16352 |

`required_features` is `Features::empty()`, so `float32-filterable` is off and a `r32float`
or `rgba32float` texture cannot be sampled with a linear filter. Only `rgba8unorm`,
`rgba8unorm-srgb`, `rgba16float` and `bgra8unorm` are filterable.

A 16 KiB uniform is the wall that makes thousands of lights impossible today: there is no
legal place to put a thousand light records. `storage` is the way out, and there is room for
three of them beside the uniform at binding zero, which is exactly what the clustered
path needs.

`downlevel_defaults()` is a floor chosen for compatibility, not a description of the hardware.
Requesting the adapter's real limits is standard practice and would raise
`max_texture_dimension_2d` well past 2048 on any real gpu, which is what lets a single shadow
map be larger than 2048 square. This is not a change stage 1 makes, because `wgpu` is an
optional feature of `device` and the crate must still build and run with no adapter at all, so
whatever is requested has to fall back to the downlevel numbers when no adapter is present.
Stage 4 raises the question and records where the answer belongs.

## Stage 1: storage buffer usage and a per frame buffer cache

**Status: done, on `feature/lighting`.**

The two halves are one unit because neither is useful alone.

### The `storage` usage word

`usage` is an unconstrained `list(string)` (`crates/graphics/src/binders.rs:404-406`), so the
graphics side needs no schema change. The device side does:

- `crates/device/src/wgpu_renderer/bindings.rs` defines `VERTEX` and `INSTANCE` as `&str`
  constants, and now `STORAGE` (`"storage"`) and `STORAGE_WRITE` (`"storage_write"`). The
  writable case is a *second word* rather than a third kind, because it is a permission on a
  storage buffer and not a kind of buffer.
- `bind_buffer` routes on the new words and uploads with `BufferUsages::STORAGE`.
- `Bound::Buffer` carries a `BufferKind` (`Uniform`, `ReadOnlyStorage`, `ReadWriteStorage`),
  because `layout_entries` has to emit the matching `wgpu::BufferBindingType`. The kind
  lives on the bound resource rather than being read back off the layout, so the two cannot
  disagree.
- **An unrecognized usage word is an error, not a silent uniform.** `buffer_kind` is the one
  place the words are read, and it refuses a word it does not know, naming the instance entry
  and listing the valid words. It also refuses `storage_write` without `storage`: a buffer
  nothing declared to be a storage buffer cannot be one a shader writes. An *absent* usage
  list is still a uniform, so no existing document changes meaning.

The naga side of the interface check already accepts storage buffers:
`crates/device/src/wgpu_renderer/shader_interface.rs:157` maps
`naga::AddressSpace::Storage { .. }` to `ResourceKind::Buffer`, and the invariant at
`shader_interface.rs:28-29` states that a buffer is a buffer whether it is a uniform or a
storage buffer. No change needed there, and no change wanted: the check compares a resource
kind, not a resource's shape.

Worth noting the gap that leaves. A shader that declares `var<storage>` where the pass bound
a uniform passes this check, then fails inside wgpu with a message that names neither
document. Catching it would mean widening `ResourceKind` to separate the two buffer
address spaces, which contradicts the existing invariant. Recorded, not fixed.

### The buffer cache

`TextureCache` exists so one instance texture entry is one texture however many passes bind
it. `BufferCache` is the same shape beside it, keyed by the same instance entry name, and it
covers the `BinderKind::Uniform` path as well as the buffer path. The per pass uniform at
binding zero is deliberately *not* cached: it is packed from that pass's own fields, so it
has no entry to key by and two passes that packed the same bytes are still two passes.

**The same entry bound at two access levels is not a problem, and that is the point of the
design.** `BufferKind::usages` gives a read only and a writable storage buffer the *same*
`BufferUsages::STORAGE`, because a backend distinguishes the two at the bind group rather
than at creation. So one buffer serves both, the access level is the pass's own business,
and the cache never has to reconcile it. What the cache does record is the usage set, since
that *is* fixed at creation: an entry that is a uniform in one pass and a storage buffer in
another is a document asking for two buffers under one name, and it is reported as one.

### What stage 1 did not close

- A buffer declaring `["vertex", "storage"]` is read by `buffer_kind` as a storage buffer and
  then taken down the vertex path, so the `storage` word is silently discarded and the buffer
  is created with `VERTEX | COPY_DST`. A document asking to be two kinds of buffer at once is
  a mistake worth reporting on the same grounds as the unknown word, and it is not reported.
  Rare enough not to block the clustered path, which never makes a vertex buffer storage.
- A shader that declares `var<storage>` where the pass bound a uniform still passes the naga
  check, because the check compares a resource kind and both address spaces are `Buffer`. It
  then fails inside wgpu with a message naming neither document. Catching it would mean
  splitting `ResourceKind` along the address space, which contradicts a deliberate invariant.
- Nothing that creates a `wgpu::Buffer` was executed: this environment has no adapter, so
  `cargo check` and the `scene` and `graphics` tests are the whole of what was verified. The
  two adapter backed render tests live in `crates/device/tests/` and cannot run here.


## Stage 2: a light names its kind

**Status: done, on `feature/lighting`.**

`light_schema()` at `crates/scene/src/standard/scene_schema.rs` is an enumeration whose tag
is the kind, with `Point`, `Spot`, `Directional` and `Environment`. The variant names are
the ones `.agents/SPEC.md` already gave, and the shape is `pass_schema`'s: the fields every
light has are written once in `shared_fields()` and each variant declares only its own, so a
`Directional` light cannot carry a `position` and an `Environment` light cannot carry a
volume.

| variant | fields beside `label`, `color` and `intensity` |
| --- | --- |
| `Point` | `position`, `shadow_extent` |
| `Spot` | `position`, `direction`, `inner_angle`, `outer_angle`, `shadow_extent` |
| `Directional` | `direction`, `shadow_extent` |
| `Environment` | `image` |

The emitter refuses what it cannot draw. `read_lights` reads the label, then the kind, and
returns `CompileError::UnsupportedLightKind` for anything that is not a `Point`, so the
refusal lands before the field reads: a directional light declares no `position`, and
reading the fields first would have reported a missing field instead of the mistake. A
light with no kind at all is `CompileError::MissingLightKind`, which is the reader's half of
the same rule — the schema refuses such a document first, as a `KindMismatch` at
`lights[0]`, and the reader refuses it by name for a document that was never validated.

**The `label` read moved above `intensity` and `color`.** Both errors name the light, and a
label is the only way to name it. The duplicate label check is unchanged and still runs
before anything else about the light is read.

`LightRead` carries a `LightKind` so the refusal has something to be about, and
`light_projection` asserts it is a point light rather than assuming one.

### What stage 2 did not close

- Only `Point` is emitted. `Spot` declares two required angles that nothing reads yet, and
  a `Directional` light declares no field for how far its shadow volume reaches; both are
  gaps in a schema nothing consumes, and stage 3 and stage 4 own filling them.
- `kit::light` writes the `Point` tag itself and takes no kind parameter, because every
  light in the gallery is a point light and a parameter would be a kind spelled nine times.
  The other three kinds have **no** builder: a builder here writes a document the compiler
  refuses, so it waits for the stage that emits it.
- Nothing was rendered. The gallery render tests need an adapter, which this machine does
  not have, so the gallery scenes were verified by compiling, by the `scene` doctests
  that compile the example scene and assert its pass labels and buffer sizes, and by the six
  `image_channel` tests — not by a frame. (The gallery holds six scenes, not the nine this
  file used to say: `Gallery::names()` is the list.)
- The `Directional` refusal was exercised by reading the code and by the `Spot` case, which
  shares the refusal; a `Spot` light carries a `position`, so it does not prove the refusal
  happens before the field reads. A test that proves it is worth having and was not written,
  because writing tests needs the user's permission.


## Stage 3: clustered assignment, computed on the cpu

**Status: 3a done, on `feature/lighting`. 3b is the lighting pass and the migration, and it
is written as one section with stage 4 below because stage 3b is blocked on the same thing
stage 4 is.**

**The assignment is not a compute pass.** This stage was originally planned as one, and that
plan is wrong for this renderer. `emit(scene, camera_index)` returns the instance and
`Renderer::render(&self, graphics, instance)` consumes an instance that is already built, so
for a given scene and camera the frustum is fixed, the lights are fixed and the cluster boxes
are fixed. Cluster assignment is therefore a pure function on the cpu, and it belongs in
`emit` beside the rest of the compilation rather than in a pass.

Doing it there collapses the work and buys something a gpu could not have. The textbook
clustered path needs a clear dispatch, an `atomicAdd` count pass, a prefix sum for the
offsets and a scatter pass, over a grid that `max_compute_invocations_per_workgroup` of 256
cannot cover in one workgroup. On the cpu there is no atomics, no prefix sum and no dispatch,
and **the per cluster cap can keep the highest contributing lights rather than the first
ones to arrive.** Truncating a list is the one thing a sparse scheme must not do, and being
able to rank exactly is the reason this stage is cheaper here than the plan assumed.

Three storage buffers, all read only, inside the limit of four per shader stage:

- light records, one per light
- per cluster light count
- per cluster light index list

The grid and the per cluster cap are schema fields on the scene, defaulting to 16 x 9 x 24,
which is 3456 clusters. The cap keeps the highest contributing lights and records that it
dropped the rest, rather than dropping them silently.

**Directional and environment lights must be exempt from clustering.** They cover the whole
frustum at every depth, so letting them take cluster slots defeats the culling that makes the
scheme work and spends the budget on lights that were never going to be culled. They get their
own path.

Stage 3 lands in two halves so that neither is unreviewable. 3a produces the data and changes
no shading: the cluster lists exist in the instance and nothing reads them yet. 3b writes the
one lighting pass that consumes them, migrates the gallery onto it and removes the per light
pass generation from `emit`, which is where "replace rather than run both" is actually paid
for.

### Stage 3a: the assignment, landed on `feature/lighting`

`crates/scene/src/standard/clusters.rs` is the whole of it. What landed:

- **`clustering` on the scene**, four optional counts — `width_extents`, `height_extents`,
  `depth_extents`, `lights_per_cluster` — defaulting to 16 x 9 x 24 and 64, at
  `scene_schema.rs:clustering_schema`. A count of zero is refused: a grid of nothing is a
  scene whose lights reach no cluster.
- **`radius` on a `Point` light**, optional, defaulting to one box radius of what the scene
  draws. It is a field of the light and not of `shadow_extent` because the two answer
  different questions. A radius of zero or less is refused.
- **three read only `storage` entries**: `light_records`, `cluster_light_counts` and
  `cluster_light_indices`. Nothing binds them yet, and nothing is created until a pass binds
  it, so an entry a later stage does not read costs one packed run in the document.
- **the light record** is `color`, `position`, `radius` — seven floats, 28 bytes a light —
  packed with `definition::PackedBuilder` against `graphics::layout::packed_layout` of
  `light_record_schema`, so the bytes and the layout cannot disagree about a stride. **No
  shadow projection is in the record**: what a light's shadow map is looked up through
  belongs to the shadow stage, and stage 4 replaces that addressing. Stage 3b will add the
  field it needs when it writes the pass that reads it.
- **`CompileOutput.notes`**, which is how a dropped light reaches an author. One note naming
  the totals, then one per overflowing cluster naming what it kept and what it dropped, with
  the labels counted past eight rather than silently elided. A host that ignores the field
  renders a scene that is quietly wrong, which is the outcome this project treats as worse
  than a refusal.

**How depth is distributed: the slices divide the projection's depth, which is uniform in one
over distance.** Equal *world* thickness slices pile up at the eye and vanish in the
distance, so most of the boxes would cover the clusters that fill the screen and none would
cover the far ones — and a cluster nearly empty in screen space but deep is still a cluster.
It is also the only division a fragment can use for free: a fragment carries its own depth,
so the clustered pass computes its box with a multiply and a cast. The cost is stated in the
module rather than hidden: the slices are not even in world space, so the nearest is a hand's
width from the eye and the furthest is most of the scene, and a light spanning a slice is
listed in a box larger than the part of the scene it lights. `Projection::depth_at`,
`distance_at` and `half_width_at` are the three readings this needs, and the doctest in
`camera.rs` holds that the first two invert, for a parallel and a perspective camera both.

**A cluster is the slab at its slice's far boundary.** A slice of a perspective view is a slab
that widens with depth, and the box at the far end contains the whole of it. The tighter box
at the near end would exclude a light that reaches the slice's far half, and a light missing
from a cluster is a light no shading pass can find, so the slack is spent where it costs work.
The slice *range* a light is tested over is also one slice wider at each end than its own
extent calls for, because which slice a volume's end falls in is decided by a projection whose
rounding puts a point exactly on a boundary in either slice beside it.

**How the cap ranks.** By `brightness / (1 + distance_squared)` to the nearest point of the
box, ties broken by document order so one scene compiles to one set of bytes. The only
property the rank needs is to be monotone in distance, and it is: any falloff of the form
`1 / (a + b*d + c*d*d)` with positive coefficients decreases with distance whatever its
coefficients, so the nearest lights are the ones a scene's own falloff would keep. No scene's
falloff constants are written into the compiler, because those are a scene's decision.

**What stage 3b has to consume.** The three entries above, bound at the binding points of its
own naming: `light_records` indexed by the cluster lists, `cluster_light_counts` for the loop
bound and `cluster_light_indices` at `cluster * lights_per_cluster`. A fragment gets its box
from its own depth and its two position indices, and reads `counts[cluster]` lights from that
one row. It also has to add whatever the record is missing that it needs — a shadow projection
per light, most likely — and it has to give `Directional` and `Environment` lights their own
path, which is the exemption this stage states and does not yet act on: they are in the record
buffer and in no cluster list, so a scene of nothing but those two kinds has empty cluster
lists, which is correct and not a bug.

### What stage 3a did not close

- **Twelve tests in `crates/scene/tests/clustered_lights.rs` and a doctest in `camera.rs` are
  the whole of what was verified.** The assignment is pure cpu code, so it is the one part of
  this effort that can be checked without an adapter, and the tests cover a light that reaches
  no cluster, one that reaches every one, a cluster no light reaches, a volume straddling a
  slice boundary, the depth distribution at both ends of the frustum, the cap keeping the
  highest contributing lights rather than the first ones, the exemption, the documented
  defaults, and the regression bar that an all `Point` scene still emits the passes and buffer
  sizes it emitted before. What none of them can cover: that the bytes are what a `WGSL`
  struct reads, which needs a shader and an adapter.
- **`the nine gallery scenes` is wrong: there are six.** `Gallery::names()` at
  `crates/scene/src/gallery/mod.rs:52` lists `dunes`, `lanterns`, `monoliths`, `studio`,
  `terraces` and `tiles`. The test that pins the pass count of each of them is the check to
  copy for 3b.
- **The note channel has no printer.** `CompileOutput.notes` is filled and tested and nothing
  in this repository prints it, because the only thing that would is `device`, which this
  stage does not touch. `crates/device/examples/render.rs` already prints per pass, so that
  is where a host's print goes.
- **The record carries no shadow projection**, so the clustered lighting pass cannot yet find a
  light's shadow map, and a scene with any directional or environment light still gets one
  pass per light from the old path, because `read_lights` still refuses those kinds.

## Stage 3 and Stage 4, together: the clustered pass and the sparse shadow page pool

**Status: 4a landed, 4b blocked on the device. See "What stage 4a did not close" below.**

### Checkpoint discipline

This effort merges to `main` at each checkpoint rather than sitting on one branch until it is
finished, so that other work can build on what has landed and stay in step with it. The branch
is `feature/lighting` in `./.worktrees/lighting`; the worktree is deleted when the effort is
done, not at a checkpoint.

**Checkpoint 1** is stages 1, 2, 3a and 4a: storage buffers and the per frame buffer cache, the
light kind, the cluster assignment on the cpu, and the general module lift with the page pool
as schema data. It is mergeable as it stands. One thing it deliberately does not do is make the
acceptance bar pass: the clustered lighting pass is not written, so a pass count still grows
with the light count, and the bar that says so is marked ignored rather than deleted.

`main` moved under this branch during that window. `1ee2ddc` moved the scene builders out of
`crates/scene/src/gallery/` into a new `crates/gallery` crate that depends on `scene`, so the
compiler can no longer see the gallery. The three tests that use gallery scenes as fixtures now
reach it through a `whiting-gallery` **dev-dependency** of `whiting-scene`: cargo allows the
cycle because it exists only for tests, and the tests stay in the crate whose behaviour they
check, with the gallery as the fixture rather than the subject.

These two stages landed as one effort and are written as one section, because they are one
mechanism: a clustered shading loop cannot reach a light's shadow without an addressing scheme,
and an addressing scheme nothing reads is storage nobody wanted.

### What landed

**The general module.** `crates/scene/src/lighting/` is a top level module beside `diff` and
`reference`, split into `assignment.rs` (the arithmetic) and `buffers.rs` (the packing and the
pool). It is not inside `standard` because nothing in it is about the standard scene. The seam
that mattered was the camera: the arithmetic needed a view's eye, axes, depth planes and its two
depth/distance mappings, and was reaching into `standard::Projection` for them. It now takes a
`ViewProjection` trait that `standard::Projection` implements **by forwarding**, so the two can
never answer differently about where a light is. `Projection` and the light document stay in
`standard`; the grid, the cap, the ranking, the record schema and the pool do not.

**The pool.** `SHADOW_MAP_WIDTH` and `SHADOW_MAP_HEIGHT` are no longer constants. The pool's
shape is three fields on the scene's `clustering` block — `shadow_page_width`,
`shadow_page_height`, `shadow_pages` — read at build time, defaulting to 1024 by 1024 and 64.
A pool deeper than the 256 layers `max_texture_array_layers` allows is **refused** rather than
truncated, and `LightPool::total_bytes` is reported through `CompileNote` with the page count,
the given count and the bytes the whole pool costs.

**One ranking, used twice.** The pool ranks by the same `contribution` the per cluster cap
uses, so there is one notion of how much a light matters. The two asks differ in what the
distance is measured to: a cluster asks how bright the light is *there*, and a light just
outside a box still counts for a little, whereas a page is worth rendering once and what
matters is how much of the image the light reaches at all. A light can therefore be kept by a
cluster and still be given no page.

**A light with no page says so.** The record's fourth field is `page`, and a light with none
carries **minus one**, because zero is the first page. The stride grows from seven floats to
eight.

**The WGSL is checked.** `naga` 30.0.1 is a dev dependency of `scene` — already in the lock
file through wgpu, so nothing new compiles. `tests/wgsl_validation.rs` parses and validates the
**29 shaders the six gallery scenes emit**, read out of the compiled instance rather than off
the templates so `terraces`' `__SUN__` style tokens are already resolved, against
`Capabilities::empty()` rather than the defaults because `WgpuRenderer` requests
`downlevel_defaults()` with `Features::empty()`. This is pure cpu work and needs no adapter.
It is not pipeline creation: `naga` cannot check that a bind group layout matches a shader,
because that lives in the device's `check_bindings` and needs a renderer instance.

### The blocker: the device cannot make a texture array

**The page pool cannot be rendered until the device can create a texture array, and the device
cannot, and this stage may not edit it.** The evidence, all of it load bearing:

- `crates/graphics/src/binders.rs:424-473` — the `Texture` payload schema has `width`, `height`,
  `format`, `data`, `sampled`, `filter`, `address` and `mip_filter`. **There is no `layers`
  field**, so a document cannot even say a texture has depth.
- `crates/graphics/src/binders.rs:221-235` — `texture_resource()`, the declaration half, is the
  same: source, width, height, format. No layer count either.
- `crates/device/src/wgpu_renderer/bindings.rs:828` — `depth_or_array_layers: 1`, hardcoded.
- `crates/device/src/wgpu_renderer/bindings.rs:832` — `dimension: TextureDimension::D2`,
  hardcoded.
- `crates/device/src/wgpu_renderer/bindings.rs:843` — `create_view(&TextureViewDescriptor::default())`,
  which on a one layer texture is a `D2` view of layer zero.
- `crates/device/src/wgpu_renderer/bindings.rs:316` — `view_dimension: TextureViewDimension::D2`
  hardcoded in the bind group layout, so even a texture that *were* an array would be declared
  as a flat one.
- `crates/device/src/wgpu_renderer/raster.rs:249-263` and `raster.rs:345-351` — the render
  target path draws into `slot.view` and has no way to select an array layer, so a shadowing
  pass could not render into page *n* even if one existed.

So a `texture_depth_2d_array` cannot be bound, and a shadowing pass cannot render into one.
This is not a naming or a plumbing gap; it is four separate hardcodings in the device plus a
missing schema field, and every one of them is in a crate this stage was told not to touch.

**What it blocks and what it does not.** It blocks stage 4b: the clustered lighting template
with its shadow lookup, and the removal of the per light passes. It does not block 4a, which is
why 4a landed: the pool's shape, the selection, the accounting and the record field are all
data, and data that nothing reads yet is exactly the state the plan asked for between 4a and
4b.

**The two ways forward**, and neither is a decision this stage may make on its own:

1. **Extend the device**, which means editing the crate this stage was told not to edit: add a
   `layers` field to the graphics texture payload and declaration, read it into
   `depth_or_array_layers`, create a `D2Array` view, declare `D2Array` in the layout, and give
   the render target path a per layer view. That is the design the handoff settled and the one
   `max_texture_array_layers` of 256 was quoted for.
2. **Make the pool a 2D atlas** — pages tiled into one `texture_depth_2d`, the page index
   resolved to a tile with a multiply and a modulo. This works with the device exactly as it
   is, needs no device change at all, and is a *different* design: the pool's bound becomes
   `max_texture_dimension_2d` of 2048 squared rather than 256 layers, so a 2048 square texture
   holds 16 pages of 512 or 256 pages of 128, and the page size and the count stop being
   independent. It also has a cost the array does not: taps taken across a page boundary read
   the neighbour's texels, so the nine tap kernel needs clamping to the tile.

Option 1 preserves the settled decision. Option 2 is the one that fits the device as it
actually is. **Recorded, not decided.**

- **The page pool is schema data; `SHADOW_MAP_WIDTH` and `SHADOW_MAP_HEIGHT` are still Rust
  constants.** They survive at `standard/pass_schemas.rs:143` and `:146` because their only
  remaining readers are the per light path that stage 4b removes — `emit.rs:462` sizes each
  light's own shadow map entry, and `emit.rs:1996`, `:2080` and `:2104` size the shadowing and
  lighting passes. The pool itself reads no constant: its shape comes from
  `shadow_page_width`, `shadow_page_height` and `shadow_pages`. Deleting the two constants is
  the last edit of 4b and not before it, because deleting them while the per light path is
  still live would leave a shadow map with no size at all.

### What stage 4a did not close

- **The device cannot create a texture array**, and stage 4b is blocked on that rather than on
  anything in this crate. The evidence is listed above. Until it is resolved there is no
  `texture_depth_2d_array` to bind and no way to render into page *n*.
- **The clustered lighting pass does not exist.** `shaders.rs` still carries a per light
  uniform at binding zero, `emit` still builds one shadowing and one lighting pass per light
  per group, and `terraces`' `fill` still has to supply a surface body written against
  `normal`, `lambert`, `radiance` and `ambient` — the last three of which are per light and so
  become sums over a cluster in the clustered template. That is the rework `fill`'s tokens
  were flagged for, and it is 4b's.
- **`Directional` and `Environment` are still refused by `read_lights`.** The exemption is
  designed, documented and acted on by the assignment (a non clustered light takes no slot and
  gets no page), but no reader accepts those kinds, so the path the exemption prepares is not
  reachable from a document yet. A directional light's orthographic shadow volume, with
  `shadow_extent` as its half width and its near and far planes from the scene bounds, is
  4b's.
- **The acceptance bar is written and marked `#[ignore]`**, not deleted:
  `the_lighting_pass_count_does_not_move_when_a_light_is_added` in
  `crates/scene/tests/shadow_pages.rs`. It fails today, and its failure is the real state of
  the renderer: `lanterns` emits 24 passes for three lights and 30 for four. Run it with
  `--ignored` to see the bar being missed, and un-ignore it when 4b lands.
- **The naga check is not pipeline creation.** It says a module is well formed and fits the
  downlevel capabilities. It cannot say a bind group layout the compiler emitted matches what a
  shader declares, because that check is `check_bindings` in the device and needs a renderer
  instance. Nothing has been executed: this machine has no adapter.
- **The array limit and the storage budget are counted, not exercised by naga.** naga
  capabilities are a feature set, not limits, so `max_texture_array_layers` and
  `max_storage_buffers_per_shader_stage` are asserted as numbers in
  `tests/wgsl_validation.rs` and enforced in the compiler, not validated by naga.
- **The pool's page size and count are read from the `clustering` block, not from `frame`.**
  The handoff left this open and the depth buffer's precedent argues for `frame`; a page is a
  scene decision about shadow quality rather than a property of the image, so it sits beside
  the cluster grid it is ranked with. Reversible in one place.

## Stage 5: image based lighting

First version, deliberately small: one environment per material, sampled as a two dimensional
octahedral map, which needs no new dimension or layer field in `texture_resource()` at
`crates/graphics/src/binders.rs:221-235`. One roughness per material means one texture with
no mip chain and no schema change at all.

This stage depends only on stage 2, so it can run alongside stage 3 or stage 4.

## Decisions

Settled. Each one is a constraint on the stage named beside it, not a suggestion.

| decision | affects | ruling |
| --- | --- | --- |
| does `Renderer` gain state across calls | 3, 4 | no, `render` stays pure; offline rendering, so temporal accumulation is invalid by construction and is replaced, not deferred |
| shadow page size and count | 4 | schema fields, no constant; the pool is configured per scene and its total is accounted for |
| how directional and environment light are handled | 3 | exempt from clustering, they get their own path |
| whether light kind becomes an enum | 2, 3, 5 | yes, and a missing `kind` is a validation error rather than a default |
| starting cluster grid | 3 | schema field with 16 x 9 x 24 as the default |
| stage order | all | clustered and shadow before image based lighting |
| replace the per light path or run both | 3b | replace; running both has no period of benefit in an offline renderer, it delivers a parallel implementation nobody uses |
| how a directional light's shadow depth is expressed | 4 | derived from the scene's `bounds`, which `light_projection` already takes; `shadow_extent` is the half width of the orthographic volume and the near and far planes come from what is drawn |
| where cluster assignment runs | 3 | on the cpu in `emit`, not in a compute pass, because the instance is built before the render and the frustum is therefore fixed |

## Still open

- Whether the renderer requests the adapter's real limits or keeps the downlevel floor. It
  keeps the floor for now, because `wgpu` is optional and the crate must run with no adapter.
  Raising `max_texture_dimension_2d` is what lets a page be larger than 2048 square, and it is
  stage 4's call.
- The page size and count sit in the scene's `clustering` block rather than in `frame`. That is
  reversible in one place, and the depth buffer's precedent for `frame` is the reason it might
  move there.

## Stage 4b part 1: the device can build a texture array, and a pass can name a page

**Status: done, on `feature/lighting`.** Three commits, in this order: the schema and the
widened interface check together, then the descriptor, then the layer selection. The first
two had to be one commit: a schema that can declare an array while the check still cannot
see the difference is a worse state than either half, because it looks like it works.

### The shape a texture has is a kind, not a count

The blocker evidence in the section above said "add a `layers` field to the payload and the
declaration". **That is type erasure and it was wrong**, and the correction matters: in
wgpu a texture and a texture array are one `TextureDescriptor` with two fields, but in
WGSL they are different types (`texture_depth_2d` against `texture_depth_2d_array`) and
the access mode differs, a `textureLoad` on an array taking an index a flat texture has no
place for. A single variant with a number in it accepts a document that reads `Texture`
while meaning something no shader can name.

What landed, in `crates/graphics/src/binders.rs`:

- **`Texture` and `TextureArray` are two variants of the payload enumeration**, and two of a
  declaration's `resource` enumeration. `Texture` declares exactly what it declared before
  this effort, so no existing document changes meaning. `TextureArray` adds a **required**
  `layers` — required, not defaulted, so there is no absent case for a reader to
  interpret — and a `Texture` payload has no `layers` field to fall back on. Both shapes
  are built from one shared field list (`texture_payload_fields`, `texture_declaration`)
  rather than written out twice, so the size, format, pixels and sampler settings cannot
  drift apart.
- **`BinderKind::TextureArray`** joins `Texture` as its own kind, with
  `BinderKind::is_texture()` for the callers that ask "is this an image" rather than "which
  image is this". `layers_of(payload)` answers a count **only** for an array; it returns
  `None` for a flat texture rather than a one, because returning one is the erasure the two
  kinds exist to remove.
- **`layer` is a field of the `TextureArray` declaration only**, read by
  `Declaration::layer()`. See "Where a pass names its layer" below.
- `texture_size` in `crates/device/src/render.rs` uses `is_texture()`, so the noop renderer
  sizes a frame from a pool's page size as it does from a flat texture's.

### Where a pass names its layer, and why

**On the declaration's `resource` field, and only on the `TextureArray` variant.** The
alternatives and why they are wrong:

- **On the target declaration in the pass** — `targets` is a `List(String)` of binding point
  names in attachment order (`crates/graphics/src/pass.rs:124-130`). An element that became
  a struct would change every existing document, and a parallel list of layers would have
  to be matched to it by position.
- **On the instance entry** — that is the whole texture, shared by every pass that binds
  it. A payload naming a layer would name one page for the whole image.
- **On the binder, generically** — that is what the declaration *is*: the binder's
  `resource` field is where `source`, the size and the format already live, and a target is
  always also declared as a binder (`bindings.rs` refuses a target that is not one of the
  pass's bindings). So the layer needed no new plumbing to reach the attachment.

A pass that only *reads* a pool names no layer, and is **refused** if it tries to: the
shader indexes the layers itself with the page its own light record carries. Absent means
page zero.

### The device side

`TextureShape` in `crates/device/src/wgpu_renderer/bindings.rs` is the one place that knows
what a payload's shape is. It is read before anything is created, and the descriptor, the
view, the layout entry and the attachment's view all derive from it, so none of them can
disagree about a shape.

- the **limit** is read from the device (`self.device.limits().max_texture_array_layers`),
  not written down, and a count above it is **refused naming the limit**, never clamped. A
  count of zero is refused too, and a zero *extent* is still rounded up to one as it always
  was, because a zero extent is a very small image and a zero layer count is no image.
- the **texture dimension stays `D2`** for an array. An array of two dimensional images is
  a two dimensional texture whose depth is its layer count, which is what
  `depth_or_array_layers` is for; a volume would be `D3` and no document here declares one.
  What changes is the **view**: a `D2Array` covering every layer, and the layout entry that
  has to agree with it.
- **a flat texture produces exactly what it produced before**: its extent is one layer, its
  dimension is `D2`, its view descriptor is `TextureViewDescriptor::default()` and its
  layout dimension is `D2`. `tests/texture_array_shape.rs` asserts all four against the
  literals the module used to write.
- an **array carrying `data` is refused**: filling a texture averages neighbouring texels
  and cannot tell a layer boundary from a row. A pool is written by passes.
- a **pass drawing into a layer the pool does not have is refused**, for the same reason a
  clamped pool is refused: it would write page zero again and hand one light another's
  shadow.
- the attachment's view is built at bind time from the declaration, not taken from the
  texture cache, because it is a function of that pass's document and one cached texture
  has to serve every pass that writes a different page.

**Two things the layer made necessary, which were quietly wrong otherwise.** Both are
recorded as invariants on the code that does them:

- **The clear bookkeeping is keyed on the entry *and* the layer** (`TextureSlot::clear_key`).
  It used to be the entry alone, so the second shadowing pass into a pool would have
  *loaded* the first one's page instead of clearing its own, and two lights would have cast
  the same shadow.
- **A frame is read back from the layer that was written** (`RenderedTarget::layer`, and the
  copy's `origin.z`). A pass that drew into page three produced page three; reading page
  zero would answer a question nobody asked. The multisampled copy keeps one layer whatever
  the target's depth, so an antialiased pass into a page resolves into that page.

### The interface check widened, in the same commit as the schema

`ResourceKind::Texture` now carries the shape (`ImageShape`: `One`, `Array`, `Cube`,
`Volume`). `kind_of` reads `view_dimension` out of the layout entry instead of discarding
it, and `declared_resources` reads `dim` and `arrayed` out of the naga image type instead
of discarding them. A shader declaring an array where the pass bound a flat texture is a
render error naming the pass, the variable and both shapes.

**This widens a deliberate invariant.** The module recorded "a resource kind is compared,
not a resource's shape", and that was right when there was one texture shape to get wrong.
The shape is now part of the *type* a shader names rather than a property of its size, so
the invariant had to become "a buffer is compared as a buffer, and a texture as its shape".
The buffer half is unchanged and its gap is still open: a shader declaring `var<storage>`
where the pass bound a uniform passes the check and fails in wgpu, which is recorded in
stage 1 and not fixed here. A one dimensional image is left alone on both sides, because
no document here declares one and the device never creates one.

The comparison was extracted out of `WgpuRenderer::check_bindings` into
`check_resources(module, entries, label)`, which is re-exported from the crate root under
the `wgpu` feature. It needs no device, so the check is testable on a machine with no
adapter, which is the only honest way to test it.

### What stage 4b part 1 did not close

- **Nothing that creates a texture, a view, a bind group or a pipeline was executed.** This
  machine has no adapter. The descriptor, the view and the layout values are asserted at
  the schema and arithmetic level, and the *device path is unexecuted*: nothing here has
  proved that a backend accepts a `D2Array` view of a depth array, that a render pass
  accepts a single-layer view of one, or that a pipeline is built from a `D2Array` layout
  entry. `crates/device/tests/gallery_render.rs` and `texture_channel_render.rs` need an
  adapter and cannot run here.
- **No shader in the repository declares a texture array**, so the widened check has never
  seen a disagreement in a real document. `texture_array_interface.rs` drives it with
  two one-line modules and hand-built entries.
- **A shader's declared interface must name the `TextureArray` variant with the same fields
  as the pass's declaration**, because `verify_pass_inputs` validates the pass's binder
  values against the shader's declared schema and a `Struct` is closed. A clustered
  lighting pass reading a pool therefore has to declare `Resource::TextureArray { source,
  width, height, layers, format }` in its own interface schema, or the closed-struct check
  refuses the pass's declaration. **This is the first thing stage 4b part 2 has to write.**
- **`page` in the light record is still not read by anything.** The record carries the page
  the pool gave a light, and nothing consumes it: that is the clustered lighting template,
  which is 4b part 2.
- **A `Directional` or `Environment` light still gets no page and no path**, because
  `read_lights` still refuses those kinds. The exemption is designed and acted on by the
  assignment and is unreachable from a document.
- The pool's own limit check in `crates/scene/src/lighting/` is a constant of 256 and the
  device's is the number the device was created with. They agree today because the renderer
  requests `wgpu::Limits::downlevel_defaults()`. The compiler cannot ask an adapter, so
  the duplication is inherent rather than an oversight: the scene side is an early, cheap
  refusal and the device side is the one that means anything.
- `SHADOW_MAP_WIDTH` and `SHADOW_MAP_HEIGHT` still exist and are still read by the per
  light passes, as stage 4a recorded. Deleting them is the last edit of 4b part 2.


### Settled since: the page pool is a texture array, and the device gets extended

The blocking question for 4b was how a clustered loop reaches a light's shadow. The page pool
is a **texture array**, each light record carrying the page it was given, and the `device` is
extended to be able to build one. `max_texture_array_layers` is 256 on this device and unused,
and the alternative, tiling pages into one 2d texture, was rejected because it makes the page
size and the page count dependent, and those are two independent schema fields by a decision
recorded above.

The alternative was not rejected for being unsafe. The project reads depth with `textureLoad`
rather than a filtered comparison sampler, so no hardware filter can bleed across a page
boundary; an atlas would have been workable. It was rejected for coupling two fields the
project decided must not be coupled.

What the device needs, all of it additive and defaulting to what a document means today. This
list is the specification stage 4b part 1 was written against; the one item that changed
shape is the first, and the reason is in the section above:

- ~~a `layers` field on the `Texture` payload~~ — **superseded**. It is a `TextureArray`
  **variant** beside `Texture`, carrying a required `layers`, and the declaration splits the
  same way. A count on one variant was type erasure: it let a document read `Texture` while
  meaning something no shader can name, because `texture_depth_2d` and
  `texture_depth_2d_array` are two WGSL types with a different `textureLoad` arity.
- `create_texture` to honour it instead of writing `depth_or_array_layers: 1` and
  `TextureDimension::D2`
- the bind group layout's `view_dimension` to follow it instead of `D2`
- the rasterize target path to select a layer, since it attaches one view and a shadowing pass
  has to be able to render into page *n*

## Working rules

Read `AGENTS.md` before touching anything. In particular: `SPEC.md` is human editable only,
`SPEC.align.md` is where the reasoning in this file should be recorded, and this handoff is
deleted when the effort lands.

# Scalable Lighting: Sparse Adaptive Shadow, Clustered Lights, Image Based Lighting

A five stage effort to let one scene hold thousands of lights without one pass per light
per material group, and to give materials an environment instead of a flat ambient term.

Worktree: `./.worktrees/lighting`, branch `feature/lighting`, based on `main` at `8eb58d8`.

## Why the current renderer cannot scale

`crates/scene/src/standard/emit.rs` builds one shadowing pass and one lighting pass per light
per group, so pass count is `O(lights * groups) * 2`. Each light also gets its own instance
texture entry named `{label}_{shadow_map}`, so shadow memory is `O(lights * 2048 * 2048 * 4)`.
One thousand lights is one thousand shadow maps. The clustered path below is what replaces this.

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


## Stage 2: light data becomes a schema

`light_schema()` at `crates/scene/src/standard/scene_schema.rs:348-371` is flat: `label`,
`position`, `color`, `intensity`, `shadow_extent`. There is no light kind, so a point light
and a directional light are told apart by nothing at all.

This stage turns it into a `Value::Enum` with a `kind` variant, starting with `Point`,
`Spot`, `Directional` and `Environment`. `read_lights` at `emit.rs:506-511` currently rejects
duplicate labels, which stays.

A document with no `kind` is a validation error. There is no default and no fallback, and
every existing scene is updated to carry its kind. This overrides the argument that a missing
field should read as `Point`: in an offline render a wrongly classified light is a wrong image
that nobody sees until the render finishes, and the cost of the error is a whole render rather
than a dropped frame. Statelessness makes that worse, because there is no previous frame in
which the mistake was visible.

## Stage 3: clustered assignment in a compute pass

Compute passes are real on this backend: `crates/device/src/wgpu_renderer/compute.rs:62-90`
builds a compute pipeline and dispatches, and `mod.rs:331-343` runs them in instruction order
before the rasterize passes. A compute pass needs a six component `workgroup` field, whose
first triple is the shader's declaration and whose second is the dispatch count.

Three storage buffers, inside the limit of four:

- light records, one per light
- per cluster light count
- per cluster light index list

Starting grid is 16 x 9 x 24, which is 3456 clusters, with 8 bytes per cell for the index
list and a per pixel cap that saturates rather than truncating.

**Directional and environment lights must be exempt from clustering.** They cover the whole
frustum at every depth, so letting them take cluster slots defeats the culling that makes the
scheme work and spends the storage budget on lights that were never going to be culled. They
get their own path.

## Stage 4: the sparse adaptive shadow page pool

The core request. The page size and the page count are schema fields, not constants, so the
shape of the pool is a per scene decision and nothing in this file fixes a number. A pool of
512 square `depth32float` pages is 1 MiB each, a pool of 1024 square pages is 4 MiB each, and
which one a scene wants is the author's to make. What the renderer owes them is a pool whose
total is accounted for and reported rather than discovered by running out of memory.

The mechanism comes from ReSTIR-Sampled Shadow Maps (Eurographics 2025): spend a fixed budget
on the few lights that contribute most, give them real shadow maps, and approximate the rest
with a single pass low resolution lookup.

What does not transfer is the mechanism underneath it. Reservoir sampling with temporal
accumulation needs state that survives between frames, and `Renderer::render` at
`crates/device/src/render.rs:54-70` is a pure call with no state to carry, so the selection is
a per image contribution ranking computed fresh rather than a reservoir carried forward. That
is a substitution, not a wait: the offline framing means there is no version of this that
works by accumulating over frames.

What does not transfer either is Epic's Virtual Shadow Maps, which is the other thing the
name "sparse adaptive shadow" could mean. Its 16K virtual resolution and its 128x128 page
pool exceed what these limits allow, its cross frame page cache needs the same persistent
state that is absent here, and the reason it exists, replacing cascades under Nanite and very
large worlds, has no counterpart in this renderer. The concept migrates; the implementation
does not.

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

## Still open

- Whether the renderer requests the adapter's real limits or keeps the downlevel floor. It
  keeps the floor for now, because `wgpu` is optional and the crate must run with no adapter.
  Raising `max_texture_dimension_2d` is what lets a page be larger than 2048 square, and it is
  stage 4's call.
- Where the shadow page size and count are authored: on the light, on the scene, or derived
  from the scene's `frame` the way the depth buffer is. The depth buffer precedent argues for
  `frame`; a light that must cast from a specific volume argues for the light. Stage 4's call.

## Working rules

Read `AGENTS.md` before touching anything. In particular: `SPEC.md` is human editable only,
`SPEC.align.md` is where the reasoning in this file should be recorded, and this handoff is
deleted when the effort lands.

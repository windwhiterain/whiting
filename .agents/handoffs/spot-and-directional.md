# Spot, Directional, and the silent errors

A handoff for three checkpoints that close what the scalable lighting effort left open. Main is at
`a9ca983`, the end of that effort, and this branch starts from it. Every checkpoint below is
independent: it passes all three gates on its own, and it merges to main on its own.

## The gates

```
cargo test --workspace
cargo test --workspace --features whiting-device/wgpu
cargo check --workspace --all-targets
```

All three, every time. The render tests carry `required-features = ["wgpu"]`, so a plain
`cargo test --workspace` silently skips them and a green first gate is not evidence.

## Why these three

`Spot` and `Directional` are not new features. Their schema is written, their clustering
exemption is written, their shadow fields are written, and `read_lights` refuses both by name.
The design is done and the compiler is the only thing standing in the way, which makes them
cheap to finish and expensive to leave.

Checkpoint 5 is first because it is the instrumentation the other two are verified with.
`CompileOutput.notes` is filled and never printed, so a scene that silently drops lights at the
cluster cap or runs out of shadow pages renders a plausible, wrong frame and says nothing. Every
later checkpoint here ends in "render it and look at it", and looking is how the bugs get found.

## This machine

There is a GPU adapter, and it is the only check that catches a whole class of bug: a binding
whose declared shape and actual shape disagree looks exactly right in a type check and renders as
the wrong texture.

```
cargo run -p whiting-device --features wgpu --example render -- <scene> out/<name>.png
```

`out/` is gitignored. naga 30.0.1 and wgpu 30.0.1 are in `Cargo.lock`, so shader validation is
pure CPU with no adapter:

```
cargo test -p whiting-scene --test wgsl_validation
```

## Checkpoint 5 — silent errors and where configuration lives

Five changes, no shader work, no new rendering.

### 5.1 Print the notes

`crates/device/examples/render.rs` already prints a per frame line at about line 130. Print
`compiled.notes` beside it, once per compile rather than once per frame, so a scene compiled for
two cameras does not print its warnings twice. A note is a `CompileNote` with a `path` and a
`message`; print the path so a note points at a line of a scene document rather than at nothing.

The reason this is not cosmetic: a host that ignores `notes` renders a frame where the cluster cap
quietly dropped lights or the page pool ran out, and the author has no way to learn it from the
image.

### 5.2 A scene with no light must render

This is a bug and it is pre-existing. `light_records` in
`crates/scene/src/lighting/buffers.rs:549` reserves `lights.len()` records, so an empty light
list finishes into a four byte buffer, and the shader's `array<LightRecord>` wants a whole 96
byte record. wgpu refuses the draw with a validation error naming a buffer size.

The fix is a minimum of one record when the list is empty. The stage 5 test worked around this
with a point light of **zero intensity**, and that workaround is in a test file while the reason
is in a comment; both should go once the bug is fixed.

Decide what the placeholder record is. A zero colour and a page of `NO_PAGE` is a record no
cluster points at, which is the same answer the environment light's own record already gives.

### 5.3 The page pool is a frame property, not a clustering property

`read_light_pool` reads `shadow_page_width`, `shadow_page_height` and `shadow_pages` out of the
**clustering** block (`buffers.rs:432`). Move them to the **frame** block, which is where the
depth buffer already lives.

Why this is not cosmetic: a page's size and count are properties of the frame being rendered, and
they are declared where a reader expects per frame resource sizes, not where they expect the
cluster grid. A scene author reading their own document should be able to tell which numbers are
about this view and which are about how lights are sorted into it.

This changes the document schema, so it changes the seven gallery scenes and the tests that name
these fields.

### 5.4 Ask the adapter for its real limits

`required_limits` in `crates/device/src/wgpu_renderer/mod.rs:72` builds its request from
`wgpu::Limits::downlevel_defaults()` and overrides only the two vertex counts. So
`max_texture_dimension_2d` is the downlevel floor of 2048 whatever the hardware can actually do,
and a scene cannot ask for a page larger than 2048 on a machine that could give it 16384.

Base the request on `adapter.limits()` and keep the two vertex overrides, which exist for a
different reason and should stay.

**The coupling to watch**: `MAX_TEXTURE_ARRAY_LAYERS` in `lighting/buffers.rs:450` is a compile
time constant of 256 that the compiler refuses a scene for exceeding, and
`crates/scene/tests/wgsl_validation.rs:111` asserts it. That const is currently a statement
about the downlevel floor. Once the device asks for the adapter's real limits it is a statement
about nothing, and a scene on a machine that can do 1024 layers is refused for asking 300.

Do **not** raise the const as part of this checkpoint. A const that tracks the hardware is a const
that is wrong on the next machine, and the honest shape is a device side clamp that emits a note,
which is 5.1's machinery and a separate decision. Leave the const where it is, and make its
documentation say that it is a floor this compiler enforces rather than a ceiling the hardware
imposes. Record the clamp design in
`.agents/proposals/adapter-limits-and-the-page-pool.md`.

Also update the `wgsl_validation.rs` header comment, which asserts that the renderer asks for
`downlevel_defaults()`. After this change that is no longer quite true, and a test whose stated
premise has gone stale is worse than no test.

### 5.5 Nothing in the shader changes

If checkpoint 5 touches a WGSL string, something above was misunderstood.

## Checkpoint 6 — Spot

A spot light stands somewhere and shines inside a cone. It takes a cluster slot, which
`LightKind::takes_cluster_slot` at `crates/scene/src/standard/emit.rs:419` already says, so it is
drawn through the cluster list like a point light.

### The decision that was made, and the shape it has to take

The cone data — `direction`, `inner_angle`, `outer_angle` — goes in a **second storage buffer**,
not in the light record. The light record stays 96 bytes and 24 floats, so the point light path
that the megalights work exists to serve pays nothing for a feature most scenes do not use.

That choice was made against the argument that a conditional binding is a type erasure in the
shape the texture array debate already rejected, and the objection stands. It is workable under
exactly one condition, and this is that condition:

**The binding point is declared unconditionally.** The shader's resource list is positional, so a
binding that comes and goes moves every number after it. `shaders.rs:302-306` already states this
for the environment map, and the environment map is the precedent: it is bound by every lighting
pass including one whose scene names no environment, and the document binds a one texel black
map. A spot cone buffer is bound the same way, and a scene with no spot light gets one zero cone
that no cluster points at. The bandwidth is still zero, because nothing reads it.

If the binding were made conditional, this would be a worse version of the problem rather than a
different one.

### What the shader has to compute

Attenuation between the inner and outer angle, and zero outside the outer one. The angles are half
angles in radians, and the schema requires both rather than deriving one, so the buffer carries
both. Between them the light is scaled down; a smooth falloff is the author's choice of the two
numbers, not a curve this renderer picks.

The cluster assignment already culls a spot light by its `radius`, so a light outside the cone is
already not in the list. The cone is an extra attenuation on top of that, not a replacement for
it.

### What the compiler has to read

`read_lights`'s `_` arm at `emit.rs:916` reads `position`, `shadow_extent` and `radius`, which is
a spot light's field set exactly. It needs `direction`, `inner_angle` and `outer_angle` as well,
and `is_emitted` at `emit.rs:436` has to admit `Spot`.

Refuse a spot light whose `outer_angle` is not greater than its `inner_angle`: the cone would be
inverted, and a light whose cone is inverted is a document whose two answers disagree. Refuse
`outer_angle` above a half turn for the same reason a zero radius is refused. `direction` needs no
normalisation check, but a zero direction is not a direction and should be refused rather than
turned into a `NaN` in the shader.

### The shadow volume

A spot light's shadow is the volume its cone cuts out of the scene, so the page is rendered
through a perspective projection the cone's outer angle sets. `shadow_extent` keeps its existing
meaning for a point light — the half width of the volume — and for a spot light the cone is the
narrower of the two. Decide which of the two wins when the author has drawn a wide cone inside a
narrow `shadow_extent`, write the answer in `SPEC.align.md`, and hold the document to it.

## Checkpoint 7 — Directional

A directional light comes from a direction rather than from a place. It covers the whole view at
every depth, so every cluster wants it equally and a cluster slot spent on one is a slot not spent
on culling anything. `takes_cluster_slot` at `emit.rs:419` already exempts it, and
`crates/scene/src/lighting/assignment.rs:298` already exempts it from every cluster list while
still emitting its record.

### Where its data lives

Not in the light record. The record is read through a cluster list, and this light is in no list,
so a fragment has no path to its record. It is a **stated input of the lighting pass**, the same
way `environment` is: a field of the `Uniforms` struct in `crates/gallery/src/shaders.rs:85`,
carrying its colour, its direction, and whether the scene named one.

Every fragment evaluates it unconditionally. That is the correct cost: there is no culling to do
and no light to choose between, and it is one BRDF evaluation beside a loop that already runs.

A uniform field is 16 bytes, and this wants a colour vector, a direction vector, a page index and
an orthographic matrix. A matrix in a uniform is fine; a thousand light records in one is not,
which is the reason the records are a storage buffer at all. Check the total against the 16 KiB
uniform floor and say so in `SPEC.align.md`.

The record this light is still given is dead weight, and it is kept on purpose: it keeps one light
one record, which is what makes the cluster indices a plain index into an array, and the proposal
at `.agents/proposals/environment-light-limits.md` item 4 already argues for keeping it for the
same reason. A light that is in no cluster list has no page either.

### The shadow volume

Orthographic. The half width comes from `shadow_extent`, and the near and far planes are derived
from the scene bounds — this was decided and is not open. An orthographic projection fitted to
the bounds is what makes a sun's shadow the same sharpness across the whole scene rather than
sharp near the camera and soft far away.

The eye is on the direction's side of the bounds, far enough back that the whole bounds is in
front of it. Getting the near plane wrong puts geometry behind the eye, and a page rendered from
behind a scene is a page of nothing.

### Cluster exemption and the page pool

An exempt light is a candidate for a page like any other. `reached` in
`lighting/buffers.rs:289` is read off the candidate walk, so a light that reached no cluster has
`reached == false` and is given no page. A directional light reaches **every** cluster, so it must
enter the candidate walk as a candidate that is exempt rather than as a light that is skipped, or
it will never be given a page and its shadow will silently not exist. This is the one place
checkpoint 7 can be wrong in a way that renders fine.

## What is deliberately not in these checkpoints

Recorded in `.agents/proposals/environment-light-limits.md` and left there: environment map
rotation, specular environment and split sum, prefiltered specular, roughness related specular
lobe reuse, the map's exposure, seam aware filtering of the level chain, and the octahedral seam
clamp's missing coverage.

`max_texture_dimension_2d` reaching past the 2048 floor is in 5.4, but only as far as asking the
adapter; what to do about a scene that asks for more than the hardware has is a proposal, not
this branch.

## Where the reasoning lives

- `.agents/SPEC.align.md` — the reasoning for clustering, the texture array, and image based
  lighting. Never merged. Add to it as each checkpoint lands.
- `.agents/proposals/environment-light-limits.md` — what image based lighting does not do and what
  each item costs. Checkpoints 5 and 7 will remove items from it.

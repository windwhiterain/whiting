# Debug channels: `Depth`, and the multi-attachment pass that a shader graph will need

Status: **item 3 partly landed. `Normal` and `Identifier` are shelved by decision** — see
*Shelved* below, which supersedes the reasoning under *Still owed* but does not delete it, because
the emitter analysis there is what a graph would be built on.

`Depth` is landed and verified. The renderer's multi-colour-attachment support is landed and
verified, with a test on a real adapter. `Normal` and `Identifier` are **not** landed.

Item 3 of [agent-scene-iteration-loop.md](../proposals/agent-scene-iteration-loop.md). The framing
contract is [agent-debug-scenes.md](../proposals/agent-debug-scenes.md). The contract itself is
recorded in [SPEC.align.md](../SPEC.align.md) under the `DebugChannel` entry, alongside the
correction that a G-buffer needs **no** graphics-schema change.

## Shelved — `Normal` and `Identifier`, and the line they sit on

**Decision, 2026-10-02: shelve both, and shelve the whole idea of adding a channel per value until
there is a shader graph.** The reasoning, which is better than anything under *Still owed* and
replaces it:

> A debug texture is only general once a shader graph exists and any value inside a shader can be
> wired to an output texture. Until then, every additional channel is a hand-wired special case.

That draws a line through the existing set, and the line is worth stating precisely because it
says the three landed channels are **not** part of what is shelved.

**A landed channel reads a frame resource. A shelved channel reads a shader's computed value.**

| channel | reads | category |
| --- | --- | --- |
| `Position` | the depth buffer, recovering a world position from it | frame resource |
| `Depth` | the depth buffer, verbatim | frame resource |
| `Grade` | the shaded image, the frame before the post stage's grade | frame resource |
| `Normal` | **a value the fragment shader computed** | shader value |
| `Identifier` | **a value the fragment shader must be told to write** | shader value |

The first three need nothing from inside a shader: they copy or reinterpret a buffer the pipeline
already produced, and they are answerable by the channel machinery as it stands. The last two reach
into the shading itself, and that is where the one-channel-per-value cost starts: a new one is an
enum variant, a format, a sidecar description, a coverage contract, and a pass the emitter has to
know how to insert.

**So the cost is not linear, it is structural.** A graph makes "wire this value to that target" a
wiring decision rather than a compiler feature, and every one of those per-channel obligations
becomes an edge in a graph instead of a case in the emitter. Building the eleventh channel by hand
is strictly worse than building the graph and getting all of them at once.

**The multi-attachment renderer work that landed here is therefore substrate, not a detour.** A
graph whose outputs are several render targets needs exactly the capability that replaced the
fixed `[Some(ColorTargetState)]` array, and it is landed and tested on an adapter. Keep it; it is
the part that will be wanted first when the graph is built.

**Two things a graph will have to answer, recorded now so they are not discovered late.** One is
the emitter blocker below, which a graph dissolves rather than solves: whether the stage is known
is a question a node answers and a hand-written WGSL string does not. The other is a property
`striking-3d-scenes.md` lists among the framework's real strengths — *shaders carry their own
source, so a scene is self-contained and renders wherever it is sent*. A graph is not source, so a
graph has to compile to WGSL to keep that property, and the question of whether a scene carries a
graph or carries compiled WGSL has an RPC dimension: a document sent to the device is a document
the far end must be able to run.

## Files this work owns

None of these had another session's uncommitted changes when this work started, which is why it is
confined to them. **Do not widen the set without checking `git status` first**: `emit.rs`,
`render.rs` (the example), `camera.rs`, `math.rs` and `SPEC.align.md` were already modified in the
tree by the camera-notes work.

- `crates/scene/src/standard/debug.rs`
- `crates/device/src/wgpu_renderer/raster.rs`
- `crates/device/src/wgpu_renderer/mod.rs`
- `crates/device/tests/multi_attachment_render.rs` (new)

## Landed 1 — the `Depth` channel

**What it is.** The depth buffer, written as an image. It is the value every other channel already
recovers from, so it needed no new arithmetic and no new pass shape: it is a `DebugChannel` variant
like `Position`, and `debug_passes` in `emit.rs` emits it **with no change at all**. That is the
cheapest possible version of this item and it is worth not making it more expensive.

**The contract, which is `Position`'s contract.**

- Coverage: exactly what the scene's `depthing` bucket drew, because it *is* the depth buffer.
  Recorded on the module in `debug.rs`. The existing note in `emit.rs` (fired when a channel is
  asked for and the scene lists nothing under `depthing`) fires for `Depth` too, because it is
  written against `!debug.is_empty()` and not against one channel.
- Encoding: `rgba16float`, for a reason that is not `Position`'s. A depth value in zero to one
  quantised to eight bits steps by four thousandths — enough to see *that* a fragment lost a depth
  test, not enough to say how near it was.
- **`rgb` is the buffer verbatim, including `1.0` where nothing was drawn.** A cleared depth buffer
  holds one, so one is what that pixel reports. Zeroing it would be the opposite error to the one
  this module keeps refusing: it would report the *near* plane where there is no surface, which
  reads as "something is right in front of the camera" rather than as "nothing is here".
- **Alpha is the marker, and it is `Position`'s marker rather than a second convention**: 1.0 where
  the scene drew a surface, 0.0 where it drew none.
- **Every channel gets the same interface, including a uniform `Depth` does not read.** The emitter
  hands every channel's pass the camera and the projection (`emit.rs:3033`), so an interface
  declaring less would describe a pass the compiler does not emit. This is why `DEPTH_SOURCE`
  declares `Uniforms` and never reads it.

## Landed 2 — more than one colour attachment on a pass

**The brief's claim was right.** The schema and the binding layer already supported several
targets: `attachments()` in `raster.rs` already resolved a pass's `targets` into a `Vec<Attachment>`
in attachment order, and `bindings.targets` was already a collection. The renderer collapsed them
in exactly two places, and only two — `raster.rs:91` took the **first** non-depth attachment with
`.find(...)`, and `raster.rs:157` built a **fixed one-element** `colour_targets` array. Both are
gone. A pass with N colour attachments builds N `ColorTargetState`s and N colour attachments in
the same order, and a shader's `@location(i)` is that order by construction.

**Three consequences that were decided rather than discovered:**

1. **`run_rasterize` returns every colour target it wrote**, so `render_outputs` can find a declared
   output by point.
2. **The "absent `outputs`" convention changed meaning for a document that has more than one
   attachment and declares nothing**: the frame is now the **last** attachment of the last colour
   pass rather than the first. Every document that exists today has one colour attachment per pass,
   so every one of them answers exactly as it did — verified by SHA-256 below, not argued. The
   change is only visible to a document that writes a G-buffer, and it is recorded in the doc
   comment on `execute` in `mod.rs` rather than guarded against, because the guard would be a schema
   change this work does not make.
3. **A depth-only pass is still depth-only.** `fragment` is still `None` when the colour list is
   empty, which is what the backend requires; the depth attachment is still the first depth one.

## Still owed — `Normal` and `Identifier`, and why they stopped

**They need a geometry pass, and the blocker is in the emitter, not the renderer and not the
schema.** The renderer half is landed and tested. Three facts, all verified in the code:

1. **A pass draws one mesh.** `raster.rs` binds one vertex buffer and issues one `draw`. A G-buffer
   over the scene is therefore **one pass per `Group`** — and `Group` (`emit.rs:2386`) is keyed by
   `(members, mesh, shader **label**, stage, light, images)`. A scene's `dunes` and `tidewater`
   have several; a scene with many instance groups has many passes.
2. **A group's shader is a label into the scene's own shader list.** That is the whole problem. The
   compiler cannot generate a G-buffer shader for a group without either asking the scene for its
   shader — which is the answer the brief rules out, because it makes **every existing scene
   change** — or reproducing the instance row's transform in generated WGSL.
3. **The transform part is tractable; the position part is the risk.** The instance row's
   `translation`/`rotation`/`scale` are *standard* instance fields, so they are the same for every
   scene and a generated shader can read them. The point layout is not: a mesh's `points` schema is
   arbitrary, and the emitter's own existing convention is `first_packed_components`
   (`emit.rs:3911`) — "the first three numbers in layout order are the position". A generated
   shader can follow that convention, and a faceted normal then comes from
   `normalize(cross(dpdx(world), dpdy(world)))`. **But** it has to reproduce the scene's vertex
   transform and `@location` numbering exactly, or the normals are quietly wrong. The branch
   `feature/dsf-bamboo-courtyard` holds `lighting_shader_with_schema` (+81 lines in
   `gallery/shaders.rs`) — the same problem for the same reason — and it is **not merged**. Decide
   whether to depend on it, or to reimplement it in `standard/`, before starting.

**The identifier answer in the brief is right and should be built:** a compiler-generated shader
writing a **constant per instance group**, so no existing scene changes. It needs (1) first, plus
two things that do not exist yet:

- a scene-wide unique index per group — the standard pipeline computes none;
- an integer target. `read_back` **refuses a depth format** (`raster.rs:329`), and `texel_bytes` /
  `decode` in `render.rs` know `r32float` but not `r32uint`, so `r32uint` needs work in
  `render.rs` before an identifier can be read back at all.

**Which coverage contract applies — and it is not `Position`'s.** `Normal` and `Identifier` written
by a G-buffer in a geometry pass are **not** bound by the `depthing` contract: a geometry pass
covers exactly what it drew. They owe a *different* statement — **which buckets the pass
enumerated** — because their coverage is a property of the emitter's enumeration rather than of the
depth buffer. That is written into the module doc in `debug.rs` now, before the channel exists, so
the day one is built the contract is written rather than discovered. The declined alternative is in
[agent-debug-scenes.md](../proposals/agent-debug-scenes.md) under *Considered and declined*, and the
cost it names there is accurate.

## Verification — what was actually run

Machine has a real adapter (`NVIDIA GeForce RTX 3060 Laptop GPU`), so the device suite really ran.

| check | result |
| --- | --- |
| `cargo build` | clean |
| `cargo clippy --workspace --all-targets` | 34 warnings, all in `definition`/`scene`/`gallery`, **none in a file this change touches**. `whiting-device` had 5 device-side warnings before and 4 after — the sibling `needless_borrow` in the colour block was fixed by writing `entry.slot`. Nothing added. |
| `cargo test --workspace` | **104 passed, 0 failed** |
| additive property | SHA-256 of `dunes`, `tidewater-camera0`, `tidewater-camera1` with **no channel**, before vs after this change: **all EQUAL**. Verified by reverting only the three files (`git stash push -- <paths>`, never a bare `git stash` — the tree holds another session's work), rebuilding, and re-rendering. |
| picture invariant | `tidewater-camera0.png` = `fde43a85…` and `camera1` = `b966e28a…` — **identical** across no-channel, `position` and `depth`. The channel PNGs are new and differ from each other (`position` `20855ffa…`, `depth` `ef61ac09…`). |
| `dunes` depth image | sky transparent, ridges layered far-to-near; covered range `[0.993, 1.000]`, background `-` at every sampled sky texel. `Position` at `(511,511)` still reports `(7.320, 19.141, 18.797)` — unchanged. |
| `cargo test -p whiting-device --features wgpu --test multi_attachment_render` | **3 passed** on the adapter |

`multi_attachment_render.rs` is the test the brief asked for. It renders one pass into **two**
colour targets of **different formats** (`rgba8unorm` and `rgba16float`), declares both in
`outputs`, and asserts: both come back under their declared names and their own formats; the two
hold **their own values and not each other's** (the second holds `-2.75`, which an 8-bit target
could never hold, so a swapped pair fails rather than merely looking odd); and a one-attachment
pass answers byte-for-byte as it did before.

## A harness defect this work found and did not fix

**The sidecar header is hardcoded to `Position`.** `probe()` in `crates/device/examples/render.rs`
writes the literal line *"world position of the nearest surface each pixel shows, in scene units"*
for **every** `rgba16float` image, so `dunes-depth.txt` claims to hold world positions in scene units
when it holds depth values in zero to one. The grid of numbers below that line is correct; only the
prose is wrong. `render.rs` is off-limits to this work and two other changes landed in it today, so
this is reported rather than fixed. **The fix is to make the header a property of the channel** —
`DebugChannel` already knows what it is — rather than a string in the harness.

## Corrections to the proposals, found while building this

- **`agent-debug-scenes.md` says a normal/identifier "needs a G-buffer — a graphics-schema and
  renderer change of its own".** The renderer half was two lines and is now landed and tested. The
  **graphics-schema half is not needed at all** — `pass.targets` was already a list and the binding
  layer already carried several. What is actually missing is *emitter* work, which that document
  does not name.
- **`agent-scene-iteration-loop.md:120` says `Identifier` covering only the `depthing` bucket "is a
  contract the same way `Position`'s is".** That is only true if the identifier is recovered from
  depth, and a G-buffer identifier is not. Building it the G-buffer way makes the sentence wrong, so
  it should be amended rather than implemented.
- **The brief's claim that `texture_channel_render.rs` is "the precedent for asserting a debug
  channel actually works" is wrong.** It tests *texture* channels — a sampled image reaching the
  fragment stage through a sampler. **There is no test anywhere in the workspace that asserts the
  `Position` channel works**, only that it compiles and emits. That is a bigger gap than this
  change's own missing tests, and it is why step 4 above is a measurement rather than a test.

# Rendering 3D scenes with this system: what stands in the way, and what to change

Status: proposal, **approved for implementation**, with the decisions recorded under
[Decisions](#decisions). The evidence below comes from reading the code and from rendering the shipped
example on a real adapter (an RTX 3060, through `wgpu`).

## Decisions

Taken on review, 2026-06-30:

- **All four tiers, then the scenes.** The graphics schema may change.
- **`time` is a field of the scene document**, and the render harness patches it across a range to
  produce a range of images spanning a period. It is not a device clock: a render stays a pure
  function of `(documents, time)`, so a snapshot taken at `t = 2.5` reproduces exactly.
- **No further constraints.** The work is judged on the images and on whether the framework changes
  are ones that would be accepted upstream.

## The question this answers

The system is asked to build visually striking 3D scenes. The framework itself is strong — a value
tree with a schema language, sparse versioned deltas, a hot-reloadable headless device, and a
`Compiler` that turns a scene document into graphics instructions. The block is not the framework. It
is the size of the vocabulary the standard pipeline currently has.

## What is already good, and worth building on

These are real strengths, verified in the code, and none of them need to change:

- **The point schema is arbitrary.** A mesh declares the shape of its own point — position, normal,
  texcoord, anything — and both the packer and the vertex layout are derived from it
  ([layout.rs](../crates/graphics/src/layout.rs)). Any vertex format is expressible without touching
  the framework.
- **The instance row is the shader's own declaration.** A row is the shader's `instance_schema`
  merged with the material's `custom_schema` ([emit.rs](../crates/scene/src/standard/emit.rs:465)).
  So per-object material variation is a scene author decision, not a framework feature request.
- **Instancing works.** Instances sharing a mesh, a shader, a stage and a light are drawn together
  by one pass and one draw. A field of a thousand identical objects is one pass.
- **The uniform layout is correct and verified.** `uniform_layout` lays a schema out the way a
  backend does, and the device parses the shader back with `naga` and compares field names and
  offsets ([uniform.rs](../crates/device/src/wgpu_renderer/uniform.rs)). A shader that reads its
  uniform wrongly is a render error naming both offsets, not a wrong image. This is the single best
  piece of engineering in the workspace and it is what makes writing WGSL against a document safe.
- **Shaders carry their own source.** A scene is self-contained and renders wherever it is sent.

## What stands in the way

Ordered by how much each one costs a scene. Every row is a fact about the current code.

### 1. There is no perspective projection

[`projection_of`](../crates/scene/src/standard/math.rs:115) always builds an **orthographic**
matrix. `orthographic` is the only projection in the crate. Foreshortening, depth convergence and
scale falloff — the things that read as "3D" rather than as a technical illustration — are all
properties of a perspective divide, and there is no perspective divide anywhere in the system.

### 2. The camera is a hardcoded convention, not a decision

[`camera_projection`](../crates/scene/src/standard/emit.rs:881) puts the eye at
`centre + 3 * half_width` on `+Z` and looks down `-Z`, framing the axis-aligned box over every
instanced point. Every scene is therefore a dead-front elevation, and a scene whose geometry is
wide and shallow gets a tiny subject in a large empty frame. The comment at that site already
concedes the point: *"an author who wants a camera writes one and the convention goes away."*
There is no camera field in the scene schema to write one into.

### 3. The lighting pass cannot shadow, and the example pretends it can

The lighting uniform is `projection` (the **camera's**), `light_position` and `light_color`
([pass_schemas.rs](../crates/scene/src/standard/pass_schemas.rs:142)). The **light's** view
projection is never supplied to any lighting pass, so a fragment shader has no way to project its
world position into the shadow map. The shipped example works around this by reading
`textureLoad(shadow_map, vec2<i32>(fragment.xy), 0)` — that is a screen-space lookup of a
light-space texture, which is not a shadow. The pipeline renders a shadow map it then cannot
correctly consume.

### 4. Nothing can move

The uniform contents of every pass are fixed by the pass schema and verified against it. There is
no time, no frame counter, no clock. Every frame the device produces is identical to the last one.
Water does not flow, nothing orbits, nothing pulses.

### 5. Pipeline state is hardcoded in the renderer

In [`raster.rs`](../crates/device/src/wgpu_renderer/raster.rs:101), three things are constants of
the backend rather than data of the pass:

| State | Current | What it forbids |
| --- | --- | --- |
| Blend | `BlendState::REPLACE` | additive glow, alpha, any compositing |
| Depth | `write: true, compare: Less` | testing without writing, so a fullscreen pass cannot sample the depth it tests against; no transparent draw ordering |
| Samples | `MultisampleState::default()` | no MSAA — every edge is hard aliased |

### 6. There is no post stage

`ScreenShader` exists in the scene schema and the standard compiler **refuses** it
([emit.rs](../crates/scene/src/standard/emit.rs:661)). So there is no bloom, no tone mapping, no
vignette, no FXAA, no god rays, and no fullscreen raymarching.

## The plan

Four tiers. Each lands, renders, and is separately reviewable, so the work stops being useful at
whatever tier the answer stops being yes.

### Tier 1 — the camera, the projection, and shadows

Three changes to the scene schema and the pass schemas. No renderer change, no new pass kind.

1. **A camera in the scene.** `{ eye, target, up, fov, near, far }`, optional, defaulting to the
   current convention so every existing scene still compiles to the same frame. A scene that writes
   one is a scene that chose its composition.
2. **A perspective projection in `math.rs`**, and `projection_of` gains the field of view it needs.
   The orthographic path stays for a scene that asks for it.
3. **`light_projection` in the lighting uniform**: the light's view projection and the shadow map's
   size. This is what turns the shadow map from a texture nobody can sample into an actual shadow,
   and it is the input a PCF kernel needs.

Together these are the difference between a diagram and a photograph.

### Tier 2 — time

The scene gains a `time` field that reaches every pass uniform. The scene document stays the only
source of truth: a render is a pure function of `(documents, time)`, so a snapshot taken at `t=2.5`
reproduces exactly. The device gains a `tick` endpoint that advances it and re-renders, reusing the
delta path the framework already has. A scene that never reads `time` is unaffected.

### Tier 3 — pipeline state as data

`blend`, `depth` and `samples` move out of the renderer and into the pass document, defaulting to
today's behaviour. This is the change that touches the graphics schema — the core of the spec — so
it is worth its own review.

### Tier 4 — a post stage

The standard compiler emits a final fullscreen pass driven by a `ScreenShader`, reading the colour
target and the depth buffer. It is what makes bloom, ACES tone mapping, vignette, FXAA and
fullscreen raymarching expressible, and it is the tier with the most leverage per line written.

### Tier 5 — the scenes

Once the vocabulary is there, the scenes are the cheap part. A gallery of procedural scenes built
against the library in `crates/scene/src/gallery/`, each a document rendered to a png by
`crates/device/examples/render.rs` (both already working — the image above is its output).

## What is deliberately not proposed

- **Vertex pulling, compute-driven geometry, or an entity graph.** The instancing the scene schema
  already has covers the scenes in question, and an entity graph is a much larger change than the
  problem needs.
- **A texture pipeline.** A mesh carrying texcoords and a material carrying a sampled albedo is the
  obvious next step after this, and it belongs to the same review as `blend`, not before it.
- **Anything that weakens the delta contract.** Each tier above is additive: a scene that uses none
  of the new fields compiles to exactly the documents it compiles to today.

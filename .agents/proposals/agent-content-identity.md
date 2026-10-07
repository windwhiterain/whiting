# Content identity: a rendered pixel resolves to the code that authored it

Status: **landed.** The measurement, the framework's two changes, the belt's parameter space, the
shader template, the landscape scene and `locate` are all built, and every step of it was checked
additively by rendering before and after. This file is the design and the argument for it; the
*decisions that were wrong at first* are under *What the building found*, and those are the part
worth reading.

## The gap

An agent authoring a landscape looks at a picture and has nothing else to look at. "That crest is
too rounded and there is a shelf under it" is a sentence about a picture, and the agent's only tool
is the picture. Every other surface it has is a document or a source file, and the join between
the two is a pair of eyes.

The proposed answers to this have been a **selection tool in the renderer** and **text or colour
annotation in the scene document**. Both are wrong in the same way, and the way is worth stating
before the design that replaces them.

- **A selection tool is the wrong shape for the reader.** An agent has no cursor, so any picker
  degenerates into a `pick(x, y)` query. The tool was never the thing; the query was.
- **The renderer cannot answer the question even when asked.** It knows a mesh and a triangle, not
  a `RidgeId`. A GPU picking pass returns `mesh=massif, triangle=4213`, and the step from 4213 to
  an authored path is not a rendering fact at all — it is the mesh's own provenance, which the
  crate that built the mesh already holds.
- **Text labels are a map for eyes.** An agent reads text better than it reads a picture, so
  drawing `Ridge(3) node 7` into the frame asks it to use its weakest instrument for the job it
  is best at. It is also the most expensive thing to draw: a font atlas, signed distance fields, a
  quad per label and occlusion, over a frame the labels then cover.
- **A scene's own shader is the one writer the framework already trusts.**
  [typed-outputs.md](typed-outputs.md) split outputs into **compiler shortcuts** — `position`,
  `depth`, `grade`, values a camera and a depth buffer make available whatever the scene does —
  and **scene-declared outputs**, which a scene fills with its own WGSL. The decision was recorded
  as *"the value side stays where it is: the scene writes WGSL"*, and that is the whole of what
  this proposal uses.

[handoffs/gbuffer-channels.md](../handoffs/gbuffer-channels.md) shelved `Normal` and `Identifier`
on the reasoning that *"a debug texture is only general once a shader graph exists"*. That
reasoning holds for a **compiler-generated** G-buffer, whose blocker is that the emitter cannot
reproduce a scene's vertex transform. **It does not hold for a scene that writes the value
itself**, which is the case here: the massif's lighting shader is already hand-written WGSL in
[crates/gallery/src/landscape.rs](../../../crates/gallery/src/landscape.rs) and needs one more
`@location`.

## The shape of the answer

The mesh is **already the index**. Every face carries the belt it was expanded from and the two
lines that bound it
([`FaceAttributes`](../../../crates/mountain/src/facet.rs#L34-L47)), and the expander assigns every
vertex a `(line, sample)` pair. The only hop that does not exist is *pixel → that face*.

So the answer is four hops, and the design question is only **who owns each one**:

| hop | from | to | owned by |
| --- | --- | --- | --- |
| 1 | a pixel | `id`, `u`, `v` | the frame, read off a rendered image |
| 2 | `id` | an authored path | the content that allocated the id |
| 3 | `id` | a belt, and `(u, v)` to `(s, t)` | the mountain crate, from the mesh's own provenance |
| 4 | a belt | the two lines and stations that bound it | the skeleton, already |

**Hop 3 is a lookup, not a second channel.** `FaceAttributes.belt` is in hand before a pixel is,
so the id the pixel carries names a row in a table the crate already has. Putting the belt id *in*
the image would add a fifth number that has to agree with that table forever — a second place for
it to be wrong, and the whole point of the design is that it cannot be.

## What is in the pixels, and what is not

Three numbers, and the test for each is **could this be recovered anywhere else?**

| | in the image | why |
| --- | --- | --- |
| `id` | **yes** | only the content that allocated it knows what path it names |
| `u`, `v` | **yes** | the rasteriser interpolated them; nothing else holds them |
| belt id | **no** | hop 3, from the mesh the crate already holds |
| `s`, `t` on each line | **no** | `Station`, derived from the line rather than stored |
| path | **no** | hop 2, from the content's own table |
| face index | **no** | an index into a build that the next edit invalidates |

The last row is the one worth arguing. [SPEC.align/mountain.md:276-278](../SPEC.align/mountain.md#L276-L278)
already says *"face indices shift after any edit; provenance is the stable handle"*, and this
design takes that literally: **no index is ever a handle.** What a pixel carries is an id the
content allocated, and the id means whatever that content's current table says it means. The
table is written by the same run that wrote the frame, and the manifest ties them together.

## The three decisions

### 1. The id is allocated by the content, globally, with zero reserved

The id space is **the content's own**, not the mountain crate's and not the compiler's. Zero is
reserved for *"this pixel is not an authored element"*, which is what the plain writes in a
landscape and what the sky writes everywhere.

The alternative — a per-mesh face ordinal — was rejected because it forces a `part` dimension into
the image, and because a face ordinal is not stable while an id the content allocated can be made
to be. Adding a ridge does not renumber the ids of ridges that were already there.

The mountain crate keeps indexing by `RidgeId`/`ValleyId` exactly as it does now. The path is a
convention of the content, and the crate is not asked to know it.

**A path is not a mountain-crate feature.** `Massif` is one skeleton and one footprint, with no
naming and no grouping, and [SPEC.align/mountain.md:274-275](../SPEC.align/mountain.md#L274-L275)
lists multi-massif composition as deferred. Putting paths inside `Massif` is a change to
[SPEC/mountain.md](../SPEC/mountain.md), which is human-editable only. The content side owns the
hierarchy; the crate owns the geometry; the join is the table.

### 2. `(u, v)` is the mountain crate's single definition, and the scene writes what it returns

A belt is a triangle strip zipped between a top path and a bottom run
([`expand.rs`](../../../crates/mountain/src/expand.rs)), so it has a natural parameter space: `s`
along the top path and `t` across the strip. **The crate defines that space once** —
`FacetMesh::belt_uv(face, corner) -> (u, v)` — and a scene writes the values it returns into its
point data, three corners carrying three corners' worth.

The rule this replaces is the one that produces a second table: a scene and the crate each
choosing their own notion of `u`, agreeing until one of them is edited. **One function, one
definition, one consumer of it.** A `u` that meant "fraction along the belt's top path" in the
crate and "first barycentric coordinate" in the shader would make every answer off by a
rotation of the triangle, silently, and only on faces where the two differ.

### 3. Every lighting stage writes the output, and a stage that writes nothing says so with zero

A scene-declared output is bounded by **which passes enumerated the buckets that wrote it** —
the coverage contract [debug.rs](../../../crates/scene/src/standard/debug.rs) already states for
this half, distinct on purpose from a built-in's. A contract that names only the massif is a
contract that has to be believed: the plain is drawn after the massif, and where it covers the
frame the id image would still be showing the massif's id for a pixel that shows grass.

So **the output is a target of every lighting group**, and every lighting shader must write it.
That is not a convention, it is enforced: `check_outputs` already refuses a declared output the
fragment shader does not write, and `render_outputs` already refuses a declared output no pass
wrote. **A scene that declares the output and forgets to write it gets an error naming the
shader, not an image of uninitialised memory** — which is the hole
[typed-outputs.md](typed-outputs.md) says the check exists to close.

Zero is the sentinel, and it is the content's choice of what it means. "Not an authored element"
is the obvious reading and the only one that composes with a second massif drawn beside the first.

## What lands where

### Framework — two changes, one of them already made

**(a) A scene may declare an output. LANDED, see below.**

```ron
outputs: {
    authored_id: { location: 1, type: f32 },
    authored_u:   { location: 2, type: f32 },
    authored_v:   { location: 3, type: f32 },
}
```

**The scene names a `location` and a `type`, never a `point`.** A point is a binding point name,
and a binding point the compiler has to create an instance entry for; letting a scene name one
would let it name a resource that does not exist, or worse one that does. The compiler generates
`output_<name>`, builds the texture from [`TexelType::format`](../../../crates/graphics/src/output.rs#L234-L239),
sizes it to the frame, and adds it to `targets` of every lighting pass in the scene's own order —
so `@location(i)` is the compiler's numbering and the declaration's `location` is the same number
by construction rather than by the author keeping two lists in step.

**Three `f32` outputs rather than one `vec4`, and the reason is precision.** A half float has an
11 bit mantissa and represents integers exactly only to **2048**; a landscape has more belts than
that. An id of 4093 in an `rgba16float` comes back as 4092, and nothing says so. `f32` is
`r32float`, exact to 2^24, so the three numbers are written as three bare integers with no
encoding step and therefore no encoding to get wrong.

**(b) An attachment's blend follows its own format. LANDED, see below.**

### Content

One function that walks the authored hierarchy and produces **both** the id assignment and the
table, so the two cannot disagree. `faceted()` writes `id` through the material's `custom_data`
and `u, v` through the point schema. The table is written beside the images.

**Neither number needs a new vertex layout.** `u, v` are two more fields of a point schema the
scene already declares, and the layout is derived from it
([SPEC.align.md:336-341](../SPEC.align.md#L336-L341)). `id` is a number in a material's
`custom_data`, which the compiler already sorts by kind and packs into the instance row. And
`face_index`, should a caller ever want it, is `@builtin(vertex_index)` — which `check_vertex_inputs`
deliberately does not check, so it costs nothing.

## Staleness: four tables, one manifest

Every hop above has a table beside the image, and a stale table is worse than no table: it answers
with confidence and is wrong. The render manifest already exists and already records a content
hash per output. **The join tables are stamped the same way, and `locate` refuses to answer when
the stamp on the image does not match the stamp on the table it is about to use.**

The tables, and who writes each:

| table | written by | goes stale when |
| --- | --- | --- |
| `id → path` | the content, as it allocates | a node is added or renamed |
| `id → belt` | the mountain crate, per `build()` | any edit re-expands |
| frame ↔ tables | the render harness manifest | the compile that wrote the frame changed |

The second row is why `id → belt` cannot be folded into the first: **the mesh is rebuilt by every
edit**, so that table is a function of the build and must be written by the run that built it.

## One query, not four

The four hops are separate because four different things know them, and they are composed by one
function because **an agent that chains them itself will chain them wrong, and will chain them
wrong in the middle**, where the answer looks like an answer.

```
pixel (742, 508) -> id 137 (u 0.61, v 0.83) -> belt valley=2 left segment=4
                 top    ridge "west_large_mountain/left_chill_hills/ring" @ t=0.312
                 bottom valley "west_large_mountain/left_chill_hills/valley" @ t=0.287
                 -> massifs[0].skeleton.ridges[5].nodes[21]
```

The last line is the point of the whole thing. Not "can look it up" — **cannot look it up wrong.**

## Deliberately not proposed

- **A picking endpoint on the device.** A render is a function of the documents; the device holds
  no frame. The query is over the image the harness returned, which is where the sidecar machinery
  already is.
- **A shader graph.** The scene writes WGSL and that stays true. What is added is an output port
  the graph could later fill, not a value source.
- **Text in the frame.** See the top of this file.
- **A stable id across builds.** Achievable only with authored, persistent integers, which is a
  naming system with its own lifetime and its own conflicts. The id is stable for as long as the
  table that defines it, and the manifest says which table that is.
- **Naming inside `Massif`.** See decision 1.

## One hop that stayed off the pixels

The design put `id` and `(u, v)` in the image and looked up the belt from the id. What shipped
puts the **belt** on the CPU too, through `FacetMesh::locate`, and that is worth stating as a
decision rather than left implicit: the image carries the three numbers that **cannot** be
recovered elsewhere, and the belt is a lookup in data the crate already holds. Baking it in would
have been a fifth number with to agree with that table forever.

## What the building found

Six things the design above did not say, five of which cost a red run. They are the part of this
document worth reading, because each is a rule the design was relying on without knowing it.

### 1. A blend is judged on whether it is *present*, not on what it does

`raster.rs` built every attachment of a pass with one `blend_of(state.blend)`, and `r32float`
refused the pipeline with `Format R32Float is not blendable`. The obvious repair — give an
unblendable target a `Replace` state — **is refused with the same message**, because the backend
tests `cs.blend.is_some()` rather than whether the factors happen to discard the destination. An
unblendable attachment gets no state at all. The three 32 bit float formats are the whole of the
unblendable set, under the `float32-blendable` feature this device does not request.

### 2. `components_schema(1)` produced a schema its own reader refused

A one component field was a one element `List<Float>` with the `components` extension left off,
and `layout::components` **rejects** a list with no declared count because it has no layout. So
the one function that writes point fields produced one its own reader would not take. It is a bare
`Float` now, which is what the workspace's rule already says a one component field is in a
document. Nothing had called it with one until a scene declared a field of its own.

### 3. An identity must be carried `@interpolate(flat)`

A varying is interpolated, perspective-correctly, so the first version answered **99.999985** for
a face that is 99, and a caller that rounded it would be *choosing* a face rather than reading
one. Flat takes one vertex's value whole, which is exact and the only thing that is true — a
triangle is one facet of one element, so its three corners carry the same identity and there was
nothing to interpolate. The belt's `u` and `v` stay smooth, because a place on a surface is
exactly what interpolation computes.

### 4. Two meshes need two sets of four stage shaders

The stages read the **point layout of the mesh they draw**. A plain drawn by a shader built over
the massif's wider point schema reads three attributes of somebody else's vertices, and nothing
checks that, so the device caught it as a layout disagreement — the good outcome, and not one to
rely on. `mesh_shaders_over` takes a prefix so the two sets' labels do not collide, which is
what made the first attempt a silent overwrite rather than an error.

### 5. A declared output's binding point moves every number after it

`output_authored_id` sorts between `light_records` and `shadow_atlas`, so appending it to the end
of a hand-ordered list put `shadow_atlas` one lower than the pass built it. That is a frame of
the wrong numbers rather than a failure, and it is the mistake that once left the lighting pass
with no depth test at all. `lighting_bindings` now **sorts its list by name and reads the numbers
off the sort**, which makes a number a function of the name — the thing the device actually sorts
by. The list was already in name order by hand, so this is additive.

### 6. The point name is a rule three places would each have to agree on

The compiler creates the entry, the pass binds it, and the scene's shader must **declare** it in
its interface — a target is a binder the pass has and the shader reads nothing, exactly as
`color_target` is. That is one name in three places. `scene::standard::output_point` is the one
place it is written, and a disagreement is a compile error rather than a wrong picture, which is
at least honest.

## Verification

Every step was checked additively on the adapter by rendering before and after: `dunes`,
`landscape`, `tidewater-camera0` and `tidewater-camera1` are byte for byte the same across every
commit. `whiting-device --features wgpu`, `whiting-scene`, `whiting-gallery`, `whiting-mountain`
and `whiting-graphics` are green. The before/after renders were checked to have actually
differed, by confirming each revert took effect before rendering the baseline — an inconclusive
before/after is worse than none.

Two stale test pins were fixed on the way, both from `landscape` having been added to the gallery
without them: `clustered_lights.rs` and `scale_notes.rs`. Both were verified to fail with the
changes stashed, and both are updated rather than weakened, which is the same update
[agent-scene-iteration-loop.md](agent-scene-iteration-loop.md) records k3 making.

## Still owed

Nothing in the design. Two things are *available* and not built, and neither is a gap in this:

- **the join tables and the manifest stamping.** `Identity::of` is the one place ids and paths are
  produced together, so writing them beside the images is a call rather than a design. `locate`
  rebuilds the massif rather than reading a table, which is correct and O(one build) — a table is
  what makes it O(1) and what makes a stale answer detectable.
- **a second scene with its own naming**, which is what turns `identity::locate`'s `scene ==
  "landscape"` into a registry. One match arm is honest at one scene.

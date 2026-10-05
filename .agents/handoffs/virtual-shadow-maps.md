# Virtual shadow maps: per-object density, adaptive levels, frustum-clipped pages

Follows [virtual-shadow-maps.md](../proposals/virtual-shadow-maps.md), which carries the design and
the five stages. This handoff records what has landed, what the landed parts imply, and what the
next session has to decide.

Status: **all five stages landed.** A spot light's shadow is pages: the compiler sizes a grid,
clips it against the camera, refuses an atlas it cannot fit, emits one rasterize pass per page into
a shared depth array, writes the page table into the light record buffer, and the lighting template
walks that table to find the page a fragment's shadow lives in. `spotlight` renders as it rendered
before the work — the default density is calibrated to be the resolution the pool gave — and
`stilllife`, whose thirty five lights all stayed on the pool, renders **bit for bit identical**.
Nothing here is merged anywhere else.

## Stage 1 — `viewport` on a rasterize pass — landed

### Files this work owns

- `crates/graphics/src/pass.rs` — the `VIEWPORT` field name, its schema, and the field in the
  `Rasterize` variant.
- `crates/graphics/src/pipeline.rs` — the `Viewport` type and its reading, as a fifth member of
  `PipelineState`.
- `crates/graphics/src/lib.rs` — the two re-exports.
- `crates/device/src/wgpu_renderer/raster.rs` — the region applied, and refused when it reaches
  outside an attachment.
- `crates/graphics/tests/pass_viewport.rs` — new.

No scene or gallery file was touched by this stage.

### What landed, and the decisions inside it

The field is `[ x, y, width, height ]` in texels of the target, optional, read as part of
`PipelineState` rather than as a sibling of it. Two choices are worth carrying forward:

**The region is applied twice, as a viewport and as a scissor rect.** The proposal names both, and
the reason is in the pass documentation: a viewport moves where a fragment lands and does nothing
about a triangle that overhangs the rectangle, so a page drawn with a viewport alone would be
painted over by its neighbour's geometry. A renderer that reads the field as one of the two is
wrong, so there is one field and no `scissor` beside it.

**A region that reaches outside its attachment is a refusal, not a clip.** The alternative — clamp
it and draw what fits — gives a page that is partly recorded and partly whatever an earlier pass
left in the same texture, which reads as a soft shadow rather than as a broken one. This is the
same rule `planet_x` holds to for atlas overflow: precision is never silently lowered.

**The check is in the device and not in `graphics`.** A rectangle is four texel counts; whether it
fits is a fact about the bound target, which only the renderer knows. `graphics` therefore refuses
a zero-length side and nothing else about the numbers.

### What the next stage can rely on

A rasterize pass can now fill one tile of a shared texture without disturbing the tiles beside it,
which is the whole of what §6 of the proposal needs from the pass model. The one thing it does not
give is per-pass selection of which *layer* of an array a pass writes — that already existed, and
`crates/device/src/wgpu_renderer/raster.rs` keys its clear-or-load decision on
`entry#layer`, so the first pass into a layer clears **the whole layer** and later ones load it. For
an atlas that is the right behaviour once, at the first page of the layer, and it is worth
re-checking against the ordering the allocator emits pages in (see *Atlas layers* below).

## Stage 2 — `scene::vshadow` — landed

### Files this work owns

- `crates/scene/src/vshadow.rs` — new, the whole allocator: grid sizing, caster→level, page
  covering, frustum clipping, the page table, the atlas geometry, and `resolve` (the shader walk
  written in rust so the two cannot drift).
- `crates/scene/src/lib.rs` — `pub mod vshadow;`.
- `crates/scene/tests/virtual_shadow_maps.rs` — new, 14 tests.

No emitter, gallery or device file was touched by this stage: the module is pure and is not yet
called by anything.

### What landed, and the decisions inside it

`planet_x`'s encoding is reused verbatim (`TABLE_HEAD_WORDS = 2`, a `MAX_LEVELS` prefix of section
offsets, row-major slots, one base word plus `ceil(pps/32)` mask words per row), with the face axis
kept in the layout even though a spot light has one face, because a point light will need it.
`Allocation::resolve` is that layout's reader and is tested against the pages it was built from, so
stage 4's WGSL has a rust oracle to check itself against texel for texel.

Six places where the module deliberately departs from the reference or the proposal, all of which
the emitter has to honour:

**`LightVolume` holds a pure view and a pure projection, not their product.** The clipping test
needs the eye, the basis and each NDC side plane separately, and every one of those is a slot read —
`projection[10]` and `[14]` for near/far, `view[12..14]` against the basis *rows* for the eye. A
composed matrix has none of them where the code looks. So `spot_projection` in `emit.rs`, which
today returns `multiply(&perspective(…), &look_at(…))` as one matrix, has to hand the two pieces
back separately when stage 3 builds a `LightVolume`.

**One atlas array, whose layers are lights, with a slot counter that does not restart per level.**
`planet_x` keeps one array per level because its counter restarts; §5's header (one atlas side per
light) implies one array, and one array is one texture binding for the shader instead of four.
`Allocation::atlas` therefore returns a single `Atlas { side_pages, layers }` and `Page::slot` is
numbered across every level of the light. If stage 3 finds this costs a copy or a resize per layer,
the four-array shape is still reachable — the table header carries `atlas_side` and would carry a
level's own side unchanged.

**A caster allocates at its own level and at every level *coarser* than it** (`for level in
own..levels`), so a receiver that wants a page the caster's own grid does not have falls back along
an unbroken chain, and the finest page it does have is at least as fine as the caster asked for.
The proposal's §3 says "every other level down the chain", which is the same direction; the cost is
that a coarse caster is drawn into every page of the coarse levels, and a coarse page covers
`4^level` fine ones, so a dense scene's pass count is dominated by the coarse levels rather than by
the fine ones. Stage 3 should count passes on a real scene before committing to baselines.

**`density <= 0` puts a caster in the coarsest level only and excludes it from `max_density`,** so a
scene whose only density asks are zero still gets a grid sized by its geometry rather than a
degenerate one-page one.

**`resolve` walks one face, and says so.** The cube face a point's shadow belongs to is the one
thing the walk does not compute yet, and guessing would answer with a slot from the wrong direction,
which is a wrong shadow rather than a missing one, so a light with `faces > 1` hits a
`debug_assert` instead. Only a spot light is routed here, which is stage 3's scope; a point light
will want the face from the dominant axis of the view vector.

**The clipping test is divide-free and two-directional.** Each page's four NDC edges are evaluated
as `p[axis] + limit·tangent·p.z ≥ 0` in light *view* space, so a camera corner behind the light's eye
cannot produce a `NaN` verdict, and both signs of each side are asked, because a page can be clipped
on the far side of the plane the camera corner reached past. Only face normals are tested, which
over-allocates at grazing angles rather than losing a shadow — §4's stated direction of error.

### What stage 2 proved about the existing helpers

`Frustum::new` takes eight world-space corners, and the test file reconstructs them from an NDC
point and a distance through `place`, which is a useful enough inverse that stage 3 may want it
promoted. Its guard assertion (that `near_far` reads back the near and far it was built with) caught
the `[8]/[12]`-vs-`[10]/[14]` slot bug, so keep it.

## Stage 3 — `scene::standard` reads a density and emits pages — landed

### Files this work owns

- `crates/scene/src/standard/scene_schema.rs` — `shadow_density` on an instance, and the scene's
  two atlas ceilings `shadow_atlas_side_pages` / `shadow_atlas_layers`.
- `crates/scene/src/standard/emit.rs` — `VirtualShadow` and its `address_of`, `plan_virtual_pages`,
  `virtual_shadow_passes`, `page_pass`, the reshaped `shadowing_pass`, the atlas instance entry, and
  the call that attaches the table.
- `crates/scene/src/lighting/buffers.rs` — `LIGHT_VIRTUAL_FIELD`, `light_record_stride`,
  `record_table_base`, `Clusters::table` / `attach_shadow_table`, `ShadowAtlas`, `read_shadow_atlas`.
- `crates/scene/src/lighting/assignment.rs` — `ClusterLight::virtual_shadow` and `virtual_page`, and
  the record builder's new field.
- `crates/scene/src/lighting/mod.rs` — the re-exports.
- `crates/scene/src/standard/pass_schemas.rs` — `view_page` and the atlas binder on the shadowing
  pass.
- `crates/scene/tests/{spot_lights,directional_lights,clustered_lights,shadow_pages}.rs` — re-recorded.

No gallery or device file was touched: the pages are compiled and nothing shades from them yet.

### What landed, and the decisions inside it

**A light record is twenty eight floats, one hundred twelve bytes.** Four for the virtual address,
appended to a record that was twenty four. It is a `vec4` and not two floats because the record has
to stay a whole number of sixteen byte blocks anyway and the pad is there either way — so `.z` and
`.w` carry [`NO_PAGE`](../crates/scene/src/lighting/buffers.rs) rather than zero, and a shader that
reads a component it should not has the sentinel answer instead of "page zero". `.x` is the word
offset of the light's table section and `.y` its atlas layer.

**The table is appended to the record buffer, because four storage buffers is the whole budget.**
`record_table_base(lights)` is where it begins — `lights.max(1) * 28` — and a record carries that
offset *absolute* so the shader adds nothing. The emitter computes the base before it has the run and
`Clusters::attach_shadow_table` checks the two answers against each other, which is the only reason
the duplication is safe.

**An atlas layer is a plan index, not a light index.** A scene that mixes a sun with three lamps
holds three layers, and `VirtualShadow::lights` is the map from plan to light. Stage 4's shader must
read the layer out of `.y` of the record and never derive it from the cluster index.

**Every shadowing pass binds both depth arrays and every shadowing pass supplies `view_page`.** A
pool page passes the identity window `[-1, -1, 2, 2]` and a page pass passes its own rect; which of
the two a pass is told apart by the texture it names as its target. This is forced, not tidy:
binding numbers follow name order, so a binder that vanished in one shape of the pass would move
every point after it for a shader that is the same shader either way. It is the
`SPOT_CONES_ENTRY` rule restated one level up. The atlas instance entry therefore always exists, at
one page by one page by one layer when a scene allocates nothing — sixty four kilobytes to keep a
binding point honest.

**A light whose grid exists but holds no page carries the sentinel, not an empty table offset.**
Every one of its pages was outside the camera; walking an all-zero mask would fall off the end of
the chain and read whatever the atlas held before, which is a shadow in the wrong place.

**An instance that names no density renders at the resolution the pool gave it.** The default is the
pool's page width in texels across the width of the cone at the depth the grid is measured at
(`page_width / (2 · far · tan θ)`), which makes the finest level of a silent scene *the page it
replaced*, to the texel. The proposal's `page_width / (2 · shadow_extent)` does not: for a cone the
extent is the near plane's half-height, so that default sat a factor of the cone's angle away from
the resolution being replaced and `spotlight` came out far coarser than it was.

**Overflow is a refusal at every level.** No casters, a grid wider than `MAX_PAGES_PER_SIDE`, an
atlas with more slots than the ceiling, a scene with more virtual lights than
`shadow_atlas_layers` — each is a `CompileError` naming the light and both numbers.

### Measured, and worth knowing

`spotlight` went from **4 shadowing passes to 242**, and the number is right rather than a bug. The
lamp's cone is 17.63 world units wide at the depth the grid measures, the pool's own resolution asks
for 8 pages to a side, and four levels over an 8×8 grid is **85 pages, every one of them
allocated**; four of the scene's instances cast, and 242 is the sum over pages of the number of
caster *groups* covering them. `atlas_side` comes out at 10, the smallest square holding 85 slots.
A scene that wants the old count names a lower `shadow_density`. `stilllife` is unaffected — its
thirty five window lights are point lights, and point lights have not moved to the virtual map.

The same measurement says where the ceiling is: at a density of **60** texels a world unit, against
the default's 58.07, the lamp demands 324 pages and the atlas of 16×16 holds 256, so the compile is
refused. The grid side is a power of two, so the first step past the resolution the pool gave doubles
a side and quadruples the pyramid, and the escape the error message names — raise
`shadow_atlas_side_pages` — is the intended one. An author who does not know this reads the refusal
as a bug rather than as a budget; it is worth a sentence in whatever documentation the scene fields
eventually get.

`cover()` pads a caster's footprint by **two pages of its own level** on every side, which is what
makes the chain never break — and also means the coarsest level's grid is normally allocated whole,
so a virtual shadow map is sparse at the *fine* levels only. On an 8 page grid the margin alone is
five pages across, which is why `spotlight`'s level zero is fully dense: four casters, each padded
to a five-by-five block, cover sixty-four pages between them. Stage 4 should not assume a page
missing at level 3 anywhere, and the allocator test
`a_point_outside_the_volume_has_no_answer_and_one_inside_it_falls_down_the_chain` now states the
consequence out loud: the centre of the cone resolves at the coarsest level, because that point sits
on the corner four coarsest pages share.

## Stage 4 — the frame reads the table — landed

### Files this work owns

- `crates/gallery/src/shaders.rs` — `SHADOW_UNIFORM` and the page remap in `shadowing`, the
  `virtual_page` field of `LightRecord`, `light_record`, `shadowed_pages`, `shadow_term`, and the
  atlas among `lighting_bindings`.
- `crates/scene/src/standard/example.rs` — the same two shaders, second copy.
- `crates/scene/src/standard/pass_schemas.rs` — the atlas entry on the lighting pass schema.
- `crates/scene/src/standard/emit.rs` — `lighting_pass` takes the atlas and binds it.
- `crates/gallery/tests/shader_bindings.rs` — new, four tests.
- `crates/scene/src/standard/mod.rs` — `SHADOW_ATLAS` and `VIEW_PAGE` re-exported, for the stage 5
  test.

### How each open question went

**The record run is declared flat.** `light_records` is an `array<f32>` and `light_record(index)`
assembles the struct from twenty eight words at `index * LIGHT_RECORD_WORDS`. That removed the
coupling rather than testing it: the offsets are now written down in the shader, and
`shader_bindings.rs` holds them against `light_record_stride()` on the rust side. The table's mask
words are read through `bitcast<u32>`, which is bit exact and does not trap.

**`PLACEMENT_UNIFORM` no longer serves shadowing**, and the reason it could not is the same name
order rule stage 3 hit: the shadowing pass's uniform is `projection`, `view_page`, `time`, and
depthing and emission still share the two-field one. `SHADOW_UNIFORM` is the third shape.

**The vertex stage remaps, not just the viewport.** A viewport moves a fragment's *destination* and
leaves the depth it carries alone, so a page drawn with a viewport and nothing else holds the depth
of the whole light's plane rather than of its own window. The page's clip output is therefore
scaled about the window's centre by `2 / extent`, which is the identity for the pool page's
`[-1, -1, 2, 2]` and the page's own rectangle otherwise. `resolve`'s y-down fold, `window_of`'s
y-up window and wgpu's y-from-the-top viewport are one convention, and the algebra that says so is
in the comment above the remap.

**Bias scales with `2^level`, and the PCF taps are clamped to their own page.** The clamp is new to
this path and deliberate: outside a page a texel is a different rectangle of the same plane, so an
unclamped tap is a *wrong* depth rather than an absent one. The pool path keeps its old behaviour,
where a neighbour page's texel is somebody else's shadow and reading it costs a soft edge.

**The record decides, not the light's kind.** `shadow_term` asks whether `virtual_page.x` is the
sentinel and routes to `shadowed` or `shadowed_pages` from there. A kind is a scene's word for a
light and a page is the compiler's decision about it, and the two can disagree — a spot light whose
every page fell outside the camera is a spot light with no table to walk.

### The one bug the GPU suite caught, and what it says about the rest

`example.rs`'s `QUAD_SOURCE` hard-codes its binding numbers, and it still said `shadow_pool` at 8
and `spot_cones` at 9 after the atlas took 7 and pushed both up. The device suite refused the frame
with a message naming the collision. The gallery's own shader is generated from the binder list and
could not be wrong the same way, so the guard for the hand-written copy is
`the_lighting_stage_declares_its_bindings_in_the_order_the_device_numbers_them`, which is a CPU test
and cannot see a device: `check_resources` compares a shader's declarations with what a pass *binds*,
not the **order** the numbers came out in, and order is the whole of the rule.

## Stage 5 — verification — landed

### Files this work owns

- `crates/scene/tests/shadow_pages.rs` — one new test, and the module doc and the pool bound
  restated now that a light can be recorded as a grid.
- `crates/scene/tests/spot_lights.rs` — `the_pages_a_spot_light_emits_are_the_pages_its_table_answers`.

### The contract test, and why it is the one worth having

A page is described twice, by two halves of the emitter that never speak: the allocator numbers it
into a table row, and the pass that draws it names an atlas slot with a viewport and a window of the
light's plane with a `view_page`. The test compiles `spotlight`, decodes the table out of the record
buffer exactly as a fragment walks it — base word plus popcount of the bits before — and decodes
every pass that carries a viewport back into `(level, column, row, slot)`, then asks whether the two
maps are the same. The level is not carried by a pass at all: it is read out of the window's extent,
which is what makes the comparison check the window rather than a label.

It found nothing, which is the expected outcome for a stage that was already verified by a frame.
What it buys is that the next edit to either half fails here rather than in an image.

### What the frames showed

`out/vsm-spotlight.png` against the pre-work `out/before-spotlight.png`: **mean difference 0.009 of
255, largest single channel 17, 742 pixels of a million above 8**, scattered along edges. That is
what the default density is calibrated to produce, and it is the claim stage 3 made about the
default being checked rather than asserted.

Because a match proves only that two paths agree and not that either draws a shadow, three controls
were run against the compiled scene (all temporary, none committed):

- forcing `shadowed_pages` to return one, i.e. unshadowed, moved **1337 pixels** in one band on the
  floor beside the tall box, from lit 160 to shadowed 55. So the pages are sampled and the shadow in
  the frame is theirs.
- every caster at `shadow_density: 5` instead of the default 58 moved **149** of those same pixels:
  the grid coarsens, the margin stops reaching, and part of the shadow stops existing. The knob is
  live.
- every caster at `60` **refused the compile** — 324 pages against a 256 slot layer, with the
  message naming both numbers.

`stilllife` renders **bit for bit identical** to `out/stilllife-before.png`, and `monoliths` to
within a channel value of 3. Neither has a spot light, so between them they cover the parts of this
work that every scene pays for: the record's extra sixteen bytes, the atlas bound by every lighting
and shadowing pass, and the `view_page` field of the shadowing uniform.

## What is deliberately not here

**Directional and point lights are still on the pool.** §8's removal waits for the last kind to
move, so `LightPool`, `shadow_pool`, the pool's notes and its ceiling are all live and all still
tested. A point light's grid wants six faces and `resolve` walks one — it says so in a
`debug_assert` rather than answering with a slot from the wrong direction.

**The atlas is sized by what was allocated, not by the ceiling.** `Allocation::atlas` reports the
largest light's own `atlas_side`, so a scene with 85 pages gets a 1280 texel array and not a 2048
one. The ceiling is what a refusal is measured against, not what a texture is.

## Verification

`cargo fmt --all` clean. `cargo test --workspace` green. `cargo test -p whiting-device --features
wgpu` green — that is the suite that runs the viewport, the atlas array and the page walk on real
hardware, and it is the suite that caught the example shader's stale binding numbers.
`cargo clippy --workspace --all-targets` reports only this project's pre-existing warning classes;
the new emitter functions join `emit.rs`'s existing `too_many_arguments`.


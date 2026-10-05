# Virtual shadow maps: per-object density, adaptive levels, frustum-clipped distribution

## Problem

The standard scene's shadow pool gives every object in a scene the same shadow
resolution. A page is a fixed-size slice of a depth texture array, one per light that
wins the brightness ranking, and its projection is fitted around the **whole scene
bounds** (`shadow_extent` around `Bounds`, padded by `FRAMING_PADDING`). Three
consequences follow:

- A coin and a castle standing in the same scene get the same texel density, however
  different their needs are. There is no field anywhere for an object to say what it
  needs.
- Every position in the light's volume is rendered at one precision. A receiver at the
  far rim of the volume is shaded against the same texel size as one at the centre.
- The distribution region is the scene's bounding box, not what the camera can see. A
  scene with most of its geometry behind the camera spends the whole page on it.

## References

- `planet_x` (`c:/resource/planet_x/px-scene/src/vshadow.rs`, `frame.rs`, and
  `px_protocol/src/scene.rs::ShadowPlan`) implements a CPU-side virtual shadow map:
  each object declares `shadow_density` (texels per world unit), the light gets a
  virtual page grid sized from its reach and the maximum density, a caster of lower
  density is placed on a coarser level (`caster_level`), pages are allocated sparsely
  around each caster's projection, a page table of row masks maps virtual pages to
  physical atlas slots, and each allocated page is rendered with its own viewport into
  the atlas layer. Overflow is a hard error — precision is never silently lowered.
- [UE5 Virtual Shadow Maps](https://dev.epicgames.com/documentation/en-us/unreal-engine/virtual-shadow-maps-in-unreal-engine)
  confirms the shape of the design: 128² pages, a spot light as a single virtual map
  with a mip chain, a point light as a cube of maps, a directional light as a clipmap,
  and — the part `planet_x` does not have — pages distributed **only where on-screen
  pixels need them** (UE does this by GPU depth-buffer feedback; the same region can be
  computed on the CPU from the camera frustum, which is what SDSM-style fitting does
  analytically).

## Decisions (confirmed with the user)

- Full VSM in one design, not a half-way single-page variant.
- **Spot lights first.** The mechanism is designed kind-agnostic; directional
  (clipmap) and point (cube) are follow-ups and stay on the existing single-page
  projection until their grids land.
- `shadow_density` is declared **per instance**, optional.
- Atlas overflow is a **compile error**, never a silent precision drop.
- The fixed-page path is **replaced**, not kept as a fallback, once a light kind moves
  to the virtual map.

## Design

### 1. Declaration: `shadow_density` on an instance

An instance listed under the `shadowing` bucket may carry an optional
`shadow_density: f32`, in texels per world unit — the resolution it asks its own
shadow to be recorded at. Absent means the scene's default: a conventional density
derived from the current pool shape so existing scenes keep their behaviour
(`page_width / (2 * shadow_extent)`). A value of zero or less means the instance takes
no part in the fine allocation: it is drawn into the coarsest level only, so it still
casts, but it never drives a page finer than the root. The field is read in
`read_instances` and validated finite and non-negative in the scene schema.

### 2. Virtual page grid per spot light

A spot light's shadow volume is a perspective frustum. The virtual map is a square
grid of `pages_per_side × pages_per_side` pages of `PAGE_SIZE = 128` texels laid over
the light's NDC plane — exactly the layout `planet_x` uses for one cube face. The grid
side is derived from the light's reach and the **maximum declared density** among the
casters that light sees:

```
wanted_texels = 2 * reach * max_density
pages_per_side = next_power_of_two(ceil(wanted_texels / PAGE_SIZE)), at least 4
```

so the finest texel is at most `1 / max_density` world units wherever a page exists,
independent of how wide the light's volume is. Up to `MAX_LEVELS = 4` levels exist;
level `k` is the same grid with pages covering `2^k × 2^k` finest pages each
(`pages_at_level(pps, k) = max(1, pps >> k)`).

### 3. Adaptive precision: a caster lands on the level its density buys

`caster_level(density, plane_width, pages_per_side, levels)` puts a caster on the coarsest
level whose texel is still no finer than it asked for, measured against the grid that was
actually built rather than against the density the light was asked for: level `k` covers
`plane_width * 2^k / (pages_per_side * PAGE_SIZE)` world units at the far plane, and a caster
asking for `density` is recorded no coarser than `1 / density`. The grid's side is rounded up
to a whole power of two, so the two differ and the grid is the one that decides.

Its pages are allocated at its own level **and at every coarser level**. Never at a finer one:
a page holds one depth per texel, so a caster cannot be recorded into a resolution finer than
the one it declared. That is what makes the reading end a **walk** rather than a lookup — a
receiver goes from finest to coarsest and a level that recorded nothing at its point does not
answer, so it reaches the coarse caster the neighbour's finer page did not carry. The two
halves are one rule: a page that merely *exists* is not a page that recorded this point.

> Superseded: this section originally said pages were allocated at the caster's own level and
> at every **finer** level down to 0. That was never implemented, and it is the opposite of what
> the design needs — it would put every coarse caster's geometry into every fine page over it,
> which is the page explosion section 5 is about. The chain the sentence was reaching for is
> unbroken by the walk, not by a caster appearing in pages it declined.

The number of pages a caster covers at level `k` is the span of its projection on the grid —
`ceil(2 * radius / world_per_page) + 2` — clamped to the grid.

### 4. Frustum clipping of the distribution region

This is the part `planet_x` does not have. A page is worth allocating **iff its ray
bundle can touch a receiver the camera can see**. For a perspective light the page is
an angular cell, so a caster and the receiver it shadows always lie in the same cell —
which makes the test exact rather than heuristic:

1. Transform the camera frustum (the eight corners, near to far) into the light's view
   space and take its footprint on the light's NDC plane.
2. For each candidate page, build its own frustumlet: the four corner rays of the page
   rect, extended over the light volume's depth range.
3. Keep the page iff its frustumlet intersects the camera frustum (conservative plane
   test; false positives cost pages, false negatives cost shadows, so the test must
   err on the keeping side).

Casters outside the camera frustum are **not** dropped: they still draw into every
kept page their projection covers, because their shadows can land on visible
receivers. Clipping is a property of the page, never of the caster.

The same footprint also tightens the grid origin: the virtual grid is centred on the
frustum footprint rather than on the scene bounds, so pages are spent where the frame
is.

### 5. Page table and physical atlas

Identical in shape to `planet_x`'s `ShadowPlan`:

- One buffer, the **page table**: per light a header (`pages_per_side`, atlas grid
  side, per-level section offsets) and per (level, face, row) a base slot plus a
  bitmask of allocated pages. A fragment resolves a page with one mask test and a
  popcount, which is the whole of the lookup.
- One depth texture array, the **atlas**: layers are lights (× faces for point lights,
  later), each layer a grid of physical page slots sized to the level's worst face.
  Atlas layers replace the `shadow_pool` texture array.

Overflow of the atlas grid, or a grid side past `MAX_PAGES_PER_SIDE`, is a
`CompileError` naming the light, the level, the demand and the limit — matching
`planet_x`'s no-silent-degradation rule.

### 6. Emission: one shadowing pass per page per caster group

For each allocated page and each caster covering it, the emitter writes a shadowing
pass whose **target is the atlas layer** and whose **viewport is the page's physical
slot**. The pass's uniform carries the light's projection plus a `view_page` rect (the
page's window in the virtual grid), and the vertex shader maps the caster's clip-space
output into that window — the same trick `planet_x` plays with `page_rect_in_face`.
The first pass touching an atlas layer clears it; the rest load.

This requires two things the pass model does not have today:

- **A `viewport` field on a rasterize pass** (in `graphics`, applied by the device
  renderer as both `set_viewport` and `set_scissor_rect` — a viewport alone does not
  clip rasterization).
- **Per-caster instance entries for page passes.** A group's merged instance buffer
  cannot draw one caster at a time, so the emitter writes one single-row entry per
  caster that any page pass draws. These are small; the rows already exist.

### 7. Shading: the page-table walk replaces the page index

A light record's `page` field becomes a **table offset**. The lighting shader, given
the fragment's world position:

1. Carries it through the light's `shadow_projection` as today.
2. Converts the light-space NDC to virtual texels at level 0.
3. Walks levels from finest to coarsest: compute the page coordinate at this level,
   read the row's base word and mask; if the bit is set, the physical texel is
   `(atlas_slot + in_page_offset)` and the depth comparison runs as today; if not, try
   the next coarser level.
4. No level mapped → lit, the same answer an out-of-volume lookup gives today.

The 3×3 PCF kernel is unchanged; it taps the physical atlas. The bias constants stay
per level, scaled by `2^level` because a texel's world size doubles per level.

### 8. What replaces what

| Today | After |
| --- | --- |
| `frame.shadow_page_width/height/pages` pool, ranked one page per light | `frame.shadow_atlas` budget (pages per side, layers); pages allocated by density, reach and frustum |
| `shadow_projection` fitted to scene bounds × 1.1 | fitted to the frustum footprint inside the light volume |
| one shadowing pass per (light, group) | one per (light, level, page, caster) |
| record field `page` (index, −1 = none) | record field `table` (word offset, −1 = none) |
| `shadow_pool` texture array | atlas texture array + page table storage buffer |

Point and directional lights keep their current single-page projection until their
grids land (clipmap for directional, cube for point); the pool machinery is removed
when the last kind moves.

## Staging

1. `graphics`: `viewport` on rasterize passes; `device`: apply viewport + scissor.
2. `scene::vshadow`: allocator (grid sizing, caster levels, spans, frustum clipping,
   table encoding, atlas packing) as a pure module with its own unit tests.
3. `scene::standard`: `shadow_density` in the instance schema; emit page passes,
   atlas and table for spot lights.
4. `gallery`: the page-table walk in `shaders.rs`; `pass_schemas` updated.
5. Baselines re-recorded; `shadow_pages.rs` acceptance tests reworked to the new
   budget semantics.

## Risks and open questions

- **Pass count.** Pages × casters can fan out; the atlas budget caps it, and the
  overflow error names the demand so an author sees the cost in the message.
- **Frustum test tightness.** A conservative plane test over-allocates at grazing
  angles; tightening it (exact frustumlet/frustum SAT) is a local change to the
  allocator.
- **Non-spot kinds.** The design generalises — directional wants a clipmap of levels
  around the frustum, point wants six faces — but neither is in this stage, and the
  page table is laid out so faces and levels are already its two axes.
- **Caching.** Reusing pages across frames (UE's main performance lever) is out of
  scope; this project's compile is a pure function of the documents, and caching is a
  renderer-side concern to take up separately.

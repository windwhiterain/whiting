# Terrain (alignment)

> Agent-side design of the `terrain` crate, proposed against [SPEC/mountain.md](../SPEC/mountain.md).
> The spec says a massif's block faces are handed to a **downstream pipeline** (R6) and that
> erosion is out of scope for the `mountain` crate. This document is that pipeline.
> Nothing in [SPEC.md](../SPEC.md) is changed here: the spec is human-editable only, and
> **what it needs from a human is at the bottom**.

## What it is

- `rasterize(&FacetMesh, cell) -> Result<HeightField, Rasterize>`: reads a massif's facet
  mesh onto a rectangular lattice of the plan.
- `erode(&ErodeParams, &HeightField, &Hydraulic) -> Result<HeightField, FieldError>`, and
  `erode_traced` which hands back the `Books` as well.

It is a **separate crate** and not a module of `mountain`, because R6 puts erosion outside that
crate and a module added there would make the spec false rather than extend it. It depends on
`mountain` and on nothing else.

## Why the conversion has no error term

`FacetMesh` guarantees **every face is exactly planar** (`crates/mountain/src/facet.rs`), so
the elevation over a face is the barycentric interpolation of its three corners. A cell centre
that lands in a face reads that face's plane. The conversion is a change of representation,
from a triangle mesh over an arbitrary footprint to a lattice over a rectangle, and the only
thing it loses is the block faces' own straightness — which is the thing R6 hands downstream
in the first place.

So there is no filter, no smoothing and no resolution argument about accuracy. There is only
a resolution argument about **cost and about what the drawing geometry can carry** (below).

## The lattice

- `HeightField` is a square lattice: one plan spacing on both axes, a `heights` buffer and an
  `inside` mask of the same length, indexed `row * width + column`.
- **A cell is inside the massif exactly when a face covers its centre.** There is no separate
  point-in-footprint test to disagree with the mesh — the massif *is* its faces. That is what
  makes the mask a measurement rather than a second opinion.
- **The lattice is square and centred on the mesh's own plan bounds**, with the spacing the
  caller asked for rounded up to whatever covers both extents. A lattice narrower than the
  mesh leaves the outermost faces off it.
- **A cell with no rock under it is frozen.** The solver never visits it, never routes to it
  and never writes it. Its height is `HeightField::base`, which is a number for a *reader*
  drawing the lattice and not an input to the solve.

### Why there is a plan-space index

Asking every cell about every face is `cells × faces`: at a 512² lattice over the landscape's
massif that is eight hundred million point-in-triangle tests for arithmetic that is otherwise
a multiply and an add. The faces are bucketed once over the plan — a counted run, two passes,
not a vector per bucket — and each cell scans only its own bucket. The build is linear in the
faces and the query is a short scan, so the conversion is linear in both. Measured over the
landscape massif: **0.65 ms to read 9,576 cells from 314 faces**.

## The solver

A transcription of `planet_x`'s `px_field_alg::erode`: D8 steepest descent per unit of step
distance, flow accumulation in descending height order, one Eulerian sediment step per round.
Kept verbatim, because each of these is a decision the reference made and a second spelling
of one is a second thing to get wrong:

- the fixed `D8` order and `INV_STEP`, so the distance-normalised drop breaks ties by
  neighbour position and by nothing else;
- `asc_key` and the four-pass radix sort, which is both linear in the cells and the source of
  the tie-break (a stable sort over a list rebuilt from the lattice every round);
- `delta_of` as **one expression**, so the height update and the suspension update cannot
  drift apart;
- the descending visit that carries **water and standing load down the same hop**, so load
  cascades exactly as far as the route reaches and is never deleted on the way;
- the deposition cap at the load actually standing there, which is what makes the books close;
- `WATER_MAX`, because an unbounded solver digs a pit whose own slope keeps it digging.

### Four things this port does not carry, and why each is a consequence rather than a change

1. **The cube atlas.** A `Domain::Cube` field has no boundary, so the reference gives it an
   ocean outlet or a degenerate drain. A massif's boundary is its own footprint, which is
   already in `grid.rs`, and there is nothing left for an ocean to stand in for.
2. **The band threading.** The reference splits its two stencil passes across threads by row
   band. There is no thread model in this workspace to split across, and the accumulation was
   serial there too — so the round is one pass and what is lost is wall clock on the two
   passes that were already the cheap half.
3. **The field width limit.** The reference carries receivers in an `f32` band buffer and
   refuses a field wider than `2^24` cells because the index stops being exact. These
   receivers are `Option<usize>`, so there is no such limit and no error for it.
4. **The scratch receiver's initial state.** The reference's scratch buffer starts with
   receiver **zero** for every cell, so on the first round every cell's rainfall is dumped
   into cell zero and that one cell's capacity is inflated for the round. There is no load to
   route on the first round — it is all zero — so this starts with no receiver anywhere, and
   the first round is a round of rainfall rather than a round of one very wet corner.

### The outlet

A **rim cell that is also a sink** loses its suspended load; an interior pit keeps what it
gathers. The rim is where a massif's valleys reach the foot of it, so it is where its rivers
leave. This is the reference's ocean-outlet rule read onto a footprint, where an ocean stood
in for "the boundary the water actually reaches".

### What is measured, and it is the contract

Over the landscape massif, at the scene's first spacing of 24 units, at the reference
parameters (release build). These are the numbers that say the **solver** is right; the
spacing itself is settled further down, by what the picture needs.

```
lattice        133 x 72 cells, 6209 covered of 9576
read           0.65 ms      solve 12.2 ms for 40 rounds
books          eroded 105656, deposited 69931, suspended 24124.6, spilled 11599.8
mass line      height 35724.45 against load 35724.44
               material 35724.44 against load 35724.44
sinks          24 interior, 58 on the rim
raw move       worst 108.49, worst at the rim 71.82, mean 22.83, over 6209 cells
```

The mass line closes to **one part in three million**, which is `f32` accumulation over forty
rounds and not a leak. A solver that did not close would still produce a plausible landscape,
so this is the number to read before any picture.

## Two defects the probe found, both worth keeping

- **A plan-bounds read that read elevation.** The first version took `point[axis]` for
  `axis in 0..2`, which is `(x, y)` and not the plan's `(x, z)`. It produced a lattice laid
  over the wrong band of the plane and reported **`NoRock`** — a lattice of 5,320 cells, none
  of them on the massif. It is recorded because `NoRock` turned out to be worth having: the
  alternative failure is a field that is quietly empty, whose solve erodes nothing and whose
  books balance perfectly on a landscape that was never sampled.
- **A neutral value that was right for one buffer and wrong for two.** `sample_run` returned
  `HeightField::base` for a point with no rock within a cell of any. That is right for
  elevations and wrong for a displacement, whose neutral is zero: every vertex outside the
  footprint was lifted by the massif's base level, 1450 units, and the scene's relief read
  1446..2900. The read returns `Option` now and each caller states its own fallback, which is
  what makes it a function rather than a convention.

## The gallery: what the render overturned

This section was written before anything was rendered, and **two of its conclusions were
wrong**. Both were found by rendering the scene and looking at it, which is the only thing
here that can be wrong in that particular way.

### The displacement went onto the mesh's own vertices, and the erosion never arrived

The scene first drew the **authored facet mesh**, displaced. The argument for it was good: a
face carries the belt it was expanded from, so a pixel resolves to the crest an author wrote,
and a lattice has nowhere to carry that.

The render said this:

```
cargo run -p whiting-device-rasterize --example render -- landscape out/erode-01
```

A massif of sharp block faces — `out/erode-01.png` — indistinguishable from the same scene
with no erosion in it at all. The arithmetic explains why: **the massif has 174 points and the
field has 6209 cells**, so the solve is evaluated 174 times and everything between those 174
samples is discarded before the frame sees it. The probe had already said the displacement was
worth up to 95 units — and 95 units on a 950-unit relief, sampled at 174 points on a
3188-unit plan, is not a picture.

So the lattice is what the scene draws now. The identity survives: each lattice cell names
**the facet it was read from**, and a facet carries its belt. The cost is resolution — a
pixel resolves to a facet to within a lattice cell rather than exactly — and the feature is
intact. `out/erode-04.png` is the same scene afterwards: gullies on the flanks, a toe that
meets the plain, and `authored_id` still naming the crest under each pixel.

### A displacement *texture* is not the mechanism here — verified on both backends

Displacing in the vertex stage from a texture is the obvious GPU-shaped answer and this
workspace cannot use it for the geometry. `scene`'s Cycles exporter reads `position` straight
out of the packed point run (`crates/scene/src/cycles_export/geometry.rs`) and writes a mesh;
it has no vertex stage and no way to sample anything. The displacement is therefore baked into
the document's points, which is also the cheaper answer here — the field is already on the cpu,
having just been solved on it.

**This was measured, not reasoned, and the measurement was the second attempt.** The first
search for a Cycles binary looked in three directories and found none, and the offline
backend was reported as unverified on the strength of that. It was wrong: the runner's first
candidate is `C:\resource\cycles\install\cycles.exe`, which is exactly where the binary is
(`crates/device-cycles/src/runner.rs`), and the search had only listed the top level of the
directory above it. A capability claim made from a search that did not look where the code
looks is worth nothing.

What both backends then drew, from the same document:

```
cargo run -p whiting-device-rasterize --example render    -- landscape out/erode-lattice.png
cargo run -p whiting-device-cycles   --example render_cycles -- landscape out/erode-cycles.png
```

The two agree on the silhouette and on the gullies, which is the part that matters: under a
path tracer those gullies are lit from the integrator's own rays, so **they are geometry and not
shading**. A vertex displacement would have produced a flat massif here, because there is no
vertex stage to apply it in.

The emitter reports nine dropped fields and all nine are pre-existing rather than new:
`authored_id` / `authored_u` / `authored_v` on both range meshes, which the Cycles XML loader
cannot carry (it reads position, normal, uv, uv tangent and tangent sign), and the
shadowing and depth shader stages, which describe passes the rasterised pipeline owns. The
massif declared those three attributes before this change too — `identity::faceted` wrote them
— so the lattice moved them from one builder to another without moving the count.

Two small dark notches at the massif's toe appear in **both** frames. That makes them
geometric rather than a rasteriser artefact, and they are not accounted for: they are most
likely a lattice cell at the boundary whose one-sided normal falls away steeply, which would
be a shading artefact in the rasteriser and a silhouette in the tracer. Left alone and named.

What the texture idea does buy is **a legible instrument**: the displacement as a scene-declared
output image, next to the `authored_id` / `u` / `v` channels this scene already writes. That
is not built.

### What the lattice's spacing is measured against now: the silhouette, not the solve

The old claim was that `CELL` should sit between the mesh's crest step and its shortest edge,
because a finer lattice solves for gullies narrower than the triangles. That was true of a
**displacement** and is false of a **drawing**: the lattice's cell is the size of this
mountain's facets.

The thing a lattice cannot hide is its own outline. A cell with a corner outside the footprint
is dropped whole, so the massif's silhouette is the footprint quantised to the cell. At 24
units the steps are about seven pixels in this frame and plainly visible along the foot; at 12
they are about three and read as rough ground; at 6 they would be about a pixel for four times
the triangles. The eroded surface itself barely changes across that range — worst displacement
**108 at 24, 87 at 12, 100 at 6** — so the cost is geometry, not physics. `CELL` is 12.

### The berm at the toe, and a fade written twice

The first lattice render came out with a **black band along the foot of the massif**. It was
not a shading fault: `--locate` on a pixel in it reported `authored_id 0`, so it was the
plain, in the massif's shadow. The massif was casting a shadow on the plain it is supposed to
meet, and the thing casting it was the solve's own boundary behaviour. Measured by distance
from the rim:

```
displacement by distance from the rim:
  under 1.5 cells      494 cells, mean   29.96, range  -44.16 to   71.82
  1.5 to 3.5           531 cells, mean   17.39, range  -49.49 to   76.97
  3.5 to 6.5           773 cells, mean    4.85, range  -58.21 to   79.34
  6.5 to 12.5         1290 cells, mean   -6.09, range  -77.33 to   77.04
  beyond 12.5         3121 cells, mean  -17.82, range  -86.26 to  108.49
```

**A +30 unit berm right round the toe, decaying to nothing over about six cells, and negative
everywhere else.** That is not erosion; it is the erosion model answering an artificial
boundary — the massif is *cut off* at its footprint rather than ending, the ground flattens
there, capacity goes to nothing, and everything arriving deposits.

Two fixes, and the second is the one that mattered. The fade is right, and it starts at the
**first ring of cells** rather than at the rim line, because a vertex standing on the rim has
no cell under it and the nearest cell with rock is one step in. But the fade was **written
twice** — once for the displaced mesh, and once, *absent*, in the lattice — so the two
drawings put the massif's foot in two different places and the lattice kept the berm. There is
now one surface, `Erosion::surface`, built once and read by everything.

### What the change cost the scene's own argument

The block-face reading is gone from the picture. That scene's module note said the normals are
written per corner precisely so the facets read as facets, and a height field shaded by its own
gradient is a smooth surface — which is what an eroded mountain is. **The two cannot both be
shown by the same geometry.** The facet mesh still exists and is still what the lattice is
read from; what changed is that it is no longer what is drawn.

### What it costs

At 12 units: about 25,000 covered cells per range, so ~50,000 triangles per range against the
authored mesh's 314. The solve is 62 ms in release and the scene compile went from 115 ms to
about 290 ms.

## What SPEC.md needs from a human

The spec is human-editable only and has no line for this crate. One is needed, in the same
shape as the `mountain` entry:

> ### `terrain` crate
>
> read a `mountain` crate's `FacetMesh` onto a rectangular height field of the plan, and
> erode it by iterative hydraulic transport. see
> [SPEC/terrain.md](SPEC/terrain.md) and, for the design,
> [SPEC.align/terrain.md](SPEC.align/terrain.md).

## Open questions

- **A height field per range, or one for the whole frame?** Each range is eroded on its own
  lattice, which is right for the erosion and wasteful for the solve — two runs where one
  would do. It is left because a lattice spanning both ranges is a lattice whose cell size is
  set by the far one, and the near range is what the frame is about.
- **Thermal erosion.** The reference scheme has none, and a talus angle is the other half of
  what makes a slope read as rock. It is a second law over the same round and a second
  `Transport` would not carry it — it is a separate term, not a different formula.
- **The berm, as a general answer rather than a scene constant.** A +30 unit lip at a free
  boundary is the erosion model's answer to being cut off, and a three-cell fade hides it from
  *this* scene because this scene levels its plain to meet the rim. A caller that let its field
  end in mid-air would get the lip, and the honest fix is at the boundary — an outlet that
  spills rather than a deposit that piles — rather than a fade over whatever the caller's
  ground happens to be doing.
- **The staircase.** Twelve units makes the footprint's quantisation read as rough ground.
  It is still a quantisation, and the fix that removes it — snapping the outer ring of lattice
  vertices onto the footprint outline — collapses triangles at the boundary, which is a worse
  artefact to look at than the one it removes. Not attempted.
- **Whether the picture is better.** It is a different picture and that is the honest claim.
  The block faces are gone and the gullies are there; nothing here says which a reader prefers,
  and the only thing that could is another render.

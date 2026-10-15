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

Over the landscape massif, at 24 units, at the reference parameters (release build):

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

## The gallery, and why the displacement goes back onto the mesh

`landscape::Erosion` is the scene's use of this crate. It rasterises the **placed** mesh (erode
after `stood_at`, not before: a solve run on the authored mesh and then scaled would carve the
near range at the far range's resolution), erodes it, and **applies the displacement back to
the mesh's own vertices**.

Drawing the lattice instead would throw away what the scene is for. A face carries the belt it
was expanded from, so a pixel still resolves to the crest an author wrote, and a lattice has
nowhere to carry that. Displacing keeps the authored skeleton as the shape of the mountain and
adds to it only what the water removed — which is what erosion is: a **displacement field**,
not a replacement for the thing it acts on.

### What the lattice's spacing is measured against: the mesh, not the mountain

A relief of nine hundred over a plan eighteen hundred across says nothing about how much of an
erosion pass the geometry can carry. This massif's own faces are sixty units on a crest and
twenty on a rib, so a lattice five times finer would solve for gullies narrower than the
triangles asked to draw them, and the answer would be thrown away before it was ever seen.
`CELL` is therefore stated against `Tessellation::step` and sits between the two. Measured,
the worst displacement is **108 at 24 units, 87 at 12, 100 at 6** — the detail exists and does
not reach the picture — while the cost goes 12 ms, 50 ms, 228 ms. This is
[SPEC.align/mountain.md](mountain.md)'s "a heightfield's coarseness budget and a skeleton's
step budget are the same budget", now with a number on each side of it.

### The rim, and the seam it would open

The massif's mesh ends at its footprint and the scene's plain begins there at the same
elevation, so anything the solve does at the rim opens a crack between two surfaces built to
meet. Measured, the raw displacement at the rim reaches **71.8 units** against a plain standing
still. The displacement is therefore faded to nothing over the outermost three cells.

The fade starts at the **first ring of cells**, not at the rim line: a vertex standing *on*
the rim has no cell under it and the nearest cell with rock is one step in, so measuring from
zero left that vertex at a third of its displacement and put the seam back. After the fade the
massif's low point reads 1449.8 against the plain's 1450, and 150 of its 174 vertices move.

### What it costs

14 ms in release, **238 ms in debug**, per range, so about half a second for a scene build in a
development build. That is the price of doing it at scene-build time rather than authoring
time, and it is the number to argue with if it becomes the wrong trade.

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
- **Whether the displacement belongs on the mesh at all.** The argument for it is identity and
  it is a good one, but the argument for drawing the lattice is that the mesh's own faces are
  what stops the erosion reading, and that costs the fine detail. Nothing here measures which
  picture is better, because this repository measures pictures with a renderer this crate
  cannot reach.

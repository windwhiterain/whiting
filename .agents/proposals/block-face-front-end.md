# Block faces are the unit of surface, not triangles

## Summary

`mountain` already knows its surface as **182 block faces**. The current front end throws that away
and subdivides each block into triangles, which then become 7146 bilinear patches, which then become
228672 drawn triangles. Superior's instruction is that the block is the face and the patch carries
the interior.

**Decided 2026-10-08, and it removes two of this proposal's questions rather than answering them:**
the polylines do not have to be preserved and the difference does not have to be minimised — the
sampled polylines are a **hint** about where the control points go. So there is no basis decision to
make, no interpolation to preserve, and nothing to optimise. A `k` by `k` control net taken from the
block's own surface is the whole construction.

Measured, by `examples/block_face_probe.rs`:

- **182 patches instead of 7146**, a 39-fold reduction.
- **A typical block is captured `2.4e-6` to `3.1e-6`**, an order of magnitude *below* the `1.526e-5`
  `f32` precision of the mesh it came from — so it is captured as well as its input can be
  represented.
- **Every block whose faces cover their own `uv` box costs at most `5.5e-6`**, at every `k`.
- **The entire worst case, `1.5e-1` to `3.7e-1`, belongs to the 89 blocks that do not.**

This proposal also carries the winding defect, because the two are the same fault from two ends — and
because the granularity change **does not fix all of it**.

## What is measured

### The granularity, as it stands

| | count |
|---|---|
| authored block faces | 182 |
| triangles the expander emits | 2382 (13.1 per block) |
| quads after Doo–Sabin | 7146 |
| bilinear patches | 7146 |
| triangles a rasteriser draws at `side = 5` | 228672 |

### What a patch per block costs

Control net = the block's own surface sampled on a `k` by `k` grid. Nothing fitted, nothing
interpolated. Distance from a block's mesh vertices to the patch drawn over it:

| k × k | worst | p99 | median | worst among the 93 blocks that cover their box |
|---|---|---|---|---|
| 2 × 2 | `1.563e-1` | `1.561e-1` | `2.984e-6` | `4.350e-6` |
| 3 × 3 | `1.460e-1` | `1.459e-1` | `2.910e-6` | `4.898e-6` |
| 4 × 4 | `2.230e-1` | `2.227e-1` | `3.078e-6` | `5.529e-6` |
| 5 × 5 | `2.790e-1` | `2.789e-1` | `2.686e-6` | `4.466e-6` |
| 6 × 6 | `2.289e-1` | `2.287e-1` | `2.561e-6` | `4.378e-6` |
| 7 × 7 | `3.676e-1` | `3.673e-1` | `2.370e-6` | `3.982e-6` |

Two things in that table are the answer, and neither is the worst column:

- **The worst case rises with `k` while the median does not fall.** That is the opposite of a denser
  net over a smooth surface, and the last column says why: where a block's `uv` box is not covered the
  net carries an *extrapolated* control point, and a higher degree amplifies it. The cost is in the
  sampling grid running off the surface, not in the resolution.
- **The last column is flat at about `4e-6`.** A denser net over a smooth surface would fall. The
  reading is that it is not resolution at all: a block's interior is a triangulated surface, its
  creases are part of the data, and no smooth patch follows them. That is a reading of a flat curve
  rather than a separate measurement, and it is why the cost plateaus instead of vanishing.

Both say the construction is nearly free where the grid lands on the surface, and that the thing to
fix is the domain, not the degree.

### The `uv` domain is not a box, and that is the authoring

`Face::uv`'s `u` is the fraction of the belt walked **by distance**, so a crest point and the valley
point below it are not the same station and the strip between them is not the whole square. 89 of 182
blocks leave part of their own `uv` box uncovered; the smallest covers `0.9048`, the median `1.0000`.

So the parameter domain of a block patch should be **the region its own faces cover**, not the unit
square. That is a change to the front end and it is what removes the worst column.

### Can neighbouring blocks share an edge as a curve

| | count |
|---|---|
| ribs on exactly two blocks | 168 |
| ribs on one block | 14 |
| ribs on three or more blocks | 1 |
| blocks whose two runs are equal | 58 |
| blocks whose two runs differ | 124 |

A **rib is two points**, so both sides can name it as a curve with the same two control points
whatever their patches' interior resolution is. That part works.

A **run is a different length on the two sides in 124 of 182 blocks**, and two patches share a
boundary as one curve only if both carry the same number of control points on it. That is a property
of the authoring rather than of the front end, and it is the one thing standing between the current
front end and a patch per block.

### The winding defect

46 of the 3351 interior edges of the sample `FacetMesh` are traversed the same way by both their
faces. Classified by how an edge is traversed against which side of it in plan the two faces' third
vertices fall:

| traversed | third vertices | count |
|---|---|---|
| both ways | opposite sides | 3305 |
| both ways | same sides | 0 |
| one way | opposite sides | 0 |
| **one way** | **same sides** | **46** |
| a third vertex on the line | | 0 |

`mountain` gives every face the same sign of plan area. That is a consistent winding **only where no
face's plan shadow crosses its neighbour's**, and a face seen nearly edge on casts a shadow so thin
that it does. Every one-way edge is an overlap edge; no other cell is occupied.

**Two explanations were measured and refuted before this one, and both are worth recording because
each looked entirely reasonable:**

- *`f32` rounding.* `push_face` reads the sign of an `f32` area, and the faces at the bad edges have
  plan areas of `1.024e-5` median against a mesh median of `1.951e+1` — six orders of magnitude down,
  where one `f32` ulp of a coordinate of order 60 is `7e-6`. Testing the sign against the actual error
  bound of that computation clears **all 2382 faces**. The sign is not noise.
- *An earlier version of the classification read the overlap count out of the branch that only ever saw
  `(1, 1)` edges*, so it saw zero of the 46 and concluded the opposite. The joint distribution is what
  settled it.

The 46 one-way edges sit inside **14 distinct authored blocks**. Of those edges, **8** are in a block
that is concave in plan. Only **3 of 182** blocks are concave in plan at all. So **38 of the 46 are
inside blocks that are convex in plan** and go away with the granularity change; the remaining 8 sit in
blocks that are concave whatever they are cut into. Coarser faces reduce this defect; they do not
remove it.

**What follows from it, corrected 2026-10-08.** An earlier version of this section claimed that "a
rasteriser with back-face culling drops everything behind them", i.e. that half the terrain would not
be drawn. **That does not happen, and it is worth saying why.** `Tessellation::triangles` and
`Face::normal` both derive the winding from the **same quad corner order**, so they agree with each
other whatever order the source quad happened to have. `raster_probe` measures it on the massif:
`facing away 0`, `wholly facing toward 7146`, `mixed 0`, with the reversed control catching all
228672. Nothing is dropped.

The real consequence is **shading and meaning, not visibility**: because the two sides of those 46 edges
disagree about which way is out, **48% of the patches carry a normal opposite to their neighbours'**,
so a lit render reverses shading across those seams — and any continuity one asks for across them
(polylines as shared boundaries, hard/soft tagging) is meaningless, because the two sides do not share
an orientation to be continuous *in*. That is why this is a granularity defect to fix rather than a
rasteriser to configure.

## Open questions, in the order they have to be answered

1. **The unequal runs.** 124 of 182 blocks have two boundaries sampled at different rates. Resampling
   the shorter one, or splitting the block, are both available; the first changes a thing that currently
   has an exactness claim and the second is a face count that has to be reported. This is the blocker.
2. **The parameter domain.** Use each block's covered `uv` region rather than the unit square. Cheap,
   and it is what turns the worst column into the last one. Does it have to be a rectangle at all, or
   can a patch be carried on the strip between two polylines directly?
3. **Which `k`.** The cost is flat in `k`, so `k` is chosen by what the interior is *for*, not by
   accuracy: how much freedom a designer wants to drag. The count is `182 * k^2` control points.
4. **The three concave blocks.** A bilinear patch through a quad that is concave in plan has a
   parameterisation whose interior is not a graph over the parameter domain, which is a separate
   question from how the patch is wound.

## Recommendation

Adopt the block face as the unit. The measurement supports it more strongly than expected: the cost is
below the precision the mesh arrived with for every block whose domain covers its own surface, and the
whole of the worst case belongs to a domain that is the front end's choice rather than the data's.

Take the open questions in the order above. Question 1 is the blocker and it belongs to `mountain`'s
authoring rather than to `patch`; question 2 is small and is what the worst column is made of.

Keep the winding fix separate and small. It is one traversal over the mesh in `mountain`, it is correct
independently of how coarse the faces are, and it still needs to cover the 8 edges in concave blocks
after the granularity change has removed the other 38.
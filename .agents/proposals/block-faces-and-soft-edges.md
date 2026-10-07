# Block faces, and the normals a block face shares

## What is being asked of the spec

`SPEC/mountain.md` R5 currently reads:

> Each triangle is one planar block face; ridge flanks read as facet fans meeting at the ridge
> line.

The first clause stops being true with this change. A triangle is still **exactly planar** — that
guarantee is untouched and `facet.rs`'s module invariants still say so — but a triangle is no
longer *the* unit of surface. The unit of surface is now a **block face**: the region a belt
spans between two of its ribs, whose four corners are `(crest a, crest b, valley a, valley b)`
with `a` paired with `a`.

The second clause survives, and is in fact what the change is for: flanks still read as fans
meeting at the ridge line, because a block boundary is a crease. What stops is every **other**
edge of every triangle showing.

### The clause this replaces

> Therefore the ridge distribution is authored as a polyline tree/network, and the polylines are
> then expanded into a polygon mesh. Triangles are the natural primitive for this: a triangle is
> exactly one planar block face, and the two flanks of a ridge are two fans of triangles meeting at
> the ridge line.

The argument "triangles are the natural primitive" is sound and is not what changes. What changes
is that a primitive being *drawn* is not the same as a primitive being *read as a surface*. A
triangle strip has to be triangulated because that is what a rasteriser draws; nothing about the
surface requires the eye to see those diagonals.

## The picture that prompted it

The `landscape` gallery scene, after its crests and valley floors were rewritten as per-node
tables of authored elevations, still read as a low-poly solid: every one of its 1005 triangles
carried its own flat normal, so every triangle boundary was a visible crease. The silhouette and
the profile were authored; the surface between them was not, and the mesh's own tessellation was
being read as rock structure it never claimed to be.

The mesh already had the thing needed to fix this and was not using it. A belt is walked by
`zip::steps` into a **general triangle strip**, not a quad grid — measured on the landscape
massif, 16 of 240 belts are two triangles and the rest are three to thirty, because nothing in
the crate requires the crest and the valley floor to be sampled alike, and `zip.rs` treats that
as a feature (SPEC R4.1). So there was no cell structure to recover, and no quad algorithm to
introduce. There was, however, a boundary the author had already drawn: the two ribs.

## What a block face is

```text
crest a --------------------------------- crest b
    |          the block, spanned             |
    |          over however many               |
    |          triangles the walk made         |
valley a -------------------------------- valley b
```

- Its ends are two authored nodes of the line its floor runs along, together with the stations
  those nodes name on this flank. `ValleyNode.left` / `.right` already are exactly those station
  pairs, so nothing new is introduced to say where a block begins and ends.
- **A triangle is the degenerate case.** One of the two runs being a single point makes the block
  one triangle, and `BlockFace::corners()` reports it by repeating that point rather than by
  returning three entries.
- **How many triangles it holds is a fact about the tessellation, not about the block.** A
  two-point run on both boundaries is the quad; three and four are what the landscape massif
  actually produces. Writing fewer nodes on a line is therefore also how an author coarsens the
  blocks — the same table that holds a valley's elevations holds its block count.

## What it adds

- `mountain::BlockFace` — the four corners as `top` / `bottom` runs, the `Range<u32>` of faces it
  is expanded into, and the `FaceAttributes` it was expanded from.
- `FacetMesh::blocks` and `Face::block`, written together by the expander, so the two cannot be
  edited apart.
- `mountain::CornerNormals` — one pass over the mesh, then a constant-time read per corner.

`FacetMesh::points`, `FacetMesh::faces`, the winding, the watertight guarantee, and every
topological method (`plan_area`, `boundary_loops`, `euler_characteristic`, `is_manifold`,
`locate`) are untouched. The geometry a block records is read back **out of the vertex table
after** the walk, so a point whose only face collapsed is absent from a run rather than inserted
into the mesh — inserting it would change the vertex count and with it the Euler characteristic.

## Normals

A corner's normal is the **angle-weighted** mean of the unit normals of the faces of **its own
block** that touch it.

Angle weighting, rather than a plain mean, because a belt zipped between two boundaries descending
at different rates produces slivers — the landscape massif has facets up to 125 times longer than
they are wide — and a plain mean gives such a face a full vote at each of its two corners. It can
tilt the shared vertex towards its own plane. Weighting by the angle at the corner is what keeps a
thin face from doing that.

Averaging over one block rather than the whole mesh is what keeps the crest lines and the ribs
sharp, which is the point of the change: the rock's structure is the block boundaries, and a mesh
that smooths across them is a different mountain.

## What this is not, and the `patch` crate

This crate (`mountain`) and [`crates/patch`](../../crates/patch) both use the words *soft edge*
and *hard edge* for the same intent — a crease versus a tangent-plane-continuous join — and mean
different things by them.

`patch::SoftEdge` is about **control points**: placing the slave face's adjacent row so the
cross-boundary derivative matches, on a tensor-product Bézier patch. Nothing here moves a control
point; there are none. This is about **interpolated shading normals** on a triangulated strip.

Two consequences worth writing down:

1. **`CornerNormals` is not called `SoftEdge`,** to keep one meaning per word in the workspace.
2. **`patch`'s measured corner-stencil defect applies here too.** `patch::continuity` measures a
   4.8e-2 rad tangent-plane gap at a soft edge against 1.8e-1 rad at a hard one, because the
   corner-adjacent control points around a vertex cannot satisfy the invariant. Angle-weighted
   normals leave the same kind of bounded defect at every block boundary: the interior of a block
   is smooth, and the join to its neighbour is a crease that is somewhat sharper than a true
   tangent-plane join and much softer than an unrelated hard edge. **A soft edge is not C1 here**,
   and a caller wanting continuity across a block boundary must join the blocks rather than smooth
   across them.

## The decision the human still owns

R5's wording, and the R1/R2 sentences that read "a real mountain flank reads as a set of planar
facets meeting at sharp edges". They are a statement about rock, and this change says the *flank*
is a smooth surface while the *facets* are what meet at sharp edges — the other way round. Whether
that is the document's intent is not an agent's call.
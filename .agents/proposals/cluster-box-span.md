# A cluster's box is not where its fragments are

**Found while implementing the global light rule** ([`photographic-scenes`](../handoffs) is
closed; this is the finding it left). Merged as `5b19431`, on top of `main` at `e1b29c2`.

## The claim

A light is culled per cluster by the distance from its own centre to the **nearest point of the
cluster's box** (`lighting/assignment.rs`, `Volume::nearest_squared` against `View::box_of`).
The box is not a superset of the slice it stands for, and it is not where the fragments that
resolve to that cluster are. So the cull can drop a light from a box that holds a fragment much
closer to it than the box's own nearest point is.

**The light is culled because of where the *box* is, not where the surface is.** The hole this
leaves in the image has edges on the cluster grid, because a box is a box.

## Why the box is wrong

```rust
fn box_of(&self, column: usize, row: usize, slice: usize) -> ClusterBox {
    let far = self.boundaries[slice + 1];
    let [half_across, half_up] = self.widths[slice + 1];   // the slice's FAR cross-section
    ClusterBox {
        across: span(column, self.width_extents, half_across),
        up: span(row, self.height_extents, half_up),
        depth: [self.boundaries[slice], far],              // the slice's whole depth
    }
}
```

`across` and `up` are placed on the **far** cross-section, which is where a widening perspective
is widest, while `depth` spans the whole slice. The slab of a widening view is a wedge, and a
rectangle cut at the wide end is not a cover of it.

Worse, the two ends of this do not even agree on where a cluster **is**. The fragment shader
picks its cluster from its own screen position:

```wgsl
let column = u32(across * uniforms.cluster_grid.x);
let row    = u32(up * uniforms.cluster_grid.y);
```

`across` there is `fragment.x / frame_size.x` — a **screen** fraction. So a fragment near a
slice's near end and at the right edge of the frame resolves to the frame's rightmost column,
whose box has an `across` span of the rightmost 1/16 **of the far cross-section** — far to the
right of where that fragment actually is. **A fragment can lie outside its own cluster's box.**
The two constructions coincide only at the far boundary of each slice.

## What it drew

`dunes`, with its lights naming a radius that reaches the whole field (430, added in `e1b29c2`
precisely so they would), still had a **black notch in the bottom-right corner with the shape
of the grid**. Measured: the changed region is x 896..1023, y 796..1023 — left edge 896 = 14x64,
a cluster column; top edge 796 = 1024 - 2x113.78, a cluster row. One 2x2 block of clusters.

A probe with the promotion disabled reported the cause directly: those clusters, in the
**farthest** slice, held 1 of the 2 lights. Disabling the promotion reproduces the pre-change
frame byte for byte, so the promotion is the only thing that changed the picture.

## What is already true, and what is still open

**The global light rule is a workaround for this, and a good one.** A light that holds the whole
scene is promoted out of the walk and evaluated for every fragment, so it is immune to the
mis-measurement. That repairs every pixel a light of that size could have lost, and it repairs
them in the only way available from the scene's own documents.

**But it only repairs them for lights big enough to be promoted.** A clustered light — a lamp
in a room, a lantern over a table — is still culled by a box that is in the wrong place, and a
lamp near the near end of a slice can still go missing in a rectangle. `stilllife` is the scene
that reaches it: eleven lights over a table, and
[`photographic-stilllife.md`](photographic-stilllife.md) §2 recorded 73 clusters lit by nothing
there, which it worked around by tightening the camera's planes rather than by fixing the walk.

## The options

1. **Make the box a real cover of the slice.** A wedge, not a rectangle: `across` and `up` would
   have to be a frustum segment per depth, and the nearest-point test would be against that. The
   honest shape is the convex hull of the slice's eight corners, which is cheap to precompute per
   slice and makes the box a true superset. It also makes the box larger, so more lights survive
   the test — which is the point, and is why the current rectangle is a *tight* fit that is tight
   in the wrong direction.
2. **Cull on the sphere against the slice, not the box.** A light reaches a slice if its sphere
   meets the slice's own volume. Same answer as 1 for the "does it reach" question, and it drops
   the per-column/row reasoning entirely — at the cost of a light that reaches the slice being
   listed in all 16x9 of its boxes, which is what the cap is for.
3. **Leave it.** The global rule removes the visible symptom for the lights that are big enough,
   and the remaining case is a small light in a wide frame, which no gallery scene currently
   shows.

1 and 2 both make the promotion what it was meant to be — a pure optimisation rather than a
repair — and that is the test of whether the walk is right: **with a correct box, a light whose
radius holds the scene is in every cluster the camera reaches, and promoting it changes no
pixel.**

## Not proposed here

- A compiler note when a light is promoted. The promotion is invisible and correct, so a note
  would report a non-event.
- Any change to the number of extents. The grid's shape is not what is wrong.

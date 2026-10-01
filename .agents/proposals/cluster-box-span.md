# A cluster's box is not where its fragments are

**Found while implementing the global light rule.** Merged as `5b19431`; **fixed** on branch
`fix/cluster-box-corners`. The finding is kept whole below because the evidence is what says the
fix works, and the measured outcome is at the end.

## The claim

A light is culled per cluster by the distance from its own centre to the **nearest point of the
cluster's box** (`lighting/assignment.rs`, `Volume::nearest_squared` against `View::box_of`).
The box is not a superset of the slice it stands for, and it is not where the fragments that
resolve to that cluster are. So the cull can drop a light from a box that holds a fragment much
closer to it than the box's own nearest point is.

**The light is culled because of where the *box* is, not where the surface is.** The hole this
leaves in the image has edges on the cluster grid, because a box is a box.

## Why the box was wrong

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
The two constructions coincide only at the far boundary of each slice, and at the centre of the
frame the two cells nest, which is why the defect reads as a screen-edge artefact rather than as
a broken walk.

## What it drew

`dunes`, with its lights naming a radius that reaches the whole field (430, added in `e1b29c2`
precisely so they would), still had a **black notch in the bottom-right corner with the shape
of the grid**. Measured: the changed region is x 896..1023, y 796..1023 — left edge 896 = 14x64,
a cluster column; top edge 796 = 1024 - 2x113.78, a cluster row. One 2x2 block of clusters.

A probe with the promotion disabled reported the cause directly: those clusters, in the
**farthest** slice, held 1 of the 2 lights. Disabling the promotion reproduces the pre-change
frame byte for byte, so the promotion is the only thing that changed the picture.

## The fix

Take the box from the cell's **eight** frustum corners — four at the slice's near boundary and
four at its far one — and keep the min and max of each axis. This is the clustered-shading
standard and not a local invention; identical code appears in
[A Primer On Efficient Rendering Algorithms & Clustered Shading](http://www.aortiz.me/2018/12/21/CG.html),
in the Skyrim Community Shaders' `ClusterBuildingCS.hlsl`, and in
[WebGPU Forward+ and Clustered Deferred](https://wrnmd.com/jichumao/Project4-WebGPU-Forward-Plus-and-Clustered-Deferred).
A cell is a rectangle, so the eight corners reduce to two spans and the box is the union of the
near cell and the far cell on each of `across` and `up`.

The box grows, and the literature's own verdict on that is worth keeping:
[Clustered Shading](https://gitmemories.com/index.php/DaveH355/clustered-shading) reports that
the boxes "must overlap to cover the frustum shape" and that the technique "does not benefit
from more accurate cluster shapes or distributions of clusters". Conservatism here is the
algorithm's accepted price, not a risk that had to be measured before shipping.

## What it measured

Eight of eleven gallery scenes render **byte for byte** as they did. Three change, and every
change is confined to the outermost columns of the frame — 1024px over 16 columns is 64px a
column, and each changed region is 192px, exactly three:

| scene | pixels | region | mean abs RGB delta |
| --- | --- | --- | --- |
| `tiles` | 13056 (1.25%) | x 0..191, y 501..568 | 321.2 |
| `tidewater` cam0 | 17745 (1.13%) | x 0..147, y 603..797 | 11.8 |
| `monoliths` | 41960 (4.00%) | x 832..1023, y 583..888 | 1.9 |

`tiles` is the one to look at: a lit strip across the frame was cut off dead at x = 192, three
columns in, and now runs to the left edge. That boundary was the defect's own shape drawn in the
image. `tidewater` is a faint rectangular seam in the dark foreground island, smoothed. `monoliths`
is a fill of about two code values, imperceptible.

**The change is monotone.** The hull contains the old box on every axis, so a light is only ever
*added* to a cluster and never removed from one. This fix cannot darken a frame.

## What is now true

**The global light rule is no longer a repair.** With a box that holds its cell, a light whose
sphere holds the scene cannot fail the distance test against any cluster that holds a surface, so
promoting it changes no pixel — which is the test this finding set out. What promotion still buys
is immunity: a global light does not depend on the cluster walk being right at all, which is what
makes "name no reach" a good default, since an author who forgets the number then gets a light
that is wrong everywhere and says so rather than one subtly absent at the edge of the frame.

## Not fixed here

**Exponential depth slicing.** The grid is divided evenly through depth; the literature prefers
`Z = near * (far/near)^(i/slices)`. That is correct in general and does not apply here without a
cost this branch does not pay.

`cluster_of` reads `fragment.z`, the framebuffer's depth, and multiplies. `View::read` builds its
boundaries as `projection.distance_at(i / depth_extents)`, the exact inverse. The two are equal
**by construction rather than by agreement**, and both sides document that this is the reason
(`shaders.rs:427-435` and `assignment.rs:539-543`). Recovering a view distance on the GPU would
need the near and far planes and a divide — what the shader's comment says the design avoids —
and would put a second copy of the depth curve in the shader at a divide and a logarithm per
fragment.

Now that the box holds its cell, what is left is only how *loose* it is, and the same literature
says this algorithm does not benefit from answering that. The one route to exponential slicing
without the trade is to change `DepthMap` from its rational form to a logarithmic curve, and
`DepthMap` also drives the depth buffer, the shadow pages and every scene's depth test. That is a
renderer change, not a cluster change, and it is not proposed here.

## Not proposed here

- A compiler note when a light is promoted. The promotion is invisible and correct, so a note
  would report a non-event.
- Any change to the number of extents. The grid's shape is not what is wrong.

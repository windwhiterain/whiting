# Arbitrary topology cannot be made smooth under a per-patch NURBS

Date: 2026-10-10. Status: **measured, not fixed. The question is closed as asked; the replacement
question is which cost to pay.**

## Question

Turn an arbitrary-topology mesh into several NURBS patches stitched together, smooth at **every**
point. Specifically: if the mesh were restricted to T-junctions and cross junctions only — no other
irregularities — could the current model guarantee no non-smooth point anywhere?

## Answer

**No, and restricting the junctions does not change it.** Three failures, independent of each other,
so there is no "solve the junctions first" path.

1. **A seam is already discontinuous away from its junctions.** The soft-edge invariant is applied
   over `1..degree`, leaving the two ends of the slave's row un-derived, and their Bernstein weights
   are largest at the ends of the seam. A **lone** soft edge — two patches, one edge, nothing to
   contest, mirror residual exactly `0.000e0` — still folds by `8.632e-2` rad at its middle and
   `5.125e-1` to `6.703e-1` rad at its two ends. This is the gap `crate::continuity` documents and
   accepts; it is not a junction effect.
2. **A regular grid is therefore not smooth either.** On a 4 by 4 grid whose every interior vertex is
   valence 4, with every interior edge soft: edge middles median `2.128e-1`, worst `6.989e-1` rad;
   valence-4 vertices median `3.529e-1`, worst `8.259e-1` rad. The grid has no junction in it at all.
3. **A T-junction is not representable.** `EdgeKey` is a pair of endpoint ids, so a vertex lying in
   the middle of a quad's edge has nowhere to be recorded: the edge never learns the vertex exists
   and reads as a **boundary edge with one face**. Merging two cells of a grid to make one T-junction
   produced **18** such one-face edges. That is a cracked mesh, not a smooth one and not a continuous
   one.

Measured by `examples/junction_probe.rs`, all three at degree 3 on a curved height field. The absolute
figures depend on the surface's curvature; what does not depend on the surface is that none of them is
zero.

## And our own input is nowhere near regular

`examples/valence_probe.rs`, on the sample massif:

| mesh | valence 4 | valence != 4 | valences present |
|---|---|---|---|
| source triangles (2382 faces) | — | almost all | 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 14, 16, 18, 20 |
| after Doo–Sabin (7146 quads, 7591 vertices) | **804 (10.6 %)** | **6787 (89.4 %)** | 2, 4, 6, 8, 10, 12, 14, 16, 20, 24, 28, 32, 36, 40 |
| authored blocks (190 nodes) | — | all | 168 nodes with **8** blocks, 1 node with **56** |

So the per-patch model covers 10.6 % of the vertices of its own front end's output, and the block
level is worse: one node has fifty-six patches meeting on it.

## What the literature already answers, and where it stops

**The mechanism is knot multiplicity on one shared net, not a per-edge tag.** A tensor-product
B-spline surface encodes continuity structurally: multiplicity `m` at degree `p` gives `C^(p-m)`, a
simple knot is a G1 seam and multiplicity `p` is a crease. There is no contest because the control
net is global and no patch owns a point. That is why `proposals/two-soft-edges-one-face.md`'s
over-determination disappears in that model rather than being solved in it.

Recorded in this repository:

- `investigate/nurbs-libraries.md`, "Multi-patch continuity is the actual gap": *"No crate in the
  table above has multi-patch assembly, a shared-edge topology, and a per-edge continuity constraint.
  For reference, OCCT does have the last of these natively"* — `GeomAbs_Shape` enumerates
  `C0, G1, C1, G2, C2, C3, CN`, `BRep_Tool::Continuity` falls back to `C0` when no regularity is
  stored, and the measured ceiling is `BRepLib::EncodeRegularity()` "can set only C0 and G1
  continuity in general case".
- `investigate/nurbs-izing-arbitrary-polygons.md`, gap 3, verbatim: *"**Multi-patch G1 continuity as
  an automatic post-process.** Nothing found."* Gap 2 is the same wall one step earlier: segmentation
  and parameterization have *"no production-grade open solution"*.
- The SubD obstruction, same file: *"a quad cage with extraordinary points (valence != 4) cannot be
  compiled to a NURBS of any degree, because a tensor-product B-spline patch has exactly four control
  points per corner."* PHT-splines are the one row of
  `investigate/hierarchical-spline-bases.md`'s table marked arbitrary-topology yes, at
  generally-not-standard-NURBS.

**So this crate is working inside a gap the 2026-10-07 survey recorded as empty**, having chosen the
per-patch side of it.

## Three things this repository has said about the representation, and one correction

The question "why Bézier and not NURBS" is a false dichotomy, and the decision on record is narrower
than either half of it. `proposals/hierarchical-nurbs-multipatch.md`: *"Exact NURBS fidelity (exact
circles/cones/spheres, CAD export)? **No.** Rational weights are dropped; everything is polynomial
B-spline."* `RectNet` carries `degree_u`, `degree_v` and `points` — no `weights`, and no knots at
all. Each patch is a Bernstein net on its own `[0,1]^2`.

**Correction to a claim made in discussion on 2026-10-08.** It was said that a point shared by four
soft edges "should have been one face", so no smoothness should be demanded there. The first half is
right — a valence-4 vertex with four soft edges is a regular interior point of a 2 by 2 block, and
four patches in a 2 by 2 arrangement with C1 joins *is* a single piecewise-Bézier surface. The second
half does not follow from this measurement: on a grid with **no** junction at all, the vertices still
read `3.529e-1` median. What fails there is the un-derived row ends of failure 1, not the junction.
Relieving the junction question would not relieve the surface.

## The replacement question

Not "how do we make it smooth", but **"which of these costs do we pay"**:

| route | what it gives | what it costs | measured yet |
|---|---|---|---|
| **degrade** — C0 or a hard tag at every extraordinary vertex, G1 only in regular regions | standard NURBS, knot insertion, CAD export | the mesh is 89.4 % extraordinary, so "G1 regions" is the minority of the surface | no |
| **eliminate** — rewrite the authoring so every vertex is valence 4 | standard NURBS | removes failure 3 and the junction question, and buys **nothing** against failures 1 and 2, because a regular grid already reads 12°–20° | partially: this table |
| **change basis** — PHT, T-splines, PN-triangles | extraordinary vertices native, local refinement | not standard NURBS, no commercial kernel implements any of them | no |
| **hybrid** — shared net in regular regions, something else at junctions | both | two representations and a seam between them that is itself a junction | no |

The two cells that are not yet measured are the honest gaps in this file. Deciding between the routes
needs them, not more of the above.

## Where the rest of this sits

- `proposals/two-soft-edges-one-face.md` — the interior control-point contest. **Secondary** to this
  file: it is what failure 2 looks like when two of a face's own edges are soft, and it is bounded
  rather than structural.
- `examples/junction_probe.rs` — measurements 1 to 3 above.
- `examples/valence_probe.rs` — the valence table.
- `examples/contested_probe.rs` — the interior contest, and the arbitration that removed its
  dependence on tag order.
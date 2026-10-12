# Hierarchical multi-patch freeform: mesh faces as editable patches, edges hard or soft

Agent's understanding of a superior-approved proposal. **Not merged into `SPEC.md`** — that file
is human-editable-only per `AGENTS.md`. The block marked *Lift this into SPEC.md* below is the part
that should become normative.

Source proposal: [proposals/hierarchical-nurbs-multipatch.md](../proposals/hierarchical-nurbs-multipatch.md)
· approved 2026-10-07 · revised 2026-10-07 after the first implementation attempt · target:
`mountain`'s `FacetMesh`

**Status: soft edges are not continuous yet.** Hard edges, shared boundaries, elevation and editing
are implemented and measured. The G1 invariant is exact where it applies and incomplete where the
adjacent row's two ends are concerned; the measured cost and the fix are in
[Continuity](#continuity) and in the handoff. Everything below that claims otherwise has been
corrected, and the corrections say what they corrected.

## Decisions on the record

| Question | Decision |
| --- | --- |
| Exact NURBS fidelity (exact circles/cones/spheres, CAD export)? | **No.** Rational weights dropped; polynomial splines only. |
| Hierarchy | Multi-level linear sum, empty level = zero bias. Coarsening out of scope. |
| Soft-edge freedom | The slave's adjacent row is **derived**, not free; one slope parameter per edge. |
| PN / blended / Hermite patches | **Rejected** — no linear tensor-product structure, so the hierarchy cannot exist for them. |
| Face basis | **Quads.** A triangle mesh is Doo–Sabin subdivided first; every patch is a rectangular Bernstein net. *(Revised 2026-10-07 from triangles — see below.)* |
| Continuity layer | **Rectangular only.** A rectangle's adjacent row lines up index by index with its boundary. A triangle's does not, and that is fatal, not inconvenient. |

## What changed on 2026-10-07, and why

The first implementation put a simplex Bernstein net on every triangle and wrote a soft-edge
invariant that mirrored one adjacent row into the other. It was written up here as exact. It was
not, and the probe found it: **1.49e-8 rad with the edge undragged, 6.92e-2 after a corner moved,
1.29 rad once every edge was soft.** The formula had passed a degree-2 single-edge check because
degree 2 is the one degree where the two sides' rows happen to line up.

The reason is structural. A degree-`n` triangle has `n` interior positions along one side of an
edge and `n-1` along the other, and no assignment makes them one to one. G1 is a statement about
the cross-boundary *derivative*, which on a triangle sums over a row that has no index
correspondence with its neighbour's. A rectangle's row has `degree + 1` entries, exactly as many
as the boundary beside it, and position `t` on one side names the same place as position `t` on the
other.

So the mesh is subdivided to all quads first. That is the price of the correspondence, and it is
paid once at construction rather than at every continuity operation.

There was a second candidate — a triangular macro-patch made of four Bézier triangles, per
Halbfleiß & Peters, <https://hal.science/hal-00319653> — which keeps triangles but costs 4× the
storage and lands on the same vertex-consistency problem. Not taken.

---

## Lift this into SPEC.md

### Construction: triangles to quads

The input mesh is triangles. Every triangle becomes **three quads** by the Doo–Sabin rule: each
corner, the midpoints of its two incident edges, and its centroid.

Doo–Sabin rather than Catmull–Clark because a boundary edge yields one midpoint and one quad,
which is the same rule applied to a triangle with no neighbour on the far side. **No boundary case
exists.** A mesh off a mountain silhouette has a boundary.

The subdivision **refines without moving the surface**: the four corners of each quad are coplanar,
and the three quads of a triangle cover it exactly. Measured over 24 triangles × 3 quads × 81
bilinear samples with an independent evaluator: worst distance off the source plane `1.047e-15`,
worst excursion outside the triangle `6.661e-16`, worst relative area error `2.214e-16`. Coplanar,
contained and equal in area means the three convex quads tile the triangle with no gap and no
overlap, so level 0 of a quad patch *is* the face.

Subdivision is a pure insertion: original vertices keep their indices, midpoints and centroids are
appended, and every new vertex records which original edge or face it came from. A drag on the
caller's own mesh needs no map back from the derived mesh.

### Representation

A model is a mesh plus, per edge, a continuity tag. A **patch** is one mesh face carrying a
rectangular Bernstein control net:

```
Patch:
  base       Vec<CP>      absolute control net, rectangular, row-major in u
  offsets    Vec<Vec<CP>> per-level displacement fields, offsets[j] empty until level j is refined
  edge_roles [EdgeRole; 4] Hard | Soft, with which neighbour each soft edge ties to
```

- The **base** net at level 0 is a **degree-1 bilinear net** through the quad's four corners, so a
  freshly built model reproduces the input mesh exactly. There is no construction error to hide.
- `offsets[j]` is the **displacement** of level `j+1` against level `j`, never an absolute net.
  This is the storage decision that makes "an empty level adds no bias" true rather than
  approximate: elevation is linear in the control points, so a zero displacement field elevates to
  a zero displacement field at every level, forever. Measured: a zero net at degree 6 after three
  elevations is `0.000e+00`, and an elevated patch still matches the reference bilinear map to
  `5.088e-16`.
- The evaluated surface is `base` plus the sum of all offset fields, each lifted to the current
  level. The lifted-and-summed net is **derived state** — cache it, invalidate on edit.
- The degrees may differ per direction. Nothing here needs them to, but a net that could only be
  bi-degree would make "elevate one side" unrepresentable, and the storage cost of allowing it is
  zero.

### Refinement

Refining raises the level by one, by **degree elevation**. It is **linear and shape-preserving**:
the surface does not move, and the new control points carry the interpolated base value, i.e. zero
offset.

Elevation is written as **two one-dimensional steps**, one per direction, composed. That is not an
approximation of a single step — elevation is separable over a tensor product and the two orders
give the same net. The tempting closed two-dimensional form is a *product* of the one-dimensional
weights; folding the two remainders into a single term is wrong, and disagrees with the separable
result at the centre point of a bilinear net.

```
B(n+1, k) = k/(n+1)·B(n, k-1) + (n+1-k)/(n+1)·B(n, k)
```

Measured, degrees 1 → 6: worst departure `7.026e-16`; linearity `9.992e-16`.

**Loop subdivision** (vertex / edge / face smoothing) is **not this design** and must not be
substituted: it moves the surface, and it is the reason people associate triangles with "infinite
hierarchical sum". If someone reaches for Loop here, that is the bug.

### Surface-point dragging

Moving a point *on* the surface by `ΔP` is an assignment, not an inverse problem:

> Move every control point in that point's support by `ΔP`.

Bernstein basis functions satisfy the partition of unity, so the support's basis values sum to 1
and the surface point moves by exactly `ΔP`. Cost is `O(|support|)`, constant time, exact, with no
iteration and no solve. Measured: `5.439e-16` over corners, edge interiors and the middle of the
domain, against a basis computed independently in the probe.

Which level receives the move is a product decision, not a numerical one — a slider from "coarse,
moves a wide region" to "fine, moves a local one".

### Continuity

This is the heart of the design, and it is **half built**. What is below is what the implementation
does and what it costs.

#### The invariant

Let `Q` be the shared boundary control row, `A` the master's adjacent row, `B` the slave's, all
indexed by position `t = 0..=degree`. The cross-boundary derivative at the boundary is

```
D(v) = degree · Σⱼ B(degree-1, j, v) · (A[j] − Q[j])
```

so both sides' derivatives are degree-`degree-1` Bézier curves whose control points are their
rows' offsets from the boundary. G1 asks for those two to be **parallel at every parameter**.

Placing

> **`B[t] = Q[t] + μ · (Q[t] − A[t])`**

makes the slave's offset the master's reversed and scaled by `μ`, so its whole derivative curve is
`−μ` times the master's: parallel everywhere.

| `μ` | meaning |
| --- | --- |
| `1` | **C1** — matched cross-boundary parameterization; `B = 2Q − A` |
| `≠ 1` | **G1** — reparameterization across the seam, tangent plane still continuous |
| `0` | `B = Q` — the slave's row sits on the boundary; the tangent plane comes from `A` alone |
| n/a | **hard edge** — no invariant; the boundary is shared and the interiors are independent |

**`μ` is one scalar for the whole edge.** *This corrects the earlier `μ[b]` per boundary position.*
A per-position slope is not available to give up: writing `B[j] = Q[j] + μ[j]·(Q[j] − A[j])` makes
the slave's derivative curve have control points `μ[j]·(Q[j] − A[j])`, which is a *different* curve
unless every `μ[j]` is equal. Allowing a non-constant `k(v)` would need `k(v)·D_master(v)` to land
back in the degree-`degree-1` Bézier space; the product is degree `2(degree-1)` and generically
does not. One free scalar per edge is the whole of it.

Consequence: on the slave side, an edge's adjacent row goes from `3(degree-1)` free coordinates
(its private interior) to **one**. At degree 3 the soft edge removes five, not the two an earlier
draft claimed. A hard edge removes none.

#### Where the invariant stops, and what that actually costs

The invariant is applied over `t = 1..degree`, **not** `0..=degree`. Positions 0 and `degree` are
points of the *perpendicular* edge's shared boundary. *This corrects the earlier claim that forcing
them would demand `μ = −1` — see "the `μ = −1` argument is withdrawn" below. It does not force `μ =
−1`; it creates a collision, and only when two seams create it at the same place.*

The skip is not a corner-case loss. `B(degree-1, 0, v)` and `B(degree-1, degree, v)` are **positive at
every interior `v`**, so the two offsets the invariant leaves free enter `D` along the entire edge.
Measured at degree 3 on 72 quads:

| | measured |
| --- | --- |
| soft edge, interior positions | `4.771e-2` rad |
| soft edge, both end positions | `1.307e-1` rad |
| hard edge, every position | `1.780e-1` rad |
| the two faces' **along**-edge derivatives | `2.107e-8` rad |
| private-interior residual against the invariant | `0.000e+00` |

A soft edge is measurably smoother than a crease and is **not** continuous. *This corrects the earlier
claim that the cost landed only on the very ends of a seam.*

#### Ruling superseded 2026-10-07, again: the `μ = −1` argument is withdrawn

The implementation and this document previously said the two ends were **contradictory** — that the
ends are shared geometry, that writing one as `Q + μ(Q − A)` while sharing demands it equal `A` forces
`μ = −1`, and that `μ = −1` is a fold on the whole edge. `quad_continuity_probe` measurement 12,
commit `b2f9777`, **refutes that** by running the same invariant over `0..=degree`, in
`PatchMesh::enforce`'s own order (share boundaries first, then soft writes, so the soft edge has the
last word exactly as in the shipped model).

| case | soft gap, interior / ends | contested places | verdict |
| --- | --- | --- | --- |
| one soft edge | `0.000e0` / `2.107e-8` rad | `0` | **exact G1** |
| two soft edges meeting at a vertex | `2.488e-2` / `7.438e-2` rad | `1` | not exact |
| the full sample (32 soft, ≤1 per face) | `2.847e-1` / `1.300e-1` rad | `13` | not exact |

Boundary splitting stays `0.000e+00` in all three, and in the lone-seam case the sharing reclaims
`5.0e-16` of what the soft write moved. **So `4.771e-2` rad is not a limit of the representation.
It is the price of a choice, and the choice buys nothing.**

#### What would fix it, and it is countable

A row end is one step in from its own edge, which puts it on the **perpendicular** edge's boundary
row, one step along. When two soft seams meet at a mesh vertex their two row ends therefore land on
the *same place on the same shared edge*: two equations, one value, and the boundary sharing can only
give it to one of them. That is the whole obstruction, and the probe counts it rather than arguing
it.

Across all three cases above, **a seam is exactly continuous if and only if no shared place is written
by two seams.** That equivalence is asserted by the probe, not merely observed.

**So the missing degree of freedom is per contested place, not per edge.** A corner scale `κ`,
independent of the edge's `μ`, would buy exactly that — and on this evidence it is the only thing `κ`
would buy.

#### Ruled out 2026-10-07: resolving a contested place by deriving one master's corner

Before building `κ`, the cheaper question: at a contested place two seams want
`μ_a · (Q − A_a) == μ_b · (Q − A_b)` on **one** shared value, so holding one master's corner free
forces the other. `A_b = Q − (μ_a/μ_b) · (Q − A_a)` always has a solution and adds no parameter.

`quad_continuity_probe` measurement 13, commit `2edfe14`. It does not work:

| | measured |
| --- | --- |
| contested places on the full sample | `13`, of which `0` have both seams on the same face |
| seams exactly continuous — shipped | **12 of 32** |
| seams exactly continuous — whole row | **13 of 32** |
| seams exactly continuous — whole row + derived corners | **9 of 32** |
| derived corners that land **on** their seam | **6 of 13**, offset `0.000e+00` |

Fewer seams repaired, and six corners collapsed exactly onto the seam — smooth, and a pinch. **So `κ`
is not decoration.**

Two instruments this also corrected. **The worst gap is the wrong number here**: it came out identical
to four figures with and without derivation, because the maximum over 32 seams sits on an untouched
one. Measurement 13 counts seams instead. And **the collision is always cross-face** — no contested
place has both seams on one face.

#### Also ruled out 2026-10-07: the corner scale κ

Superior chose a corner scale `κ` on the strength of measurement 12. `quad_continuity_probe`
measurement 14, commit `605e67d`, sweeps the whole feasible family and finds it **inert**:

| κ | seams exactly continuous | worst gap | boundary split | derived corner's distance from its seam |
| --- | --- | --- | --- | --- |
| 0.1 | 9 of 32 | `2.847e-1` rad | `0.000e+00` | `3.196e-1` |
| 1 | 9 of 32 | `2.847e-1` rad | `0.000e+00` | `2.840e0` |
| 10 | 9 of 32 | `2.847e-1` rad | `0.000e+00` | `3.446e3` |

Four orders of `κ` and the count never moves — below both the shipped model's 12 and the whole-row
model's 13. The sweep doubles as its own control: the derived corner travels across four orders, so
`κ` **is** read and applied; the count simply does not depend on where that corner lands.

**The decision is reversed.** The reasoning behind it was sound — a collision is a point with two
equations and one value — but the mechanism was never measured, and it does not work.

#### Earlier corner-stencil findings, and what survives them

Measurement 10 (`4c33909`) and measurement 11 (`597ecf6`) ran a **corner stencil** algebra over every
interior vertex: a recursion `q_(i+1) = −m_i · q_(i−1)` on `n` corner-adjacent points, which because it
steps by two splits into `gcd(2, n)` chains, with `m_i` being `μ_i` or `1/μ_i` depending on which side
of the seam the lower face id falls. Its measured results stand as results about **that algebra** —
chain counts matching `gcd(2, valence)` everywhere, and non-degeneracy only at even `n ≡ 0 (mod 4)`
(`29 of 29` valence-4 vertices non-degenerate at `μ = 1`; `0` of 12 valence-2, `0` of 29 valence-3,
`0` of 6 valence-6), with projection driving `μ` to `−1`.

What measurement 12 withdraws is the reading of that algebra as **the** obstruction. A lone seam on
this same mesh is exactly G1, and it has a valence-4, valence-3 and valence-2 vertices on it like any
other. So the parity condition is a property of the `q_(i+1) = −m_i · q_(i−1)` *model*, not of the
geometry. Treat it as **superseded**, not as evidence about what the implementation can do.

The escapes, none of them cheap. One was cheap to *measure*, and has been — `4c33909`, `597ecf6`,
`quad_continuity_probe` measurements 11 and 12:

| Escape | What it costs |
| --- | --- |
| ~~Higher degree~~ | **Measured: buys nothing, on either count.** `4.771e-2`, `4.599e-2`, `4.642e-2`, `4.686e-2` rad across degrees 3–6, worst relative drift `0.036`. The worst measurement sits at the first interior position, where the contaminating end weight is `(1 − 1/degree)^(degree−1)`: `0.444` at degree 3, `0.402` at degree 6, heading for `e⁻¹` and not for zero. **Elevation is not slack.** |
| ~~Derive one master's corner at a contested place~~ | **Measured and rejected** (`2edfe14`, measurement 13): repairs **fewer** seams (`13 → 9` of 32) and puts 6 of 13 corners exactly **on** their seam, offset `0.000e+00` — smooth, and a pinch. The formula always has a solution; the solution is the expensive part. |
| ~~A corner scale `κ` independent of the edge's `μ`~~ | **Measured and rejected** (`605e67d`, measurement 14): inert. Four orders of `κ` and the exact-seam count never leaves 9 of 32. The derived corner travels `3.196e-1` → `3.446e3`, so `κ` *is* applied — the count just does not depend on it. |
| ~~Lazy subdivision (only subdivide faces a soft edge needs)~~ | **Not a fix for this problem.** It cannot remove a contested place: a soft edge needs both its faces to be quads, so every soft edge survives (`caf5fe2`, measurement 15). Contested places stay at 13. |
| A soft-edge set where no two seams share a mesh vertex | **Measured sufficient**: `0` contested places gives exact G1. Not a general answer — a Doo–Sabin mesh cannot be triangulated by seams without meeting — but it is the configuration the current model is already exact on, and it is now measurable rather than assumed. |
| Accept the measured gap | Zero work. The join is smoother than a crease (`4.8e-2` against `1.8e-1` rad) and not continuous. |

#### Ruling, superseded 2026-10-07: project a violating `μ`, do not refuse

Ruled by superior on the morning of 2026-10-07. **Measured the same afternoon and withdrawn**: see
below. Kept here because the withdrawal is the finding.

#### What the algebra actually is, and why projection is not a fix

`quad_continuity_probe` measurement 10, commit `4c33909`, runs this over every interior vertex of
the real subdivided mesh.

At a vertex with `n` faces in cyclic order, writing `q_i` for the corner-adjacent point of seam `i`
measured from the vertex itself, the invariant at the corner says

```
q_(i+1) = −m_i · q_(i−1)
```

**Three things come out of that, none of which were in the earlier draft.**

1. The recursion steps by **two**, so the `n` equations are not a coupled system. They split into
   `gcd(2, n)` independent chains, each closing on its own scalar condition. Measured: zero vertices
   whose chain count disagreed with `gcd(2, valence)`.

2. **`m_i` is not `μ_i`.** A seam's master is whichever of its two faces has the lower id, which does
   *not* go round a vertex consistently, so `m_i` is `μ_i` on some seams and `1/μ_i` on others. The
   condition `(−1)^valence · Πμ == 1` written earlier assumed a consistent orientation and is wrong
   in general. The real condition is on `m`.

3. **A vertex is never unsolvable.** When the product is wrong the only solution is `q == 0`, the
   pinch. It is *non-degenerate* solutions that the condition decides — and there the result is bad.
   Every `μ` and every `1/μ` is positive when `μ` is positive, so the product is positive, and an
   **odd-valence vertex can never be non-degenerate at any positive `μ`.** On this mesh that is the
   **29 Doo–Sabin face centroids — one pinch per face of the source mesh** — plus 12 valence-2 and
   6 valence-6 vertices. Only the 29 valence-4 vertices are non-degenerate at `μ == 1`.

Projecting onto feasibility drives one `μ` to **`−1`**, measured (`1.000 1.000 −1.000` at every
valence-3 centroid), and `μ = −1` is `B = A`: the master row *copied* rather than reflected, a fold
in the row, on the whole edge and not only at the corner. It then propagates to the vertex at the
seam's other end. **The projection ruling does not survive contact with the algebra.**

**What this does not say.** The implementation does not apply the invariant over a row's ends at all;
the corner-adjacent points are governed by boundary sharing and nothing pinches. At the time this
was taken to mean that exact G1 at a seam's ends is unreachable on a Doo–Sabin mesh with positive `μ`.
**Measurement 12 takes that back**: exact G1 *is* reachable, and the parity result above does not
stand in its way. See "the `μ = −1` argument is withdrawn" above.

#### Over-determined points inside one patch

*A soft edge's private interior is claimed by exactly one edge per face; this corrects the earlier
claim that rows never collide.* A quad's four adjacent rows meet at `(1,1)`, `(1,degree-1)`,
`(degree-1,1)` and `(degree-1,degree-1)`, so two adjacent soft edges on one face do share a point
and cannot both hold there. Measured: the two-edge sample's worst gap is `1.389e-1` rad against
`4.771e-2` with at most one per face.

The model **refuses the edit** rather than guessing — `Edit::Overconstrained { by: 2 }`, and a
refused edit changes nothing (verified by comparing every control point before and after). Applying
anyway would let the enforcement order pick a winner, so the same drag would give different results
depending on the order the edges were tagged in. The **tags** are still allowed to coexist, because
a user asking for a smooth patch is entitled to more than one smooth seam; what it costs is the
number above.

At every degree ≥ 2 there are points owned by exactly one row — at degree 3, `(1,2)` and `(2,2)`
on a `MinU`/`MaxU` pair — so single-owned dragging is available at degree 3, which the triangle
basis never offered.

### Where layers may live

A layer summed across the whole model would smooth every crease, because the sum is global.
**Layers are therefore partitioned by hard edges**: a hard edge cuts the model into connected
components, layers never cross it, and creases are structurally impossible to erase. Inside a
soft-edge component all faces share one level.

The continuity invariant only has to hold on the **lifted-and-summed net at the current level**.

**This is what makes option A better than an explicit normal condition, and it is measured.** The
invariant is a *homogeneous linear operator* in `(A, B, Q)` for a fixed `μ`, so the set of
satisfying configurations is a linear subspace: if two of them sum, the sum satisfies it too. A
layer therefore keeps satisfying the invariant when it is added to another, and the levels stay
independent. An explicit tangent-plane condition is linear in the slave's points but *not* in the
master's, because the master's normal is itself nonlinear in the master's points — it would force a
re-enforcement after every edit of any level.

**This claim is now measured rather than argued** (`layers_probe`, `628ca7a`). Homogeneity is
stronger than affineness and the difference is the design: `E(base) + E(offset)` equals
`E(base + offset)` only if `E` has no constant term. Measurement 1 is the only evidence for that —
a fresh level is `0.000e+00` before and after enforcement, exactly — and measurement 3 is the
composition that depends on it, `2.035e-15` with base and displacement both broken first. Neither
number means anything without the other.

**A second claim this section made implicitly turned out to be false, and only the hierarchy could
show it.** `enforce` never gave a mesh vertex a single value: a face's corner is the end of the
shared boundary row of *each* of the two edges meeting there, so two sharings write it and the later
one wins. On a base level every corner is the same `positions` entry, the copies start out equal,
and the ordering never shows. On a displacement level nothing forces that, and the leftover measured
`3.666e-1` across 19 corners, every one of them at a row end. `enforce` now reconciles each vertex
through the same `write_vertex` `set_control_point` already used, and the four existing probes'
numbers are bit-identical afterwards.

---

## What this is not

Not hierarchical B-splines, not PHT, not T-splines, not T-NURCC. The hierarchy is a storage and
editing decomposition over one flat spline; there is no nested element structure and no T-junction.
Not CAD: no booleans, no fillets, no STEP. Not a fitting pipeline.

## Open

1. ~~**Soft edges cannot be exactly continuous when two seams write the same shared place.**~~
   **Accepted 2026-10-07.** Superior's decision, after measurements 12–15 measured and closed
   every route found for closing it. What ships:

   | | |
   | --- | --- |
   | measurably smoother than a crease | **yes** — `4.771e-2` rad interior, `1.780e-1` for a crease |
   | never a crack | **yes** — boundary split `0.000e+00` |
   | tangent-plane continuous *along* the edge | **yes** — along-edge derivatives agree to `2.107e-8` rad |
   | tangent-plane continuous *across* the edge | **no** — and this is the accepted cost |

   The gap is **not a limit of the representation**: measurement 12 measures exact G1 at `0.000e+00`
   rad on any seam no other seam collides with. `quad_continuity_probe` measurement 3 asserts the
   promise above against a regression ceiling of `6.0e-2` on the interior, and measurements 12–15 are
   kept as **guards** — each passes while the route it ruled out stays ruled out, and any of them
   going red means this decision is stale.

   **This closes the soft-edge question.** What it does not close is the representation question
   below: Doo–Sabin's 29 valence-3 centroids are what put two seams on the same shared point in the
   first place, and that is a floor, not a parameter.

2. **Seam tessellation.** **Done, `8e8e84e`.** `PatchMesh::tessellate` samples every shared boundary
   once and hands both faces the same indices, so the two cannot disagree by construction. The
   predicted numerical reason turned out to be wrong (`0.000e+00` measured, with a control that
   fires at `7.587e+02`); what remains is the storage and the double evaluation.
   **The rasteriser's half is done as of `c8677fa`**: `Tessellation::triangles()` with the winding
   derived from the grid, and `Face::normal` for a caller that would rather ask the patch than derive
   a normal from a triangle. It measured the soup's rim against an exact integer identity (3552 =
   888 × 4), confirmed it has no crack and no degenerate triangle, and found that 48% of it faces the
   wrong way. See the verification status.
3. **Face granularity — decided in direction, blocked on the authoring.** `mountain` authors **182
   block faces** and the front end cuts each into 13.1 triangles and then 7146 patches, which is 39×
   more faces than the authoring structure calls for. Superior ruled 2026-10-08 that the block is the
   face and that **the sampled polylines are a hint, not an invariant**: no interpolation to preserve,
   no difference to minimise, so no basis change. A `k`×`k` control net taken from the block's own
   surface is the whole construction, and `examples/block_face_probe.rs` measures what it costs — a
   median of `2.4e-6` to `3.1e-6` against the mesh's own `1.526e-5` `f32` precision, and at most
   `5.5e-6` for every block whose faces cover their own `uv` box. The worst column, `1.5e-1` to
   `3.7e-1`, belongs entirely to the 89 blocks that do not, because the grid runs off the surface and
   carries extrapolated control points. The blocker is the **unequal runs**: 124 of 182 blocks have two
   boundaries sampled at different rates, so neighbours cannot share a run as one curve. Ribs are two
   points and work regardless. Full argument in `proposals/block-face-front-end.md`.
4. **Degree policy.** All patches currently share one degree, because the invariant assumes both
   faces of an edge agree. Peters' lower bound ([arXiv:1709.02994](https://arxiv.org/abs/1709.02994))
   says arbitrary-topology quad meshes generally cannot be covered by G1-connected *bi-cubic* Bézier
   patches in the standard layout, and Ueshiba & Roth's Doo–Sabin route
   ([10.1109/IM.1999.805360](https://doi.org/10.1109/IM.1999.805360)) succeeds partly because that
   mesh has special structure. This has **not** been tested here and is the obvious thing to try if
   the corner stencil turns out to need more points.
5. **Storage integration.** `SPEC.md` `Mesh` carries `Topology::{Triangles, Segments}`. **This needs
   a `Quads` variant** — the design's faces are quads after subdivision. Whether the control net is
   stored as mesh point data or its own structure is undecided. **The net is now one buffer per
   level** (`crate::level::Level`, a face-major arena with a borrowed face view), so "its own
   structure" has an answer for the net; what is still open is where it sits against `SPEC.md`'s
   point schema.
6. **Where the front-end lives.** **Settled 2026-10-08: `gallery` is the composition point.**
   `mountain` must not know a freeform surface exists and `patch` must not know what a massif is,
   neither may depend on the other, and `gallery` is the only crate that sees both. The conversion is
   `gallery::patchmesh::from_facet`, one function; the probe dev-depends on `gallery` through the
   cycle `scene` already uses for its tests, which the published dependency graph never sees.
7. **Dependency choice.** `axiolid-nurbs` (MPL-2.0) has `invert_surface_certified` (the drag entry
   point), `insert/remove_surface_knot_u/v`, `elevate_surface_degree_u/v`, `analyze_surface`, and
   `axiolid-surface::BSplineSurface` exposes public mutable control nets. It is **rectangular**, so it
   now matches the basis. Licence MPL-2.0.

## Open items

1. **Give a block patch the domain its own faces cover.** 89 of 182 blocks leave part of their `uv` box
   uncovered, and sampling across the gap puts an extrapolated control point in the net that a higher
   degree amplifies. This is what turns the `1.5e-1` worst case into the `4e-6` one.
2. **Resolve the unequal runs.** 124 of 182 blocks have two boundaries sampled at different rates, so
   neighbouring blocks cannot share a run as one curve. Belongs to `mountain`'s authoring.
3. **Orient the source's faces**, as one traversal in `mountain`, independently of the granularity
   change and still needed for the 8 edges in concave blocks.
4. **A face with two or more soft edges cannot satisfy all of them.** Analysed and ruled on 2026-10-07
   — see "Over-determined points inside one patch" above. What was new on 2026-10-08 is that `enforce`
   resolved the contest by **write order**, which is the objection the ruling gives for refusing edits,
   and it was unguarded: tagging one mesh's 24 interior edges forwards and backwards moved 9 control
   points by `6.142e-2` and took the worst fold from `2.325e-1` to `3.330e-1`. **Fixed 2026-10-10** by
   keying the arbitration on `EdgeSlot`: same measured quality, order-independence exact. Averaging was
   measured and rejected — it lets the error spread, so the worst fold grows with the grid. The
   over-determination is untouched. `proposals/two-soft-edges-one-face.md`,
   `examples/contested_probe.rs`.
5. **Arbitrary topology cannot be made smooth under this representation at all.** Measured
   2026-10-10, and larger than item 4. A **regular** grid with no junction in it reads a median
   `2.128e-1` rad along its edge middles and `3.529e-1` at its valence-4 vertices, because the
   invariant leaves the two ends of the slave's row un-derived. A T-junction is **not representable**:
   `EdgeKey` is a pair of endpoint ids, so a vertex inside an edge has nowhere to be recorded and that
   edge reads as a boundary edge with one face. And our own front end's output is **89.4 %**
   extraordinary, valences 2 to 40, with one block-level node carrying 56 patches. **The question as
   asked is closed.** What is open is which cost to pay: degrade at extraordinary vertices, eliminate
   them by rewriting the authoring — which buys nothing against the first two failures — change basis,
   or go hybrid. `proposals/arbitrary-topology-smoothness.md`, `examples/junction_probe.rs`,
   `examples/valence_probe.rs`.
6. **Lazy subdivision**, a separate decision from the soft-edge one.
7. **Back-face culling is a non-issue, and an earlier claim of one is withdrawn.** See the correction
   in the verification status above.

## Verification status

Branch `feature/nurbs-multipatch`, HEAD `c681032`. **Eight probes green, one red on the winding defect
in `mountain`, and one new defect found and written up in `proposals/two-soft-edges-one-face.md`.**

The red one is `facet_front_end_probe` measurement 6, and it is red because the defect is real and it
cannot see it from its own angle alone. `mountain::expand::push_face` gives every face the same sign of
plan area, which is a consistent winding only where no face's plan shadow crosses its neighbour's; 46 of
3351 interior edges are exactly such a crossing. It is not to be weakened to pass. Full classification
and the two refuted explanations are in the handoff and in `proposals/block-face-front-end.md`.

> **Correction, 2026-10-08.** The first draft of this section said that "a rasteriser with back-face
> culling drops everything behind them". **That is wrong**, and it was wrong in the same sentence that
> established the count. `raster_probe` measured it directly: `facing away 0`, `wholly facing toward
> 7146`, `mixed 0`. `Tessellation::triangles` and `Face::normal` both derive the winding from the
> **same quad corner order**, so they agree with each other whatever that order is, and the drawn
> triangles are consistently wound regardless of the source's winding. The real consequence of the 46
> edges is **not** dropped triangles; it is that **48% of the patches carry a normal opposite to their
> neighbours'** -- reversed shading, and continuity across those 46 seams that means nothing because
> the two sides disagree about which way is out.

| probe | result |
| --- | --- |
| `subdivision_probe` | 4/4 pass |
| `rect_elevation_probe` | 5/5 pass |
| `elevation_probe` | pass |
| `quad_continuity_probe` | 15/15 pass |
| `layers_probe` | 6/6 pass |
| `tessellation_probe` | 5/5 pass |
| `contested_probe` | 7/7 pass — a lone seam, crowded faces, tag order, fixed point |
| `junction_probe` | diagnostic, no assertions — where a seam fails, and what a T-junction does |
| `valence_probe` | diagnostic, no assertions — how irregular our own input is |
| `facet_front_end_probe` | **5/6 pass** — measurement 6 red on the 46 same-direction edges |
| `raster_probe` | **5/5 pass** — was red on the same defect; see the correction above |
| `block_face_probe` | 3/3 pass — what a patch per authored block would cost |
| `nurbs_probe` | 4/4 pass — the NURBS demo scene, and the crease/soft separation |

`whiting-patch` and `whiting-gallery` are clippy-clean. `whiting-definition`'s 7 pre-existing
warnings are untouched.

**`subdivision_probe` — 4/4 pass, commit `66f3820`**

| Property | Measured |
| --- | --- |
| Degenerate quads | `0` |
| Directed edges traversed twice | `0` |
| Worst distance off the source plane | `1.047e-15` |
| Worst excursion outside the triangle | `6.661e-16` |
| Worst relative area error | `2.214e-16` |
| Edges on more than two faces | `0` |
| Boundary edges, original → derived | `14 → 28` |
| Vertices moving when their definition does not depend on the drag | `0` |

**`rect_elevation_probe` — 5/5 pass, commit `9957e13`**

| Property | Measured |
| --- | --- |
| All-zero net after three elevations | `0.000e+00` |
| Elevation drift, degrees 1 → 6 | `7.026e-16` |
| Linearity of elevation | `9.992e-16` |
| Translating a whole support | `5.439e-16` |
| Elevated patch against an independent bilinear reference | `5.088e-16` |
| Corner error at degree 3 | `0.000e+00` |

**`quad_continuity_probe` — 15/15 pass, commit `dad5646`**

| Measurement | Measured | Verdict |
| --- | --- | --- |
| 1 all-hard model reproduces the mesh | `0.000e+00` corners, `6.661e-16` faces | pass |
| 2 shared boundary is one curve | `0.000e+00` | pass |
| 3 soft seam: smoother than a crease, never a crack, not continuous | `4.771e-2` interior, `1.307e-1` ends, `1.780e-1` crease | pass (accepted) |
| 4 both kinds present | 32 soft, 98 hard | pass |
| 5 a drag crosses a soft edge, stops at a hard one | `9.375e-2` / `0.000e+00` | pass |
| 6 elevation preserves every tag | `0.000e+00` residual at degree 4 | pass |
| 7 a boundary drag keeps the seam | `0.000e+00` split before and after | pass |
| 8 `μ` places the slave's row exactly | `0.000e+00` residual, all three values | pass |
| 9 two owners refuse, refusal changes nothing | `Overconstrained { by: 2 }` | pass |
| 10 the corner stencil's algebra | chain mismatches `0`; non-degenerate only at `n ≡ 0 (mod 4)` | pass (superseded) |
| 11 what a higher degree buys | `0.036` worst relative drift; nothing | pass |
| 12 whole-row invariant, lone seam | `0.000e0` / `2.107e-8` rad, `0` contested places | pass |
| 12 whole-row invariant, full sample | `2.847e-1` rad, `13` contested places | guard |
| 13 derive the contested corner | `13 → 9` of 32 seams, 6 of 13 corners pinch | guard |
| 14 sweep κ over four orders | `9 of 32` at every κ, corner travels `3.196e-1` → `3.446e3` | guard |
| 15 lazy subdivision | saves 0 of 24 here; 23 of 24 at one soft edge, 2 T-junctions | guard |

**`elevation_probe` — pass** for the simplex basis it still measures. Its fifth measurement,
which reported `1.490e-8` rad for a triangle soft edge, was **removed as a false claim** — it
measured one hand-built degree-2 pair where the rows happen to line up.

**`layers_probe` — 6/6 pass, commit `628ca7a`**

| Measurement | Measured | Verdict |
| --- | --- | --- |
| 1 a fresh level is exactly zero, before and after enforcement | `0.000e+00` / `0.000e+00` | pass |
| 2 the pass reaches a displacement level (control for 1) | `2.273e+00` written, `6.529e+00` left | pass |
| **3 levels are independent** | `2.035e-15`, ceiling `1.0e-12` | pass |
| 4 clearing a level returns to the base | undone `6.529e+00`, `==` the base | pass |
| 5 elevation, all hard, no displacement / with one | `6.661e-16` / `2.176e-15` | pass |
| 5 elevation, 32 soft, no displacement / with one | `8.170e-3` / `5.785e-2` | shipped property, guarded |
| 6 every shared hard edge is one curve, on every level | `1.540e+00` → `0.000e+00`, 520 → 0 positions | pass |
| 6 one value per mesh vertex | `2.273e+00` → `0.000e+00` | pass |
| 6 enforcement is a fixed point | `0.000e+00` | pass |

**Measurement 3 is the claim the layered design rests on**, and it needs measurement 1 to be true:
separately enforced levels give `E(base) + E(offset)` and the enforced sum gives `E(base + offset)`,
which agree only when `E` has no constant term. Measurement 1 is the only evidence there is for that,
so neither number means anything without the other.

**Measurement 6 found a fault.** `enforce` never gave a mesh vertex a single value. A face's corner is
the end of the shared boundary row of *each* of the two edges meeting there, so two sharings write it
and the later wins — invisible on a base level, where every corner is the same `positions` entry, and
`3.666e-1` across 19 corners on a displacement level. Fixed in `628ca7a`; the four existing probes'
numbers are bit-identical afterwards.

**`facet_front_end_probe` — 5/5 pass, commits `29a516a`, `e669b95`**

Runs `mountain::starter`'s authored massif rather than the regular f64 grid every other probe uses,
because the grid is the easy case twice over: it is manifold, and it has twice the precision of what
terrain arrives as. Since `e669b95` it measures `gallery::patchmesh::from_facet` itself rather than a
copy of it, and every number below is unchanged.

| Measurement | Measured | Verdict |
| --- | --- | --- |
| 1 the massif, and its edges | 1 414 points, 2 382 triangles, 3 795 edges: 444 boundary, 3 351 interior, **0 non-manifold**, 0 degenerate | pass |
| 2 Doo–Sabin keeps it manifold | 7 591 points, 7 146 quads, 14 736 edges: 888 boundary, 13 848 interior, **0 non-manifold** | pass |
| 3 level 0 reproduces it corner for corner | `0.000e+00`, exact because `f32` → `f64` is | pass |
| 4 the precision floor | adjacent `f32`s are `1.526e-5` apart at this scale (`2^-16`) | shipped property |
| 5 hard sharing, and elevation holding still | `0.000e+00` split; `1.776e-15` over 21 438 samples | pass |

**Measurement 4 is the one that constrains the design.** The surface is only ever as accurate as the
`f32` it arrived as, so `1.526e-5` is the floor under every edit — eleven orders coarser than the
`1e-16` the rest of this crate measures in. "Reproduces the mesh exactly" and "can be shaped finely"
are different questions, and only the first one is answered exactly.

**`tessellation_probe` — 5/5 pass, commit `8e8e84e`**

The hierarchy's first caller: `PatchMesh::tessellate` samples the surface into an indexed mesh, so
the surface is read for the first time rather than its control points. Runs on the same authored
massif at degree 4, side 5.

| Measurement | Measured | Verdict |
| --- | --- | --- |
| 1 `evaluate` and `composed` agree | `5.092e-13` over 28 584 points, ceiling `1.0e-12` | pass |
| 2 a second displacement moves the sampled surface | `8.211e+02` | pass |
| 3 the two faces on an interior edge hold the same indices | **0 of 13 848** disagree | pass |
| 4 what independent boundary sampling would cost | `0.000e+00`, control on a broken mesh `7.587e+02` | pass |
| 5 elevation holds still through a whole tessellation | `8.534e-14` over 116 113 points | pass |

**Measurement 4 refuted the reason this module exists.** The design sampled a shared edge once because
two faces sampling it independently sum the same Bernstein terms in opposite orders, and `f64` does
not round two orders the same way. Measured on this surface: `0.000e+00` over 69 240 positions. The
probe carries a positive control — the *same function* on a mesh whose rows genuinely disagree reports
`7.587e+02` — so the zero is a result and not a measurement that could not fail. **The reason to
share an edge is not numerical.** It is that the alternative stores a copy per side, evaluates twice,
and leaves a rasteriser to reconcile two numbers that happen to match today.

**Three faults, all caught by measuring rather than asserting.** A mesh vertex was sampled once per
incident edge, leaving **13 802 of 13 848** interior edges holding different indices for the same
point — agreement in value, not in place, which a rasteriser cannot detect. Sampling was also done by
control index, so the degree moved the samples and elevation appeared to move the surface by
`5.206e0` when it had not moved it at all. Welding vertices cut the point count from 137 994 to
**116 113**.

**The hazard that did not fire, and is now refused rather than tolerated.** `enforce` shares a
boundary when *exactly two* faces meet on it, and `subdivide` gives every triangle on an edge one
shared midpoint — so an edge with three triangles on it would come out looking fine and **crack
silently**. `gallery::patchmesh::from_facet` now returns
`Err(FacetError::NonManifold { edges })` instead. A fold is not a case to handle: a per-edge tag
describes a crease. This massif has no such edge, so the guard has never fired and **its refusal path
is not exercised by a probe**; the count on real data is, twice, by two independently written counts.

Verified from primary sources (docs.rs, OCCT reference and source):

- `axiolid_surface::BSplineSurface` public mutable fields incl. `control_points: Vec<Vec<DVec3>>`
  row-major in `u` then `v`; `jet(u, v)`.
  <https://docs.rs/axiolid-surface/latest/axiolid_surface/struct.BSplineSurface.html>
- `axiolid_nurbs` surface API: `iso_curve_at_u/v`, `invert_surface_certified`,
  `insert_surface_knot_u/v`, `remove_surface_knot_u/v`, `elevate_surface_degree_u/v`,
  `reduce_surface_degree_u/v`, `analyze_surface`. <https://docs.rs/axiolid-nurbs/latest/axiolid_nurbs/>
- OCCT has per-edge continuity natively — `GeomAbs_Shape` = `C0, G1, C1, G2, C2, C3, CN`;
  `BRep_Builder::Continuity(E, F1, F2, C)` writes, `BRep_Tool::Continuity(E, F1, F2)` reads and
  falls back to `C0`. <https://dev.opencascade.org/doc/refman/html/class_b_rep___builder.html>
  `BRepLib::EncodeRegularity()` sets only C0 and G1 in general.
  <https://occt3d.com/dev/tickets/0027383>
- Peters' lower bound on bi-cubic G1 stitched Bézier patches, with an explicit counterexample to
  ASC17: <https://arxiv.org/abs/1709.02994>
- Ueshiba & Roth, triangle mesh subdivided to quads then given rectangular Bézier patches:
  <https://doi.org/10.1109/IM.1999.805360>
- Halbfleiß & Peters, triangular G1 interpolation and the vertex consistency problem:
  <https://hal.science/hal-00319653>
- PN triangles guarantee *visual* smoothness, not everywhere tangent continuity — the reason an
  explicit normal condition was not simply taken from hardware tessellation:
  <https://doi.org/10.1145/364338.364387>

**Not verified — treat as an assumption, not as established fact:**

- The name "IJB-spline / interpolatory Jacobian blended" as the literature home of the
  displacement-vs-absolute framing. The framing itself is what is being decided; the citation is not.
- Loop subdivision rules, mentioned only as the thing to avoid.
- Whether biquartic lifts the corner stencil problem here. Peters' lower bound is about bi-cubic in
  a particular layout; whether the stencil at higher degree has more freedom is untested.
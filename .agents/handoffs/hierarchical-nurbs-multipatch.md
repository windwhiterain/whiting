# Handoff: hierarchical multi-patch freeform over `mountain`'s FacetMesh

Status: **direction reversed and re-landed; soft edges are exact but not yet continuous. The
hierarchy is in and it found a fault the base level alone could not reach.**
Branch `feature/nurbs-multipatch`, worktree `.worktrees/nurbs-multipatch`, HEAD `8f6aa66`.
`clippy` is clean and `cargo fmt` has been run. **Five probes, all green.**

Agent's understanding: [SPEC.align/hierarchical-nurbs-multipatch.md](../SPEC.align/hierarchical-nurbs-multipatch.md)
Proposal: [proposals/hierarchical-nurbs-multipatch.md](../proposals/hierarchical-nurbs-multipatch.md)

## Read this first

The design was **triangles**. It is now **quads**, because the triangle soft-edge invariant was
measured false and the reason is structural, not a bug:

> A degree-`n` triangle has `n` interior positions along one side of an edge and `n-1` along the
> other. No assignment makes them one to one. G1 is a statement about the cross-boundary
> *derivative*, which on a triangle sums over a row that has no index correspondence with its
> neighbour's.

A rectangle's adjacent row has `degree + 1` entries — exactly as many as the boundary beside it —
and position `t` on one side names the same place as position `t` on the other. So the mesh is
Doo–Sabin subdivided to all quads first, and every patch is a rectangular Bernstein net.

If you were reading an older version of this handoff, **ignore its "do not smuggle in a rectangular
control net" warning.** That trap was real under the triangle decision and is now exactly backwards.
The other traps below still stand.

## What landed

`crates/patch` — `whiting-patch`, no dependencies.

| File | What |
| --- | --- |
| `src/point.rs` | `Point3`. Shared by both nets; a displacement is the same type as a position on purpose. |
| `src/bernstein.rs` | `binomial`, `multinomial`, `basis`. Shared by both nets. |
| `src/simplex.rs` | `SimplexNet` — kept, still used by `elevation_probe` and as the subdivision's input shape. |
| `src/rect.rs` | `RectNet` — `count`, `from_corners`, `evaluate`, `elevate_u`/`elevate_v`, `elevate`. |
| `src/subdivide.rs` | Doo–Sabin. `subdivide`, `QuadMesh`, `Origin::{Corner, EdgeMidpoint, FaceCentroid}`. |
| `src/topology.rs` | Quad edges. `EdgeSlot::{MinU, MaxU, MinV, MaxV}`, `boundary_index`, `interior_index`, `mirrored_position`, `Topology`. |
| `src/continuity.rs` | `Continuity::{Hard, Soft}`, `slave_interior`, `SoftEdge`. The derivation, the corner-stencil condition, the accepted gap and the elevation cost live in the module doc. |
| `src/level.rs` | `Level` — every control point of one level in **one buffer**, face after face. `Face<'a>` is the borrowed hot-path view and allocates nothing. |
| `src/enforce.rs` | `Rules<'a>` (the mesh facts enforcement may read) and `enforce`, a free function over *any* `Level`. Free rather than a method because a hierarchy runs it once per level and the total surface is itself a level. |
| `src/patch_mesh.rs` | `PatchMesh` — `level`, `offsets`, `Edit`, enforcement order, seam routing, and the read-out split (`base_net` / `evaluate` / `composed`). |

Probes, all `cargo run -p whiting-patch --example <name>`:

| Probe | Verdict |
| --- | --- |
| `subdivision_probe` | 4/4 pass |
| `rect_elevation_probe` | 5/5 pass |
| `elevation_probe` | pass (simplex basis only) |
| `quad_continuity_probe` | **15/15 pass.** 3 asserts the promise actually shipped; 12 to 15 are guards, not aspirations. |
| `layers_probe` | **6/6 pass.** The hierarchy, and the fault it found. |
| `facet_front_end_probe` | **5/5 pass.** A real `FacetMesh`, not the sample. |
| `tessellation_probe` | **5/5 pass.** The hierarchy's first caller, and three faults it found. |

`examples/support/mod.rs` holds the one sample mesh and the hand-written bilinear reference the
probes share. **That reference is deliberately not the crate's evaluator** — a check that reuses the
code under test proves only that the code is self-consistent.

## The blocker, precisely

The soft-edge invariant is

```
B[t] = Q[t] + μ · (Q[t] − A[t])
```

and it is applied over `t = 1..degree`, **not** `0..=degree`. Positions 0 and `degree` are points
of the perpendicular edge's shared boundary.

> **The `μ = −1` argument this file used to make here is withdrawn.** Measurement 12, commit
> `b2f9777`, refutes it. Skip to "The obstruction, measured" below before planning anything.

That does not cost the ends of the seam. **It costs the whole seam.** The derivative is

```
D(v) = degree · Σⱼ B(degree-1, j, v) · (A[j] − Q[j])
```

and `B(degree-1, 0, v)` and `B(degree-1, degree, v)` are positive at **every interior `v`**. The two
free offsets enter everywhere.

Measured at degree 3 on 72 quads:

| | measured |
| --- | --- |
| soft, interior positions | `4.771e-2` rad |
| soft, both end positions | `1.307e-1` rad |
| hard, every position | `1.780e-1` rad |
| the two faces' **along**-edge derivatives | `2.107e-8` rad |
| private-interior residual against the invariant | `0.000e+00` |

The last two say the invariant is being enforced exactly. The gap is not a precision problem.

**Raising the degree does not fix it.** The end weights shrink but never reach zero. Do not spend a
turn on that.

### The obstruction, measured — this is the current state

`quad_continuity_probe` measurement 12, commit `b2f9777`. It runs the **same** invariant over
`0..=degree`, in `PatchMesh::enforce`'s own order — share boundaries first, then soft writes, so the
soft edge has the last word exactly as in the shipped model.

| case | soft gap, interior / ends | contested places | verdict |
| --- | --- | --- | --- |
| one soft edge | `0.000e0` / `2.107e-8` rad | `0` | **exact G1** |
| two soft edges meeting at a vertex | `2.488e-2` / `7.438e-2` rad | `1` | not exact |
| the full sample (32 soft, ≤1 per face) | `2.847e-1` / `1.300e-1` rad | `13` | not exact |

Boundary split stays `0.000e+00` in all three. In the lone-seam case the sharing reclaims `5.0e-16`
of what the soft write moved — so the row end survives the sharing, and **`4.771e-2` rad is not a limit
of the representation. It is the price of a choice that buys nothing.**

A **contested place** is a `(shared edge, position along it)` that two soft seams both write as a row
end. A row end is one step in from its own edge, which puts it on the *perpendicular* edge's boundary
row, one step along — so when two seams meet at a mesh vertex, their row ends land on the same place
on the same shared edge: two equations, one value.

**A seam is exactly continuous if and only if no shared place is written by two seams.** The probe
asserts that equivalence, it does not merely report the three data points.

**What this changes for the design.** The missing freedom is **one per contested place**, not one per
edge. A corner scale `κ` independent of the edge's `μ` buys exactly that. The parity/`gcd(2, valence)`
story below is a property of one model of the corner, not of the geometry — treat it as superseded.

### The zero-parameter route is closed — measurement 13

Superior chose `κ`. The cheaper question came first: can a contested place be resolved with no new
parameter? At one, two seams want `μ_a · (Q − A_a) == μ_b · (Q − A_b)` on **one** shared value, so
holding one master's corner free forces the other:

```
A_b = Q − (μ_a / μ_b) · (Q − A_a)
```

That always has a solution. It is not cheap. `quad_continuity_probe` measurement 13, commit `2edfe14`:

| | measured |
| --- | --- |
| contested places on the full sample | `13`, of which `0` have both seams on the same face |
| seams exactly continuous — shipped | **12 of 32** |
| seams exactly continuous — whole row | **13 of 32** |
| seams exactly continuous — whole row + derived corners | **9 of 32** |
| derived corners landing **on** their seam | **6 of 13**, offset `0.000e+00` |

Fewer seams repaired, and six corners collapsed exactly onto the seam. **`κ` is not decoration.**

Two instruments this corrected, both worth remembering:

- **The worst gap is the wrong number here.** It came out identical to four figures with and without
  derivation, because the maximum over 32 seams sits on an *untouched* one. Measurement 13 counts
  seams instead.
- **The collision is always cross-face.** No contested place has both seams on one face, so the
  sharing copy is never the cause — a second order bug (sharing *all* edges instead of only hard
  ones) silently undid every derivation and looked like a null result. `share_hard_boundaries` is the
  correct pass, matching `PatchMesh::enforce`.

**What `κ` now has to answer, which derivation cannot:** how much exactness to give up at one point,
on purpose, in a quantified way.

### …and κ is inert — measurement 14, which reverses the decision

Superior chose `κ`. Measurement 14 swept the whole family rather than trusting one member of it:

```
A_b = Q − κ · (μ_a / μ_b) · (Q − A_a),   κ from 0.1 to 10
```

Scaling the loser instead of the keeper is the same family under `κ → 1/κ`, so one sweep covers both.

| κ | seams exactly continuous | worst gap | split | derived corner, furthest from its seam |
| --- | --- | --- | --- | --- |
| 0.1 | 9 of 32 | `2.847e-1` rad | `0.000e0` | `3.196e-1` |
| 1 | 9 of 32 | `2.847e-1` rad | `0.000e0` | `2.840e0` |
| 10 | 9 of 32 | `2.847e-1` rad | `0.000e0` | `3.446e3` |

**Four orders of κ and the count never moves.** Worse than the shipped model's 12 and than the
whole-row model's 13.

The sweep is also its own control, and that is the part worth keeping: the derived corner travels
across four orders, so **κ is read and applied** — the count simply does not depend on where that
corner lands. An inert parameter and an unimplemented one look identical in the output; the travel
column is what tells them apart.

**So the corner scale does not buy exactness in the only family constructible without a crack.** This
reverses the decision taken on 2026-10-07. The reasoning behind it was sound — a collision is a
point with two equations and one value — but the mechanism had not been measured, and it does not
work.

### Lazy subdivision: not a fix, but a large independent win — measurement 15

Hard edges need nothing, soft edges need a quad, and only soft edges create contested places, so the
rule is the obvious one: **subdivide a triangle if and only if it carries a soft edge.**

It cannot remove a contested place — a soft edge needs **both** its faces to be quads, so the rule
keeps every soft edge — and the probe asserts that rather than assuming it.

On this sample it saves nothing, and the reason is structural: every triangle of a grid
triangulation carries a row-aligned edge, and row-aligned edges are what is soft here. All 24
triangles, 72 quads either way, no frontier.

The curve is the real answer, since one tag pattern is one point on it:

| soft edges | faces left alone | quads avoided | T-junctions |
| --- | --- | --- | --- |
| 1 | 23 of 24 | 69 | 2 |
| 2 | 22 of 24 | 66 | 4 |
| 4 | 20 of 24 | 60 | 7 |
| 8 | 12 of 24 | 36 | 7 |
| 16 (all) | 0 of 24 | 0 | 0 |

**Read this as two separate decisions.** It is *not* a fix for the soft-edge defect — contested
places stay at 13 whatever the rule does. It *is* a large win on control-net size and construction
cost whenever few edges are soft, at one T-junction per frontier edge. Deciding it has nothing to do
with the soft-edge question.

### Soft edges: accepted 2026-10-07, and the probe now passes

Superior accepted the measured gap. What that changed in the code is worth knowing before anyone
reads measurement 3 and thinks it is soft:

- **Measurement 3 no longer asserts "is it continuous".** It asserted that for months and therefore
  failed forever, and a probe that can never go green stops reporting anything at all. It now asserts
  the promise actually made — measurably smoother than a crease along the interior, never worse than
  one at the ends, along-edge derivatives agreeing, no crack — with the interior gap under a
  **regression ceiling** of `6.0e-2` against a measured `4.771e-2`.
- **Measurements 12 to 15 are flipped the same way.** Each one measured a route out and rejected it,
  so each is now a guard that passes while its route stays rejected and goes red the day it starts
  working.
- **All four probes exit 0.**

The lesson, which cost a day: **a probe that cannot go green is not a canary, it is noise.** The
first version of this measurement was a canary for a defect that had been *accepted*, so it was
firing forever and would have fired just as loudly on the next real regression.

### What the algebra was, and what survives of it

**Read this before touching the corner stencil. The "project" policy was withdrawn on the same day it
was ruled, and the reading of this algebra as *the* obstruction was withdrawn by measurement 12.**
`quad_continuity_probe` measurement 10, commit `4c33909`, runs the algebra over every
interior vertex of the real subdivided mesh.

At a vertex with `n` faces in cyclic order, `q_i` is the corner-adjacent point of seam `i` measured
from the vertex itself, and the invariant at the corner says

```
q_(i+1) = −m_i · q_(i−1)
```

Three results, none of which were in the previous version of this file:

1. **The recursion steps by two.** The `n` equations are not a coupled system; they split into
   `gcd(2, n)` independent chains, each with its own scalar closing condition. Measured: zero
   vertices whose chain count disagreed with `gcd(2, valence)`.
2. **`m_i` is not `μ_i`.** A seam's master is the lower-id of its two faces, which does not go round
   a vertex consistently, so `m_i` is `μ_i` on some seams and `1/μ_i` on others. The condition in the
   older drafts, `(−1)^valence · Πμ == 1`, assumed a consistent orientation and is **wrong**.
3. **A vertex is never unsolvable.** When the product is wrong the only solution is `q == 0`, the
   pinch. What the condition decides is whether a *non-degenerate* solution exists — and since every
   `μ` and every `1/μ` is positive when `μ` is positive, the product is positive and an **odd-valence
   vertex can never be non-degenerate at any positive `μ`.**

Measured on the real mesh:

| valence | interior vertices | non-degenerate at `μ == 1` |
| --- | --- | --- |
| 2 | 12 | 0 |
| 3 (Doo–Sabin face centroids) | **29** | **0** |
| 4 | 29 | 29 |
| 6 | 6 | 0 |

Projecting onto feasibility drives one `μ` to **`−1`** — measured `1.000 1.000 −1.000` at every
valence-3 centroid — and `μ = −1` is `B = A`, the master row copied rather than reflected: a fold in
the row, on the whole edge, and it propagates to the vertex at the seam's other end. **The
projection ruling does not survive contact with the algebra.**

**What survives.** The numbers above are correct *about this model*. What does not survive is reading
them as the obstruction: a lone seam on this same mesh is exactly G1, and it runs over valence-2, -3
and -4 vertices like any other seam. Measurement 12 supersedes that reading.

The escapes. **One has been measured and is dead — do not spend a turn on it again.**

| Escape | Cost |
| --- | --- |
| ~~Higher degree~~ | **Measured, buys nothing** (commit `597ecf6`, measurement 11). Gap `4.771e-2` → `4.599e-2` → `4.642e-2` → `4.686e-2` rad over degrees 3–6, worst relative drift `0.036`. |
| A corner scale `κ` independent of the edge's `μ` | **Now the only candidate.** Measurement 12 says the freedom needed is one per contested place, which is what `κ` is. Costs a new parameter, and `μ` stops being the only thing on an edge. |
| A soft-edge set where no two seams share a mesh vertex | **Measured sufficient**: `0` contested places ⇒ exact G1. Not available as a general answer — a quad mesh cannot be partitioned by seams without seams meeting — but it is now a measurable property rather than an assumption. |
| Accept the measured gap | Zero work. Smoother than a crease (`4.8e-2` against `1.8e-1` rad), not continuous. |

**Why the degree sweep came out flat.** Two separate reasons, and the second was a wrong prediction
of mine, corrected by the measurement rather than argued around:

1. The corner constraint cannot gain freedom. At a corner the cross-boundary derivative is
   `degree · (A[0] − Q[0])`, because `B(degree−1, j, 0)` is non-zero only at `j = 0`. The row's other
   entries never enter. So a higher degree does not give an odd-valence vertex a solution it does not
   already have — it already has one, the pinch, and that is the only one.
2. The gap does not shrink either. The worst measurement sits at the **first interior position**,
   where the contaminating end weight is `(1 − 1/degree)^(degree−1)`: `0.444` at degree 3, `0.422`
   at 4, `0.410` at 5, `0.402` at 6. It heads for `e⁻¹ ≈ 0.368`, not for zero. The defect is
   asymptotically a fixed fraction of the seam, so **elevation is not slack** — a phrase worth
   keeping, because the natural next thought is "just raise the degree".

The same problem in the literature, if you want the reference: Halbfleiß & Peters,
<https://hal.science/hal-00319653>, condition (IT). They name it *vertex consistency problem* and
their (IT) is an additional condition, which is consistent with this: the corner needs more than the
G1 constraints, and the something-more is not free either.

### Why the implementation does not just apply the invariant over the whole row

Because it **collides**, not because it cracks. Measurement 12 measured both: boundary splitting
stays `0.000e+00` in all three cases and the sharing reclaims `5.0e-16` in the lone-seam case, so a row
end is perfectly compatible with shared geometry. What fails is two seams writing the same place.
`PatchMesh::enforce_soft` keeps its `t in 1..degree` because the alternative is silent, order-dependent
loss of one seam's row end, and a silent wrong answer is worse than a bounded smoothness defect.
`quad_continuity_probe` measurement 3 fails so the bounded defect can never be shipped by accident.

## Things the probes caught, all fixed

1. **A degree-0 simplex net is one point, not three.** `(0+1)(0+2)/2 = 1`; the ladder starts at
   degree 1. `from_corners` now builds a degree-1 net.
2. **`from_corners(a, b, c)` is barycentric order, not storage order.** A degree-1 net stores the
   `k = 1` corner first.
3. **Enforcement order: hard edges first, then soft.** On quads the soft pass *reads* the shared
   boundary, so copying a hard edge's boundary afterwards would leave a row derived from a curve that
   has since moved.
4. **A corner belongs to every face around it.** Writing only one face's copy let the next
   enforcement pass restore a stale value from a face that owns an incident edge.
5. **The closed two-dimensional elevation formula is wrong** if you fold the two remainders into one
   term. Elevation must be two one-dimensional steps; the product form is what the separable result
   gives, and the shorthand disagrees with it at the centre point of a bilinear net.
6. **A green probe asserting a false claim.** `elevation_probe` measured `1.490e-8` rad for a triangle
   soft edge and passed; `continuity_probe` measured the same claim on a real mesh and got
   `6.92e-2` after a drag. The isolated measurement had used one hand-built degree-2 pair, the one
   degree where the rows line up. It was removed in `bacdc57`.
7. **A fault that only a second level could reach.** `enforce` never gave a mesh vertex a single
   value, and nothing showed it for two days of work: on a base level every face's corner is the same
   `positions` entry, so the copies start out equal and the write order never matters. A
   displacement level has no such guarantee, and the leftover measured `3.666e-1` across 19 corners.
   See "The hierarchy landed" below.
8. **A `0.000e+00` that was a measurement which could not fail.** Twice this session, and both times
   the cause was the same: a question asked at a degree or a size where the arithmetic cannot show
   the effect. A degree-1 row carries two terms and floating-point addition of two terms is
   commutative, so "does summation order matter here" could not come out any other way — and it
   reported `0.000e+00`, which read like a result. Separately, a null result in `tessellation_probe`
   was only made meaningful once the **same function** was run on a mesh known to disagree.
   **Every null result needs a positive control, not a plausible story about why it happened.**

## Six measurements that were measuring the wrong thing

These are the ones to not repeat:

- **Bending must be sampled inside a patch, not on the seam.** The seam is shared geometry and never
  moves whatever its tag says, so a probe that evaluates there reports zero departure for soft and
  hard edges alike. It now drags first, because on a planar level-0 mesh *every* `μ` leaves both
  patches planar — the mirror of an in-plane offset is in-plane.
- **`μ` must be checked as a residual, not as a tangent-plane ratio.** The ratio cannot be exactly
  `μ` while the row ends are free; that is a statement about the ends, not about `μ`.
- **Elevation preserving a tag means the invariant residual is still zero**, not that the gap has the
  same number. Elevation re-places the two free row ends, and those are what the gap is made of.
- **A collision must be counted across faces, not within one.** The first version of
  `contested_places` counted claims per `(face, i, j)` and reported `0` everywhere — including the
  two cases that demonstrably fail. It could not fire: with one soft edge per face no face can ever
  write a row end twice. A row end lives on the **perpendicular** edge, so the key is
  `(shared edge, position along it)` normalised to the canonical end order. Fixed in `b2f9777`; it
  then reported `0` / `1` / `13` against gaps `0.000e0` / `2.488e-2` / `2.847e-1`.
- **Enforcement order has to match the thing being protected.** `share_every_boundary` copies soft
  edges too, which a derived master corner cannot survive: the corner is on the perpendicular edge's
  boundary, so the copy restores what the derivation just wrote. The first run of measurement 13
  shared all edges and reported the gap **unchanged to four figures** — which read as "the derivation
  does nothing" and was wrong. `share_hard_boundaries` is the correct pass. Same mistake as
  measurement 12's first run; **check that a change did something before believing a null result.**
- **A maximum can hide a null result.** Deriving corners repaired fewer seams while the worst gap was
  bit-identical, because the maximum over 32 seams sat on an untouched one. Count the thing you
  changed.

## Traps

- **Loop subdivision is not this design.** It moves the surface. The refinement here is degree
  elevation, which does not. Reaching for Loop is the characteristic bug.
- **A plane angle measured through `acos` floors at about `1.4e-8`.** Anything at or below that is
  the measurement. Do not tighten a tolerance against it. The floor is `sqrt(2e-16)`: two unit
  vectors that close have a dot product `f64` cannot resolve below `1 - 1e-16`.
- **Tessellate shared edge curves once.** Two patches independently tessellating a shared boundary is
  how a soft seam cracks in the rasterizer. Not started.
- **`SPEC.md` has no `Topology::Quads`** and this design needs one. Agent does not edit `SPEC.md`.
- **Do not reuse the crate's evaluator as a probe's reference.** See `examples/support`.

## The hierarchy landed, and what it measured

`PatchMesh` is now a **base level plus any number of displacement levels**, and the surface is their
sum. `offsets` starts empty, so a model that uses no hierarchy holds none — the cost of an unused
one is no memory at all, not merely no effect.

### The load-bearing claim, measured

The handoff used to say "the soft invariant is a homogeneous linear operator, so levels stay
independent" and cite nothing. `layers_probe` measurements 1 and 3 are that argument, split in two,
because **affineness alone is not enough** and the difference is the whole design:

- Write enforcement as `E`. Separately: `E(base) + E(offset)`. On the sum: `E(base + offset)`. For an
  affine `E(x) = M x + c` those differ by `c`, so the claim needs `c = 0`.
- **Measurement 1** is the only evidence there is for `c = 0`: a fresh level is `0.000e0` before
  enforcement and `0.000e0` after, exactly.
- **Measurement 3** is the composition that depends on it: **worst difference `2.035e-15`**, base and
  displacement both deliberately broken first so the map has real work on both sides.

Neither alone is the argument. A probe asserting only one of them passes on a broken design.

### It found a fault

Measurement 6 asserts what enforcement guarantees, and the first run was red. A face's corner is the
end of the shared boundary row of **each** of the two edges meeting at that vertex, so two sharings
write it and the later one wins. On a base level every corner is the same `positions` entry, the
copies start out equal, and the ordering never shows. On a displacement level nothing forces that,
and what was left over:

| | before enforcement | after |
| --- | --- | --- |
| worst split across a shared hard edge | `1.540e0` | `0.000e0` |
| split boundary positions | **520** | **0** |
| worst disagreement about a mesh vertex | `2.273e0` | `0.000e0` |

All 19 contested positions were at a **row end** and none anywhere else in the row — the fingerprint
of a contested vertex rather than of a mis-shared edge, and that is what identified it.

`enforce` now reconciles each vertex the way `set_control_point` always has, through one shared
`write_vertex`. Commit `628ca7a`. The four existing probes' numbers are **bit-identical** afterwards,
which is the check that says the fix changed only what was broken.

### Elevation is not shape-preserving once soft edges are there

Measurement 5 is a two-by-two — soft or not, with or without a displacement — over 216 sampled
surface points:

| | no displacement | one displacement |
| --- | --- | --- |
| every edge hard | `6.661e-16` | `2.176e-15` |
| 32 soft edges | `8.170e-3` | `5.785e-2` |

The all-hard row is rounding, so **a hierarchy does not make elevation lossy**. The soft row is the
accepted gap again, one refinement further in: the invariant leaves a slave's row ends un-derived,
elevation blends them into the derived interior positions, and the re-derived row sits slightly off
where the surface was. Degree elevation itself is exact — `rect_elevation_probe` measures `7.026e-16`
on a plain net. This is a **shipped property**, recorded in `crate::continuity` and in
`PatchMesh::elevate`, not a defect to chase to zero.

### Traps this added

- **`base_net` is level 0, not the surface.** With a displacement above it they differ. The read-out
  is split three ways on purpose: `evaluate` for a point (zero allocations, `O(levels)`),
  `composed` for the whole thing (one buffer), `base_net` for one face's borrow.
- **An enforcement that silently read a zero instead of panicking would look like a smooth surface.**
  Every read in `enforce.rs` panics out of range on purpose.
- **Do not clone `soft` in an enforcement loop.** `enforce` is a free function over `&Rules` and
  `&mut Level` precisely so the edge list can be iterated while the points are written; the old
  `&mut self` method had to clone it every pass.

## The front-end lives in `gallery`, and it refuses a folded mesh

Superior's decision, 2026-10-08: **`gallery` is the composition point.** The argument is the
dependency graph rather than taste — `mountain` must not know a freeform surface exists, `patch` must
not know what a massif is, neither may depend on the other, and `gallery` is the only crate that sees
both. The alternative was a crate that exists to hold three functions.

`crates/gallery/src/patchmesh.rs`, one public function:

```
patchmesh::from_facet(&FacetMesh) -> Result<PatchMesh, FacetError>
```

Three steps, order forced: widen `f32` → `f64`, Doo–Sabin the triangles into quads, one degree-1
bilinear patch per quad. `gallery` takes `patch` as a normal workspace dependency; `patch`'s **probe**
takes `gallery` as a dev-dependency, which is the same cycle `scene` already uses for its tests and
which the published dependency graph never sees.

**The probe now measures that function rather than a copy of it** (`e669b95`), and every number came
out bit-identical — which is the check that says only the code moved. The probe's own edge count is
written separately from the conversion's, so the two agreeing on a real mesh is a check on the guard
rather than a restatement of it.

### What it refuses, and why refusing is the answer

`from_facet` returns `Err(FacetError::NonManifold { edges })` when any edge carries three or more
triangles. That is not defensive coding; it is the one thing about a triangle mesh this cannot turn
into a surface:

- Doo–Sabin gives every triangle on an edge one shared midpoint, so the points agree.
- `enforce` shares a boundary when **exactly two** faces meet on it, so with three there is no pair
  to share and all three copies keep their own.

The model then tears along that edge, silently, with nothing reporting it. A mesh like that is not a
case to handle — its shape is a fold, and a per-edge tag describes a crease. This massif has none, so
the guard has never fired and its refusal path is **not exercised by a probe**; the count on real data
is, twice.

### The number that decides how finely the model can be shaped

**`1.526e-5`** — the distance between adjacent `f32`s at this mesh's largest coordinate, which is
`2^-16` as it must be for a value between 128 and 256. That is the floor under every edit anyone
makes to this surface, and no `f64` arithmetic downstream moves it. It is **eleven orders of
magnitude coarser** than the `1e-16` the rest of this crate measures in, so "exact" in measurement 3
and "shapeable" are not the same question at all.

A first version of this measurement printed `0.000e+00` and was wrong: it measured the distance from
a widened point back to *its own* `f32` rounding, which is zero by construction, because the widened
point already **is** an `f32`. Adjacent is the whole word that was missing.

## The hierarchy has a caller, and writing it found three bugs

`tessellation_probe` and `crates/patch/src/tessellate.rs` (commit `8e8e84e`). `PatchMesh::tessellate(side)`
samples the surface into an indexed mesh. It is the first thing in the crate that reads the
**surface** rather than control points, so the questions change shape.

Three faults, all in code written minutes earlier, all caught because the probe measured rather than
asserted:

1. **A mesh vertex was sampled once per incident edge.** The two faces on a shared edge held the same
   *interior* samples but reached the shared vertex by different routes — **13 802 of 13 848** interior
   edges disagreed on indices. That is agreement in value and not in place, and a rasteriser cannot
   tell the difference. Vertices are now sampled once for the whole mesh, which also cut the point
   count from **137 994 to 116 113**, about 16%.
2. **Sampling was done by control index, so the degree moved the samples.** `across = k * degree /
   (side - 1)` is integer division, so at degree 4 with `side = 5` it gave `0, 1, 2, 3, 5` and at
   degree 5 it gave `0, 1, 2, 3, 5` again — different *parameters*. Elevation appeared to move the
   surface by `5.206e0` when it had not moved it at all. Sampling is by **parameter** now, which also
   removed the `side - 1 <= degree` precondition I had added to paper over it.
3. **The measurement for the hazard was itself vacuous.** See below.

After the two fixes, elevation moves the sampled surface `8.534e-14` over 116 113 points.

### The hazard I predicted, and the measurement that makes the zero worth reading

The case for sampling a shared edge once was that two faces sampling it independently sum the same
Bernstein terms in opposite orders, and `f64` does not round two orders the same way. **Measured:
`0.000e+00`, over 69 240 positions.** On this surface the two rows come out equal to the last bit.

That is only worth reading next to a run of the same function that does *not* report zero, so the
probe carries one: scramble the base level's sharing and the same function reports **`7.587e2`**. The
measurement can see a disagreement, and there was none. **So the reason to share an edge is not a
numerical one.** It is that the alternative stores a copy per side, evaluates it twice, and leaves a
rasteriser to reconcile two numbers that happen to match today.

The trap this sets is recorded below, because it is the second time this session that a `0.000e+00`
turned out to be a measurement that could not fail.

## The rasteriser's half exists, and it found that half the massif is inside out

`tessellate::triangles()` turns each face's sample grid into two triangles per cell, and `rect` and
`level` gained the analytic normal a rasteriser can ask instead of deriving one from a triangle.
`raster_probe` measures four things about the result.

Two pass outright, and both are worth having:

- **The soup is a mesh.** 3552 rim edges where the quad rim identity says 888 × 4 — exact, in
  integers, not a tolerance — no edge over two triangles, no point no triangle uses, Euler
  characteristic 1.
- **No triangle is degenerate**, shortest edge `1.874e-7`, and the check catches a collapsed triangle
  when handed one.

One fails, hard, and it is the reason this section exists. **109920 of 228672 triangles point away
from their own patch**, and the number is *identical at every sampling resolution* — so it is not a
faceting effect. Broken down by face: 3435 faces wholly away, 3711 wholly toward, **zero with some of
each**. The fault is in how the faces are wound, not in a surface folding through itself.

Traced to the source. 46 of 3351 interior edges of the sample `FacetMesh` are traversed the same way
by both their faces, bounding a connected blob of 1145 of 2382 triangles. Doo–Sabin carries that into
3435 of 7146 quads, and `triangles()` inherits it into the drawn soup.

`facet_front_end_probe` measurement 6 classifies every interior edge by how it is traversed against
which side of it, in plan, the two faces' third vertices fall:

| traversed | third vertices | count |
|---|---|---|
| both ways | opposite sides | 3305 |
| both ways | same sides | 0 |
| one way | opposite sides | 0 |
| **one way** | **same sides** | **46** |
| a third vertex on the line | | 0 |

`mountain::expand::push_face` gives every face the same sign of plan area. That is a consistent
winding **only where no face's plan shadow crosses its neighbour's**, and a face seen nearly edge on
casts a shadow thin enough to do it. Every one-way edge is an overlap edge; no other cell is occupied.

### Two explanations that were measured and thrown away

- **`f32` rounding.** The faces at the bad edges have plan areas of `1.024e-5` median against a mesh
  median of `1.951e+1`, six orders of magnitude down, where one `f32` ulp of a coordinate of order 60
  is `7e-6`. Tested against the error bound of the actual computation, **all 2382 faces are readable**.
  The sign is not noise, and widening the arithmetic is not the fix.
- **An earlier version of the table read the overlap count out of the branch that only ever saw
  `(1, 1)` edges**, so it saw zero of the 46 and concluded the opposite. The joint distribution is
  what settled it. The probe now prints the table rather than a summary number, for this reason.

### `mountain` is subdividing far below its own structure

Superior's read on this, and the numbers back it: the massif has **182 block faces**, which the
expander cuts into **2382 triangles** (13.1 per block), which become **7146 quads** and 228672 drawn
triangles. One patch per block would be 182 patches each carrying real interior detail instead of
7146 flat tiles — a 39-fold reduction in faces.

**Decided 2026-10-08: the sampled polylines are a hint, not an invariant.** They do not have to be
preserved and the difference does not have to be minimised. That removes the basis question this
section previously carried — there is no Bézier-versus-B-spline decision to make, nothing to
interpolate and nothing to optimise. A `k` by `k` control net taken from the block's own surface is
the whole construction.

`block_face_probe` measures what that costs:

| k × k | worst | median | worst among the 93 blocks that cover their own `uv` box |
|---|---|---|---|
| 2 × 2 | `1.563e-1` | `2.984e-6` | `4.350e-6` |
| 5 × 5 | `2.790e-1` | `2.686e-6` | `4.466e-6` |
| 7 × 7 | `3.676e-1` | `2.370e-6` | `3.982e-6` |

against an `f32` precision of `1.526e-5` at the mesh's largest coordinate.

Two things in that table are the answer, and neither is the worst column. **The worst case rises with
`k` while the median does not fall**, which is the opposite of a denser net over a smooth surface;
the last column says why, because 89 of 182 blocks leave part of their own `uv` box uncovered
(smallest `0.9048`) and the grid there carries an *extrapolated* control point that a higher degree
amplifies. **The last column is flat at about `4e-6`**, so where the grid lands on the surface the
construction is essentially free — below the precision the mesh arrived as.

`Face::uv`'s `u` is the fraction of the belt walked **by distance**, so a crest point and the valley
point below it are not the same station and the strip between them is not the whole square. A block
patch's domain should therefore be the region its own faces cover rather than the unit square. That is
the fix, and it is small.

**One blocker, and it belongs to the authoring rather than to `patch`.** A **rib** is two points, so
both sides can name it as a curve whatever their interior resolution is — 168 ribs are on exactly two
blocks. A **run** is a different length on the two sides in **124 of 182** blocks, and two patches
share a boundary as one curve only if both carry the same number of control points on it.

**Coarser faces do not fix the winding defect completely.** The 46 one-way edges sit inside 14
distinct blocks; 8 are in a block that is concave in plan, and only 3 of 182 blocks are concave at
all. So 38 come from cutting convex blocks into triangles and go away with the granularity change,
and 8 need handling of their own.

The full proposal, with every number and the open questions in order, is
`proposals/block-face-front-end.md`.

## The NURBS demo scene, and what it found in `patch`

`gallery::nurbs` is the first scene whose geometry is a patch surface rather than a generated mesh: a
9 by 9 sheet, every interior edge soft except an 8-edge crease down `x = 0`, raised to degree 3, one
displacement layer carrying a ripple, sampled into the triangles the scene document holds.
`crates/gallery/examples/nurbs_probe.rs` measures it, 4 measurements, and it is the only place the
crate's own claims about continuity are checked against a scene.

**Building it turned up a defect in `patch`, and the hunt for it turned up a larger one.** Two
separate findings, in order of size.

**The larger one: arbitrary topology cannot be made smooth under this representation.** Measured
2026-10-10, `examples/junction_probe.rs` and `examples/valence_probe.rs`:

- A **regular** grid with no junction in it reads a median `2.128e-1` rad along its edge middles and
  `3.529e-1` at its valence-4 vertices. The soft-edge invariant is applied over `1..degree`, so the
  two ends of the slave's row are never derived and their Bernstein weights are largest at the seam's
  ends. **A lone seam, with nothing to contest, already reads `8.632e-2` at its middle.**
- A **T-junction is not representable.** `EdgeKey` is a pair of endpoint ids, so a vertex lying in
  the middle of a quad's edge has nowhere to be recorded; merging two cells produced **18** edges with
  one face, which is a cracked mesh rather than an unsmooth one.
- **Our own input is 89.4 % extraordinary**: after Doo–Sabin only 804 of 7591 vertices are valence 4,
  valences run 2 to 40, and at block level 168 of 190 nodes carry eight patches and one carries
  **56**.

So the question "how do we make an arbitrary-topology mesh smooth everywhere" is closed as asked, with
three independent reasons. The open question is which cost to pay: degrade at extraordinary vertices,
eliminate them by rewriting the authoring (**which buys nothing against the first two**), change
basis, or go hybrid. `proposals/arbitrary-topology-smoothness.md`.

**The smaller one, already fixed: a face carrying two or more soft edges cannot satisfy all of them.**
`interior_index` maps each of a face's four edges onto the same interior block, so the corners of that
block are each claimed by two of them. This was **not** a new finding — `SPEC.align` records it under
"Over-determined points inside one patch", ruled on 2026-10-07. What the demo scene added is that
**`enforce` resolved the contest by write order**, which is the objection that ruling gives for
refusing edits:

| 4 by 4, every interior edge soft | before | after |
|---|---|---|
| worst fold, tagged forwards | `2.325e-1` | `2.325e-1` |
| worst fold, tagged backwards | `3.330e-1` | `2.325e-1` |
| control points the two orders differ | 9 of 256, by `6.142e-2` | **0 of 256, by `0.000e0`** |

Fixed by keying the arbitration on `EdgeSlot`. **Averaging was measured first and rejected** — it
halves the mirror residual but the worst fold then grows with the grid (`1.546e-1` to `2.245e-1` at
4 by 2), because half of each disagreement becomes a half-disagreement at the points a neighbouring
edge reads next. `examples/contested_probe.rs` guards the ceiling. Full write-up, controls and the two
refuted explanations in `proposals/two-soft-edges-one-face.md`.

**Correction to a claim made in discussion.** It was said that a vertex with four soft edges "should
have been one face", so no smoothness should be demanded there. The first half holds — four patches in
a 2 by 2 arrangement with C1 joins *is* one piecewise-Bézier surface. The second does not follow: on a
grid with no junction at all, the vertices still read `3.529e-1` median. Relieving the junction question
would not relieve the surface.

**An earlier version of `nurbs_probe` measurement 3 asserted that a soft edge reads zero, and was
red.** The probe was wrong, not the crate: `continuity` states that a soft edge is measurably
smoother than a crease and **not** continuous across the seam. Measurement 3 now asserts what is
actually promised — the crease is the sharpest thing on the sheet — plus a regression ceiling.

That assertion is what forced the scene's crease slope from `0.55` to `1.10`. At `0.55` the crease's
smallest fold (`2.00e-1`) was **below** the soft worst (`2.06e-1`): on its own measurement the sheet
was a sheet whose creases were softer than its seams. At `1.10` it reads a crease of `6.11e-1` to
`7.28e-1` against a soft median `2.24e-2` and worst `9.75e-2` — a separation of 6.27.

**Two measurement traps, both recorded because each produced a confident wrong answer.** `EdgeSlot`
names the parameter an edge **fixes**, so `MinU` varies `v`; reading it the other way round asks each
face for a normal off its perpendicular edge and returns a plausible fold that is not there. And
`PatchMesh::set_soft` calls `enforce` itself, so a before/after comparison built by tagging edges
soft has already been enforced on both arms.

## Next slice

1. **Give a block patch the domain its own faces cover.** `Face::uv`'s `u` walks the belt by distance,
   so 89 of 182 blocks leave part of their `uv` box uncovered, and sampling across a gap puts an
   extrapolated control point in the net that a higher degree amplifies. Carrying the covered region
   instead of the unit square is what turns the `1.5e-1` worst case into the `4e-6` one.
2. **Resolve the unequal runs.** 124 of 182 blocks have two boundaries sampled at different rates, so
   neighbouring blocks cannot share a run as one curve. Resampling the shorter run or splitting the
   block are both available; this belongs to `mountain`'s authoring and it is the blocker.
3. **Orient the source's faces**, as one traversal in `mountain`, independently of the granularity
   change and still needed for the 8 edges in concave blocks.
4. **Lazy subdivision**, as a **separate** decision from the soft-edge one: it is worth a large win
   on control-net size whenever few edges are soft, at one T-junction per frontier edge. Its curve is
   in measurement 15.

A hierarchy now has a caller, so the things that were "only a probe writes displacements" are no
longer true: `tessellate` reads every level at every sample point.

`raster_probe` needs `--release`: the finest resolution there puts 3.6 million triangles on the
massif. `f64` operations are IEEE in both profiles, so the numbers are the same, only sooner.

### Known, deliberately not fixed

- **Two probes are red, on the same defect in `mountain`**: `raster_probe` measurement 1 and
  `facet_front_end_probe` measurement 6. They are red because the defect is real and neither can see
  it from its own angle alone. They are not to be weakened to pass.
- **`whiting-definition` carries 7 pre-existing clippy warnings**, untouched by this branch
  (`git diff main..HEAD -- crates/definition` is empty). They surface in `cargo clippy --workspace`
  regardless. One of them — two `infallible TryFrom` impls in `packed.rs` and `bytes.rs` — would mean
  changing a public API, so fixing the set belongs in its own decision rather than in a slice about
  NURBS.

## Not decided, do not assume

- Degree policy. All patches share one degree because the invariant assumes both faces of an edge
  agree. Peters' lower bound (<https://arxiv.org/abs/1709.02994>) says arbitrary-topology quad meshes
  generally cannot be covered by G1-connected **bi-cubic** Bézier patches in the standard layout.
  Measurement 11 measured that raising the degree buys nothing *here*; that is not the same as Peters'
  claim being about this representation.
- Whether a patch's net is stored as `Mesh` point data via the existing `point_schema: Schema`, or
  as its own structure.
- Whether to take `axiolid-nurbs` (MPL-2.0) for point inversion. It is rectangular, so it now
  matches the basis — that changed with this revision.

## Commits this session

```
b2f9777  Measure what the two skipped row ends actually cost: nothing, unless two seams collide
0587057  Let the continuity probe measure a scratch copy of the nets
597ecf6  Sweep the degree: it buys nothing, on either count
4c33909  Measure the corner stencil's algebra, and find that projection is not a fix
da531db  Quad topology, quad patches, and a G1 invariant that is exact but incomplete
2106d55  Point3 moves to its own module
bacdc57  Remove a passing probe that asserted something false
9957e13  Rectangular Bernstein net, and the elevation rule it actually needs
66f3820  Doo-Sabin subdivision: the triangle mesh becomes all quads, without moving the surface
```
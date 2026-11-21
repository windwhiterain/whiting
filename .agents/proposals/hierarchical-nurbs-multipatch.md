# Hierarchical multi-patch NURBS: every mesh face is an editable patch, every edge is hard or soft

Status: **approved 2026-10-07. Superseded in substance again on 2026-10-07**, after the first
implementation attempt. The two rulings folded in below — **the face basis is triangles** and **the
continuity layer must be written basis-agnostically** — are **both overturned**. A third claim, that
the soft edge's row ends force `μ = −1` and couple into a corner stencil, was **overturned again** by
measurement 12 (`b2f9777`). The current
authority is
[../SPEC.align/hierarchical-nurbs-multipatch.md](../SPEC.align/hierarchical-nurbs-multipatch.md).

> **Ruling, 2026-10-07, after measurement.** A degree-`n` triangle carries `n` interior positions
> along one side of an edge and `n-1` along the other, and no assignment makes them one to one. G1
> is a statement about the cross-boundary *derivative*, which on a triangle sums over a row with no
> index correspondence to its neighbour's. The basis-agnostic formulation in section 3 below is
> therefore not merely inconvenient on triangles — **it has nothing to bind to.** Measured on a
> real 24-face mesh: `1.49e-8` rad undragged, `6.92e-2` after a corner moves, `1.29` rad once every
> edge is soft. The triangle basis and the `μ[b]` per-position slope are both withdrawn. Faces are
> **quads** (the mesh is Doo–Sabin subdivided first), `μ` is **one scalar per edge**, and the
> residual defect — the two ends of an adjacent row, which the invariant cannot own without
> cracking the perpendicular seam — is documented with its fix in `SPEC.align`.
>
> The sections below are kept unedited as the decision record. Read them for what was decided and
> *why*, not for what is currently true.

This supersedes the framing of
[investigate/nurbs-izing-arbitrary-polygons.md](../investigate/nurbs-izing-arbitrary-polygons.md)
and most of
[investigate/sparse-nurbs-representations.md](../investigate/sparse-nurbs-representations.md).
Those surveys answered a question the superior does not have. The relevant survey is
[investigate/nurbs-libraries.md](../investigate/nurbs-libraries.md).

## The target, stated exactly

> Take a low-poly mesh. Every face becomes an editable freeform patch. Every shared edge is
> either **hard** (a crease; the two sides move independently) or **soft** (tangent-plane
> continuous; the crease curve is still shapeable). Refinement is layered and linear.
> Coarsening is out of scope.

**There is no fitting step.** Nothing here approximates the mesh — the mesh is only a source of
topology and of an initial control net. The whole NURBS-ization pipeline surveyed in
`nurbs-izing-arbitrary-polygons.md` (QuadriFlow, quad meshing, global parameterization,
LSPIA fitting) is therefore **irrelevant to this project**. This is the single largest change
from the original brief and it deletes most of the research burden.

The result is a **multi-patch N-BRep**, which is a solved, textbook problem. It is not a
research-grade hierarchical basis problem.

## Decisions on the record

| Question | Decision |
| --- | --- |
| Exact NURBS fidelity (exact circles/cones/spheres, CAD export)? | **No.** Rational weights are dropped; everything is polynomial B-spline. |
| Hierarchy | Multi-level linear sum; a level of all zeros contributes nothing. Coarsening out of scope. |
| Soft-edge freedom | Both boundary-adjacent control rows stay free; only tangent matching is enforced. **Withdrawn 2026-10-07**: on a rectangle the slave's row turns out to be *derived* from the master's by one scalar, not free. |
| Soft-edge alternatives (PN / blended / Hermite patches) | **Considered and rejected** — see *What this is not*. |
| Face basis | **Triangles** (ruled 2026-10-07, after this document was written), then **overturned the same day**: **quads**, via Doo–Sabin subdivision. `mountain`'s `FacetMesh` is still the first target. |
| Continuity layer | **Basis-agnostic** (ruled 2026-10-07), then **overturned the same day**: rectangular only. A basis-agnostic layer is not achievable here because the triangle case is not a notation problem. |

## Three ideas that carry the design

### 1. Store displacements, not absolute values — this is what makes "0 = no bias" true

The superior's sparse rule is: *a level of all zeros adds no bias, so the surface is exactly the
root*. That rule is exact **only if every level stores a displacement against a common root**,
and the root is the only level carrying absolute geometry.

```
level 0 (root)   C_0  on U_0            absolute shape
level k > 0       D_k  on U_k            displacement, added to the root
evaluation        lift every net to U_L, then sum:
                  C_L[i][j] = lift(C_0)[i][j] + Σ_k lift(D_k)[i][j]
```

Single-knot insertion (Boehm / *The NURBS Book* A5.1) is **linear**, so a zero displacement net
lifts to a zero displacement net at any resolution. A layer that is empty stays exactly empty
after any number of refinements, and the surface stays exactly the root. Knot vectors nest:
`U_0 ⊂ U_1 ⊂ … ⊂ U_L`, degree is constant across levels, and because all levels live in the same
basis family the sum is again a tensor-product spline.

*Provenance note: Boehm insertion being linear and shape-preserving is textbook, not fetched
during this session. The "displacement vs absolute" framing is the part that is actually being
decided here, and it is what makes the superior's rule hold.*

**Refinement** is therefore: shape-preserving insertion into every existing net, plus a fresh
level of zeros. Refining never moves the surface.

**Cost note:** the combined net `C_L` is derived state. Cache it and invalidate per edit;
recomputing it from scratch is O(|C| · L) and is only acceptable as a fallback.

### 2. The "inverse" the superior is worried about does not exist and is not needed

Moving a point *on* the surface by `ΔP` looks like an inverse problem. It is not:

> Move every one of the `(p+1)(q+1)` control points in that point's support by `ΔP`.

B-spline basis functions satisfy the **partition of unity**, so the basis values over a point's
support sum to 1 and the surface point moves by exactly `ΔP`. This is `O((p+1)(q+1))`, constant
time, exact, with no iteration and no solve.

*Which level receives the move* is a product decision, not a numerical one. Every control point
carries (or derives) the refinement level that introduced it — for a nested knot vector the level
is a function of the knot position, so it may not need to be stored. Dragging into a coarse level
moves a wide region; dragging into a fine level moves a local one. That is a slider.

### 3. The soft-edge constraint is a closed form with one scalar per boundary point

This is the technical heart of the proposal, because it is the part the superior expected to be
expensive.

Setup: a soft edge shared by patches `A` and `B`, running along the shared parameter. Both
patches must have **the same degree and the same knot vector along the seam** — enforce this at
build time. Let `Q_b` be the shared boundary control row, and `A_1` / `B_1` the immediately
interior rows on either side. At a clamped boundary of degree `p`:

```
∂S_A/∂α |_α=0  =  p · (A_1 − Q)
∂S_B/∂β |_β=0  =  p · (B_1 − Q)
```

Geometric `G1` means those two cross-boundary tangent vectors are parallel (and, for a
non-folded join, opposed). So the invariant is:

> **`B_1[b] = Q[b] + μ[b] · (Q[b] − A_1[b])`,  μ[b] ≥ 0**
>
> **Withdrawn 2026-10-07, measured.** `μ[b]` is not available to give up: a per-position slope
> makes the slave's derivative curve a *different* curve from the master's unless every `μ[b]` is
> equal. The live form is one scalar for the whole edge, `μ`. The derivative lines above are also
> only the endpoint case — the real sum runs over the whole adjacent row, which is exactly why the
> triangle basis fails. See `SPEC.align`.

- `μ = 1` recovers **C1** — matched cross-boundary parameterization, `B_1 = 2Q − A_1`.
- `μ ≠ 1` is **G1** — a reparameterization across the seam, still tangent-plane continuous.
- `μ = 0` puts `B_1` on `Q` — `B` is developable along that edge and the tangent plane is
  defined entirely by `A`'s side.

**A hard edge carries no invariant at all.** `A_1` and `B_1` are simply independent.

The store is one `Vec<f64>` of `μ` per soft edge. After any edit to `Q` or `A_1`, recompute
`B_1` by the formula above. `O(n)` per edge, **no iteration, no equation solve, and no coupling
between edges** — each face's interior row belongs to exactly one edge, so a vertex where `k`
patches meet generates `k` independent constraints, not a coupled system.

> **Withdrawn 2026-10-07, measured.** The "no coupling" claim is false in two separate places. A
> quad's four adjacent rows meet at `(1,1)`, `(1,degree-1)`, `(degree-1,1)` and
> `(degree-1,degree-1)`, so two adjacent soft edges on one face **do** share a point. And the row's
> two ends belong to the perpendicular edge's shared boundary.
>
> **The second half is now measured too, and it does not say what this note said.** The claim was that
> this couples every soft edge meeting a vertex into a *corner stencil*, and that the stencil is what
> soft edges are waiting on. `quad_continuity_probe` measurement 12 (`b2f9777`) ran the invariant over
> the whole row and found a **lone** soft edge exactly G1 (`0.000e0` / `2.107e-8` rad) with `0`
> contested places. The coupling is not every vertex; it is **only** where two seams write the same
> place on the same shared edge — `0`, `1` and `13` such places on the three measured cases. The
> missing degree of freedom is one per contested place, which is what a corner scale `κ` would buy.

Freedom per boundary control point, which is the part worth checking the superior's taste against:

| Edge kind | Free parameters | Count |
| --- | --- | --- |
| hard / untagged | `Q`, `A_1`, `B_1` | **9** |
| soft, G1 | `Q`, `A_1`, `μ` | **7** |
| soft, C1 (`μ` pinned to 1) | `Q`, `A_1` | **6** |

> **Withdrawn 2026-10-07, measured.** These counts assumed the slave's row stayed free under `μ`.
> It does not: with the master and the boundary fixed, G1 forces the slave's whole offset curve to
> be a scalar multiple of the master's, leaving **one** free parameter. On the slave's private
> interior that is `3(degree-1)` coordinates down to `1` — at degree 3 the soft edge removes five,
> not the two claimed below.

A soft edge removes exactly the two components by which the two sides could otherwise tilt the
cross-boundary direction independently — it can still be **scaled** freely by `μ`. That is the
intended feel: the crease curve `Q` stays fully shapeable on both sides, and the crease *angle*
loses one scalar of asymmetry.

**One consequence to accept:** each soft edge picks a master side. Editing the master's boundary
propagates to the slave; editing the slave's interior row has to route through the master.

### 4. Where the hierarchy is allowed to live

A layer summed across the whole model would smooth every crease — the sum is global, so it
cannot respect a hard edge. The resolution is that **layers are partitioned by hard edges**:

- A hard edge cuts the model into connected components; layers never cross it, so creases are
  structurally impossible to erase.
- Inside a soft-edge connected component, all faces share one knot hierarchy, and a soft edge
  requires the two faces to agree on degree and on the seam's knot vector at every level.
- **The continuity invariant only has to hold on the combined net at the finest level.** Since
  the evaluated surface *is* a spline on that net, `G1` of the surface is a property of that net
  alone. The levels are a storage and editing decomposition, not a separate continuity problem.

## The basis question this project has not answered yet

`SPEC.md` gives `Topology` only `Triangles` and `Segments`. There is no quad variant, and
`mountain` produces a planar-facet **triangle** `FacetMesh`. **A triangle cannot be a
tensor-product patch.** This is the largest remaining fork and it is not cosmetic:

| | Quad faces | Triangle faces |
| --- | --- | --- |
| Basis | tensor-product B-spline on a rectangle | Bernstein / triangular spline on a simplex |
| Control net | rectangular array | triangular array — matches the mesh directly |
| Refinement rule | Boehm knot insertion | the triangular refinement rules (Loop-style), same role |
| Partition of unity | yes | yes — drag propagation above is unaffected |
| Continuity layer | **identical** | **identical** |

The continuity invariant depends only on "there is a boundary control row and one interior row
adjacent to it". Both bases have that. **So the whole of section 3 survives either choice, and
the design deliberately does not commit yet.** What does change is the evaluation kernel and the
refinement operator.

## What this is not

- **Not hierarchical B-splines (HB/THB), not PHT, not T-splines, not T-NURCC.** The hierarchy
  here is a storage and editing decomposition over one flat spline with nested knot vectors.
  There is no nested element structure, no cell basis matrices, no T-junctions.
- **Not PN / blended / Hermite patches.** Those make `G1` constructive — two patches sharing
  endpoint tangents are tangent-plane continuous for free, which is genuinely more elegant than
  section 3 — but they have **no linear tensor-product structure**, so knot insertion and
  layered displacement, and therefore the entire hierarchy, do not exist for them. The
  superior's own requirement and this requirement are mutually exclusive. The hierarchy won.
- **Not NURBS fitting.** No QuadriFlow, no quad meshing, no global parameterization, no LSPIA.
- **Not coarsening**, at the superior's instruction. Note that this costs almost nothing: the
  representation supports it, since coarsening is knot removal on the root net, which is the one
  operation in this whole design that can fail and needs a tolerance.
- **Not CAD.** No booleans, no fillets, no subdivision, no STEP.

## Open

1. **Quad or triangle** — see the previous section. Everything else survives either answer, but
   the evaluation kernel and the refinement operator differ.
2. **Seam tessellation.** A shared edge with `μ ≠ 1` may crack in the rasterizer if the two
   patches tessellate their boundary independently. Either tessellate shared boundary curves once
   and share the result, or weld within a tolerance. This is a rendering-path decision and it
   should be settled before the first patch is drawn.
3. **Initial control net.** A bilinear corner interpolation gives a degree-(1,1) net that must then
   be elevated, which costs knot insertions and shifts the surface. Constructing a bicubic net
   directly from corner tangents avoids that but needs a tangent convention per face. Open.
4. **Degree policy.** Fixed `(3,3)`, or per-patch but required to agree across a soft edge.

## Dependencies

The tensor-product algebra is available today and needs no new research:

- `axiolid-surface::BSplineSurface` exposes its value as **public mutable fields** —
  `control_points: Vec<Vec<DVec3>>` row-major in `u` then `v`, plus knot vectors, multiplicities
  and closure flags — so moving a control point is a field write with no API obstruction. `jet(u, v)`
  supplies the partials needed to verify continuity.
- `axiolid-nurbs` supplies `iso_curve_at_u` / `iso_curve_at_v` (extract a patch boundary as a
  curve, which is how a shared edge is obtained), `invert_surface_certified` (point → `(u, v)`,
  the drag path's entry point), `insert_surface_knot_u/v`, `remove_surface_knot_u/v`,
  `elevate_surface_degree_u/v`, and `analyze_surface`.

Licence is **MPL-2.0**, which is a dependency decision, not a blocker. For reference, OCCT has
the per-edge continuity concept natively (`BRep_Builder::Continuity`, `BRep_Tool::Continuity`,
defaulting to `C0`) but carries a static-linking LGPL exposure and is not proposed here.
# Two soft edges on one face cannot both be enforced

Date: 2026-10-08, revised 2026-10-10. Status: **arbitration fixed and measured**; the
over-determination is **not** solved, and is **not** a representation limit.

**This is a secondary problem.** `proposals/arbitrary-topology-smoothness.md` measures something
larger: a *regular* grid with no junction in it already reads a median `2.128e-1` rad along its edge
middles, because the soft-edge invariant leaves the two ends of the slave's row un-derived. What is
written here is what a face's *interior control points* do on top of that, and it is bounded and
order-free rather than structural.

## What was changed, and what it bought

`enforce` no longer writes a soft edge's interior row as it visits it. Every edge's demand is
collected off one snapshot, and a contested point is settled **by slot** — `MinU < MaxU < MinV <
MaxV` — rather than by which edge happened to be tagged last.

| 4 by 4, every interior edge soft | before | after |
|---|---|---|
| worst fold, tagged forwards | `2.325e-1` | `2.325e-1` |
| worst fold, tagged backwards | `3.330e-1` | `2.325e-1` |
| control points the two orders differ | **9 of 256**, by `6.142e-2` | **0 of 256**, by `0.000e0` |
| lone seam (must not move) | `5.846e-2`, residual `0.000e0` | `5.846e-2`, residual `0.000e0` |
| `gallery::nurbs` soft worst / separation | `1.011e-1` / `6.05` | `9.754e-2` / `6.27` |

The quality numbers are unchanged and the lone seam is untouched. What is gone is the dependence on
how a caller walked the mesh to tag it — the same objection the 2026-10-07 ruling raised when it made
`set_control_point` refuse a contested point rather than guess. `enforce` was guessing.

## Why the point is contested at all

`interior_index` maps each of a face's four edges onto the same interior block, one point per (edge,
position). At degree 3, over `t` in `1..degree`:

| edge | points written |
|---|---|
| `MinU` | `(1, 1)`, `(1, 2)` |
| `MaxU` | `(2, 1)`, `(2, 2)` |
| `MinV` | `(1, 1)`, `(2, 1)` |
| `MaxV` | `(1, 2)`, `(2, 2)` |

**Exactly four points are contested: the corners of the interior block**, each claimed by two of the
face's four edges. An earlier version of this file said every interior point is written by exactly two
edges; **that is true only at degree 3**, where the block is 2 by 2 and the four corners are all of
it. At degree 5 the block is 4 by 4 and four of sixteen points collide. At degree 2 the two columns
coincide and **all four** edges write the single point `(1, 1)`.

The two claimants want different values there and only one can hold. That is the over-determination.

## Measurements

All at degree 3 on grids built from a sinusoidally knocked height field, read at the **middle** of
each edge. "mirror residual" is the distance from each soft edge's slave row to the mirror
`slave_interior(boundary, master_interior, mu = 1)` asks for.

| case | faces carrying 2+ soft edges | mirror residual | worst fold |
|---|---|---|---|
| 2 patches, 1 soft edge (a lone seam) | 0 | **`0.000e0`** | `5.85e-2` rad (3.3 deg) |
| 2x2, all interior edges soft | 4 | `8.26e-2` | `1.58e-1` rad (9.0 deg) |
| 3x2 | 6 | `7.91e-2` | `1.53e-1` rad (8.8 deg) |
| 4x2 | 8 | `6.59e-2` | `1.55e-1` rad (8.9 deg) |
| 4x4 | 16 | `6.84e-2` | `2.33e-1` rad (13.3 deg) |

The lone seam reaches residual exactly zero and is the control for the rest: everything above it is
the contest's share. The per-edge split on the 2x2 shows the partial survivor — `1.579e-1` and
`1.505e-1` beside `1.649e-2` and `1.245e-2`, a tenfold difference between neighbours with nothing in
the mesh to predict which side of the line an edge lands on. That is what the arbitration removed.

## Two routes measured and rejected

**Averaging the claims.** The obvious symmetric answer, and worse. It halves the mirror residual
(`8.263e-2` to `4.131e-2` at 2x2) but the error spreads, and the worst fold then **grows with the
grid** where last-writer-wins stays flat:

| grid | last writer wins | averaged |
|---|---|---|
| 2x2 | `1.579e-1` | `1.579e-1` |
| 3x2 | `1.529e-1` | `1.790e-1` |
| 4x2 | `1.546e-1` | `2.245e-1` |
| 4x4 | `2.325e-1` | `2.446e-1` |

Half of each disagreement is a half-disagreement at the points that edge's neighbours then read, and
it compounds. Recorded in `apply_claims` so it is not tried again blind.

**Deferring the writes while keeping last-writer-wins.** Tried first, because it is what makes the
arbitration possible at all. It reproduced the old numbers **bit for bit** on 2x2, 3x2 and 4x2. So
the "a later edge reads a value an earlier one overwrote" hazard — believed for most of a session —
**does not bite**: at degree 3 every contested point is write-against-write, never
write-against-read.

## Not solved

The over-determination itself. Four points per face are still claimed by two edges each, two
equations and one value, and only one holds. The fix decides which one; it does not make the other
true. `examples/contested_probe.rs` measurement 2 guards the ceiling so the arbitration cannot be
traded for worse.

Making them all hold needs a formulation that does not decide each face's interior from its own
boundary plus its neighbours' interiors edge by edge. A uniform B-spline net satisfies every mirror
at once — that is the shared-net model, and `proposals/arbitrary-topology-smoothness.md` records
both its cost and the fact that the survey found no automatic multi-patch G1 post-process at all.

## Two corrections to claims made in discussion

**This was called a new finding on 2026-10-08. It was not.** `SPEC.align`'s "Over-determined points
inside one patch" records the same structure and a measurement of the same magnitude: *"A quad's four
adjacent rows meet at `(1,1)`, `(1,degree-1)`, `(degree-1,1)` and `(degree-1,degree-1)`, so two
adjacent soft edges on one face do share a point and cannot both hold there. Measured: the two-edge
sample's worst gap is `1.389e-1` rad against `4.771e-2` with at most one per face."* The 2026-10-07
ruling covers **editing** — `set_control_point` refuses with `Edit::Overconstrained { by: 2 }` — and
not `enforce`, which is what this file changed.

**The measurement was wrong twice before it was right, and both times returned a plausible number.** A
sweep that put the soft folds at `1.7e-1` against a crease at `3.4e-1` was reading each face's normal
off its *perpendicular* edge: `EdgeSlot` names the parameter an edge **fixes**, so `MinU` varies `v`.
A second sweep appeared to show enforcement changing nothing, because `PatchMesh::set_soft` calls
`enforce` itself and both arms of the before/after had already been enforced. Both are recorded
because both produced a confident wrong ordering.

## Where the guard lives

`examples/contested_probe.rs`, four measurements: the lone seam as the control, crowded grids with the
per-edge split printed so a partial survivor stays visible, the tag order, and that a second pass is a
fixed point.
# Hermite instead of per-patch NURBS: continuity as shared data, not as an enforced invariant

Date: 2026-10-15. Status: **probe measured, all five green. The regular-grid case is settled; the
irregular case is the open half and no production code has been written.**

Supersedes nothing on its own; competes with
[proposals/arbitrary-topology-smoothness.md](arbitrary-topology-smoothness.md)'s "change basis" row
and with `SPEC.align/hierarchical-nurbs-multipatch.md`'s "PN / blended / Hermite patches — Rejected".

## The instruction this answers

Refactor the NURBS/patch module to a Hermite surface, where every patch joins C1 because it reads
adjacent control points and their parameters in sync.

The **conclusion is right and it is a real improvement on what ships.** The route is not the one the
phrase suggests, and the difference matters enough to be the whole document.

## Correction: parameter synchronization alone is not sufficient

The claim as stated is that synchronizing parameters makes the join C1. Two patches sharing an edge
curve with the same parameterization are **C0 plus a matching tangential derivative**. That is not
C1. C1 also requires the **cross-boundary** derivative to match, and in a bicubic patch that
derivative is *not* determined by the boundary data:

```
S_u(1, t) = 3 * (row3(t) - row2(t))
```

`row3` is the shared boundary row; `row2` is one step inside, and it is **private to the face**.
So a face that agrees perfectly with its neighbour about the edge curve, its parameterization, and
its step still has a free interior row, and therefore a free cross-boundary derivative. Measuring
only the boundary gives `0.000e+00` and proves nothing about continuity across the seam.

This is the same defect the current model has, one level of abstraction up. `crate::continuity`'s
`4.771e-2` rad is not a Bernstein failure; it is "the boundary does not determine the normal".

**What does work, and is exact:** prescribe the cross-boundary derivative as **shared edge data**,
and let each face *derive* its own interior row from it:

```
G(t) = (1 - t) * G_p + t * G_q                 one vector per edge endpoint, shared by both faces
row2(t) = row3(t) - G(t) / 3                   derived, private to this face, never shared
```

Then `S_u(1, t) = 3 * (row3 - row3 + G/3) = G(t)` on one side and `S_u(0, t) = G(t)` on the other, with
the same function. **Exact C1, at every edge, at every valence, with nothing enforced.**

This is the classical bicubic C1 spline, on an arbitrary quad mesh. It is the resolution of the
*vertex consistency problem* of Halbfleiß & Peters
(<https://hal.science/hal-00319653>) — the problem the current handoff cites as unsolved here is
solved by the fact that **each face derives its own interior and no two faces write the same point.**

## The parameter synchronization, and why it is free

`G` above is `dS/du` in **local** parameter units, so both faces can only share it if they agree on
the length that local `u` stands for along the shared edge. Write that length as `Delta_e`. Then

```
dS/du|_A  = Delta_e * dS/dU          dS/du|_B = Delta_e * dS/dU
```

and C1 needs `Delta_A == Delta_B`. So the synchronization is not optional bookkeeping — **it is the
whole reason C1 rather than G1.**

The useful part: `Delta_e` is a property of the **edge**, not of either face. Two faces traversing
edge `e = (p, q)` see the same endpoints, so seeding `Delta_e` from the chord length `|P_q - P_p|`
makes it shared *by construction*, with no stored data and no rule for keeping it in step. A
per-face `Delta` is not a synchronisation at all — it is two independent reparameterisations, and
what comes out is G1.

So the instruction is right that parameter synchronization is what delivers C1; it is right that it
should come from the adjacent control points rather than from per-patch data; and the one thing it
misses is that the cross-boundary derivative has to be *prescribed and shared* as well, because no
amount of agreeing about the edge determines the normal across it.

`V_p` and `V_q` are seeded from the same chord, which is what makes the boundary curve a
Catmull-Rom-style cubic rather than a straight line — the overshoot risk on a non-uniform grid, which
probe measurement 6 is for.

## What the representation becomes

| today | Hermite | shared? |
| --- | --- | --- |
| mesh vertex positions | mesh vertex positions | yes, `O(1)` per corner |
| shared degree-3 boundary row, 4 points per edge | shared boundary Hermite cubic `(P_p, V_p, P_q, V_q)` | yes, per edge |
| — *(nothing plays this role)* | **cross-boundary derivative `G`**, per edge endpoint | yes when smooth, per-face when creased |
| 4 free interior control points per face | derived from `G` | **no — private and computed** |

The first two rows are the same function class: a cubic Hermite curve and a cubic Bézier row are a
bijective linear reparametrisation of each other, so **the shared boundary is unchanged**, bit for
bit in the limit. Storage is also unchanged: 4 corners + 8 boundary interior + 4 private = 16 points
either way, and the 12 shared floats per edge become 4 tangent vectors.

So this is not a change of capacity. It is a change of **which coordinates are independent**: today
they are Bézier control points related by an invariant that a pass must apply; after, they are edge
fields that are *read* by both faces.

## What it deletes, and the deleted things are the documented defects

| deleted | what it was |
| --- | --- |
| `enforce.rs` | the whole pass: share boundaries, apply the mirror, arbitrate the contest by `EdgeSlot` |
| `Continuity::Soft(mu)`, `SoftEdge`, `slave_interior` | one scalar per edge, chosen to make a mirror land |
| `Edit::Overconstrained { by }` | `proposals/two-soft-edges-one-face.md` in full — impossible when nothing is owned twice |
| the accepted `4.771e-2` rad gap | `arbitrary-topology-smoothness.md` failures 1 and 2 |
| the "contested place" | 13 of them on the probe's sample, all cross-face, all gone |
| the "`c = 0`" argument in `layers_probe` | there is no operator `E` to be affine any more |
| the `1.389e-1` two-soft-edges-one-face cost | same |

The hierarchy survives unchanged in shape and gets *stronger*: a displacement level is a second set
of edge fields, summed. There is no invariant to re-establish because there is no invariant; the
`enforce`-per-level pass and its ordering question (`hard first, then soft`) disappear with it.

## What it costs — the part to decide before any code

**1. The face loses its private interior.** Under exact C1 the interior row is *determined* by `G`.
The algebra above says there is nothing left: `row2 = row3 - G/3` uses every degree of freedom. So a
patch can no longer be sculpted by dragging its own interior control points; every edit is an edit to
shared edge data and reaches every incident face.

That is a different editing model, not a worse one for terrain — an edit that cannot create an
unsmooth ridge by accident — but it is a different one, and `crates/patch/src/lib.rs` currently
promises "one editable patch per face".

**This algebra is marked for measurement, not accepted.** The argument is clean and this repository
has a record of clean arguments that a probe refuted
(`proposals/arbitrary-topology-smoothness.md` § "Correction to a claim made in discussion"; the
`mu = -1` argument; the "contested places are always cross-face" claim). The first probe result must
be: *does exact C1 leave any per-face freedom at all, on a quad of valence != 4?* — with a positive
control.

**2. Creases need per-face storage, and that reintroduces one branch.** A crease is a cross-boundary
derivative that is *not* shared: the edge keeps a private copy per face. The tangential data stays
shared either way, so a crease is still never a crack — which is the crate's stated promise and the
one thing the current model gets exactly right.

**3. T-junctions stay unsolved.** `EdgeKey` is a pair of endpoint ids, so a vertex in the middle of
an edge is unrepresentable. Unchanged by this proposal, and unchanged by the alternative.

**4. Twist control is gone.** The mixed derivative is what option 1 consumes. Parametric modelling
wants it; terrain does not. Worth one sentence from whoever will author surfaces.

## Options, with what each costs

| | exact C1 | per-face freedom | code |
| --- | --- | --- | --- |
| **A. Derive the interior from shared `G`** | everywhere | none | one path |
| **B. Prescribe `G` per face as `lambda * G_shared`** | never; G1 only | 4 private scalars per face | one path, reparameterized |
| **C. A with a per-face "free interior" mode** | unless the face opts out | 4 private scalars per face | two paths, and a face's own continuity depends on its mode |

Option B is the current model's expressiveness with the defect removed — G1 everywhere, exactly,
which is already better than the shipped `4.771e-2`. Option C is the only one that keeps local
sculpting, at the cost of two behaviours in one crate.

## The probe that decides, before the rewrite

`crates/patch/examples/hermite_continuity_probe.rs`. **Measured 2026-10-15, all five pass.**

| # | question | measured | verdict |
| --- | --- | --- | --- |
| 0 | can the probe see a discontinuity when there is one | `6.105e-1` rad one side only, `2.980e-8` when both sides move | fires, and the void arm is kept |
| 1 | is a soft edge **exactly** C1 | split `0.000e0`, along `2.107e-8`, across `0.000e0`, normal `2.980e-8` rad | **exact C1**, at every valence, on 17 seams |
| 2 | what does the soft tag buy, against a private cross-boundary derivative | `across` moves `0.000e0` → `6.440e-1` rad; `split` and `along` unchanged and exact | the correction, as a number |
| 3 | does a hard edge keep its boundary while the interiors move freely | split `0.000e0`, crease survives | yes |
| 4 | is the boundary the same curve as the Bernstein row it replaces | `1.388e-17` | same curve, a reparameterization |

`across = 0.000e0` on measurement 1 is not the `acos` floor — it is exact, because the two faces'
cross-boundary derivatives are the same function rather than two functions that agree. The
`2.980e-8` on the normal is two times the floor and is a cross-product rounding artifact of vectors
that are not unit length.

**Measurement 0 has two arms on purpose.** Perturbing a node column and measuring the perturbed grid
reports the floor, because both faces on every touched seam move together — the first version of this
control was exactly that and could not fail. Perturbing one side reports `6.105e-1` rad, `2.0e+07`
times measurement 1.

Three bugs the probe caught are recorded in the handoff. The third is worth repeating here because
it is not a typo: **the cubic Hermite basis array interleaves value and derivative** — `a = 0` is the
near edge's position, `a = 1` its cross-boundary derivative, `a = 2` the **far** edge's position,
`a = 3` its derivative — so the interior rows are `1` and `3`, and nudging `a = 2` moves the far
edge's corner positions.

### What is settled and what is not

**Settled.** On a regular grid, with the jet stored per node, a soft edge is exactly C1 and a hard
edge is exactly C0 with a free interior, and neither costs an enforcement pass. That answers the
`arbitrary-topology-smoothness.md` "change basis" row for the regular case and closes the
`4.771e-2` rad accepted gap for it.

**Not settled, and it is the larger half.** On an irregular quad mesh there is no per-node `Fu` — a
valence-8 vertex has four distinct u-directions, so the slot is not well defined — and per-edge data
runs into **16 equations on 12 unknowns for a face with all four edges soft**. A face with at least
one hard edge is unconstrained, which is the escape, and it is also why the ruling's hard/soft split
is load-bearing rather than cosmetic. Our own front end is **89.4 % extraordinary**, so the escape
has to be usable at extraordinary vertices too. **That is the next measurement, and the rewrite
should not start before it is answered.**


## Where this would land

- New `crates/patch/src/hermite.rs` replacing `rect.rs`'s evaluator role; `bernstein.rs` survives for
  the degree-elevation identity and the boundary-curve change of basis.
- `continuity.rs` rewritten from "the invariant" to "where a cross-boundary derivative is stored".
- `enforce.rs` deleted. `level.rs`, `patch_mesh.rs`, `tessellate.rs` re-pointed at the new storage.
- `gallery/src/nurbs.rs` is the first caller and would be renamed off "NURBS", which is now a
  misnomer for the crate in either direction.

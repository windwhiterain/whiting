# Handoff: a Hermite surface in place of per-patch NURBS

Status: **the three things this branch was asked for are built, measured and green, and so is the
massif.** Branch `feature/hermite-surface`, worktree `.worktrees/hermite`.

| asked for | where | measured |
| --- | --- | --- |
| the Hermite surface | `crates/patch/src/hermite.rs`, `hermite_surface.rs` | `across 0.000e+00` on every soft seam; `split 0.000e+00` everywhere |
| friendly agentic editing | `crates/patch/src/edit.rs` | a batch of edits leaves soft seams **bit-identical**; undo exact in all three stores |
| a gallery scene to show it | `crates/gallery/src/hermite.rs`, registered `"hermite"` | crease `1.034e0` rad against `2.980e-8` beside it |
| **the massif on it** | `crates/gallery/src/blockmesh.rs`, `from_belts` | one surface, 182 belts as cells, 147 shared valley seams |

Every probe green, including the massif's. Scene census `13 passed`. `cargo fmt --all --check` clean.

**Not verified:** that any scene *renders*. They compile to a document and emit the passes the census
expects, and their geometry is measured directly, but nothing here puts a frame on screen.

## The massif: one belt is one patch, and the joins are the caller's choice

`from_belts` builds **one** `HermiteSurface` for the whole massif, with each belt a group of faces in
it, rather than a surface per belt. That is not tidiness: `Join` hangs off an `EdgeKey` — two face
ids in one `Topology` — and two separate surfaces have no shared topology to tag, so while each belt
was its own surface **soft and hard were not available at all**.

The seam is made shareable from above. `expand.rs`'s `valley_run` depends on `(valley, segment)` and
not on `side`, so a valley's Left and Right belts carry the same run; it is read off the faces at
`v = 1`, resampled by arc length, and both belts are handed the **same vertex ids** through a pool
keyed by `(valley, segment, column)`.

```text
182 belts at 4x4, 1386 patches, one surface
147 inter-belt edges, all tagged hard
split 0.000e+00   along-edge 2.980e-8 rad   normal 2.980e-8 rad
retagging one seam soft:  normal 2.980e-8 -> 1.490e-8 rad
```

Hard is C0 exactly with the two faces already disagreeing by pi, which is what a crease is. Softening
one edge lands at `1.490e-08` — the `acos` floor itself, so the seed had already agreed and the tag
was all that was missing. **Every seam starts hard**, because a crease is the conservative default;
`set_join` on the surface is how a caller softens one.

Measured, the first seam's two endpoints are literally the same vertex ids in both faces: face 6 has
`[8,9,13,12]` and face 14 has `[24,25,13,12]`. That is what makes the split zero.

## A belt is a block on this massif, and that is measured, not assumed

`belt_ratio_probe` sweeps eight seeds and five sets of generator parameters: `blocks/belt` is
**exactly 1** every time. The reason is structural — `expand.rs` claims one block index per belt
*before* walking its faces, and pushes once *after*, with no inner split. So "can a belt hold several
blocks" asks whether the generator does something it structurally cannot. The grouping that can
still change is how many belts a valley carries: **median 13, over 7 valleys.**

## The massif fit residual: the threshold was wrong, and three theories were disproved first

`block_surface_probe` read `floor * 100` where `floor` is the `f32` precision of the rock's **vertex
coordinates** — not of its surface. Those differ by five orders of magnitude, and the ceiling is now
**the rock's own spread across a cell**:

```text
side   departure (median)   the rock's own spread across one cell
   2         2.767e-06                    4.105e+01
   4         1.799e-03                    1.318e+01
   8         2.288e-01                    5.581e+00
```

A block is 33 triangles on average, so a cell spans several and the rock is not flat across them; a
surface interpolating the rock's corners cannot follow that. **825 of 825 blocks** have a departure
below their own cell's spread.

Three candidate causes were disproved by measurement before this one was accepted, and all three are
in `step_fit_probe.rs`:

- **the shared per-direction step** — a block alone and inside the mesh give **identical** numbers to
  every digit. The reason is that `from_facet` already gives each block its own `HermiteSurface`, so
  there was never a mesh-wide step to be wrong.
- **cell anisotropy** — the space-measured cell aspect is `0.376` at every resolution.
- **a shifted patch index** — `push_quads` drops collapsed cells, but side 3 is 728 cells to 728
  patches and side 4 is 1638 to 1638; nothing was dropped.

What the front end does, now the ceiling stops hiding it: at side 4, blocks where **no corner was
moved** sit at `4.152e-06`, the f32 floor, and the Hermite patch matches the bilinear reference
through the same corners at every resolution. So the patch costs nothing, and what remains is the
rock's own shape and the projection onto a ragged region.

## Measurements that read the wrong thing

Five in this branch, all in probes, all of which produced a confident number:

1. The seam probe sampled the grid's `u = 0` **side** rather than the valley (`v = 1`), and reported
   `1.185e+02` as a gap. I reported that to the superior as "not even C0". It was `2.412e-02`.
2. Corrected to the right edge, it still compared two faces at the same parameter — **two faces on a
   shared edge traverse it in opposite directions** — so the same `t` names two different places.
3. The block probe sampled only at grid corners, where a Hermite patch interpolates by construction:
   `0.000e+00` at every resolution. Its own "the departure must not be zero" assertion caught it.
4. The precision floor came out `NaN` from an `f64` bit trick and sat inside the threshold, so a
   `2.5e+02` departure **passed**. A `NaN` threshold passes everything.
5. Blocks were paired with the massif by list position while failures were skipped, so block *i* was
   measured against the wrong rock — `2.5e+02` again.

What finally pinned #1 and #2 was measuring the **shared vertex ids**, which is a fact about the
surface rather than about how it was sampled.

Agent's understanding: this file.
Proposal: [proposals/hermite-c1-by-shared-derivative.md](../proposals/hermite-c1-by-shared-derivative.md)
Fit notes: [investigate/blockmesh-sampling-fit.md](../investigate/blockmesh-sampling-fit.md)
Probe: `crates/patch/examples/hermite_continuity_probe.rs`

## What landed

| File | What |
| --- | --- |
| `crates/patch/src/hermite.rs` | `Jet` — the four values at one corner. `HermitePatch` — sixteen coefficients, `evaluate`, exact `tangent_u`/`tangent_v`, `normal`, `boundary`, `corner`/`set_corner`. |
| `crates/patch/src/hermite_surface.rs` | `HermiteSurface`, `Join::{Hard, Soft}`, `SurfaceError`. The mesh on top, and every mutator. |
| `crates/patch/src/topology.rs` | `EdgeSlot::is_u_edge`, one method, the whole of "which derivative an edge carries". |
| `crates/patch/examples/hermite_continuity_probe.rs` | **now measures the crate**, not a copy of it. |

## The model, as built

```
per mesh vertex   position, du, dv         read by every face meeting there
per face corner   duv (the twist)          the face's interior
per edge          Hard | Soft
```

Two faces on a soft edge have **equal corner jets**, and two patches with equal corner jets are the
same surface — so C1 is structural. Measured on 31 interior seams: `across 0.000e0`, normal
`2.980e-8` rad.

**The crease comes from the twist and nowhere else.** `duv` appears in the cross-boundary derivative
and in no boundary curve, so a face can hold its own without touching what it shares. Measured on one
seam: `split` and `along` stay at `0.000e0` in both arms; `across` moves `0.000e0` → `1.572e-1` rad.

`set_twist` carries a value across every **soft** edge at the corner, transitively, so the write is
the whole edit. **Transitive is load-bearing**: the faces around a vertex are a cycle, not a tree,
and the first version carried one level — which measured `1.699e-1` rad on seams the edit was not
aimed at. Tag an edge hard and the carrying stops on it, which is how a crease appears.

## Measured

`cargo run -p whiting-patch --example hermite_continuity_probe` — five measurements, exit 0.

| # | measurement | result |
| --- | --- | --- |
| 1 | every edge soft | `split 0.000e0`, `along 2.107e-8`, `across 0.000e0`, `normal 2.980e-8` rad — **exact C1** |
| 0 | control, one seam broken | `across 1.572e-1`, `normal 1.571e-1` rad; every other seam `2.980e-8` — `5.0e+06` times larger |
| 2 | hard against soft, same seam | soft `normal 1.490e-8`, hard `normal 1.571e-1`; `split` and `along` exact in both |
| 3 | both sides twisted independently | crease survives, `split 0.000e0`, rest of the surface untouched at `2.980e-8` |
| 4 | boundary against an independent Bernstein basis | `1.110e-16`; twist moved `1.030e0`, shared boundary moved `0.000e0`, interior moved `9.669e-3` |

## What it costs, recorded not hidden

- **A face has four free vectors on a fully hard mesh and none on a fully soft one.** The same count
  as the old model's four private interior points — but the freedom is *twist*, and there is no
  longer a handle that moves one patch's middle without its neighbours.
- **A private cross-boundary derivative is not available in this representation at all.** A boundary
  curve and the cross-boundary derivative are the same data, so sharing one forces sharing the other.
  **This is why the rewrite removes the enforcement pass and not the fact that a crease costs
  something.** `crate::PatchMesh`'s free interior row is the escape it does not have.
- **Twist is stored per face corner and carried, not stored once per edge.** That is a second copy of
  a number, bought on purpose: sharing one slot per edge forces a corner onto one answer, and a lone
  hard edge would then produce **no crease at all**.
- **T-junctions are still unrepresentable**, unchanged.
- **Extraordinary vertices are still the open half.** Per-vertex `du`/`dv` need every face at a
  vertex to agree about which way is `u`; they do on a consistently wound quad mesh because
  `EdgeSlot::MinU`/`MaxU` are the edges that fix `u`, so no computation is needed — and
  `HermiteSurface::build` **refuses** a mesh where two faces disagree on an edge's kind rather than
  producing a surface that disagrees with itself.

## The gallery scene, and the probe that is red

`crates/gallery/src/hermite.rs`, registered as `"hermite"`. A 13 by 9 sheet built from a
`HermiteSurface`, **one column of edges tagged `Join::Hard`, and the two sides twisted apart.** That
pair of writes is the entire crease: the positions carry a dome and no fold at all, so the fold is
attributable to the tag and not to the geometry. Normals are read per face, because a hard edge means
two faces genuinely disagree and one shared normal would round the fold into a ridge.

`crates/gallery/examples/hermite_probe.rs` measures the surface rather than the document. **It is
red, on purpose, and the commit message says so.**

| | split | normal |
| --- | --- | --- |
| soft seams | `2.892e-3` | `2.526e-3` rad |
| crease | `1.853e-3` | `5.722e-1` rad |
| control — retagged soft **and re-twisted** | `1.853e-3` | `4.469e-4` rad |

The crease is `2.3e+05` times sharper than the seams beside it, so **the scene shows what it is
for**. What is unexplained is that the seams are not at the `acos` floor here, as they are in the
crate's own probe, and `2.9e-3` of **split** — a distance between two points both faces read from
the same vertices and the same `du`, `dv` — has no cause yet. It is an order below what the old model
shipped (`4.771e-2`) and it is not zero. **Not to be weakened to pass.** This is the first thing the
next round should chase.

> **Resolved, and the cause was the step.** The scene's seams read `2.892e-3` of split because the
> parameter step was computed **per face**, so the two faces of a shared edge scaled their boundary
> curve by different amounts. One shared step per direction took the same reading to `0.000e0` and
> the normal to `2.980e-8`. It is the reason `HermiteSurface` carries `steps` and `step_spread` at
> all, and the cost is recorded in "What it costs" rather than hidden.

The control had to be **rebuilt**, not merely untagged: the tag governs whether a write is *carried*,
not whether values already stored happen to match, so untagging alone left the fold in place and the
control reported a null result.

## The slot bug the scene found

`EdgeSlot::MinU` is the edge that **fixes** `u`, which means **`v` runs along it**. So `MinU`/`MaxU`
are the `v`-direction edges and `MinV`/`MaxV` the `u`-direction ones — and this was written the other
way round.

It does not crash. It rotates the seeded tangent field a quarter turn and makes a seam comparison ask
for `S_u` where it wanted `S_v`. Every interior edge of a 13 by 9 sheet read a split of `8.1e-1`,
one cell away, which is the signature. `EdgeSlot` now answers the two questions separately:

- `varies_along_u` — which derivative the edge carries as its tangent.
- `separates_u` — which way the two faces are stacked, and so which pair of ends a caller compares.

Neither can be inferred from the other without the winding. **This is the trap
`crates/gallery/src/nurbs.rs`'s own module documentation already records** — "`EdgeSlot` names the
parameter an edge *fixes*, so `MinU` varies `v`; reading it the other way round asks each face for a
normal off its perpendicular edge and returns a plausible fold that is not there" — and it was walked
into anyway. Read it before touching either method.

## The residual, and what chasing it found — this is the load-bearing section

The gallery scene's soft seams read a split of `2.892e-3` where they should read `0.000e+00`, and
that residual was chased to ground this round. **The cause is the parameter step, and the resolution
is that this representation needs one more shared field than it has.**

A boundary curve's Hermite tangent is `dP/du * step_u`. So the curve along a shared edge is scaled
by a number that, today, belongs to **the face**. Two faces sharing an edge therefore draw slightly
different curves along it whenever their `step_u` or `step_v` differ — which they do whenever the
mesh's cells are not exactly equal. On the crate's regular grid they agree and the split is
`0.000e+00`; under the gallery sheet's dome and knock they differ by about one part in a hundred,
and that is the `2.9e-3`.

**Two fixes were built and measured. They are mutually exclusive, and that is the finding.**

| assembly | cross-boundary derivative (`across`) | split |
| --- | --- | --- |
| four corner jets + two per-face steps (**shipped**) | **`0.000e+00` — exactly C1** | `2.892e-3` |
| four boundary curves, each scaled by **its own edge's chord** | `3.010e-2`, with zero-length normals | **`0.000e+00`** on the crease |

The second looks obviously right — "each face's boundary is the shared edge's curve" is literally
what sharing means — and it fixes the split. It destroys continuity, and **the reason is structural
rather than a coding slip**:

> `dS/du` at the `u = 0` boundary reads the **`a = 1` row** of the coefficient array. That row's two
> end coefficients come from this face's own `MinV` and `MaxV` edges — the two edges
> *perpendicular* to the shared one — and those are not shared with anybody. So once each boundary
> line is scaled by its own edge, the cross-boundary derivative stops being a function of shared
> data, and the normal mismatch a soft edge exists to eliminate comes straight back.

**So: a bicubic Hermite face cannot have both a per-edge-scaled boundary curve and a shared
cross-boundary derivative.** A corner jet has two derivative slots; a shared edge's boundary tangent
and its cross-boundary derivative are not the same slot; and the array's derivative rows reach the
perpendicular edges. This is the vertex-consistency problem of Halbfleiß & Peters
(<https://hal.science/hal-00319653>) — the thing the previous branch cited as unsolved here and could
not resolve — arriving from the direction of the patch rather than the vertex.

**What follows, and it is the next slice:** the cross-boundary derivative has to be **stored per
shared edge** rather than derived from the corner jets. That is the `G` field of the original
proposal, and the measurement above is why it is load-bearing rather than a refinement: without it
the choice is C1 with a residual split, or C0 with a residual normal — and there is no third option
at degree 3.

### The gallery probe's thresholds, and why it is still red

`crease` at `5.722e-1` rad and `soft seams` at `2.526e-3` rad is a separation of `2.3e+05`, so **the
scene shows what it is for** and the crease is unambiguously the sharpest thing on the sheet. What
is red is the absolute claim: the seams are not at the `acos` floor, and the `2.892e-3` split is not
explained by anything other than the step, which is now explained but not fixed.

**Not to be weakened to pass, and not to be "fixed" by the per-edge assembly**, which trades this
failure for a worse one.

## The agentic editing layer, and what it is allowed to say

`crates/patch/src/edit.rs`. `Editor` over a `HermiteSurface`: `lift`, `tilt`, `fold`, `unfold`,
`undo`, `commit`. `examples/hermite_edit_probe.rs` measures it.

**The claim being checked is not that the edits work. It is that there is no edit here that can put
the surface into a state the next read has to repair.** That is what the old model asked a caller to
know — that a control point might be claimed by two edges, that writing it re-runs a pass, and that
the pass resolves any contest by write order — and it is why `Edit::Overconstrained` and the
arbitration existed.

Measured on a batch of `lift` + `tilt` + `fold`:

| | measured |
| --- | --- |
| soft seams **before** the batch | split `0.000e0`, normal `2.107e-8` rad |
| soft seams **after** the batch | split `0.000e0`, normal `2.107e-8` rad — **identical** |
| the fold | split `0.000e0`, normal `1.616e0` rad |
| after `unfold` | split `0.000e0`, normal `1.490e-8` rad |
| region lift, interior vs outside its support | `8.601e-1` against `0.000e+00` |

The edits are named in the shape a request arrives in — a centre and a radius, not a control point
index — the falloff reaches zero *with zero slope* at the edge of its support so a region edit leaves
no lip, and `fold`/`unfold` are one call each rather than a tag plus two order-sensitive writes.

### Undo is exact, and getting there was two separate bugs

Measurement 6 read `positions()[v].z` to ask how far a region lift moved things, which also reads the
dome — up at `0.7` everywhere — so it reported the whole sheet rising by its own shape. It measures
the **displacement** now. Recorded because it is the third time this repository has produced a null
result that was a measurement rather than a result, and because the two previous ones were both found
the same way.

## The `nurbs` scene is retired, not kept alongside

`crates/gallery/src/nurbs.rs` and `examples/nurbs_probe.rs` are **deleted**, not deprecated.

They drew the same picture as `hermite` — one patch sheet with a crease down it — on the other
representation. Keeping both would have been two scenes doing one job, which is the duplication
`AGENTS.md` names, and the newer one is strictly better: the crease is one call rather than a tag plus
two order-sensitive writes, and the seams beside it are exactly C1 rather than measurably smooth.

**The pass counts are unchanged**, which is worth noticing rather than assuming: one caster group, one
mesh, one light, one post, before and after. The document the scene compiles to is the same size. So
the swap is not a cost, it is a removal.

### The census caught what its own comment warns about

`crates/scene/tests/clustered_lights.rs` holds a row per scene and its comment says a scene added
without a row "does not fail loudly — it fails as a list that differs by one entry". Renaming `nurbs`
to `hermite` did exactly that, and the first run was red: **the expected list is ordered, and
`hermite` sorts between `foldyard` and `landscape`, not where `nurbs` sat.** The row moved and the
thirteen tests pass. This is the third time this session that a measurement caught something an
argument had already called settled, and it is the reason the file carries the comment.

### What the old model's probes are now guarding

`crates/patch` still carries `PatchMesh`, `enforce.rs`, `continuity.rs`, `level.rs`, `rect.rs`,
`simplex.rs`, `tessellate.rs` and the six probes that measure them. **Nothing in the product path
calls them any more** — `gallery` draws only `hermite` now — so they are dead weight kept on purpose,
because their numbers are the evidence that the Hermite model replaced something rather than
arriving. Their measurements are recorded in `proposals/two-soft-edges-one-face.md` and in the
previous branch's handoff before the code goes.

## The massif, and the answer to whether the refactor can finish by deletion

`examples/hermite_massif_probe.rs`. This is the question that decides whether `crate::PatchMesh`
can simply be deleted, and **the answer is no**.

`gallery::patchmesh::from_facet` turns the authored massif into 7587 vertices and 7140 quads by
Doo-Sabin. `HermiteSurface::build` **refuses it**:

| | measured |
| --- | --- |
| vertices not valence 4 | 6781 of 7587 -- **89.4 %** |
| shared edges whose two faces traverse them as **different kinds** of edge | **13746 of 13834** |
| vertices of odd valence | **0** |
| positive control -- one quad wound the other way round | refused, as it must be |

**99.4 % of shared edges disagree, and the mesh is not at fault.** Zero odd-valent vertices means it
is a proper quad mesh with a valid structure; what it lacks is *grid-ness*. On a regular grid the
left face of a vertical seam calls it `MaxU` and the right calls it `MinU` -- the same kind both
sides -- and on a horizontal seam `MinV` and `MaxV`. Here almost nothing pairs up that way, because a
Doo-Sabin mesh has no consistent `(u, v)` frame to have a shared one in.

**That is the representation's boundary, stated by measurement rather than by argument:**
`HermiteSurface` covers quad meshes that are laid out like a grid. It does not cover arbitrary quad
topology, and the earlier rounds' suspicion that it might was wrong in the useful direction -- it
does not even try, and it names the edge at which it stops.

**So `PatchMesh` stays, and it stays because of this.** The front end that reaches the massif needs a
different mesh, and the previous branch's handoff already names the one: **`mountain` authors 182
blocks**, and a block-structured mesh *is* structured. One Hermite patch per authored block, rather
than one per Doo-Sabin quad, is a front end that satisfies the frame condition.

> **Built, and the recorded blocker was not one.** `blockmesh::from_belts` lays a regular grid over
> each belt's own belt-domain — which is a rectangle by construction, where a *block's* domain is a
> ragged subset of its box — and every seam it needs is made shareable from the `valley_run` that
> both sides of a valley already share. The earlier note that "unequal run counts across block
> boundaries" blocked it was about blocks; the belt grouping sidesteps that entirely, and the
> unequal-run worry turned out not to apply because `blocks/belt` is exactly 1 on this massif for
> structural reasons. Measured, in "The massif" above.

### A first theory, and why it is wrong

I expected the refusal to come from odd valence: a valence-3 vertex has an odd number of edge-ends and
so cannot pair them into consistent `(u, v)` slots. **Measured: zero odd-valent vertices.** The mesh
is even everywhere and is refused anyway, so parity is not the mechanism and a fix aimed at it would
fix nothing. Recorded because the argument was sound and would have been written into a proposal as
a fact -- the third time this line of work has produced one.

## Next

Everything this list used to hold is now done, and the list was stale for a while:

1. **The cross-boundary derivative `G` as per-shared-edge data** — superseded. C1 turned out to come
   from corner jets alone: equal corner jets make two patches the same surface, which is why the
   massif's soft seams read exactly C1 without it. See "The model, as built".
2. **The agentic editing layer** over `set_position` / `set_derivative` / `set_twist` — done, in
   `crates/patch/src/edit.rs`, and measured bit-identical across a batch of edits.
3. **`PatchMesh` beside `HermiteSurface`, and `gallery::nurbs` drawing the old model** — `nurbs` is
   retired; `PatchMesh` **stays**, on a measurement rather than on taste. `HermiteSurface::build`
   refuses the massif's Doo-Sabin mesh outright (13746 of 13834 shared edges name a different kind on
   its two faces), so removing it would remove the only path the massif has. `blockmesh::from_belts`
   is the replacement that works, and it exists.
4. **Measure the irregular case on the authored massif** — done. It builds, 182 of 182 blocks, and
   every measurement above is from it.

What is actually open, in the order it should be looked at:

- **A block-structured front end is not needed**, because the belt grouping already reaches the massif.
  The thing that *would* open is a generator that puts more than one block in a belt; on this one
  `blocks/belt` is exactly 1 for structural reasons, so there is nothing to gain by changing it.
- **`step_fit_probe` is a diagnostic, not a gate.** It proves the shared step and the anisotropy are
  not the residual; the residual itself is the rock's own faceting under a cell that spans several
  triangles, which no smooth surface can track and which is now the measured ceiling.
- **Nothing here renders.** Every scene compiles to a document and is measured geometrically, and no
  frame has been put on screen.



## What was asked

Refactor the NURBS/patch module to a Hermite surface, where each patch joins C1 because it reads
adjacent control points and their parameters in sync, with a configurable hard/soft tag per edge.

Superior's ruling on the one open trade, 2026-10-15:

> 边可配置硬边还是软边，硬边 c0，自动同步相邻点位置，软边同步剩余

which is: **hard = C0 with the boundary shared, soft = the rest synced too.**

## Read this first — the correction, and why it matters

**Parameter synchronization alone does not give C1.** Two faces agreeing about the edge curve, its
parameterization and its step agree about the *tangent along* the edge. C1 additionally needs the
**cross-boundary** derivative, and on a bicubic patch that derivative is not boundary data:

```
S_u(1, t) = 3 * (row3(t) - row2(t))
```

`row3` is the shared boundary row and `row2` is one step inside it, **private to the face**. So a
face can be perfectly synchronized with its neighbour and still have a free normal across the seam.
A probe that measures only the boundary will read `0.000e+00` and prove nothing — this is the same
class of error as `continuity`'s along-edge `2.107e-8` measurement, one level up.

**What the superior chose is exactly the fix.** A soft edge syncs "the rest" — the cross-boundary
derivative — which is the missing quantity. A hard edge syncs only positions, so the boundary is
shared and never a crack, and the face keeps a free interior.

## The model as it now stands

Per mesh edge `e = (p, q)`:

| quantity | stored | shared? | gives |
| --- | --- | --- | --- |
| `P(p)`, `P(q)` | mesh vertex positions | always | C0, never a crack |
| `V(p)`, `V(q)` — tangent **along** `e` | per edge endpoint | always, on both kinds of edge | the boundary curve is one cubic, identical parameterization |
| `G(p)`, `G(q)` — derivative **across** `e` | per edge endpoint | **soft only**; private per face when hard | soft ⇒ exact C1; hard ⇒ the face's interior stays free |

`Delta_e` — what local `u` means along `e` — is the chord `|P(q) - P(p)|`, a property of the **edge**.
Two faces traversing `e` see the same endpoints, so the step is shared with no stored data and no
rule. **That is the parameter synchronization, and it is free.** It is also load-bearing: a per-face
step is not a synchronization at all, it is two independent reparameterizations, and what comes out
is G1. See the proposal for the derivation.

The boundary curve `(P_p, V_p, P_q, V_q)` is a bijective reparameterization of today's shared
degree-3 Bernstein row, so **the shared geometry does not change** and neither does the storage
count. Probe measurement 4 checks that claim rather than asserting it.

## The open question, and it is the whole irregular-topology question

A face with **all four** edges soft is over-determined **on itself**: the four cross-derivative
constraints are 16 equations on the face's 12 one-step-in control points. A face with at least one
hard edge has no such problem — that is the free interior the hard tag buys.

So the free data is not `G` per edge. Measured or not yet:

- **On a regular grid** the over-determination resolves for free, because the data can be stored per
  **node** — `Fu`, `Fv`, `Fuv` at each grid point — and every cell reads the same values. No contest
  exists to have. This is the classical bicubic C1 spline and probe measurement 2 is it.
- **On an irregular quad mesh** there is no per-node `Fu`: a valence-8 vertex has four distinct
  u-directions, so the slot is not even well defined. Per-edge `G` needs the 16-on-12 system solved,
  and whether it is solvable is exactly what this probe is for.

**The handoff from the NURBS work says our own front end is 89.4 % extraordinary** (valences 2 to 40,
after Doo–Sabin), so the regular-grid answer is necessary and is not sufficient. Read
`arbitrary-topology-smoothness.md` failures 1 and 2: this design is a candidate answer to both, and
the probe is what would settle it.

## What the probe measured

`cargo run -p whiting-patch --example hermite_continuity_probe`, 5 by 4 node grid, 12 cells,
**17 interior seams**, seven parameters along each including both corners. All five pass, exit 0.

| # | measurement | result | verdict |
| --- | --- | --- | --- |
| 1 | every interior edge soft | split `0.000e0`, along `2.107e-8`, **across `0.000e0`**, normal `2.980e-8` rad | **exact C1** |
| 0 | control, one side moved | across `6.109e-1`, normal `6.105e-1` rad — `2.0e+07` times measurement 1 | fires |
| 0 | control, **both** sides moved | normal `2.980e-8` rad | stays at the floor, and **that is the point** |
| 2a | cross-boundary derivative shared (soft) | split `0.000e0`, along `2.107e-8`, across `0.000e0`, normal `2.107e-8` | exact C1 |
| 2b | cross-boundary derivative private (hard) | split `0.000e0`, along `2.107e-8`, across `6.440e-1`, normal `6.435e-1` rad | folds, and **never opens** |
| 3 | hard edge, both interiors dragged | split `0.000e0`, along `2.107e-8`, crease `6.435e-1` → `5.673e-2` rad | boundary survives the drag |
| 4 | Hermite jets vs Bezier control points | `1.388e-17` | same curve |
| 4 | interior moved without the boundary | boundary shift `0.000e+00`, interior moved `9.843e-2` | interior is free |

**Measurement 1 is the decisive number.** `across = 0.000e0` is not a floor — it is exact, because
the two faces' cross-boundary derivatives are literally the same function of the edge parameter
rather than two functions that agree. Compare the current model, whose soft edge reads `4.771e-2` rad
on the same kind of sample and is an accepted cost with a regression ceiling guarding it.

`2.980e-8` on the normal is **two times the `1.4e-8` acos floor, not a defect.** A normal is a
cross product of two vectors that are not unit length, so the direction error being measured is a
few ulps. The ceiling is `1.0e-7` and is documented as a decade above the floor for exactly that
reason; tightening against the floor is how a probe ends up reporting its own instrument.

### Measurement 2b is the correction this file leads with, now a number

Both arms of measurement 2 share the boundary curve, its parameterization and its step. `split` and
`along-edge` are **identical in both and exact in both**. Only `across` moves, from `0.000e0` to
`6.440e-1` rad.

> Two faces agreeing about an edge, its curve and its parameters is **C0 with a matching tangential
> derivative**. It is not C1, and it is not close to C1. The soft tag has to carry the
> cross-boundary derivative, which is what the superior's ruling — 硬边 c0, 软边同步剩余 — says.

### The control that was void, and why it is now two arms

The first version perturbed a node column and measured the perturbed grid. It reported `2.980e-8`,
indistinguishable from the passing arm, and would have been a control that could not fail: moving
every node on a column moves **both** faces of each seam it touches, so nothing becomes
discontinuous. Perturbing **one side only** reports `6.105e-1` rad. Both arms are now in the probe,
because the void one is the evidence that the other is real.

## Three bugs the probe caught, all in the probe

Recorded because they are the shape of thing this rewrite will produce again, and because the
third is not a typo.

1. **Bernstein weights used as the Hermite basis.** The degree-3 Bernstein weights
   `(1-t)^3, 3t(1-t)^2, 3t^2(1-t), t^3` and the cubic Hermite basis `h00..h11` are **different
   functions of the same degree**. Substituting one for the other evaluates the four stored jets as
   a Bezier curve — which takes `P_0` at the near end and `d_1` at the far end instead of `P_0` and
   `d_0` — and every seam read `2.501e0` apart. It looked like a discontinuous surface; it was a
   tangent where a corner belonged. **Measurement 4 keeps both evaluators, written independently,
   precisely so the two are compared rather than assumed interchangeable.**
2. **`Cell::from_nodes` slot table in the wrong corner order**, so the third and fourth corners were
   placed on the wrong sides. Same symptom, different cause, and the first fix did not help.
3. **The Hermite basis array interleaves.** `a = 0` is the near edge's position, `a = 1` its
   cross-boundary derivative, `a = 2` the **far** edge's position, `a = 3` its derivative. The
   interior rows are `1` and `3`. Nudging `a = 2` — which reads as "the interior" — moved the far
   edge's **corner positions** and reported a drag opening a seam by `5.469e-1`. The boundary lines
   are `a = 0` and `a = 2`, which is also the one place `far` has to name `2` and not `3`.

## What is still open, and what the next probe has to answer

**The grid case is settled. Irregular topology is not, and that is where the real question is.**

- **On a regular grid** the free data is `P`, `Fu`, `Fv`, `Fuv` **per node**, every cell reads the
  same four values, and there is no contest to have. That is what measurement 1 measures.
- **On an irregular quad mesh** there is no per-node `Fu`: a valence-8 vertex has four distinct
  u-directions, so the slot is not even well defined. Per-edge data runs into it instead, and the
  algebra is **16 equations on 12 unknowns for a face with all four edges soft** — over-determined by
  four. A face with at least one hard edge has no such problem, because a hard edge constrains
  nothing.

The handoff from the NURBS work measures our own front end at **89.4 % extraordinary vertices**
(valences 2 to 40 after Doo–Sabin), so the regular-grid result is necessary and is not sufficient.
What is not measured: whether the irregular case is solved by a per-vertex solve, by hard edges at
irregular faces, or by neither.

**Do not start the rewrite before that is answered.** The rewrite is a replacement of `rect.rs`,
`enforce.rs` and `continuity.rs`, and it is large enough that getting the irregular case wrong means
rebuilding the storage rather than the evaluator.

## Traps carried in from the previous branch

These cost that branch days and are the same hazards here:

- **A boundary-only measurement proves nothing.** The shared curve and the shared parameterization
  are already exact today; the defect is always one step inside. Measure across the seam.
- **Every zero needs a positive control, and the control must be able to fail.** See measurement 0's
  two arms.
- **Count the thing that changed, not its maximum.** A worst-case number hid a null result before.
- **A plane angle floors at about `1.4e-8`**, and a **normal reads a small multiple of it**. The
  ceiling on a C1 claim is a decade above the floor, and the reason is written next to it.
- **`f64::max`/`min` swallow NaN.** Every reduction counts non-finite readings first; the count is a
  reported field of `Reading`, not a discarded one.
- **The Hermite basis array interleaves value and derivative.** See bug 3 above.
- **This worktree uses a private `CARGO_TARGET_DIR`.** Six other worktrees are live against this
  repository; see the note at the end of the previous handoff.

## Not decided

- How irregular faces get their cross-boundary derivatives: a per-vertex solve, hard edges at
  extraordinary faces, or a third option. **This is the next measurement.**
- Whether the twist (`Fuv`) is free per face or derived. On a grid it is free per cell.
- Whether `gallery/src/nurbs.rs` is renamed, and whether the crate keeps the name `patch`.
- Nothing here touches `SPEC.md`, which is human-editable only.


## The massif renders, and what the render says that the probes could not

`crates/gallery/src/hermite_massif.rs`, scene `hermite_massif`:

```text
cargo run -p whiting-device-cycles --example render_cycles -- hermite_massif out/hermite_massif.png 48 CPU
hermite_massif: 48 samples on CPU, 2.54 s in the process, 0 dropped
```

It is the `landscape` scene's massif and the `landscape` scene's **eye**, read back through
`landscape::massif` and `landscape::eye_at` rather than written a second time — an eye written
twice is a camera nobody renders. So the two pictures differ in exactly one thing: `landscape`
draws the massif's own triangles, this draws the Hermite surface they were sampled into.

**Three things the render caught that six rounds of measuring did not:**

1. **The scene compiled and produced a picture that was empty.** The `hermite` sheet scene still
   renders blank; `Gallery::bounds` reports it at the origin while the exported document carries
   real coordinates. Undiagnosed.
2. **`SAMPLES` per patch is the difference between showing the claim and hiding it.** A patch cut
   into its own four corners is two flat triangles, which is the picture the massif's own triangles
   give. The first render came back visibly faceted — the opposite of what the scene is for.
3. **The shared-valley resampler was broken**, and only the render said so. It selected a segment
   with `position(|start| *start + 1.0 < target)` — a unit-less epsilon against an arc length —
   which answered the **last** segment for any target below the second segment's start, putting
   the run's whole opening stretch on its far end. Seam points landed on one place, cells were
   dropped, and the massif rendered with holes. Fixed by walking the spans: `0 dropped` of 1638.

### A hypothesis the render disproved, kept because it is about the domain

The seam rows still show fern-like spikes, and with no cell dropped they are not holes — they are
slivers between two rows placed by two different parameterisations. The obvious fix was to place
every row's column by the valley's own arc-length `u`, since a column of a belt runs crest to
valley. **The render says no, unambiguously**: the crest rows fell outside the belt's domain,
`sample_block` projected them back, and the massif came apart into separated plates. Reverted; the
reverted render's hash matches the pre-experiment one exactly.

What it says is true and worth keeping: **a belt's `u` is not a coordinate that survives down the
flank**, so the valley row and the rows above it cannot be placed by the same numbers. That is a
real open question, not a typo.
## The artifacts are folds, and dropping them costs more than it saves

The backdrop behind the massif settled what the dark patches are: **every one of them stayed
dark**, so none is a hole, and the mesh is complete. They are surface — parts of a flank that
curl back on themselves and face away from the sun, which is what a normal a whole `pi` out
looks like, and 2554 of the massif's 29484 drawn triangles are shaded that way.

The cause is the front end's fallback: a grid point no triangle covers is projected onto the
nearest point of the region that is covered, and a cell built from an invented corner folds.

**Measured, then reverted.** Carrying a mask of which corners were found rather than projected,
and dropping every cell with one:

- **It works.** The render changed exactly as the backdrop predicted — every dark patch became
  a hole showing the backdrop. The folds are gone.
- **It costs 17.6% of the surface**, 1350 of 1638 cells at `side` 4 and 8918 to 7617 at
  `side` 8. A uniform grid over a bounding box that is mostly not rock can only be built from
  invented corners, so the drop is the honest fallback and not the answer.
- **It invalidates every positional index in the probes**, because a cell's place in the grid
  is its face id only while no cell has been dropped. `block_surface_probe` read side 4 as
  `8.056e+00` where the true figure is `1.799e-03`, because it was reading the wrong patch. Third
  time in this branch a probe has indexed by position and been wrong about a model whose faces
  do not all exist.

**The real fix is a grid whose columns and rows are the covered region's own** — placed on rock
by rank rather than over a bounding box by value. It needs no projected corner and loses no
cell, and it is a change to how `grid_points` chooses its parameters rather than to which cells
it keeps. Not built yet.

Reverted to the state that renders and measures correctly. Verified after the revert: 1638
patches, departure `1.799e-03`, every probe green.
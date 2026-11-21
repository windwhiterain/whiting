# Does Catmull-Clark deliver what the per-patch design needs?

Started 2026-10-14. Branch `feature/catmull-probe` (worktree `.worktrees/catmull`, from `main` at `3c9dc55`).

## What this is

A probe, not an implementation. It answers three questions that decide whether the
Catmull-Clark direction is worth a representation rewrite, and nothing else.

The superior ruled on 2026-10-14: **abandon per-patch control followed by stitching; use
Catmull-Clark.** Before any of that is written down, these three facts are measured.

## Why measure first

The replacement is a rewrite of `subdivide.rs`, `RectNet`, and the deletion of `enforce.rs`.
Guessing at the destination of a rewrite is how a rewrite gets thrown away. Each question
below has an outcome that would kill the direction outright, and each is cheap.

## The three questions

**Q1 — Is the limit surface an explicitly evaluable bicubic patch in regular regions?**
This is what makes a displacement hierarchy possible at all. It needs a limit surface per
face with its own `(u, v)` domain and a finite control net; if the limit surface were only a
limit of meshes with no such patch, there would be nothing to add an offset field to.

**Q2 — Is the boundary curve determined by the boundary ring alone, i.e. is it planar?**
Derived on 2026-10-14 from the boundary rules `vertex point = (b1 + 6b + b2)/8` and
`edge point = (b1 + b2)/2`, which are the Chaikin-style refinement of a quadratic uniform
B-spline, and every one of which is a convex combination within the boundary ring's plane.
**Unverified.** If false, the boundary keeps genuine three-dimensional freedom and the
boundary-rule question in `SPEC.align` closes on its own.

**Q3 — Is the surface normal at the boundary determined by the interior, not the ring?**
The normal is `dS/du x dS/dv`; the boundary ring fixes only `dS/du`. If the transverse
derivative is independent of the interior, then `VTX_BOUNDARY_NONE` is the only usable
boundary mode and the choice is made. If it is *not* independent, that is still compatible —
it would mean the boundary curve is pinned and the normal is not, which is what is wanted
either way, but it must be measured rather than argued.

## Outcomes and what each one means

| Q1 | outcome | meaning |
|---|---|---|
| bicubic confirmed | keep going | a displacement field per patch is well-defined; `offsets` survives the rewrite |
| bicubic refuted | **stop** | there is no patch to carry a hierarchy; the direction is wrong |

Q2 and Q3 do not stop the direction. They select a boundary mode.

## What is deliberately not built

No Catmull-Clark code is added to `crate::patch`. The probe carries its own implementation
because a probe that reuses the code it is meant to audit cannot falsify it, and because
this must not be mistaken for a half-finished migration. Nothing in `RectNet`, `enforce`,
`continuity` or `PatchMesh` changes until Q1 is answered.

## Standing hazards from the discussion that led here

- **The cage hierarchy is not a surface hierarchy.** Catmull-Clark level 1 is a mesh, not a
  patch, so "add two surfaces on one domain" has no referent on the cage. It has one on the
  patch table. Conflating the two makes the hierarchy look impossible when it is not.
- **A pyramid's regularity is not a guarantee of smoothness elsewhere.** Catmull-Clark's
  extraordinary handling is C1, not C2. OpenSubdiv's own documentation says a smooth surface
  at irregular features can still produce "undesirable artefacts", and that "an arbitrary
  polygonal mesh will often not make a good subdivision cage". Quoted from
  <https://opensubdiv.org/docs/subdivision_surfaces.html>, fetched 2026-10-10.
- **A zero result needs a positive control.** Every measurement here has one: Q1 compares two
  refinement depths against each other rather than against zero, Q2 runs a boundary ring that
  starts in the plane as its own control, Q3 changes only the interior and requires the
  normal to move.
- **`f64::max`/`min` swallow NaN silently.** Every worst-case reduction counts non-finite
  readings explicitly before folding, or a broken run reports a clean zero.

## Measured so far, and what it cost to get there

### Q0: OpenSubdiv's own conventions, fetched 2026-10-14

Source: <https://raw.githubusercontent.com/PixarAnimationStudios/OpenSubdiv/dev/documentation/far_overview.rst>

### Patch types and sizes

| Type | #CVs | Description |
|---|---|---|
| REGULAR | 16 | Bi-cubic B-spline patches |
| GREGORY_BASIS | 20 | Bi-cubic quadrilateral Gregory patches |
| GREGORY | 4 | Legacy Gregory patches |
| LOOP | 12 | Quartic triangular Box-spline patches |

Confirms the irregular case needs **20 CVs of a bi-cubic Gregory basis**, not a B-spline net. Our
`RectNet` cannot hold it. Both are per-face, so the *parameter-domain* part of the hierarchy is
unaffected — see below — but the *storage* part is not.

### A patch's parameter domain is a subdomain of its face, and it shrinks

> Each patch represents a specific portion of the parametric space of the coarse topological face
> identified by the PatchParam FaceId. As topological refinement progresses through successive
> levels, **each resulting patch corresponds to a smaller and smaller subdomain of the face.** The
> PatchParam UV origin describes the mapping from the uv domain of the patch to the uv subdomain of
> the topological face.

**This is the assumption the probe got wrong**, and the error is worse than a wrong window width.
It is not that one face's patch covers `3x3` or `7x7` cells. It is that a patch's uv domain is a
subdomain of the face, and it is **not the same subdomain for every patch**. Feature-adaptive
refinement makes patches of different levels abut one another inside a single face:

> Transition edges occur during feature adaptive refinement where a patch at one level of refinement
> is adjacent to pairs of patches at the next level of refinement.

So a single face can hold a 1x1 arrangement, or 2x2, or 2x4, and which one depends on where its
extraordinary vertices are. **No fixed cell-to-uv rule exists**, and that is why every attempt at a
fixed window was wrong: the thing being sampled was never a function of position in one global uv
frame.

### The finding that matters most, and it is not about patches

> Aside from extraordinary points, **every location on the limit surface can be expressed as a
> closed-form weighted average of a set of coarse control vertices from the 1-ring surrounding the
> face.** … A stencil is then generated for each limit location simply by factorizing the
> bi-cubic Bspline patch weights over those of the contributing basis of control-vertices. The use of
> bi-cubic patches also allows the accumulation of analytical derivatives, so **limit stencils carry
> a set of weights for tangent vectors.**

A limit location is a **weighted sum of coarse control vertices**, and the tangent vectors come from
the *same* factorization, already carrying their own weights.

This is the whole hierarchy question, settled by one sentence. The superior's rule — *a level of all
zeros adds no bias, so the surface is exactly the root* — is **structurally** true here, not
approximately: a level whose contributions are zero has stencil weights summing against zero
vertices, so it contributes nothing at every location, and its tangent contribution is zero too.

It also means **the hierarchy does not have to be a displacement field on patches at all.** It can
be a sum of stencils over one shared control cage:

```
value  = Σ_levels Σ_stencils w · P_cage
tangent = Σ_levels Σ_stencils w' · P_cage
```

with the weights fixed by topology and the per-level coefficients on the cage. That is a
*global* net, so it is the model our own `proposals/arbitrary-topology-smoothness.md` already
identified as the one that removes the contest — "the mechanism is knot multiplicity on one shared
net, not a per-edge tag … the control net is global and no patch owns a point" — arriving from the
other direction. The knot-multiplicity note in that file is the tensor-product reading of this;
the stencil reading is the subdivision-surface one, and it does not need knots.

**Open question this raises, and it is the thing to check next.** If every limit location is a
weighted average of cage vertices, then the *entire surface* is linear in the cage — so a
displacement on the cage is a displacement of the surface, with no separate hierarchy needed at
all. Whether a per-region displacement (a level that is non-zero only over one area) can still be
represented sparsely under that model is what decides if our `offsets` survive, or whether the
honest answer is that a global net is the only form and sparsity has to come from the *sparse
matrix*, not from per-level nets. That question is not answered by this document.

## Q2 is answered, and it refutes the reasoning it was meant to confirm

**A non-planar boundary ring stays non-planar.** A cage whose boundary ring leaves the plane its
other three corners define by `RING_LIFT`, refined four times:

| | reading |
|---|---|
| ring at the cage | `3.500e-1` off its own plane |
| after 4 refinements | `3.486e-1` |
| positive control, a ring that starts in one plane | `0.000e0` |

The departure survives essentially intact, and the control is exact, so the measurement is real.

**The 2026-10-14 reasoning behind Q2 was wrong.** It ran: the boundary rules are convex
combinations of the ring's own vertices, therefore the limit curve lies in the ring's plane.
Convexity bounds a point to the convex hull; it says nothing about coplanarity, and a non-planar
ring refined by convex combinations stays non-planar. This is recorded because the argument looked
sound and would have been written into a proposal as a fact.

**Why it looked true, which is the part worth keeping.** A cage whose height is `z = f(x, y)` has
a *planar* boundary ring — every ring vertex on a given edge lies in that edge's own vertical
plane. Such a cage cannot distinguish the two claims, and it is the natural sample to reach for. A
second trap sits one level down: four consecutive ring vertices are always coplanar whatever the
ring is, so a local four-point test cannot detect global non-coplanarity either. Only a plane-fit
over the whole ring can, and only a ring with one corner lifted clear of the others' plane is
non-planar at all.

### Q3 is answered

**The interior decides the normal at the boundary.** Two cages identical on the ring, interior
lifted by `8.0e-1`:

| | reading |
|---|---|
| boundary normal moved | `6.301e-1` rad (36.1 degrees) |
| same lift read one row further in | `6.294e-1` rad |

Large, and the control confirms the normal is being read off a surface that the interior moves —
so this is independence, not a normal pinned by construction. The tangent is the ring's; the
normal is not, because it also needs the transverse derivative. For the design this means
`VTX_BOUNDARY_NONE` remains the usable mode and boundary shape is no longer the open question.

### Q1 is not answered, and the reason is a measurement design, not a verdict

**Q1 must not be read as refuted.** The blocker is that the probe cannot yet fit a plane, and
that is a defect in the probe.

A control was added for exactly this: measurement 0 fits a bicubic to a planar cage over the same
windows and must land at rounding. It reads `2.318e-1`. Until that reaches `1e-12`, measurement 1
is measuring the window rather than the limit surface, and no reading of it counts.

What was tried, each time with the control failing:

1. **Least squares over the whole window** via normal equations. The normal matrix is near-singular
   at 225 samples through 16 Bernstein columns; a planar cage fit itself to `1.5e-1`.
2. **Interpolation on a 4x4 lattice.** Same failure, so the conditioning was not the whole story.
3. **Widening the window to a full two-ring.** The residual got *worse*, not better.

The likely root cause, unconfirmed: **a face's patch parameter box is not the window this probe
assumes.** Sampling a 7x7 block of cells and renormalising to `[0,1]^2` mixes points from more than
one patch, because adjacent patches meet at a shared boundary with *different* parameter domains.
A patch's own box is `3x3` cells, not `7x7`; the two-ring is control support for *reading* the
patch, not the patch's extent in the sample. Conflating the two is what every version so far has
done, and it is why widening made things worse.

The fix is to sample exactly the `3x3` cells a patch covers, using only the extra two rings as
control support rather than as samples — or to fit against a patch model that carries the correct
per-patch parameterisation. **Not attempted yet.**

Also unrefuted, from earlier in this session: the first cage used `0.15 * x * y` on an 8-per-side
grid, putting the surface at `z ~ 7.4` across a unit face. That curvature was itself enough to make
a bicubic miss its samples by `2.8e-3` without improving with depth. The cage is now gently curved
and both facts are recorded, but Q1's verdict has never actually been tested on a window known good.

## Status

Q2 and Q3 measured and reported above. Q1 blocked on the fitter's own control; the working tree is
stashed as `catmull probe wip` on `feature/catmull-probe` rather than committed, because a probe
whose control fails should not enter history as though it ran.

Next step is narrow and stated: make measurement 0 reach rounding by sampling a patch's own `3x3`
parameter box, then read Q1. Nothing about the Catmull-Clark direction is decided until then.
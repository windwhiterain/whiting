# The `shape` crate: one curve, one sweep, one set of mesh predicates

Status: **DRAFT, revision 1 — P0 landed.** `math` and `curve` are built and measured (see *What P0
measured*); `sweep` and the mesh predicates are not, and no implementation exists for them. Every claim
about existing code is read from a source file at the commit named, and every number quoted from another
document says which. Date: 2026-10-17.

The superior's verdict that opened this work, recorded verbatim so the reason survives:

> 先做共享（尤其是曲线很值得做出来，后面必然导出都会依赖它）；曲线同意 Cubic Hermit。

Decisions taken with the superior, 2026-10-17:

- **The shared crate comes first**, before the `vegetation` crate's implementation, because a curve is
  what every later exporter must speak.
- **Cubic Hermite** for the curve. Not Catmull-Rom, not NURBS.
- **Crate name `shape`**, publish `whiting-shape`.

Related: [agentic-vegetation-generation.md](agentic-vegetation-generation.md) is the first consumer and
[agentic-building-generation.md](agentic-building-generation.md) already recorded the demand
(*"a shared mesh crate … is the unification to propose when a second caller needs it"*).

## Motivation

Three crates now need the same arithmetic and the same predicates, and two of them have already paid for
a copy each.

1. **`building` takes `mountain` for `math` alone, and says so in its own manifest.** Its Cargo.toml
   carries the comment *"`mountain` for the plan/space arithmetic only"* and *"the unification of the two
   is a future proposal rather than a copy made here"*; its D2 named *"a shared mesh crate (with the
   plan-geometry helpers `mountain` keeps private)"* as the unification to propose when a second caller
   needs it. `building` was that second caller. `vegetation` is the third, and it needs four more
   predicates that `building` wrote because `mountain`'s are inherent to `FacetMesh` — boundary loops,
   manifoldness, Euler characteristic, and signed volume — plus consistent winding, which a gallery
   probe already measures ([sphere-is-a-hemisphere-twice.md](sphere-is-a-hemisphere-twice.md)).
   **Measured, not inferred from a comment:** `crates/building/src/math.rs` is already nothing but a
   re-export of `mountain::math`, and `crates/terrain` calls `mountain::math` directly from
   `rasterize.rs` and `heightfield.rs` — so the plan/space arithmetic has **four** callers and one
   implementation, and the fourth is the one writing this file.
2. **A spine is a space curve, and nothing in this workspace has one.** `mountain` samples *skeleton
   lines* (`Sampling`, `Station`) and a massif's frame is a height field's; `patch::HermitePatch` is a
   bicubic **surface** whose boundary curves exist only as four jets handed out of a face. So the thing
   a branch, a culm, a leaf's midrib, an envelope, a root and an exported spline all need — a curve with
   tangents, an arc length, a transported frame, and a Bezier form — does not exist yet.
3. **The curve is what an exporter speaks.** Every external format that carries a spline carries a
   **Bezier** one (a cubic Bezier's four control points, or a path of them), and a cubic Hermite segment
   *is* a cubic Bezier with `P1 = P0 + d0/3` and `P2 = P3 − d1/3` — the identity
   `patch::hermite::HermitePatch` already states for its boundary (*"The four Hermite jets of a boundary as
   the four Bezier control points that draw the same curve"*). A curve designed as Hermite with a stated
   conversion is therefore the export primitive; designing it as "a polyline with tangents" and
   reconstructing Bezier control points later would be the same conversion written twice, in the second
   place, by someone with less information.
4. **SpeedTree's own warning is the requirement.** Its branch generator's `Accuracy` property says
   *"Changing accuracy may drastically affect the shape of your spines. Set this value to the maximum
   desired fidelity early"* — because its spine is a sampled point list. A curve whose **shape is its
   control points** has no such coupling: the station count changes the triangle count and nothing else.
   That property is the whole reason this crate exists separately from the crates that consume it.

## The one idea

> **A shape is its control points, and every consumer derives from one parameterisation of them.**

Concretely, one curve carries three things that every consumer needs and that would otherwise be computed
three ways:

- **a parameter `t`** in `[0, 1]` that means the same thing everywhere (see D2 — it is an arc-length
  fraction);
- **a frame** — a tangent and two directions perpendicular to it, transported so it does not spin (D3);
- **a section law** — what is drawn in the plane perpendicular to the tangent at each `t` (D4).

A branch's skin, a leaf's blade, a culm's ringed taper and a canopy's envelope are then *one* primitive:
a spine swept with a section that varies along it. That unification is the design; the rest of this file
is its consequences.

## What is in the crate, and what is deliberately not

| module | contents | why it is here rather than in a consumer |
| --- | --- | --- |
| `math` | the plan/space arithmetic `mountain::math` already is — `Vec2`/`Vec3`, `plan`, `lift`, `difference`, `lerp`, `cross`, `dot`, `span`, `normalized`, `signed_area`, `face_normal`, `clamp` | it is the one definition of which axis is up, and three crates authoring into one scene must not each have one. `building`'s P1 found that this — not `FacetMesh` — was what it actually needed |
| `curve` | `Curve` (cubic Hermite control points), `Jet { position, tangent }`, `frame`, `curvature`, `insert`, `split`, `sub`, `closest`, `segments` (Bezier) | the subject of this proposal |
| `sweep` | `Sweep { spine, section }`, `Section` (circle, ellipse, flat, authored outline), `Loft` (a sampled grid of quads as triangles) | the ring loft, the leaf, and the envelope are one primitive at three sections |
| `mesh` | `boundary_loops`, `is_manifold`, `euler_characteristic`, `signed_volume`, `winding_is_consistent`, and the edge-keyed count they share | `building` wrote them because `mountain`'s are inherent to `FacetMesh`; `vegetation` would write them a third time |

**Not in the crate, and the first row is the load-bearing one.**

- **A mesh type.** `mountain`'s per-face attributes are keyed by a **belt** and the gallery reads exactly
  that field as the identity handle; `building`'s are keyed by **element** (a mass face, a bay, a roof
  plane). Reusing either type would mean writing one document's provenance onto the other's surfaces,
  which is a document that says something untrue about itself — `building`'s P1 finding, in its own
  words. So what is shared is the **arithmetic and the predicates over a triangle list**, never a mesh
  type that claims to know what a face means.
- **NURBS, T-splines, subdivision.** [investigate/nurbs-libraries.md](../investigate/nurbs-libraries.md)
  and [investigate/hierarchical-spline-bases.md](../investigate/hierarchical-spline-bases.md) surveyed the
  surface side: ordinary tensor-product NURBS is the only representation every commercial kernel
  implements, and the hierarchical families are research. None of that argues for a knot vector here: a
  knot vector and a weight per control point are two more things an agent must write to get a property —
  exact rational conics — that no branch, blade or envelope needs. **Rejected as the representation:**
  NURBS (machinery with no consumer), Catmull-Rom (D1), and a polyline (no shape independent of its
  samples).
- **Booleans.** `building`'s D3 nominated `manifold` as an opt-in consolidation and nothing in this crate
  needs one. A boolean kernel in the bottom crate would put a search under every caller that has not
  asked for one.
- **A scene, a document, a renderer.** Nothing here knows what a frame is.

## The curve

```rust
/// One cubic Hermite segment's data at a knot: where it is, and which way it is going.
pub struct Jet {
    pub position: Vec3,
    /// The derivative **with respect to the knot parameter**, not a unit tangent: the magnitude is
    /// what makes the segment's length and curvature, and normalising it here would delete both.
    pub tangent: Vec3,
}

/// A cubic Hermite polyline: `n` jets, `n − 1` segments, C1 at every interior knot by construction.
pub struct Curve {
    jets: Vec<Jet>,        // at least two
    arc: ArcTable,         // lengths per segment, and their prefix sums
}

impl Curve {
    pub fn new(jets: Vec<Jet>) -> Result<Self, CurveError>;
    /// The arc-length fraction: `t = 0` at the first knot, `t = 1` at the last.
    pub fn at(&self, t: f32) -> Vec3;
    pub fn tangent(&self, t: f32) -> Vec3;
    pub fn frame(&self, t: f32) -> Frame;
    pub fn curvature(&self, t: f32) -> f32;
    pub fn length(&self) -> f32;
    /// `k` stations, each with its `(t, position, frame)`, for a consumer that expands a surface.
    pub fn stations(&self, k: usize) -> Vec<Station>;
    /// The same curve as cubic Beziers, which is the form every format exports.
    pub fn segments(&self) -> impl Iterator<Item = Bezier>;
    pub fn closest(&self, point: Vec3) -> (f32, f32);   // (t, distance)
    pub fn split(&self, t: f32) -> (Self, Self);
    pub fn insert(&mut self, t: f32) -> usize;
    pub fn sub(&self, from: f32, to: f32) -> Self;
}
```

### D1: cubic Hermite, and the exact reason (decided with the superior)

A cubic Hermite segment is determined by two positions and two tangents, so a chain of jets has **one
derivative at every shared knot** — continuity is structural and there is no pass to run. This is the
same argument `patch::hermite` makes for its surface (*"the corner jets are shared rather than
reconciled, so continuity is structural and there is no pass to run"*), one dimension down, and it means
the curve's data is exactly what a Bezier exporter needs (motivation 3).

**Rejected:**

- **Catmull-Rom.** Its tangents are *derived* from the neighbouring positions, so inserting a control
  point changes the tangents at its two neighbours: an edit is not local in shape. That is the exact
  property a hand-authored spine needs and cannot have, and it is why the shape-control argument of
  `agentic-vegetation-generation.md` D3 (an offset is local to one node) has to hold here too.
- **A polyline with a smoothing rule.** It has no shape to speak of: the shape is whatever the samples
  made, which is SpeedTree's `Accuracy` problem (motivation 4).
- **NURBS.** See above: a knot vector and weights for a property nothing needs.

### D2: `t` is an arc-length fraction, and the cost is stated

`t = 0.5` is the halfway point **along the curve**, not halfway in the knot index. The reason is that
every authored number in the consumers is a fraction of real length: a leaf band ("the top third of the
culm"), a taper ("the radius at 70%"), an envelope radius, a wind weight's profile curve, a leaf's spacing
along its twig. `mountain::math::span`'s own doc makes the argument for the massif — *"62% of the way
along it is a fraction of the line's real length; a plan distance would flatten it"* — and it holds here
with the same force.

**The cost, stated rather than discovered:** `t` is not affine in the knot parameter, so an edit that
changes a segment's length moves the physical point a given `t` names. That is accepted because the
alternative is worse in the direction that matters: a knot-parameter `t` moves whenever a control point is
**inserted** anywhere before it, so "the top third" would jump when an agent refines a branch's base.
Arc length changes only with the shape itself, and it changes *smoothly*.

**`insert(t)` is therefore meaningful and is specified as shape-preserving at the parameter, not in the
geometry:** inserting a jet does not change the curve's shape only if the inserted jet's tangent is the
curve's own derivative there — which is what `insert` computes. Splitting a Hermite segment at `u` is exact
(de Casteljau's Hermite form), so `split` and `insert` are the same arithmetic and `insert` must be `split`
followed by keeping both halves.

**The invariant is a tolerance, not bit-identity, and the reason is worth stating rather than discovering:**
a split *re-parameterises* the curve, so a sample is evaluated through the sub-segment's own local `u`
instead of the parent's global one, and the two round differently. And because `t` is an arc-length
fraction (D2), the arc table's quadrature over two halves does not sum to the identical float the whole
segment's did, so `at(t)` shifts by the quadrature difference. The probe therefore asserts what is true:
**after `insert`, every sampled position agrees with the un-split curve to within `1e-5` of the curve's
length**, and the total length agrees to `1e-6` relative. Both numbers are to be measured, not assumed.

The arc-length table is `n` prefix sums, built once (O(n)), and `at(t)` is a binary search plus one
segment evaluation (O(log n)). That is the workspace's rule — *"always choose O(1), at least O(log n),
never O(n)"* — and it is also why `arc` is a field of `Curve` rather than a function of it.

### D3: the frame is transported, and its invariants are checkable

`frame(t)` returns the tangent plus two perpendicular directions. The one property that matters is **no
accumulated twist**: a frame that spins as the curve turns makes a swept tube's surface shear, and a
leaf placed by that frame rolls for no reason the author wrote.

Two invariants are the contract, and both are checkable by a probe without naming an algorithm:

1. **A straight curve's frame is constant.** Any transport that includes a projection satisfies this and
   a Frenet frame does not (its normal is undefined where curvature vanishes).
2. **A closed planar curve's frame returns to itself** — zero holonomy, no twist after one loop. A Frenet
   frame fails this too; it is the reason a Frenet frame is rejected outright rather than offered as an
   option.

A rotation-minimizing frame by **double reflection** is the intended implementation; the *contract* is the
two invariants above, so a different implementation that satisfies them is not a change of design. The
authored `roll` and `twist` of a branch are applied **on top of** the transported frame, in that order,
and that order is stated because two orders give two different tubes.

**Rejected:** a Frenet frame (undefined on a straight span, twist on an inflection); a frame derived from
a world up vector (fails on a vertical trunk, which is the one case every plant has); and "whatever the
first segment chose" (the spin is then a function of the seed rather than of the shape).

### D4: a sweep is a spine, a section, and two laws — and the section is two shapes

```rust
pub enum Section {
    /// A circle at any aspect ratio: `Ellipse { x: 1.0, y: 1.0 }` is a circle, `flat` is a lens.
    Ellipse { x: f32, y: f32 },
    /// A closed authored outline in the section's own `(x, y)`, normalised counter-clockwise.
    Outline(Outline),
}

pub struct Sweep {
    pub spine: Curve,
    pub section: Section,
    /// The radius law: one number multiplying both axes, in the spine's parameter space.
    pub scale: Scalar,
    /// The roll law: radians in the section's plane, from `x` toward `y`.
    pub twist: Scalar,
    /// Whether the two ends are closed by a fan: the difference between a tube and a solid.
    pub capped: bool,
}
```

**The four sections collapsed into two, because they were one shape at different aspect ratios.**
`Circle` is `Ellipse { x: 1, y: 1 }`; `Flat` is `Ellipse { half_width, thickness }`, a lens; and
`Ellipse`'s `roll` is the sweep's `twist`, which is now the only place an angle is said. What is left is
an analytic ellipse and a general outline, and the four names survive as constructors —
`Section::circle`, `Section::flat`, `Section::ellipse`, `Section::outline` — so a caller reads the
vocabulary revision 1 promised while the code behind it has two arms.

**The taper moved out of the section and became `Sweep::scale`, of the new type `Scalar`.** Revision 1
wrote `Circle { radius: Curve }`: it asked a three-dimensional curve for a number, and a `Curve`'s `t` is
a fraction of *its own* length while a radius is not a length of anything. The type that sentence meant is
one-dimensional. It is `Scalar`: the same Hermite basis — `scalar` is `curve` with the space taken out, and
both now evaluate one `hermite` module — with knots that are total on `[0, 1]` and a `t` read as **the
spine's** arc-length fraction, which is exactly the superior's *"a radius curve in the curve's parameter
space"*. Putting it on the sweep instead of inside each of four sections is what makes a branch's taper, a
blade's width and a crown's reach one law. A section that must change *shape* along the spine is an
`Outline`, not a second law.

**`closed: bool` became `capped: bool`.** Every section is a closed loop, so a flag called "closed" could
only have been saying that the *ends* are closed — and the name now says it. An uncapped sweep is a tube
with two boundary loops; a capped one is a solid, which is what `mesh::signed_volume` grades and what D18's
thin leaf needs. A station whose `scale` is zero collapses its ring to a point and the fan is still built,
so a leaf's tip is closed by geometry that has degenerated, not by a special case.

**The section's plane, stated once.** `x` is the station frame's `normal` and `y` its `binormal`, so a
section wound counter-clockwise in `(x, y)` faces along `+tangent`. Which way `x` points is therefore the
**spine's** statement, made by `Curve::set_initial_normal`, and `twist` is the only other rotation — two
sections cannot be rotated against each other unless their spines were given different initial normals.

| what | spine | section | scale |
| --- | --- | --- | --- |
| a branch, a trunk, a culm | the branch's curve | `Section::circle()` | the taper |
| a leaf blade | the midrib | `Section::flat(half_width, thickness)` | the blade's width |
| a canopy envelope, coarse | the trunk's curve | `Section::circle()` | the crown's reach |
| a canopy envelope, refined | the trunk's curve | `Section::outline(points)` | the crown's reach |

The last two rows are the superior's verdict made into data: *"包络线粗版本用曲线参数空间的 radius 曲线，
可以精化到曲线参数空间上的垂直平面上的曲线的曲面"* — the envelope is a radius curve in the spine's
parameter space, and its refinement replaces the section with a curve in the plane perpendicular to the
spine. **Refining an envelope is therefore a `Section` change and not a new type**, which is what makes
"start coarse, refine later" a one-line authoring move rather than a rewrite.

`Loft` also owns the **`(u, v)` definition** every consumer writes into its point data: `u` is the station
fraction `t` (D2's arc-length fraction) and `v` is `slot / around`, the fraction around the section's loop
measured the same way — by arc length. Both are read from the same arrays as the positions, which is
`mountain`'s `belt_uv` discipline — one function, one definition, one consumer. The grid is triangulated
by the walk that produced it (the scene layer's `Topology` has no quads), the cap fans reuse the end rings'
own vertices so `positions.len() == rings * around` always, and the winding is counter-clockwise seen from
outside, so a positive volume is a check rather than a coincidence.

**Rejected:** a per-section radius or scale (four ways to say one thing, and a blade's width and a branch's
taper are the same law); a `roll` on a section (two places to say an angle); a zero-thickness ribbon arm
(an `Ellipse` with `y == 0` is the same surface and the sweep of it is still a closed loop); a general
"implicit surface" or "distance field" primitive (no consumer, and a field is a different crate's job); a
quad-mesh output (`Topology` has exactly `Triangles` and `Segments` — a hard gate); and a per-section
function pointer (an `Outline` is data an agent writes, and a closure cannot be written into a document
later).

### D5: the mesh predicates are over a triangle list, and one of them is new to the workspace

```rust
pub struct Triangles<'a> { pub positions: &'a [Vec3], pub corners: &'a [[u32; 3]] }

pub fn boundary_loops(t: Triangles<'_>) -> usize;
pub fn is_manifold(t: Triangles<'_>) -> bool;
pub fn euler_characteristic(t: Triangles<'_>) -> i64;
pub fn signed_volume(t: Triangles<'_>) -> f32;
pub fn winding_is_consistent(t: Triangles<'_>) -> bool;   // every edge used once in each direction
pub fn same_direction_edges(t: Triangles<'_>) -> usize;   // the count the sphere probe grades with
```

**One new thing, named as new.** `signed_volume` is a divergence-theorem sum over the triangles, and it is
the check that turns an inside-out solid from an invisible artifact into a named violation — `building`
had to write it because `mountain` has none. It is also the reason `vegetation`'s branches will be closed
solids rather than open tubes. It is O(triangles) and exact enough for the tolerances the two callers
state.

**What the predicates do not promise.** `is_manifold` and `boundary_loops` are *counts over an edge
table*, so they cost O(triangles) and one hash map; they say nothing about self-intersection, which is a
different question with a different cost (and `building`'s `plans_share_area` is the measured precedent
for how much a *correct* answer costs: an ear-clip and a pairwise triangle clip, not a predicate one
calls in a loop).

**Rejected:** moving `FacetMesh` or `BuildingMesh` here (see *What is in the crate, and what is
deliberately not*: the two disagree about what a face knows, so the shared thing is the arithmetic);
copying the predicates a third time into `vegetation` (this proposal exists so that does not happen); and
a half-edge mesh (a new representation with no caller, for predicates an edge table already answers).

## Migration

The change is **additive at every call site**, which is the property that makes it landable while two
other crates are in flight (`building` merged its volume model into `main` while the vegetation design was
being written, and again while this file was).

1. **`shape::math` is `mountain::math`, moved verbatim.** `mountain` re-exports it — `pub use shape::math;`
   — so every existing `mountain::math::…` path resolves unchanged. **Measured, and it makes the migration
   nearly free:** `building/src/math.rs` is already a re-export (`pub use mountain::math::{…}`), so it
   becomes `pub use shape::math::{…}` and every `building::math::Vec3` call site is untouched; `terrain`
   and `gallery` reach `mountain::math` directly and need change only if `mountain`'s re-export is ever
   dropped, which this proposal does not do. A re-export is not a copy: the one definition stays one
   definition, and the four callers do not learn a new path on the same day.
2. **`mountain` keeps `FacetMesh`, `building` keeps `BuildingMesh`, `vegetation` gets its own.** No mesh
   type moves. What each stops doing is writing its own `signed_volume`/`euler_characteristic` call.
3. **`gallery` depends on `shape`** for the curve (a scene that draws a spine, a `--debug` channel over a
   loft) and keeps `patch` for the surface it already draws.
4. **`patch` is untouched.** `HermiteSurface` stays where it is; when a leaf becomes a surface rather than
   a loft, that is a `patch` question and it is open, not assumed.

## What lands where

- **The crate**: `crates/shape`, publish `whiting-shape`, modules `math`, `hermite` (the one basis,
  private), `curve`, `scalar`, `section`, `sweep`, `loft`, `mesh` — the arc-length table, the basis and the
  loft each in its own small file ("organize large file into multiple small ones for sparse exploring").
- **`mountain`**: one `pub use`, and its own arithmetic deleted rather than kept beside the shared one.
- **`building`**: `mountain` → `shape` in `Cargo.toml`, and its private predicates become calls.
- **`gallery`**: a probe that draws a curve and a sweep, so the crate is exercised by a picture before
  anything depends on it — the lesson `building`'s P3 recorded (*"the point of a generator with no caller
  is that nothing executes it"*).
- **Nothing else.** No SPEC change is required to add a crate (a workspace member is not a contract), but
  `SPEC.md`'s one-line workspace description gains it, and that line is the SPEC owner's to write.

## Phasing

- **P0 — the curve, alone.** `Jet`, `Curve`, arc length, `at`/`tangent`/`frame`/`stations`, `split`/
  `insert`/`sub`, `segments`, and a probe that measures the four invariants: C1 across knots (the
  derivative from both sides agrees to rounding), shape-preserving insertion (every sample within `1e-5`
  of the curve's length, and the length within `1e-6` relative), a straight curve's constant frame, and a
  closed planar curve's zero holonomy. **Exit: those numbers printed, with a positive control for each** —
  an insert whose tangent is *not* the curve's derivative must be able to fail the second one.
  **Landed** (`57b1184`).
- **P1 — the sweep, and `math` moving.** `Section`, `Sweep`, `Loft`, the `(u, v)` definition, and the
  `(position, u, v)` return that the consumers pack. `mountain::math` becomes `shape::math` behind a
  re-export; `building`'s dependency switches. **Exit: a tube, a blade and an envelope built from one call,
  and the vertices that disagree about `u` are zero.** **Landed** with `scalar`, `section`, `sweep` and
  `loft`: see *What P1 measured*. The gallery picture is the one item of the exit that is still owed, and it
  is the gallery's file rather than this crate's.
- **P2 — the predicates, and the first consumer.** The `mesh` functions land with their first consumer: the
  loft, whose `view()` is what `sweep_probe` grades — and which is why the predicates arrived in the same
  checkpoint as the sweep rather than before it. `building`'s probes still read their own copies and can
  switch when one of them needs a predicate this crate does not have.
- **P3 — `vegetation` P1** starts on top of `curve`, `section`, `sweep` and `scalar`.

Phases P0 and P1 are the ones the superior's verdict asks for; P2 rides with the first consumer that needs
a predicate, because a predicate with no caller is the shape this repository has already learned to
distrust.

## What P0 measured (2026-10-17)

P0 is **landed**: `crates/shape` holds `math` (moved, with `mountain` re-exporting it) and `curve`, and
`cargo run -p whiting-shape --example curve_probe` prints seven readings, each with the control that could
have failed it, and exits non-zero if any reading leaves its threshold. Every number below is the probe's
output, not a claim this document makes:

| invariant | reading | threshold |
| --- | --- | --- |
| arc length: the table against a 4000-segment polyline of the same curve | relative difference **0.000e0** | `< 1e-5` |
| 65 stations are evenly spaced by arc length | ratio **1.000015** (the chord ratio, 1.000950, is curvature) | `< 1.0001` |
| inserting a knot at `t = 0.37` does not move the shape | worst sample **7.311e-8** of the length; length 4.390321 → 4.390321 | `< 1e-5` |
| …and the control: one knot moved by 1e-3 of the length | reads **1.022e-3** | `> 1e-4` |
| a straight curve's transported frame is constant | worst turn **0.000e0** rad over 33 stations | `< 1e-6` |
| an inflection does not flip the frame's normal | transported **1.416e-2** rad against a Frenet control's **3.127e0** rad (179.2°) | `< 0.05`, control `> 1.0` |
| a closed planar loop has no holonomy | twist after one loop **0.000e0** rad | `< 1e-4` |
| …and its length against the analytic circle | **5.2e-4** short — the Hermite representation, not the quadrature | `< 1e-3` |
| the Bezier form is the same curve | corner gap **0.000e0**, handle error **9.664e-8** | `< 1e-6` |

Four things P0 decided that this design did not say, recorded because each is a boundary a consumer will
meet:

1. **`Curve::knot_parameters()` had to exist.** A knot's *arc-length* fraction is what every authored band
   and station is written in, and without the accessor a probe can only find a knot by scanning for the
   nearest sample — which is how a C1 check becomes a guess. It is `cumulative[i] / total`, and it is public.
2. **A split rescales the parameter, so `jets()` is not a round trip.** Each half of a split carries its own
   interval, so the derivative entering the new knot and the derivative leaving it differ in magnitude while
   a [`Jet`] holds one number. `Curve::new(curve.jets())` after a split therefore replaces the left-hand
   magnitude with the right-hand one and changes the previous segment's shape. Documented at `jets()`: a
   *segmented* curve is its segments, and a caller that wants the shape back clones the curve.
3. **The frame table is eight samples per segment** and `frame(t)` transports from the nearest one. The
   straight reading is exactly zero and the inflection reading is 0.8° — the tangent genuinely turning across
   the inflection, not the table's resolution.
4. **A Hermite arc is not a circle**: eight segments reproduce a circle's length to **5.2e-4**. A caller that
   needs an arc to close — a ring, a radius — has to know that before it measures its own gap.

**Deliberately not in P0:** `closest(point)`. Nothing consumes it until the envelope's trimming law (P1), and
a query with no caller is the shape this repository has already learned to distrust.

## Open questions

Answered on 2026-10-17 under the superior's directive to choose by **strongest expressiveness and the most
elegant architecture**; each answer is a decision above or a measurement below, and the alternative is
recorded so it is not re-derived.

1. **Does the sweep belong in `shape` or in `vegetation`?** — **`shape`.** The envelope's refined form
   settles it: `Section::Outline` in the normal plane is not a branch, and a `Loft` that owns `(u, v)` is a
   statement about a surface rather than about a plant. The recorded objection — *a loft that knows `(u, v)`
   is opinionated about rendering* — is true and accepted: `mountain` made the same bet with `belt_uv`, and
   it is why its consumers never wrote a second table.
2. **The section's own orientation.** — **`x` is the frame's normal, `y` its binormal; the spine's initial
   normal is the one place a caller says which way the first section faces, and `twist` is the only other
   rotation.** An ellipse starts at `+x` and an outline at `points[0]`, and those are the only two starting
   points in the vocabulary. `sweep_probe` checks it against each spine's own frame rather than assuming it.
3. **`t` for a closed curve.** — **`at(0)` and `at(1)` are the first and last knots exactly.** A Hermite
   span's end value is `h01(1) = 1` times the last knot's position, so the basis was never the obstacle; the
   arc-length *inversion* was, and it landed one bisection step (`1 − 2⁻³³`) short of the end. `locate` now
   answers `t = 0` and `t = 1` with the segments' own ends, so a closed loop's seam closes bit for bit
   (`curve_probe`'s *closed ends* row) and a swept envelope's first and last rings coincide.
4. **How far does `closest` go?** — **Not yet, and the obligation is named rather than deferred
   silently.** Its first caller is the envelope's trimming law; when it lands the answer owes (a) its
   per-segment sample budget, (b) a measured worst-case distance error, and (c) a stated answer for a point
   whose nearest place is a knot, where two segments meet. A query whose accuracy is not printed is a query
   a caller will trust wrongly.

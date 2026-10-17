# Flat roofs: the third primitive, a plate

Started 2026-10-16. Branch `feature/building-flat` (worktree `.worktrees/building-flat`, from `main` at
`7ed7d8a`, which is P3 closed). The design is
[`agentic-building-generation.md`](../proposals/agentic-building-generation.md); P1–P3's records are its
three *What … found* sections. **This file and the proposal are the whole record**, and this file goes
when the slice is closed.

## The decision this slice starts from

P1 named the gap: *"a flat roof"* is one of the two things that phase deliberately did not have. This
slice answers **what a flat roof is**, and the answer is not "a roof with `rise = 0`":

**The roof model's skin is a strip between a crest row and an eave row.** With `rise = 0` those rows
coincide in plan, the strip has no width, and every facet of it has no area — the same failure shape the
square hip already taught this crate (a ridge of no length became one point and two zero-area facets,
which the degenerate-face check then had to be taught to distinguish from structure). A flat roof
expressed as a zero-rise roof would be a vocabulary fighting its own geometry.

**A flat roof is a plate, and the crate has no plate.** It has exactly two primitives:

- `Mass` — a **room**: a shell with a floor disc and a slab under it;
- `Wall` — a **box along a line**: a bottom that follows the ground and a level top.

A plate over a plan polygon is the third shape, and it is the one a building's vocabulary is missing:
flat roofs, canopies, plinths, terraces, steps and balcony decks are all the same solid. So this slice
adds **`Slab`: a level prism over a plan polygon**, and `RoofStyle::Flat` derives one over the mass's own
eave outline instead of a roof skeleton.

**A slab is level.** `bottom` and `top` are elevations, not per-node values, so the prism is planar
everywhere by construction; a *sloped* plate is a roof and this type refuses to be one. That is the
domain being named rather than guessed — the alternative (per-node elevations) would make the caps
non-planar for any real outline, which is the ruled-surface fault the floor already has to avoid.

## The slice, in order

1. `slab.rs` — `SlabId`, `Slab { outline: Vec<Vec2>, bottom, top }`, `SlabSurface { Top, Bottom, Edge
   { index } }`, `thickness`/`plan_area`/`volume`.
2. Wired through `SolidId::Slab`, `Element::Slab`, `expand_slab` (two caps through the existing ear
   clipper, one quad per edge), two document checks (`SlabNotThick`, `SlabTooThin`), and `identity`'s
   naming.
3. `derive`: `RoofStyle::Flat` produces a slab over the eave outline at `top − sink` with the starter's
   own `roof_thickness`.
4. `examples/slab_probe.rs`: a flat-roofed shed against arithmetic that can be written down — the prism's
   volume is `plan area × thickness`, and the derived starter must validate with **0 violations**.

## Conventions that bind (unchanged from P1–P3)

- Plan `(x, z)`, space `(x, y, z)`, `y` up, `z` south; a positively wound plan loop puts the solid on the
  walker's **right**, so a side face is wound `[bottom_i, top_i, top_{i+1}, bottom_{i+1}]` and the caps
  take the loop's own winding with `up` deciding which way they face.
- Solids stay separate and may interpenetrate (two-level watertightness). Each solid is closed, Euler 2,
  manifold, positive volume.
- `validate` reports everything in a deterministic order and repairs nothing; a derivation's answer is
  never "nearly right".

## Status log

- **2026-10-16** — worktree and branch from `7ed7d8a`; the decision above written; slice in progress.
- **2026-10-16** — **implemented and measured.** `slab.rs` (`SlabId`, `Slab { outline, bottom, top, host }`,
  `SlabSurface { Top, Bottom, Edge { index } }`, `thickness`/`plan_area`/`volume`), wired through
  `SolidId::Slab`, `Element::Slab`, `expand_slab` (two caps through the **existing ear clipper**, one quad
  per edge), two document checks (`SlabNotThick`, `SlabNotAPolygon`), `identity` naming, `seat` (a hosted
  slab moves with its building; an unhoused one does not), and `derive`'s `RoofStyle::Flat` which now
  produces **a plate and no roof**.

  Measured by `examples/slab_probe.rs` on a flat-roofed 7 × 11 starter:

  - the document holds **1 mass, 0 roofs, 1 slab** — a plate is not a roof;
  - the plate's plan is `8.4000 × 12.4000 = 104.159988 m²` (`w + 2 × overhang` by hand), `0.2800` thick,
    so `29.164793 m³` written, and the mesh's signed volume is **`29.164804`**;
  - **12 faces, loops 0, Euler 2, manifold** — the same count a box has, because the caps of a rectangle
    triangulate to two triangles each;
  - its underside is at `3.4500` = the wall top `3.6000` less the sink `0.1500`;
  - **the building under the plate does not depend on the roof style**: mass volume `78.245979` and `28`
    faces for both the flat and the hipped starter, with the hipped one holding `1 roof, 0 slabs` and the
    flat one the other way round;
  - seated on `z = 0.3 x`, `1` mass and `1` slab move, the plate running `y 4.5000 .. 4.7800` with its
    thickness still `0.2800`, and the seated document has **0 violations**;
  - a plate with **no host** stays where its author put it (`masses seated 0, slabs moved 0`, still at
    `5.0000 .. 5.4000`);
  - both faults fire and are refused: `SlabNotThick { thickness: 0.0 }` and `SlabNotAPolygon { nodes: 2 }`,
    with the document's own `DidNotExpand` carrying them;
  - identity names it: `slab 0, its top`, `slab 0, its underside`, `slab 0, the edge from node 0..3`.

  Two probes failed on the way and are **fixed rather than worked around**: `part_probe` and
  `validate_probe` both had exhaustive matches over `SolidId`, and the new variant broke them — the
  second of which now compares the slab's **authored plan loop** with the mesh's own coverage, the one
  solid whose loop the probe can measure directly. That is the same lesson the wall slice recorded: a new
  variant is a compiler-enforced audit of every `match` in the crate.

- **Next**: **closing this slice** — fold its record into the proposal (the flat roof is the last item
  P1's *"two things P1 deliberately does not have"* named), delete this handoff, and let the branch be
  merged and removed.

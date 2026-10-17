# A plan with a hole: the courtyard's wall ring

Started 2026-10-27. Branch `feature/building-patio` (worktree `.worktrees/building-patio`, from `main` at
`435383b`). The design is [`agentic-building-generation.md`](../proposals/agentic-building-generation.md), section
*The design for a plan with a hole*; the state it starts from is in the same file's *Where the system stands*.

## What is being built

A `Mass` gains **`voids: Vec<Vec<PlanNode>>`** — inner loops, each strictly inside the outline, each a courtyard. A
void adds a **wall ring around itself**; nothing else about the mass changes.

## What is already true, and must not be redone

- **`crate::fill::ring_fill(outer, inner)`** fills the region between two convex loops, **exactly**: 84/84, 92/92,
  92/92 on the measured cases and 0 triangles for a non-convex loop. `examples/ring_fill_probe.rs` is its acceptance.
  **A courtyard's wall top is that ring** — the void loop and the loop offset outward by the wall's thickness.
- **A courtyard's floor is the plate.** `FloorTop` and `SlabSoffit` are untouched; there is no plate work in this
  slice.
- **`expand::offset_loop` offsets inward.** To get the courtyard wall's room-facing side, offset the void loop
  **reversed** — the same call, the opposite side.

## The three faces a void edge needs

The mass already names three faces per outline edge: `OuterWall { edge }`, `WallTop { edge }`, `InnerWall { edge }`.
A void mirrors that, with a discriminator so a face traces back to the courtyard an author wrote:

- **the courtyard-facing wall** — the void loop's own edge, seen from the courtyard;
- **the room-facing wall** — the same edge on the outward offset loop, seen from the rooms;
- **the courtyard's wall top** — the ring between the two loops, per edge.

## Acceptance, in the order it must be measured

1. **`validate` is silent** for a mass with one rectangular void, and a second probe asserts that a void *outside*
   the outline, or two overlapping voids, are **reported** rather than repaired.
2. **The solid is manifold and closed**, and its **Euler characteristic is 0** — the crate already measures
   `2 − 2 × resolved holes`, so a courtyard is the first shape to exercise the non-zero branch. A plain box stays 2.
3. **`--where` names the courtyard**: `void 0, the wall over edge 2` covers pixels, and `void 0, its wall top` covers
   a thin ring.
4. **The nine-scene baseline does not move** (the hashes are in the design document; a mass with no void must expand
   to exactly the bytes it did before).

## Traps this project has already paid for

- `cargo fmt` writes the **main** checkout's working tree; run it on the branch, and never let it dirty the main
  checkout before a squash merge (`Aborting` follows).
- Edit Rust sources with the `edit` tool, not with scripted multi-line replacements: CRLF has made such a pattern
  match nothing, and the build then "passes" because nothing referenced the new symbol.
- A mass's own wall offset self-intersects at some thicknesses only if the crossing test is not **strict**; that is
  fixed, but any new offset inherits the rule: a touch is not a crossing.
- **A symmetric case needs a centre test**: an ear whose chord passes through a hole's corners crosses nothing
  properly and contains no vertex, yet runs through the hole. `ring_fill` learned this; anything that bridges loops
  will meet it again.
- Attachment stations land off the corners: `split_outline` always inserts, so a station on a corner makes a
  degenerate edge.

## Verified claims (2026-10-27, before any implementation)

Every symbol this handoff names was read in the tree, not remembered:

| claim | where it was checked |
| --- | --- |
| `fill::ring_fill(outer, inner) -> Vec<[Vec2; 3]>` | `crates/building/src/fill.rs:30`, module at `lib.rs:58` |
| `offset_loop` offsets **inward** and is `pub(crate)` | `expand.rs:1986` — usable inside `expand`, which is where the void's outward offset belongs |
| the mass's three faces per edge | `mesh.rs:30` `OuterWall`, `:35` `InnerWall`, `:40` `WallTop` |
| the crate measures boundary loops and Euler characteristic | `mesh.rs:347` `boundary_loops_of`, `:444` `solid_boundary_loops`; `expand.rs` reports `euler` in its own diagnostics |

## A nuance the acceptance must carry

**The crate's existing diagnostics read `euler 0` as a fault.** `expand.rs` records a real incident that way: *"an L
written from its concave node expanded to `loops 2, euler 0, manifold false` with no problem of any kind"*. A
courtyard's `euler 0` is the opposite — it is the correct answer for a solid with one hole.

So the acceptance is **not** "Euler characteristic 0" alone. It is:

> **Euler 0 together with one authored void**, and a mass **without** a void must still measure 2.

Any diagnostic that treats `euler 0` as a defect — and there is at least one in `expand.rs`'s comments — must learn
to ask whether the document authored a hole before it calls the number wrong. That is the difference between a
courtyard and the same number from a topology defect, and it is the one thing this slice has to get right that the
design did not anticipate.## The map of `expand_mass`, read before writing any of it

`expand_mass` (in `crates/building/src/expand.rs`, `fn expand_mass`) builds one mass in this order:

1. **the floor loop** — `offset_loop(&mass.outline, id, mass.wall, mass.plan_area(), &mut problems)` answers `inner`,
   the room-side loop. **The void needs its own call**: reverse the void loop and offset it by the same `wall`, which
   puts the loop on the courtyard's outer side (the room-facing face of the courtyard wall).
2. **four corners per node**, pushed as `Solid::point`s:
   `outside_base` = outline at `elevation − slab`, `outside_top` = outline at `top`,
   `inside_floor` = `inner` at `elevation`, `inside_top` = `inner` at `top`.
3. **one `WallFace::of(mass, edge, &inner)` per edge**, then `EdgeNode { outer_base, outer_top, inner_floor,
   inner_top }` per node, then **`resolve_holes`** turns authored openings into stations on those loops.
4. emission, which reads the per-edge faces and the per-node stations.

**So a void is a second, smaller copy of steps 1–4** — with two differences:

- its `inside_floor` sits at the **outline's own elevation** (the courtyard's floor is the plate, and the plate is
  already there), and its base corner (`outside_base`) sits at the *same* elevation, because the courtyard wall
  stands **on** the plate rather than hanging below it;
- its **wall top** is the ring between the two loops, which is `fill::ring_fill`'s job — the one place in this slice
  that needs it.

**The two loops wind oppositely**, and the ring filler wants them the same way; it normalises that itself, so pass
the loops as authored.

**Names**: the mass already distinguishes `OuterWall { edge }` (from outside) from `InnerWall { edge }` (from the
rooms). A courtyard mirrors that with a void index: the courtyard-facing face, the room-facing face, and the wall top
between them. `gallery`'s `building_identity` matches on `MassSurface`, so **every new arm is compiler-enforced** —
which is the reason to add them in one commit with the emission, not after it.

**Acceptance, restated for this slice**: a mass with one rectangular void has **0 violations**, builds, measures
**Euler 0**, and the nine scene hashes do not move. `Problem::VoidNotDrawn` is deleted **in the same commit** — its
own documentation promises that.


## The correction: the floor **does** need the ring (2026-10-27, later the same day)

An earlier note in the design said *"a courtyard's floor is the plate, so no plate work is needed"*. **That is
wrong**, and the reason is topological rather than aesthetic.

Follow the shell by hand. The courtyard wall's courtyard-facing face meets the plate at the void loop, at the
outline's own elevation. If the plate's top is **one** polygon spanning the whole outline — courtyard included —
then that edge has the wall going up on one side and the plate continuing on **both** sides of it. Three surfaces
meet along one edge. The solid is **non-manifold there**, and no amount of "the courtyard's floor is the plate"
makes it a corner: a corner needs the plate to **stop** at the wall.

So the plate's top is **two faces**, not one:

| face | what it is | how it is filled |
| --- | --- | --- |
| the rooms' floor | between the outline's room-side loop (`inner`) and the courtyard's **outer** loop | `fill::ring_fill` |
| the courtyard's paving | the void loop's own polygon | the polygon itself |
| the courtyard wall | courtyard-facing, room-facing, and the wall top ring | two vertical strips and `fill::ring_fill` at the top |

Four new `MassSurface` kinds, not three, and both `FloorTop` and the new paving live in the same plane — which is
exactly why the crate needs two elements there and not one: they are two regions of one plane, told apart by the
wall between them.

**`ring_fill` is therefore used three times** (rooms' floor, wall top, and — if the mass's own outline ever becomes a
ring — the mass's wall top), which is what it was built for. The design's "no plate work" line is struck; the tool was
never the problem.
## The code that makes the correction concrete

`FloorTop` is not assembled per edge — it is **one fill of the inner loop**:

```rust
let walked = polygon(&mut solid, &inner_loop, &floor_points, floor_top, true)
    && polygon(&mut solid, &outside_loop, &soffit_points, slab_soffit, false);
```

(`expand.rs`, in `expand_mass`, at the `floor_top`/`slab_soffit` pair). So a courtyard authored in a document whose
floor is this call would have the plate drawn straight across its own opening — which is exactly the three-surface
edge the correction describes, now located in one line.

That line is also where the change goes: the floor becomes **two walks** — a ring from the room-side loop out to the
courtyard's outer loop, and the courtyard's paving as its own polygon — and `fill::ring_fill` is what supplies the
first one's triangles. The helper that turns triangles into faces has to be written beside `polygon`, because
`polygon` fills one loop and the ring needs two.

## Status log

- **2026-10-27** — handoff written. Nothing implemented yet: `main` carries `ring_fill` and this plan.
- **2026-10-27** — claims verified in the tree; the acceptance corrected: `euler 0` alone is a fault in the
  existing diagnostics, and it is a courtyard only together with one authored void.

- **2026-10-27** — `Mass::voids`, `Problem::VoidNotDrawn` and the three `validate` faults landed (`8e83ea8`, `0ac2b49`);
  `expand_mass`'s structure is mapped above, so the drawing is the only thing left.
- **2026-10-27** — the floor simplification is struck: the courtyard wall meets the plate along a three-surface
  edge unless the plate top is split into a rooms floor ring and the courtyard paving.
- **2026-10-27** — the emitter is on `feature/building-patio3` (worktree `.worktrees/building-patio3`) and is red:
  a courtyard expands now (offset, paving, two wall strips and the wall top are all emitted) and the solid comes out
  `SolidOpen { loops: 4 }` with `SolidNonManifold`. The faces exist and do not meet, so the next session starts from
  that branch rather than from `main`, and its first job is to close the seams: the floor ring winding, the wall
  strips orientation against it, and the paving edge where it meets the courtyard-facing strip.
- **2026-10-27** — corners now shared: `Solid::point` interns nothing, so the plate ring, the paving and the wall
  strips must be built from **one** set of ids. That closed three of the four seams in one edit (`loops 4 → 1`).
  Measured windings, so the next session does not repeat them: the courtyard-facing strip and the wall top as first
  written give **1** loop; flipping the wall top gives **3**; flipping the courtyard-facing strip gives **3**. So the
  remaining seam is *not* in either of those — try the paving (`polygon(..., true)`), the room-facing strip, or the
  floor ring, and remember `euler 3` is odd, so exactly one group still disagrees.
- **2026-10-27** — the last seam is identified and needs a **fifth face kind**. The courtyard wall stands on the
  plate, so its **underside** is a 0.3 m annulus at the outline elevation between the void loop and the wall outer
  loop. Nothing emits it, so the shell is open there (`loops: 1`, `euler 3`). Two attempts confirm it: carrying the
  plate ring inward to the void loop instead puts **three** faces on that edge (ring, paving, courtyard-facing wall)
  and measures `loops: 2`, non-manifold. So the fix is a face, not a retraction: a `VoidUnderside { void }` (or a
  reused name that is honest) filled by `ring` between the wall outer loop and the void loop, facing **down**.
- **2026-10-27** — `VoidUnderside` exists and is emitted (a ring between the wall outer loop and the void loop at the
  outline elevation). **It did not close the shell**: `loops: 1` with the ring facing down *and* facing up, so the
  opening is not that face orientation. Two readings to guide the next attempt: `euler 3` is still odd (one group
  disagrees), and the seams that remain are the plate ring inner edge (`outer_base`, closed by the room-facing
  strip), the paving edge (`base`, closed by the courtyard-facing strip), or the paving own facing. Locate the loop
  before flipping anything else: print the boundary loop vertices (`BuildingMesh` has `boundary_loops_of`; a
  temporary debug print of the loop indices is the cheapest next instrument).

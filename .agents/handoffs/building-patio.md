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
design did not anticipate.## Status log

- **2026-10-27** — handoff written. Nothing implemented yet: `main` carries `ring_fill` and this plan.
- **2026-10-27** — claims verified in the tree; the acceptance corrected: `euler 0` alone is a fault in the
  existing diagnostics, and it is a courtyard only together with one authored void.

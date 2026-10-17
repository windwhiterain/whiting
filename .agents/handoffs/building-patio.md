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

## Status log

- **2026-10-27** — handoff written. Nothing implemented yet: `main` carries `ring_fill` and this plan.

# The identity a building's frame can carry

Started 2026-10-16. Branch `feature/building-identity` (from `main` at `b24faa8`, which closed the floor
slice). The design is [`agentic-building-generation.md`](../proposals/agentic-building-generation.md).
**This file and the proposal are the whole record.**

## The debt this slice pays

`building::identity` (P2) answers *"what is this element called"* from a document and its mesh. What it
cannot do is answer it **from a picture** — and a picture is all an agent looking at a frame has. The
mountain side of the gallery already has the join (`gallery::identity`: an `authored_id` output, a packed
point buffer, and `locate` to turn a pixel back into a belt); a building has none of it.

This slice is the **CPU half** of that join, and it is where the crate's own vocabulary ends: the numbers
per corner, and the scene's declaration that it writes them.

## What is here

`crates/gallery/src/building_identity.rs`:

- **`building_extras()`** — one output, `authored_id` at location 1, read from the points and carried
  **flat** (an element is a property of a face, not a place on it, so interpolation is the wrong operation
  and flat is the right one).
- **`outputs_field()`** — the scene's own `outputs` declaration, `f32` because `r32float` is exact where an
  `rgba16float` is exact only to 2048.
- **`ids(built, identity)`** — one id per point, three per face, and **the definition `packed` writes** so a
  probe or a tool can read the numbers without decoding a buffer.
- **`packed(built, identity)`** — the building's points with that id beside the corner normals, the
  positions and the plan `texture`, laid out through `packed_layout` on the schema's own order.

`crates/gallery/src/shaders.rs` gained `Extras::outputs()` (and made `Extra`'s fields readable), because a
tool that numbered a target off a second list would be reading somebody else's picture.

## Invariants

- **The id is written per corner and identical at all three.** The mesh is not indexed, so a value at one
  corner is interpolated across the face; a differing corner is a pixel pulled between two elements.
- **A face whose element has no row is refused, not written as zero.** Zero means "not an authored element"
  — the sky, the ground — and a wall answering zero would be a picture lying about what it shows. The table
  is built from the same mesh, so a miss is a bug, and it panics with that sentence.
- **The layout is the schema's own**, read with the same `packed_layout` the renderer uses.

## Acceptance so far, measured

`examples/building_identity_probe.rs` on a document holding a two-storey shell, a plate, a courtyard wall
and a part in a bay:

| measurement | value |
| --- | --- |
| the table | **100 faces, 50 elements numbered `1..=50`** |
| points per face | `300` for `100` faces, and **0 faces whose corners disagree** |
| the numbers written | **50 distinct ids against 50 rows — equal**, and zero is **never** written |
| what a pixel would answer | `id 1 → mass 0, outer wall over edge 0`; `id 44 → wall 0, its top` |
| the declaration | `authored_id` at location 1, `f32`, one output |
| an empty document | 0 faces, 0 elements, 0 ids |

## What is *not* in this slice, and where it belongs

**The frame half.** A scene must declare `outputs_field()` and draw its meshes with `building_extras()`,
and the renderer must keep the extra target and hand it back — the mountain's `landscape` scene is the
template for the first part (`stage_set` is its own private helper, so either lift it or write the building
equivalent). Nothing in this slice touches a pixel; what it establishes is the numbers a pixel will carry
and the sentence a missing one is refused with.

## Status log

- **2026-10-16** — branch from `b24faa8`; module, extras accessor, probe, and the measurements above.

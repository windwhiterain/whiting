# Instancing: the template's own mesh and the frame that stands it

Started 2026-10-16. Branch `feature/building-instance` (worktree `.worktrees/building-instance`, from
`main` at `d6ee447`, which closed the slab slice). The design is
[`agentic-building-generation.md`](../proposals/agentic-building-generation.md); P1–P3's records and the
slab slice's are its *What … found* sections. **This file and the proposal are the whole record.**

## The debt this slice pays

D4 says *"plan baked, vocabulary instanced"*. The document side is honoured: one `Template` is authored
once and every `Placement` names it **by index**. The mesh side is not: `expand_placement` **rebuilds the
template's skeleton for every placement**, so a building with forty identical windows expands forty copies
of the same twenty-eight faces. The proposal recorded the fix as the gallery's, and then named what the
gallery would need: *one mesh per template, one instance transform per placement*.

**The gallery cannot get there from what the crate hands out today.** `BuildingMesh` is baked: every
part's points are in world space with provenance, which is right for validation and wrong for instancing —
a scene receives 40 × 28 faces and no way to know they are one shape 40 times.

So this slice gives the crate **the two pieces a scene needs**, and keeps the baked mesh as the thing
`validate` measures:

1. **`Template::mesh()`** — a template expanded **in its own frame**, once, as an ordinary `BuildingMesh`.
2. **`PartFrame`** — where a placement stands its template: an origin, an `along`, an `up` and an
   `inward` direction, with `PartFrame::apply(point)`. `BuildingSkeleton::part_frame(index)` resolves it
   through the same `placed` path the validator uses, so a mounted part's frame is its joint's and not a
   second reading of its pin.

**The invariants that make it one derivation rather than two.** The crate's own `build` is then rewritten
to use **the same two pieces** — `Template::mesh()` then `PartFrame::apply` — so the instanced path and the
baked path cannot disagree, and the probe proves it by comparing the two point for point. A scene that
draws one template mesh with *n* transforms therefore draws **exactly** the geometry the validator
measured, rather than something that merely looks the same.

## What is deliberately *not* in this slice

- The scene side. `shed` and `yard` keep drawing the baked mesh; switching a scene to instances is a
  gallery change with its own questions (per-instance materials? per-instance shadow pages?) and this
  slice's job is to make it **possible** and to measure the duplication it removes.
- Caching inside `build`. A cache would make one expansion per template per document, but it is a
  performance change; the *contract* (a template's mesh is a function of the template alone) is what this
  slice establishes, and a caller can cache on it.

## Acceptance

`examples/instance_probe.rs`:

- a document with **six** placements of one door leaf: the baked mesh holds `6 × 28 = 168` faces, the
  template's own mesh holds **28**, and the two are the same geometry — `PartFrame::apply` on the
  template's points reproduces the baked part's points **to the digit**, checked as a set comparison (the
  same multiset of coordinates, not the same order);
- the frame of a **jointed** part is its joint's: a canopy hung on a doorway's head has a frame whose
  origin is `hole.high` above the bay's sill, so instancing cannot drift from mounting;
- `PartFrame::apply` on the template's bounding box reproduces the baked part's bounds exactly;
- a placement that does not resolve has no frame, and the probe says so.

## Status log

- **2026-10-16** — worktree and branch from `d6ee447`; the decision above written.
- **2026-10-16** — **implemented and measured.** `parts.rs` gained `PartFrame { origin, along, up, inward }`
  with `apply` (one dot product per direction, no matrix) and `Template::mesh()` (the template expanded in
  its own frame); `expand.rs` gained `part_frame(skeleton, index) -> Result<Option<PartFrame>,
  Vec<Problem>>` — the frame resolved through the same `placed`/`offset_loop`/`WallFace` path the expansion
  used, with the wall's problems **propagated rather than swallowed** — and `expand_placement` was rewritten
  to `Template::mesh()` + `PartFrame::apply` and nothing else.

  Measured by `examples/instance_probe.rs` on a shed with **six** placements of one shutter plus a canopy
  hung on a bay's head:

  - the baked mesh holds **168** part faces and the template's own mesh holds **28** — six times fewer, for
    the same geometry;
  - **0 placements whose instanced points differ from the baked ones**, compared as point **sets**
    (rounded to `1e-4`) rather than by order — so the two paths are the same geometry, not two renderings
    that agree approximately;
  - **0 placements whose instanced box differs**, axis by axis to `1e-5`;
  - the mounted canopy's frame is its **joint's**: origin at `y 1.3000`, which is the bay's head `1.4000`
    less the socket's `0.10`, and the baked canopy runs `y 1.3000 .. 1.4000` — the origin plus the
    template's own box;
  - a placement whose slot has gone has **no frame** (`Ok(None)`), so a scene cannot instance a part the
    document cannot resolve.

  The existing acceptance is unchanged by the rewrite, which is the point of doing it as a *rewrite* rather
  than a parallel path: `part_probe` still reads `Part(0)` `0.026083` and `Part(1)` `0.045825`, and
  `socket_probe` still shows the jointed canopy exactly `0.0000` above a head that moves.

- **Next**: **closing this slice** — fold the record into the proposal (D4's mesh side is now expressible),
  delete this handoff, and let the branch be merged and removed.

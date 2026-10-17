# A hall out of the style

Started 2026-10-27. Branch `feature/building-hall` (worktree `.worktrees/building-hall`, from `main` at
`de865ab`). The design is [`agentic-building-generation.md`](../proposals/agentic-building-generation.md).

## What this slice is

The sixth building scene, and the first derived **from a style rather than from a hand-written `Starter`**:
`HALL` states its relations, and `HALL::starter`, `HALL::bays` and `HALL::glazing` answer for a 10 × 16 footprint.
Nothing in the scene chooses a wall thickness, a rise or a window's size.

- `crate::hall::document()` — `HALL.starter(10.0, 16.0, 1)`, the gable roof and the plates derived from it, the
  facade split into `HALL.bays(10.0)` = **3** bays a face, and one window a bay at `HALL.glazing`'s own inset.
- `crate::hall::hall()` — the frame: ground, sun, camera, and the identity channel from its first line
  (`outputs_field()`, `packed_where`, `stage_set` for the authored meshes and the plain ground).
- registered in `Gallery`, and **in `locate_in` and `id_in`** — the second registration this project has learned
  to make in the same breath as the first.

## Acceptance, measured

```
render hall out/hall
locate hall (240, 600): mass 0, outer wall over edge 2
where hall mass 0, outer wall over edge 2: id 3 covers 33184 pixel(s) in x 180..=442, y 526..=733
```

The picture: a long steep-gabled hall, three narrow tall windows on the gable end and the same rhythm down the
long side, a broad eave, and its own shadow on the ground — the style's own numbers, drawn.

## Status log

- **2026-10-27** — worktree and branch from `de865ab`; the scene, the registry, the two naming arms, and a camera
  pulled back twice until the building filled the frame without hiding the ground.
- Two process notes: a **single-quoted `` `n `` in a PowerShell replace writes the two characters**, not a
  newline — it happened again in `lib.rs` and in both `building_identity` arms and cost three repairs; and
  `cargo run … --example render` writing `out/hall.png` with an **unchanged sha** is how I caught that a camera
  edit had silently not applied.

# The identity channel as a debugging tool

Started 2026-10-27. Branch `feature/building-identify` (worktree `.worktrees/building-identify`, from `main`
at `d068c3c`, which closed the style slice). The design is
[`agentic-building-generation.md`](../proposals/agentic-building-generation.md). **This file and the proposal
are the whole record.**

## The gap this slice closes

The identity slice proved the join — a pixel carries an `authored_id`, and `locate(id)` names the element —
but it was a **proof rather than a tool**: `locate` knew one scene, and the render harness's `--locate x,y` flag
answered for the mountain side only. An agent debugging a frame could read the id image and nothing else.

## What is here

- **`building_identity::locate_in(scene, id)`** — names an `authored_id` for **any building scene the crate
  knows**, by rebuilding that scene's own document: `identity`, `shed`, `workshop`, `yard`. `locate(id)` is
  that call with the identity scene, kept because it is the scene the crate documents as its identity picture.
- **The scenes expose their documents** — `shed::document()`, `workshop::document()`,
  `yard::document()` — which is what makes naming possible at all: a name is a row of
  `Identity::of(built, 1)`, and the built mesh comes from the document.
- **`gallery::locate::locate` asks the building domain first** and falls through to the landscape's, so one
  function answers for both subsystems instead of one per subsystem.

## Invariants

- **A document is rebuilt rather than remembered**, so a number from a frame is named by the *same* document
  that drew it.
- **A scene the crate cannot name says so rather than guessing** — the harness's own words: *"this harness has
  no domain reading for `workshop` … a scene whose content does not name its own."*
- The yard's document is the **seated** one, because the seat is part of the picture.

## Acceptance, measured

```
cargo run -p whiting-device-rasterize --example render -- identity out/ident --locate 300,420
  locate identity (300, 420):
    authored_id          [4.0, 0.0, 0.0, 1.0]
    ->
  mass 0, outer wall over edge 3
```

and the refusal, from a scene that declares no channel:

```
render workshop out/ws --locate 300,420
  -> this harness has no domain reading for `workshop`. …
```

`building_identity_probe` still reads `id 3 → mass 0, outer wall over edge 2`, `0 → None`, `9999 → None`.

## What is not in this slice

**The channel is declared by one scene.** `identity` writes `authored_id`; `shed`, `workshop` and `yard` do not,
so the harness can name their ids only if they are made to declare the output — which means giving each of them
`outputs_field()` on the scene, `building_extras()` on every material's stage set, and `packed()` for the
meshes. That is a per-scene change and the next slice's subject.

## Status log

- **2026-10-27** — worktree and branch from `d068c3c`; `locate_in`, the three exposed documents, the harness's
  building branch, and the acceptance above.

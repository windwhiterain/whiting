# Scenes that draw instances

Started 2026-10-27. Branch `feature/building-instancing` (worktree `.worktrees/building-instancing`, from
`main` at `9d41aa0`, which closed the identity slice). The design is
[`agentic-building-generation.md`](../proposals/agentic-building-generation.md). **This file and the proposal
are the whole record.**

## The debt this slice pays

D4 says *"plan baked, vocabulary instanced"*. The crate honoured the **mesh** half in the instancing slice
(`Template::mesh()` plus `PartFrame`, with `build` rewritten on both so the two paths cannot diverge), and the
gallery never used it: every scene still draws whatever `build` baked, so a facade with five shutters uploads
five copies of the same twenty-eight faces. The proposal recorded exactly what was missing — *"one mesh per
template and one instance transform per placement"* — and this slice is the scene side of that sentence.

## What is here

`crates/gallery/src/instancing.rs`:

- **`standing(frame) -> (translation, rotation, scale)`** — the transform a scene instance carries, read from
  the frame's own axes: `yaw = atan2(inward.x, inward.z)`, translation = the frame's origin, unit scale.
- **`template_points(template)`** — one expansion in the template's own frame, as a packed mesh.

`crates/gallery/src/workshop.rs` — the scene changed in three ways, all of them consequences of the above:

- **five shutters** were added to the document, one per window, from **one** template;
- the shutters are drawn from `template_points` and stood five times by `standing`, not read off the baked
  mesh;
- the five windows were made one height (`SHUTTER_HEIGHT`), because one template behind five placements is
  the point and five heights would be five templates.

**The workshop had been drawing no parts at all** — its meshes filtered `Mass`, `Roof` and `Slab`, and a part
is `Element::Part`, so a canopy it used to carry was authored and invisible. This slice's shutters are the
first parts it draws.

## Invariants

- **A wall-mounted frame is a yaw about `y` and a translation, and nothing else.** Its `along` and `inward` lie
  in the plan and its `up` is world up, so a scene's instance transform carries it exactly.
- **The angle is read from the frame's own `inward`**, not from a wall's direction, so a second derivation
  cannot disagree with the first about which way a wall faces.
- **The template's points are in the template's frame** — the space the instance transform expects.

## Acceptance, measured

`examples/building_instancing_probe.rs`, six placements of one shutter:

| measurement | value |
| --- | --- |
| the upload | **168** baked part faces against **28** in the template — six copies, one upload; `84` points once instead of `84 × 6` |
| frame vs the scene's instance transform | **0** placements disagree, compared as point **sets** at `1e-4` |
| frame vs the baked geometry | **0** placements disagree |
| the rotation | one yaw per part (`-3.1416 rad` on the front wall, where `along` is `(-1, 0)`) |

The scene renders at **16 passes** (up from 13: the shutter mesh adds its three stages) and its own document
assert is clean. The gallery suite is **2 passed**.

## Status log

- **2026-10-27** — worktree and branch from `9d41aa0`; module, probe, the workshop's five shutters, and the
  measurements above.

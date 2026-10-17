# Two halls out of one style

Started 2026-10-27. Branch `feature/building-hall2` (worktree `.worktrees/building-hall2`, from `main` at
`61b77d9`). The design is [`agentic-building-generation.md`](../proposals/agentic-building-generation.md).

## What this slice is

The hall scene was one footprint and one camera. This slice makes the style **composable**: the scene is now
`hall_at(width, depth, storeys)`, and **the frame follows the building** — the camera's distance and height and the
sun's shadow page are derived from the ridge the style answers with, so a second size is a second picture of one
building rather than one picture with a bigger box in it.

Registered as two scenes from one file: `hall` (10 × 16) and `hall_large` (14 × 20). Both are in `locate_in`,
`id_in` and the probe's scene list.

## Acceptance, measured

| scene | rows in the identity table | the wall's edge 2 |
| --- | --- | --- |
| `hall` | 72 | id 3 covers **82,813** px |
| `hall_large` | **104** | id 3 covers **59,833** px |

**The style changed the building, not the scene**: `HALL::bays(10.0)` = 3 and `HALL::bays(14.0)` = **5**, so the
same relations give a gable end with three windows and one with five; the rise follows the span; the walls stay a
twenty-sixth of it. The probe reads both tables two-way.

## Status log

- **2026-10-27** — worktree and branch from `61b77d9`; `document_at`, `hall_at`, the two registered sizes, the
  derived camera and shadow page, and the two naming arms.

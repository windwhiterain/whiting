# What the erosion lattice puts in the picture

Answers one question: **what makes the eroded massif read as texel points — steps, dots or a grid —
and which of the candidate causes the measurements support.** Measured 2026-10-16 with
`cargo run -p whiting-gallery --example texel_probe`, which reads the landscape scene's own massif
onto the scene's own lattice, solves it, and measures the drawn surface. The tree moved while this
was written, so the solve is measured in both of its states: the committed **single-receiver D8**
and the working tree's **fan-out routing** (`erode::grid::fractions`, one share per lower neighbour
weighted by slope).

## The frame's scale, which is what a cell is measured against

The eye stands 2.55 radii back at a 0.72 rad field of view, which puts the frame's height at
**3496 world units** over the massif. One lattice cell is therefore:

| spacing | pixels per cell at 1080p |
| --- | --- |
| 12 units | 3.71 |
| 6 units | 1.85 |

Both are at or below the few-pixel scale where a lattice can hide. Nothing measured below is a
shader artefact: the massif's lighting stage is declared with an **empty surface body**
(`landscape/mod.rs`, `stage_set("massif", "", ...)`) and shades a flat grey by the interpolated
normal alone, so the picture is the geometry plus the normal field and nothing else.

## Cause 1 — the solve leaves cell-scale micro-relief (the root cause)

The authored mesh is planar per face, so its field's second difference is **exactly zero** on face
interiors (measured median 0.000). What the solve leaves, at six units:

| | single-receiver D8 | fan-out |
| --- | --- | --- |
| second difference median | 1.16 | **0.30** |
| second difference p90 | 10.14 | **2.32** |
| cell edges steeper than 45° | 41% | **37%** |
| interior sinks (dots) | 191 | **12** |
| shading grain, p99 between neighbouring vertices | 40.2° | **31.1°** |
| shading normal vs the mesh's own triangles, median | 9.0° | **2.0°** |

The fan-out routing attacks this at the source and is the largest single effect measured: the
micro-relief's p90 falls 4.4×, the interior dots 16×, and the normal statistics with them. The
residual grain is what is left for a stencil or a smoothing pass to argue over.

The micro-relief is **not grid-aligned in the way the fan-out's note claims**: with the fan-out the
second difference along the lattice axes against along the diagonals reads **0.61** (axes the
smoother direction), and under single-receiver D8 it read 0.94. Neither state shows the
axis-concentrated channels the note describes; the single-receiver state was isotropic noise, and
the fan-out state is diagonal-dominant. What the fan-out fixes is the *magnitude*, not an
alignment.

## Cause 2 — the normal stencil chooses a baseline; it cannot remove the grain

Grain is the angle between the shading normals of neighbouring vertices; mismatch is the angle
between the shading normal and the mesh's own triangle normals. At six units, on the fan-out field:

| stencil | grain median | grain p90 | grain p99 | mismatch median | mismatch p99 |
| --- | --- | --- | --- | --- | --- |
| one cell (committed) | 2.1° | 12.8° | 43.8° | 1.2° | 25.7° |
| two cells (working tree) | 1.9° | 9.0° | 31.1° | 2.0° | 33.7° |

The two-cell span trades a little geometry agreement for a little less grain — a much smaller trade
than it was before the fan-out, because there is far less micro-relief left to disagree about. On
the single-receiver field the same trade was severe (grain p99 66° → 40°, mismatch p99 67° → 77°).
The lesson generalises: **stencil width is a second-order knob; what the solve leaves in the field
is first-order.**

## Cause 3 — the rim's one-sided normal is sign-inverted (a bug)

`Erosion::normal_at` (`crates/gallery/src/landscape/eroded.rs`) falls back to a one-sided
difference at the footprint's boundary, and the **ahead-only branch has the wrong sign**:
`(Some(a), None) => (here - a) / cell`, where the difference from `here` to the neighbour ahead is
`(a - here) / cell`. The behind-only branch beside it is correct. Measured on the fan-out field:
1003 vertex-axes take the ahead-only branch at six units (499 at twelve), worst slope error
**39.1 per unit** — a near-horizontal normal pointing the wrong way.

The vertices affected are the west and north half of the mesh's outer ring, where the massif rises
from its foot; their normals tilt outward instead of inward. On this scene's high sun (0.85 rad)
that reads as a dark band along two sides of the footprint — at the one place the frame is composed
around. The working tree's stencil rewrite kept the branch, so the bug survives both solver states
and both spacings. It is the cheapest certain fix in this note.

## Cause 4 — interior sinks are dots, and the spacing decides how many

Interior sinks on the drawn surface: **5 at twelve, 12 at six** under the fan-out (11 and 191 under
single-receiver D8; the raw solve's own counts are in `erode_probe`). Each is a cell-scale cone the
mesh must draw, and with the normal averaged over the pit each reads as a bright or dark dot two to
four pixels across. A finer lattice resolves more of them, not fewer, because the depressions are
one to two cells wide — unless the routing stops making them, which the fan-out largely does.

## What is not a cause

- **The identity outputs.** `authored_id` is `@interpolate(flat)` and `authored_u`/`authored_v` are
  per-cell constants (`Erosion::lattice`), so the sidecar images are blocky at the cell — by
  design, and documented as such. They are separate `r32float` attachments; the frame's shader
  never reads them.
- **The `texture` slot.** The lattice writes `[0.0, 0.0]` and the massif's lighting body is empty,
  so nothing reads it.
- **Interpolation of the field.** `HeightField::sample` is bilinear over the four surrounding
  cells, which is what the mesh's geometry already is; bicubic would ring on cell-scale noise
  rather than remove it.
- **The winding.** The lattice's `[0, 3, 1], [1, 3, 2]` is `geometry::heightfield`'s, and the
  geometric normal of every drawn triangle has a positive `y` by construction — no backfaces.

## Ranked by impact against cost

1. **Fix the rim sign inversion** — one line in `normal_at`'s ahead-only branch. Removes a
   wrong-shaded band along half the footprint, in every state measured. Certain, free.
2. **Keep the fan-out routing** — the largest effect measured (cause 1: p90 micro-relief 10.1 →
   2.3, interior dots 191 → 12, grain p99 40° → 31°, all at six units). Nothing to do but not lose
   it; if it is ever reverted, every number in this note goes back with it.
3. **Six units is now affordable.** With the fan-out the grain's p99 is 31° and there are a dozen
   interior dots, against 3.71-pixel cells and visible steps at twelve. The spacing is a trade, not
   a fix, and the trade currently favours six.
4. **Fill the interior sinks** — one priority-flood pass before `with_surface`, or an outlet rule
   for interior pits. Worth a dozen dots at most now; the books already spill at rim outlets and an
   interior pit keeping its load is the lake rule, but a lake one cell wide is a dot, not a lake.
5. **Smooth the drawn surface before meshing** — a few laplacian passes masked to the covered
   cells. Still the only change that moves geometry and normals together, but the residual
   micro-relief it would remove is now p90 2.3 units at six, and the risk of smoothing the
   channels (tens of cells) is unchanged. Measure before committing.
6. **Not recommended:** per-triangle geometric normals (the mismatch goes to zero and the grain
   stays, because the grain is the geometry's), and bicubic drawing (rings on cell-scale noise).

# Mountain Visual Optimization

Branch `feature/mountain-visual-opt`, worktree `.worktrees/mountain-visual-opt`.
Renders are produced with
`cargo run -p whiting-device-rasterize --example render -- <scene> out/<name>`
from the worktree with `CARGO_TARGET_DIR` set to the worktree's own `target`.

## Goal

Fix the rendering artifacts of the mountain series and improve the framework where the
artifacts lived:

1. the Hermite surface massif's black blocks;
2. the erosion lattice's texel point artifacts;
3. general visual quality of the mountain scenes and the shared machinery they draw with.

## What is committed and verified in a render

- `5207087` **Hermite tangent seed** (`crates/patch`): `seed_derivatives` seeded half a *unit*
  direction at every vertex of every mesh; the boundary curve's end tangent is `du * step`, so
  against chords of forty the curves left every corner flat (faceted plates) and at boundary
  vertices the one-sided tangent pointed back into the patch, looping every boundary row and
  column of every patch. Now each edge contributes half its own vector in step units, oriented
  by the parameter direction read from the face slot, at both ends.
- `9060235` **belt valley rows** (`crates/gallery/src/blockmesh.rs`): valley rows were shared by
  vertex id (one `dv` serving two flanks) and `push_quads` dropped thin cells, each drop a hole
  showing the lit inside of the rock behind it — which is what the black diamonds of the old
  renders mostly were (the old "the backdrop shows through nothing" evidence was vacuous: the
  backdrop stood past the far plane and never drew). Rows are now duplicated per belt at
  coincident arclength stations; a cell is dropped only when two corners are one place.
- `ea267e7` **hermite scene** (`crates/gallery/src/hermite_massif.rs`): the far plane was
  `1450.0` outright while the eye stands some 4600 units back, so the whole scene rendered as
  bare sky at the branch tip — a clipped scene and an empty one are the same pixels. It is
  eight radii now. Normals and windings turn to the `+y` hemisphere (the old per-cell corner
  cross product follows the patch frame handedness and shaded whole belts by ambient alone);
  each cell's diagonal is tested by the triangles it emits (the previous per-cell diagonal was
  inverted, which is why its own commit recorded that it changed nothing); `SAMPLES` 4 → 6.
- `cc3bc65` **erosion drawing** (`crates/gallery/src/landscape/eroded.rs`): `CELL` 12 → 6 (the
  three-pixel steps were the texel artifact), the drawn normal reads a two-cell span where the
  field has one, and the one-sided ahead read had its sign inverted (rim normals downhill on
  two sides of the footprint).
- `1c3e4ed` + `3fdf578` **multiple flow direction** (`crates/terrain`): single-receiver D8
  routing concentrated runoff on grid-aligned paths — the dotted diagonal rills. Each cell now
  hands water and load to every lower neighbour in slope shares; mass conservation doc-test
  passes. Verified in `out/land1.png` / `out/land2.png`: rills gone, flanks read as drainage.
- `8a23f8c` **pit fill** (`crates/terrain/src/heightfield.rs`): `fill_pits`, one priority-flood
  heap sweep, raises interior pits to their spill level on the drawn surface; the solve and its
  books stay unfilled. Verified in `out/land2.png`.
- `8174f1c` handoff, `texel_probe` example and the investigate note
  `.agents/investigate/what-the-erosion-lattice-puts-in-the-picture.md`.

## In flight (subagents, this session)

- **Hermite seam flaps.** After all of the above the hermite massif still shows one-cell dark
  diamonds *on the valley seam lines* (stacked chevrons down the front valley, isolated
  diamonds on side valleys). Established: front-facing geometry (backfaces are culled; genuine
  holes in the same frame show sky), belonging to a different patch face than the surface
  around them (false-colour render with the face id in the texture slot), i.e. flaps of one
  belt's patch poking through the neighbour at the seam, or seam cells tilted hard away from
  the sun. Ruled out: shadows (Shadowing stage removed, pixel-identical), sliver holes, fold
  census skips, diagonal choice, global orientation, far plane. A subagent is hunting it with
  `BELT_DUMP` grid dumps in `blockmesh.rs` (to be removed before its commit) and
  `examples/seam_probe.rs`.
- **Sky lattice blocks.** The landscape sky showed axis-aligned hard-edged rectangles; a
  subagent is fixing the term responsible in `crates/gallery/src/landscape/mod.rs` (its WIP
  already renders clean in `out/land2.png`).

## Known issues not being fixed here

- **Range junction gash.** Where the second range's lattice crosses the first's flank
  (`out/land2.png`, left of the main massif) a dark gash and a thin spike show: the two ranges'
  lattices interpenetrate and the buried rim staircase of the nearer one notches the crossing
  curve. Pre-existing composition artifact of drawing two independent lattices, not one of the
  two named artifacts; clipping each lattice to where it is the uppermost surface would fix it
  and is not attempted.
- The hermite massif's valleys are creases by construction (coincident vertices, private
  derivative fields); a caller wanting a soft valley has no join to tag any more and would ask
  `patch` for one.

## Merge state

Checkpoint 1 merged to `main` at `8174f1c`. Merge again when the two in-flight fixes commit.

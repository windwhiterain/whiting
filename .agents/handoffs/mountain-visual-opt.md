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

## Sky lattice blocks

Commit `d49d3f9` "Draw the sky's weather from gradient noise, so its lattice stops showing", on
`feature/mountain-visual-opt`. File: `crates/gallery/src/landscape/mod.rs`, the `NOISE` const only.

**Guilty term: the cirrus mix, and only it.** Two A/B renders, both from the same build:

- `out/ab-nocirrus.png` — the cirrus `mix` weight set to zero, everything else as written. Every
  rectangle in the upper half of the frame is gone; the sky is the painted ranges and nothing else.
- `out/ab-cirrus-only.png` — the `ridges(...)` call and the shaft add zeroed, the cirrus left as
  written. Every rectangle is back. (The black sky in that frame is the zeroed `ridges` call and
  nothing else; it is a diagnostic, not a defect.)

So `crate::ridges::RIDGES` and the crepuscular shafts are not implicated. That is also what their
maths says: `ridge_noise` is a function of the ray's azimuth alone and the shafts are a function
of the sweep angle alone, so neither can draw an axis-aligned box whatever it is multiplied by.

**Cause.** The cirrus is `smoothstep(0.56, 0.82, ...)` over `noise(...)`, so the window's *input*
is drawn on the noise's lattice and the window turns every kink in that field into an edge. The
field was value noise — corner **values** interpolated bilinearly, faded with the cubic
`3t²-2t³`. It is continuous across a cell boundary and its slope is not, so each cell's iso-line
has a corner on the boundary; thresholded, that corner is a hard edge running along the lattice.
The deck projection `ray.xz / max(abs(ray.y), 0.16)` then stretches one cell until it spans a
hundred pixels, which is why the plates are large and the artifact worst in the upper half, where
the deck coordinate is smallest and the cells are biggest. A second, weaker cause sits under it:
`hash` was `fract(sin(dot(point, vec2(127.1, 311.7))) * 43758.5453)`, and a cell coordinate is the
deck coordinate times an octave frequency and (now) a turn, so the dot products reach tens of
thousands — past about `1e4` an `f32` keeps only the leading bits of the sine's argument and the
hash returns flat plates of its own.

**Fix.** Gradient noise under a **quintic** fade, `6t^5 - 15t^4 + 10t^3`, which arrives at a cell
boundary with its first *and* second derivative at zero, so the same window draws a cloud edge
rather than a seam. Corner seeds come from an integer scramble (`scramble`, a Wang/murmur style
finalizer) instead of a sine hash, which is exact at every magnitude the sky reaches. Each octave
**turns** the domain about 40 degrees (`OCTAVE_TURN`) so the finer octaves do not put their seams
on the coarser ones'. `noise` now returns `total * 1.14 + 0.5`: gradient noise sums around zero
where value noise summed around one half, so the centring is explicit rather than left to the
range of the octaves. Cost is one integer scramble, one `normalize` and four dot products per
octave — about the same as the four `sin`-based corner hashes it replaces, and cheaper on the sky's
coordinates.

**Scope.** `NOISE` is a private const in `landscape/mod.rs` and is spliced into this scene's
shaders only. `terraces.rs`, `stilllife.rs` and `foldyard.rs` each carry their own `NOISE_WGSL`
and `ridges.rs` carries `ridge_noise`; none of them read this one, so no other scene's authored
composition can have moved. `hash` and `value_noise` were removed rather than kept beside
`gradient_noise`: they had no reader outside the `noise` they fed. `crates/terrain`, `crates/patch`,
`blockmesh.rs` and `hermite_massif.rs` were not touched.

**Verified.** `out/sky1.png` (before) against `out/sky-fixed.png` (after), 1024x1024, same build:

- Ground, `y >= 700`: **2 of 331776 pixels differ, by 1 of 255 in one channel** — last-bit
  rounding in the shared post pass, not a change in the picture. Massif and plain are unchanged.
- Sky, `y < 700`: 275597 pixels differ, max channel delta 60 — the cirrus, as intended.
- Hard edges, measured as the largest single-pixel luminance step in the band above the painted
  ranges, `y = 0..400`: horizontal `|dx|` max **21.5 → 1.9** of 255, and the count of steps over
  4/255 goes **624 → 0**. There is no horizontal hard edge left anywhere in that band. Vertical
  `|dy|` max drops 22.3 → 6.1, and the surviving steps are the wisps' own edges, which the 2x and
  3x crops show spread over ten to twenty pixels.
- A 2x crop of the same 300x200 patch (`out/crop-before.png`, `out/crop-after.png`) is the
  clearest read: flat plates with hard boundaries before, a continuous wisp after.
- `cargo build -p whiting-gallery` prints no warning from the file. `rustfmt --edition 2024
  --check` is clean and the file keeps CRLF throughout (1152 CRLF, 0 bare LF).

**Cost.** Nothing re-rendered against another scene, because nothing outside this one moved. The
sine hash in `ridges.rs`, `terraces.rs`, `stilllife.rs` and `foldyard.rs` has the same weakness at
large coordinates; none of those is drawn at a coordinate anywhere near `1e4`, and changing them
would move an authored composition, so they are left as they are.


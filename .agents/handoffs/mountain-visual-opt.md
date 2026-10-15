# Mountain Visual Optimization

## Goal

Fix rendering artifacts across the mountain series:
1. Hermite surface black diamond holes along ridge lines
2. Erosion lattice visible grid/texel pattern on mountain surface
3. General visual quality improvements

## Status

### Hermite black blocks

Root cause identified from render inspection (`out/hermite_massif.png`):
- Black diamond-shaped holes appear along ridge lines and valleys
- The `tessellate()` function in `hermite_massif.rs` already handles normal flipping and diagonal selection
- Remaining issue: when `agree(0,1,2,3)` returns false, the fallback diagonal `(0,1,2),(0,2,3)` is used without verifying it also produces agreeing triangles
- Some cells may have BOTH diagonals producing disagreeing triangles — these should be dropped entirely rather than emitting a broken cell
- The `facing` cross product used as reference for `turned()` can itself be near-degenerate for collapsed cells

### Erosion texel artifacts

Two separate reads on the same render, both kept:

1. **Dotted diagonal rills (CODE DONE, RENDER NOT RE-CHECKED).** `out/erode-lattice.png` shows
   channels running exactly along lattice diagonals, reading as rows of dots or dashes rather
   than drainage. Cause confirmed in `crates/terrain/src/erode/grid.rs`: `receiver()` is
   single-flow (D8) routing — every cell donates all its water to the one steepest neighbour, so
   flow concentrates on grid-aligned paths. Fix: multiple flow direction — each cell hands its
   water and load to **every** lower neighbour, in shares of `slope^p` (p = 1.0 default,
   normalised to sum to one). `receiver()` itself stays (the `erode_probe` example reads it
   to count sinks, and "no steepest receiver" is still exactly "no lower neighbour").
2. **Rectangular grid / faceting (earlier analysis, below).**
   - Visible rectangular grid pattern on mountain flanks
   - CELL=12.0 creates ~85x85 cells; each cell is 2 triangles with per-vertex normals from central differences
   - Normals change discontinuously at cell boundaries because each vertex's normal is computed independently
   - The height field's bilinear interpolation is smooth but the normal computation (finite differences) creates faceted appearance

## Plan

1. Fix Hermite: verify both diagonals, drop cells where neither works
2. Fix Hermite: increase SAMPLES from 4 to 6 for finer tessellation
3. Fix erosion: widen normal stencil or use Sobel-like operator for smoother normals
4. Fix erosion: reduce CELL from 12.0 to 8.0 for finer resolution
5. Improve sky noise sampling in landscape post-processing

## Erosion MFD

Commit `3fdf578` "Route a cell's water and load to every lower neighbour, in shares", on
`feature/mountain-visual-opt` after `1c3e4ed`. Files: `crates/terrain/src/erode/{mod,grid,order}.rs`.
Nothing outside `crates/terrain` is in it.

**Was.** Each round read a field and computed one `grid::receiver` per cell — the steepest
distance-normalised drop, first strict maximum in the fixed `D8` order winning ties. The
accumulation swept the cells in descending height and handed the whole cell's water and its
`carry` share of standing load to that one neighbour. Single flow concentrates every cell's
runoff onto grid-aligned paths, which is the dotted diagonal rills above.

**Is.** The same round, routed by `grid::fractions(field, heights, at, FLOW_EXPONENT)` with
`FLOW_EXPONENT = 1.0`: every neighbour strictly below the cell takes a share of its
distance-normalised drop, and the shares are normalised to sum to one. `Scratch` now carries
two `Vec<[f32; 8]>` share buffers (`routing`, `computed`) in place of the two
`Vec<Option<usize>>` receiver buffers, and the descending visit splits both the water and the
carried load over the slots, `water[to] += share * water[at]` (still clamped by `WATER_MAX`)
and `moving = carried * share` subtracted from the source and added to the destination slot
by slot. Details that matter:

- **The shares are the previous round's.** Computed at the top of the round from the heights
  as they stand, swapped in only after the accumulation, so the round is still read ->
  accumulate -> exchange and no cell hands anything to a neighbour whose height it had not
  seen. Slots are read in `D8` order, and `grid::neighbor` is re-resolved per slot because
  the mask and the lattice do not move under erosion.
- **Conservative across a fan-out, not just across a hop.** The source is decremented by
  exactly what each destination was credited, so the `f32` residue of shares summing to 1 is
  left behind rather than invented or deleted; subtracting a single `carried` would have
  leaked at that order of magnitude. `carried` keeps the old clamp to `[0, sediment[at]]`.
- **A pit is a lake.** All-zero shares means nothing leaves the cell, which is what
  `grid::is_sink` (new, `pub(crate)`) answers for the outlet spill test that used to read
  `routing[at].is_none()`.

**API.** Unchanged: `erode_traced`, `ErodeParams`, `Hydraulic`, `Books`, `D8`, `INV_STEP`,
`delta_of`, and `grid::receiver` all keep their signatures and meanings, so
`crates/gallery/examples/erode_probe.rs` still reads `receiver` to count sinks. No field was
added to `ErodeParams` — the exponent is a private constant of the scheme, not a knob, so no
caller has to fill a new field in and `ErodeParams::default()` is untouched.

**Verified.** `cargo test -p whiting-terrain -j 2 --doc` passes (1 passed, 0 failed — the
mass-conservation assertion) after a `cargo clean -p whiting-terrain`, and
`cargo build -p whiting-gallery --example erode_probe -j 2` relinks, both naming this worktree
in every `Compiling` line. `cargo doc --no-deps` adds no new warning from the touched files
(the three it prints are pre-existing, in `lib.rs` and `heightfield.rs`).

**Not done.** Nothing re-rendered: the probe was built and not run, per the brief, so the
dotted rills are fixed in the solver and still unconfirmed in a picture. `erode-lattice.png`
is the file to look at next.

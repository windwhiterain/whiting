# Fitting a grid to a block whose uv box is not its own

Date measured: 2026-10-15. Sources: crates/gallery/examples/block_surface_probe.rs on
feature/hermite-surface, massif starter(11, Starter::default()).

Moved here from the repository root, where a subagent had left it.
# Why `blockmesh`'s grid runs off the rock — measurement notes

Worktree `C:\resource\whiting\.worktrees\fitfix`, branch `fix/block-sampling`, based on
`feature/hermite-surface` at `fbf3d54`. Every number below is copied from
`cargo run -q -p whiting-gallery --example block_surface_probe -j 1` run in this worktree;
the raw output is committed beside this file as `notes-before.txt` and `notes-after-final.txt`.

## 0. The brief's paths were wrong in two ways

- **The worktree `C:\resource\whiting\.worktrees\fitfix` did not exist when this session
  started.** It appeared between my first two commands, created by another session, on branch
  `fix/block-sampling` at `fbf3d54`. I worked there and did not create or re-point anything.
- **`surface_from_faces` is not on `main`.** It exists only on `feature/hermite-surface`.
  `main`'s `crates/gallery/src/blockmesh.rs` has `sample_block` and `nearest_vertex` but no
  `surface_from_faces`. The defect is on `main` too, in an older shape, so the fix will need
  forward-porting.

## 1. The brief's hypothesis is wrong as stated, and the bilinear reference says why

The brief proposes `nearest_vertex` as the cause. To test it I added a **bilinear reference** to
the probe: the average of a cell's own four corners. A bicubic Hermite patch whose corner
derivatives are central differences and whose twist is zero evaluates **at its centre** to exactly
that average, so the two numbers say how much the Hermite representation costs as opposed to how
much the corners cost.

| side | departure (Hermite) | bilinear reference |
|---|---|---|
| 2 | 1.527e-6 | 1.527e-6 |
| 3 | 1.710e-1 | 1.710e-1 |
| 4 | 1.727e-1 | 1.942e-1 |
| 6 | 4.933e0 | 4.747e0 |
| 8 | 4.099e0 | 3.931e0 |

They agree to three significant figures, at every resolution, on the untouched blocks as well
(`1.437e-6`/`1.437e-6`, `4.152e-6`/`4.115e-6`). **The Hermite patch is already the best C1 surface
through its own four corners. The front end's corner placement is the whole problem**, which
confirms the brief's "the Hermite surface itself is not the problem" — but for a stronger reason
than the one given.

The derivative seeding is fine, incidentally, and I checked why: `seed_derivatives` *assigns*
rather than accumulates, so an interior vertex's `du` is whichever incident `u`-edge is last in
`BTreeMap` order. On this grid that is safe, because edges are stored in ascending vertex id and
the grid's id grows along both axes, so both incident `u`-edges point the same way. Worth knowing
if a grid with a non-monotone id order ever meets it.

## 2. The fallback is a real contributor, but not the 10^5

Splitting cells by whether their block had *any* corner the sampler could not place:

| side | untouched cells: median / p90 / max | snapped cells: median / p90 / max |
|---|---|---|
| 2 | 1.437e-6 / 2.839e-6 / 2.296e0 | 1.713e-6 / 9.939e-1 / 2.765e0 |
| 3 | 1.182e-1 / 1.653e-1 / 1.607e0 | 3.625e-1 / 5.402e0 / 9.668e0 |
| 4 | 1.937e-6 / 4.462e-6 / 1.624e0 | 1.102e-1 / 2.648e0 / 6.562e0 |

Snapped blocks are 10–30x worse at the median and up to 6x worse at the worst. That is a genuine
defect. It is **not** the whole 10^5, because blocks that never used the fallback are also far from
the floor at sides 3, 6 and 8.

## 3. Two hypotheses I measured and dropped

- **`(u, v)` folds.** If the parameterization were many-to-one, `sample`'s average over covering
  triangles would be a point on neither sheet. Measured per cell over the centre and all four
  corners: 11 of 1637 cells at side 4, 1 of 4519 at side 6, 18 of 8854 at side 8. Effectively
  single-valued. **Dropped.**
- **Grid non-uniformity.** `HermiteSurface::build` gives every vertex one step per direction.
  Median `step_spread` is 1.30, 1.53, 1.57, 6.91, 9.69. Splitting on `spread <= 2` does not
  separate good blocks from bad ones. **Dropped as the cause.**

## 4. What is left, precisely

After the fix in section 5, the residual is entirely in blocks that **never used the fallback** —
their corners all lie on the rock — and their bilinear reference departs by the same amount:

| side | median over untouched cells | bilinear reference, same cells |
|---|---|---|
| 3 | 1.182e-1 | 1.354e-1 |
| 6 | 7.294e-2 | 2.451e-1 |
| 8 | 5.838e-2 | 1.559e-1 |

So: four corners genuinely on a rock that is piecewise linear, and the rock between them is not
bilinear. A cell that lies inside one triangle is reproduced exactly (that is where the `1.4e-6`
readings come from — departure is at the `f32` floor, not above it); a cell straddling a triangle
edge is not, and refining does not remove that, it only shrinks it.

This is a statement about the **domain**, not the sampler: a uniform grid over the block's `uv`
bounding box is a grid over a *rectangle*, while the block's triangles cover a ragged subset of it.
`Face::uv`'s own documentation makes `u` the fraction of the *belt's* upper boundary walked, so
that raggedness is structural. The existing `from_belts` path is the designed answer to exactly
this, and its doc comment says so; on this massif it was measured not to help, which is worth
re-testing against this diagnosis rather than taken as settled.

## 5. The change, and the trap in it

`sample_block` no longer invents a position. A parameter point no triangle covers is **projected
onto the nearest point of the region the block does cover**, and is then moved a thousandth of the
way into that triangle's centroid.

Two things went wrong on the way, both recorded because both were silent passes at first:

1. **The projection landed on a triangle edge**, where `f32` rounding puts it just outside
   `barycentric`'s `-1e-9` tolerance, which reads as "nothing covers this" and takes the **whole
   block** with it: 95 of 182 blocks survived at side 4 and **1** at side 8, and the probe printed
   `all measurements pass` while discarding four fifths of the massif. Moving the point inside is
   the fix; the guard below is the belt-and-braces.
2. **Collapsed cells.** Two corners can project to the same point, and `HermiteSurface::build`
   refuses a quad with a repeated corner. Such a cell is now left out — **only that cell**, never
   the block.

All 182 blocks build at every resolution and the patch counts are unchanged
(182 / 728 / 1638 / 4550 / 8918), which is the check that catches both traps.

## 6. Result

| side | patches | before | after |
|---|---|---|---|
| 2 | 182 | 1.527e-6 | 2.687e-6 |
| 3 | 728 | 1.710e-1 | 1.361e-1 |
| 4 | 1638 | 1.727e-1 | **1.799e-3** |
| 6 | 4550 | 4.933e0 | **2.329e-1** |
| 8 | 8918 | 4.099e0 | **2.288e-1** |

Worst departure over all blocks at side 4: 6.562e0 -> 2.088e0.

**The probe still exits 1** at sides 3, 6 and 8, against `floor * 100 = 1.526e-3`. Side 4 passes.
Section 4 says what is left and that it is not a sampler defect.

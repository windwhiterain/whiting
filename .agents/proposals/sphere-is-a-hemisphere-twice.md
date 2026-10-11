# `sphere` builds a hemisphere, twice, and has never been a sphere

> Found while bringing the gallery into line with the winding convention in
> [SPEC/render/offline_backend.md](../SPEC/render/offline_backend.md#winding). The winding fix
> made the defect visible rather than causing it. **Not fixed there**, because every fix moves
> every rasterised frame in the gallery and that is a different decision.

## What it does

`crates/gallery/src/geometry.rs:356`:

```rust
let vertex = |theta: f64, phi: f64| -> ([f64; 3], [f64; 2]) {
    let normal = [theta.sin() * phi.cos(), phi.sin(), theta.cos() * phi.cos()];
    (scale(normal, radius), [theta / TAU, phi / PI])
};
```

with `phi` swept from `0` to `pi` across `divisions` rows.

Two facts fall out of that one line, and neither is a matter of taste.

**`sin(phi)` is never negative over `[0, pi]`, so no point of the mesh is below the equator.**
Measured on `sphere(10, 1.0)`: all 600 corners have `y >= 0`. The function documents itself as
*"Builds a sphere"*, with the invariant *"the normal of a point is the direction it sits in from
the centre, so a sphere lit by its own normals is round rather than faceted"* — and half of it
is a hemisphere.

**`cos(phi)` changes sign at `pi / 2`, so the second half of the sweep retraces the first with
`theta` shifted by a half turn.** Same measurement: `sphere(10, 1.0)` emits **600 corners at 51
distinct positions**, and the 60 corners that lie on the equator are only **10 distinct points** —
the rim is written six times over.

So the mesh is one hemisphere drawn twice, with a degenerate band at the pole between the two
copies. That is where the 800 same-direction edges in the survey's coherence column come from,
and it is why `monoliths` is 40 copies of an 800-triangle mesh with only 288 distinct vertices
welded.

## What it costs

- **Twice the triangles for the same picture.** A closed `sphere(20, r)` is 800 triangles; this
  one is too — but half of them land on a surface that is already there, and they shade it.
- **A pole band that carries no orientation.** The row whose ring is `phi = pi / 2` has four
  coincident corners per cell, so its second triangle has two identical corners. Measured on
  `monoliths`: those triangles sit at an area of **3.7e-19** where the mesh's smallest real face
  is **1e-3**.
- **A figure nobody chose.** `terraces` calls `sphere(10, 1.0)` for a fir's `crown` and
  `stilllife` calls `sphere(36, ·)` for a `lemon`. A lemon is not a hemisphere drawn twice, and
  neither is a crown a reason to spend 2592 triangles on.

## What the fix is

Use `phi` as the polar angle from `+y` rather than as the elevation, and drop one of the two
sweeps:

```rust
let vertex = |theta: f64, phi: f64| -> ([f64; 3], [f64; 2]) {
    let normal = [theta.sin() * phi.cos(), phi.cos(), theta.sin() * phi.sin()];
    ...
};
```

For that map `dP/dtheta x dP/dphi = sin(phi) * n`, which is outward for every `phi` in `[0, pi]`
— so the grid becomes uniformly wound, the pole collapses into one genuine pole, and the winding
follows the normal without a conditional anywhere. (This is the same derivation the winding fix
uses, and it is why the current code needs `cos(phi)` tested at all: the current map gives
`cos(phi) * n`.)

**This changes positions**, so it changes the rasterised frames in `monoliths`, `terraces` and
`stilllife`, and those are the frames the agreement numbers are measured against. That is the
whole reason it is a proposal and not part of the winding change: re-measuring three scenes'
baselines and re-deriving their correlations is its own piece of work, and it should be done on
purpose rather than as a side effect.

## The cheaper half, if the baseline is not worth re-measuring

Dropping the second sweep alone — `phi` over `[0, pi / 2]` — halves the triangle count and
removes the duplicated rim, and needs no parameterisation change. It leaves a hemisphere, which
is what three of the four call sites may well have wanted in the first place. Which of the two is
right depends on what `sphere`'s four callers want, and that is a question for the scenes rather
than for this document.

## Sources

- `out/probe-orientation.py`, over `out/w1-terraces.cycles.xml`: 600 corners, 51 distinct
  positions, 60 equatorial corners at 10 distinct points, none below `y = 0`.
- The same probe's `collapsed` column: 1 600 triangles over `monoliths` and 680 over `stilllife`
  below `1e-12` of the mesh's squared bounding-box diagonal.
- `crates/gallery/src/geometry.rs:356` and the four call sites: `monoliths.rs:174`,
  `terraces.rs:306`, `stilllife.rs:315`, `stilllife.rs:535`.
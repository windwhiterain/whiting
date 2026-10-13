# Every sky agrees now, and the ground does not

Measured 2026-10-12, every gallery scene that renders on both backends, raster against Cycles at
32 samples on CPU.

## The question

The chapter's numbers are pearson correlations of luma between two frames that do not hold the same
kind of thing: the tracer writes scene-linear radiance into the file and the rasterised post stage
tone-maps before writing. A correlation is also blind to a gain. Having found both faults, what is
left, across the whole gallery rather than one scene?

## The method

The raster frame's tone curve is inverted — exposure, white balance, ACES or Reinhard, gamma,
saturation, S curve, vignette, in reverse order — which recovers the radiance the post stage was
handed. That and the tracer's linear file are the same kind of thing and can be compared directly.

**The statistic is a ratio of medians, not of means.** Reinhard maps every radiance above a few
units to within a thousandth of one, so one near-white pixel moves a mean further than the rest of
the frame does and the mean says nothing about the level. `lanterns` would have read 55412 for this
reason alone.

## What it says

| scene | whole | sky | lower 62% | ground |
|---|---|---|---|---|
| `dunes` | 1.030 | **1.001** | 1.047 | 1.048 |
| `landscape` | 1.125 | **1.040** | 0.848 | 0.948 |
| `tidewater` | 0.920 | **1.059** | 0.769 | 0.699 |
| `foldyard` | 0.342 | **1.085** | 0.384 | 0.274 |
| `sunlight` | 0.850 | both black | 0.959 | 0.957 |
| `spotlight` | median 0 | both black | 1.177 | 0.971 |
| `tiles` | 1.184 | both black | 1.134 | 1.285 |
| `lanterns` | not comparable — see below | | | |

**corrected 2026-10-13, `foldyard`, re-measured after the albedo was applied once.** the table
above read the ground at `0.274` and the whole frame at `0.342`; the cause was the scene's ground
body multiplying by `in.tint` a second time, which made the rasterised ground the tint squared
against a tracer that is handed the tint once. it now reads:

| scene | whole | sky | lower 62% | ground |
|---|---|---|---|---|
| `foldyard`, after | **1.042** | 1.089 | 0.963 | **0.982** |

so the worst whole-frame scene in the table above became the best matched, and the only reason to
look at this table again is that its other rows may hide the same shape of mistake.

**Where both backends have a sky, they now hold the same radiance.** The four scenes that declare
an environment light read `1.001`, `1.040`, `1.059` and `1.085` — from a scene that was at `0.111`
and one that was at `2.817` an hour earlier. The four that declare none have a black sky on **both**
sides, which is the correct agreement: the tracer draws black for a scene with no world, and so
does the raster post stage once it stops painting one of its own.

**The ground does not follow, and two scenes are the reason.** `foldyard` at 0.274 and `tidewater`
at 0.699 are the two that paint aerial perspective in their **post** stage — which the SPEC permits
and requires, having moved atmosphere out of the surface bodies. A path tracer draws no fog unless
it is given a volume, and the exporter carries none, so a scene that paints haze in its grade has
painted something the tracer cannot have. That is the same class of difference as a painted ridge
silhouette, sanctioned by a decision that is otherwise right.

## The two that are not comparable

- **`lanterns`** writes a frame with no shaded radiance in it: its post stage replaces the colour
  outright with two night-sky literals and a vignette. There is nothing left to invert, and its
  earlier correlation of +0.8571 is a statement about a night sky, not about shading.
- **`spotlight`** has a black median over the whole frame — more than half its pixels are zero — so
  its whole-frame ratio is undefined. Its ground reads 0.971.

## What is still open, and it is not a units error any more

- ~~`tiles` at 1.285 on the ground is the largest remaining ground figure and is **not**
  explained.~~ **Explained 2026-10-12**: the two backends are drawing two different surfaces. The
  exporter drops the material's `image` and the mesh's `texture` attribute, so the tracer draws a
  flat floor, and the ratio is a texture's brightness against a plain tint. The exporter is right to
  refuse and right to report it — the target format will not accept a colour connection at all, and
  that is now measured rather than asserted:
  [a-material-image-cannot-be-carried.md](a-material-image-cannot-be-carried.md).
- `foldyard` and `tidewater` need either volume fog in the export or an acceptance that a graded
  scene and a traced one differ by the grade. **There is no route for the first**: measured
  2026-10-12, `volume_absorption` and `volume_scatter` are not registered in this build and
  `principled_volume` leaves a 360000 pixel frame bit-identical at a density of 8.0 —
  [this-build-cannot-express-a-volume.md](this-build-cannot-express-a-volume.md). So it is the
  second: their post-stage haze is a permanent difference, and the pictures are the better ones.
- The chapter's pearson table still cannot see any of this. It is kept because it is a useful shape
  measure, and it is no longer a measure of agreement in level.

## Not measured

- Whether `tiles`' 1.285 is a calibration, a material, or the texture sample the exporter drops —
  the SPEC already records that `tiles` hands the tracer a flat floor.
- Whether volume fog can be expressed in the target format. The exporter's node graph carries
  `mix_shader` and the material path carries `principled_bsdf`; whether it carries a volume closure
  is **not measured**.

## Sources

- Measurement: `out/probe-linear-radiance.py`, with the per-scene curves read out of the gallery's
  own `GRADE` by `out/show-grade-constants.py`.
- The sky conversions this measures: `83316c0`, `373c5a2`, `06972f6`.
- The unit fix: `f5555ef`, and
  [investigate/a-directional-irradiance-is-used-as-a-radiance.md](a-directional-irradiance-is-used-as-a-radiance.md).
- The surface body that was painting its own air: `3fd48e7`, and
  [investigate/the-ground-was-dark-because-a-surface-body-painted-its-own-air.md](the-ground-was-dark-because-a-surface-body-painted-its-own-air.md).
- Frames: `out/gb-*.png`, `out/sky-foldyard-*.png`, `out/env-sky-traced.png`,
  `out/sky-landscape-traced.png`.
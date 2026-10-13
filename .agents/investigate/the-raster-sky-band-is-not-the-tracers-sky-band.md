# The raster's sky band is not the tracer's sky band, and it is not the map's texels

Measured 2026-10-12, on `landscape`, raster against Cycles at 32 samples on CPU.

## The question

Three gallery scenes started painting their sky out of the environment map once the post stage
could read it. Two got better and one got worse. `landscape`'s whole frame rose by five thousandths
while its sky band fell from **+0.7785 to +0.5439**. Why does reading the declared map produce a
worse sky than evaluating the same atmosphere's law in WGSL?

## What was measured

Luma over the sky band, rows 0 to 182 of a 480 by 480 frame, both frames against the same traced
frame.

| | raster before | raster after | traced |
|---|---|---|---|
| band mean | 141.84 | 115.27 | 61.39 |
| band span | 39.51 | 57.17 | 15.51 |
| band pearson | +0.7785 | +0.5439 | — |

The span went the wrong way while the mean went the right way: reading the map made the sky
brighter-to-closer and more varied-than-not.

## The first answer, and why it is wrong

The obvious reading of that table is that `environment_sky` reads level zero with `textureLoad`, a
256 by 128 map is coarse, and the sky comes out stepped where `air_sky`'s analytic law was smooth.
That is a testable claim and it was tested by drawing the map at **2048 by 1024** — sixty-four times
the pixels — and measuring again.

| | 256 by 128 | 2048 by 1024 |
|---|---|---|
| band mean | 115.27 | 115.27 |
| band span | 57.17 | 57.17 |
| band pearson | +0.5439 | +0.5432 |
| whole frame | +0.9633 | +0.9635 |

The frames do differ — 477078 of 1048576 pixels, a maximum channel difference of 11 — but the sky
band is unchanged to the second decimal. **Texel size is not the cause.** The change was reverted
rather than committed, because it costs sixty-four times the memory and generation time and buys
five thousandths of a correlation.

This is the record of a falsified hypothesis and it is kept because the table above is what
dismisses it.

## What is actually in that band

Both frames were looked at. The raster sky carries axis-aligned rectangular blocks and horizontal
streaks, and they are **present in the frame from before the change as well** — fainter, because
that sky was lighter, and not at all at 2048 by 1024, so not texels.

They are `landscape`'s own. Its grade paints three layers of ridge silhouette and a set of crepuscular
shafts into the pixels no geometry reached. The traced frame has none of them, because a path tracer
models the ranges and derives its own shadow and depth rays, and the exporter says so every run:

> `shader stage 'ShadowingShader' describes a stage of the rasterised pipeline. Cycles derives
> shadow and depth rays from its own integrator and has no equivalent pass to emit`

So the band being compared is not a sky on one side and a sky on the other. It is a sky plus
painted mountains, painted light shafts and a painted vignette on one side, and a smooth gradient on
the other. Its correlation measures that composition, not the sky.

## What follows

- **A sky-band correlation is only meaningful for a scene that paints nothing into its sky.** Of the
  three converted scenes, `dunes` is that scene, and its sky band went +0.8645 to +0.8710 with the
  change. `foldyard` paints ridges and went -0.3511 to +0.4799. `landscape` paints ridges, shafts and
  a vignette, and its band number should not be quoted as a sky measurement at all.
- **The whole-frame number remains the one to quote**, and for `landscape` it moved +0.9585 to
  +0.9633.
- **The two tone curves are still the largest single disagreement.** The raster sky sits at luma
  115.27 and the tracer's at 61.39 while reading the same radiance, and a curve that compresses
  cannot account for a raster side that is the brighter one.

  > **The reasoning here was sound and its premise has moved.** The tracer's 61.39 was measured with
  > the environment map read as scene linear, so the tracer was reading a sky it had decoded wrongly.
  > Naming the colour space puts `landscape`'s traced sky at 0.3817, and putting that through the
  > rasterised scene's own curve gives 0.5176 against a rasterised 0.4407 — the tracer's side is now
  > the brighter one. The conclusion drawn from it, that the curve was not what was wrong with the sky,
  > still holds; what was wrong turned out to be the colour space. See
  > [what-is-left-of-landscapes-level-gap](what-is-left-of-landscapes-level-gap.md).

## Not measured

Whether the sky bands agree once the painted layers are taken out of the comparison, which needs a
sky-only region that `landscape` does not currently have — its shafts reach the top of the frame.

Whether any map size changes any sky-band number on `dunes` or `foldyard`, whose skies are painted
over less.

## Sources

- Frames: `out/sky-landscape-before.png`, `out/sky-landscape.png`, `out/map-big.png`,
  `out/sky-landscape-traced.png`, measured with `out/probe-agreement.py` and `out/probe-sky-band.py`,
  and compared pixel by pixel with `out/compare-frames.py`.
- The painted layers: `crates/gallery/src/landscape/mod.rs`, `GRADE`.
- The exporter's own words about stages it has no equivalent for: the `render_cycles` run's
  dropped-field report.
- The read: `crates/gallery/src/shaders.rs`, `ENVIRONMENT_SKY`.
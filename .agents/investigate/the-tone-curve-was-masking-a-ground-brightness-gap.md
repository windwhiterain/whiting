# The tone curve was masking a brightness disagreement in the ground

Measured 2026-10-12, on `dunes`, `landscape` and `foldyard`, raster against Cycles at 32 samples on
CPU.

## The question

The largest disagreement between the two backends is that the rasterised post stage tone-maps the
frame and the tracer writes scene-linear radiance into the file. How much of the disagreement is
that curve, and what is underneath it?

## The method

A correlation between the two raw frames answers two questions at once: whether the two renderers
computed the same picture, and whether they agree about what a linear radiance looks like at eight
bits. Putting the traced frame through the raster scene's **own** curve before comparing answers only
the first.

The three curves are not the same curve, and reading them out of the gallery's own `GRADE` rather
than writing them into the probe is the point:

| scene | exposure | curve | saturation | S curve | vignette |
|---|---|---|---|---|---|
| `dunes` | 1 | Reinhard `c/(c+1)` | 1.22 | 0.30 | 0.28 |
| `landscape` | 1 | Reinhard `c/(c+1)` | 1.18 | 0.26 | 0.30 |
| `foldyard` | 0.42 | ACES `(2.51c^2+0.03)/(2.43c^2+0.59c+0.14)` | none | 0.25 | 0.24 |

`foldyard`'s dither is not reproduced: it is a sub-one-in-256 jitter that the eight bit file it was
written to has already lost.

## What the curve is worth

Pearson correlation of luma, raw against curve matched.

| scene | region | raw | curve matched | change |
|---|---|---|---|---|
| `dunes` | whole | +0.3020 | +0.3447 | +0.0427 |
| `dunes` | sky | +0.8711 | +0.9198 | +0.0487 |
| `dunes` | ground | +0.8035 | **+0.9893** | +0.1858 |
| `landscape` | sky | +0.5444 | +0.6374 | +0.0930 |
| `landscape` | ground | +0.8692 | +0.7684 | −0.1008 |
| `foldyard` | sky | +0.4734 | +0.5342 | +0.0608 |
| `foldyard` | ground | +0.2372 | +0.2412 | +0.0039 |

**The sky's shape improves on all three and the ground's only on one.** That split is the finding,
and it is not what the curve question predicted: a curve is a curve, and it should move every region
of the picture the same way.

`dunes` is the scene whose ground is geometry both backends really do share, with nothing painted
into it, and its ground reaches **+0.9893** — the two backends draw the same ground. `landscape`
and `foldyard` both paint into their ground as well as their sky, so their ground numbers were never
about shading.

## What is underneath it

Mean luma over the region, and the traced frame put through the curve.

| scene | region | raster | traced | traced graded |
|---|---|---|---|---|
| `dunes` | sky | 157.03 | 150.25 | 157.84 |
| `dunes` | ground | 109.06 | 152.95 | 157.66 |
| `landscape` | sky | 115.32 | 61.22 | 112.73 |
| `landscape` | ground | 199.63 | 189.93 | 165.34 |
| `foldyard` | sky | 54.97 | 27.50 | 48.37 |
| `foldyard` | ground | 121.32 | 182.58 | 168.57 |

**Every sky now agrees**: 157.03 against 157.84, 115.32 against 112.73, 54.97 against 48.37. The sky
read from the scene's own map at the light's own radiance is the tracer's world, and this is the
measurement that says so.

> **Superseded on 2026-10-13.** Every sky in this table agreed because two errors were cancelling. The
> environment map was being read as scene linear — an eight bit sRGB file used directly as linear
> radiance — which left the traced sky short by roughly the amount the curve lifts it. With the colour
> space named, `landscape`'s traced sky grades to **0.5176 against a rasterised 0.4407**, the sign is
> the other way, and the three skies here would no longer agree. The ground half of this table is
> unaffected and is confirmed: after grading, `landscape`'s ground rows come out 1.004, 1.024 and
> 1.034. See
> [what-is-left-of-landscapes-level-gap](what-is-left-of-landscapes-level-gap.md) and
> [the-air-image-reaches-cycles-with-no-colour-space](the-air-image-reaches-cycles-with-no-colour-space.md).

**No ground agrees**, and not in one direction: `dunes` is 49 levels dark, `foldyard` 47 dark,
`landscape` 34 bright. `dunes`' ground correlates +0.9893 while sitting 49 levels away, which is the
clearest statement of the difference available: **the shape is right and the brightness is not.**

A tone curve cannot be the cause of a disagreement that changes sign between scenes, and the earlier
belief that "one radiance cannot align both sky and ground" is the same fact seen from the other
side — once the sky reads its own radiance, the number left over is the ground's, not the sky's.

## Not measured

- Whether a traced radiance survives the round trip through eight bits before the curve. The traced
  frame is already quantised, so the graded column carries that error and it is not separated here.
- Which term of the ground's brightness is wrong on either side: the environment's radiance, the
  sun's, the albedo, or the two lighting models themselves.
- Any of this on the four scenes that declare no environment, whose sky the tracer draws black.

## Sources

- Constants: `out/show-grade-constants.py`, reading `crates/gallery/src/dunes.rs`,
  `crates/gallery/src/landscape/mod.rs` and `crates/gallery/src/foldyard.rs`.
- Measurement: `out/probe-grade-matched.py`.
- Frames: `out/env-sky.png`, `out/env-sky-traced.png`, `out/sky-landscape.png`,
  `out/sky-landscape-traced.png`, `out/sky-foldyard-camera0.png`, `out/sky-foldyard-traced.png`.
- The transfer curve between the raw frames, which is bowed rather than a gain:
  `out/probe-transfer.py`, where a traced 8 comes back as a raster 209 and a traced 248 as a raster
  222.
# Do the two backends resolve the same environment image to the same direction?

**Question.** `terraces` reads **0.781** on its sky against a tracer drawing the same law at
1.000, when the rasteriser's post stage is drawing nothing but `environment_sky(ray)` and no grade
at all. Its row profile says the two frames do not merely disagree in brightness — **their sky
gradients run in opposite directions**: the rasteriser's gets brighter towards the horizon and the
tracer's gets brighter towards the top of the frame. Both are handed one 256x128 image. **Do they
read that image the same way, and if not, which of them is wrong?**

**Answer, measured 2026-10-13 on `main` at `a1397e3`, 1024x1024, 32 samples on CPU.** They do not
read it the same way, and **the fault is located: the tracer reads the image mirrored in azimuth.**
Mirroring the exported map along `u` and re-tracing the exported document takes the sky from a
ratio of 0.58 to 0.86 across ten rows — gradients running opposite ways — to **0.945 to 0.983**,
gradients running the same way. What was measured first is below; the control that makes the
reading safe is in its own section after it.

## The census's 0.857 for `terraces` is not a sky measurement, and this note is not about it

`terraces` paints a cloud deck, three ranges and the valley's fog **into its sky**.
[the-raster-sky-band-is-not-the-tracers-sky-band.md](the-raster-sky-band-is-not-the-tracers-sky-band.md)
established the rule: **a sky-band figure is only meaningful for a scene that paints nothing into
its sky**, and of the scenes converted to the map, `dunes` is that one. Decomposed on 2026-10-13
through `out/probe-linear-radiance.py`:

| what the sky band is made of | sky ratio |
|---|---|
| the map alone | 0.781 |
| plus the cloud deck and the three ranges | 0.764 |
| plus the fog, which runs on geometry and sky alike | **0.857** |

**so the census's 0.857 is mostly the fog**, which is a painted layer the tracer cannot have, and
the number to reason about is the **0.781**. What follows takes the painted layers out as well — not
to make the scene score better, but because a lookup question is only answerable against a frame
whose sky is nothing but the lookup.

## The measurement, with no inversion anywhere in it

The band figure and the row profile both come out of `probe-linear-radiance.py`, which has to undo
a grade first — so both are measurements of an *inversion*. To take the inversion out of the
question, the post stage was reduced to

```wgsl
color = environment_sky(ray_through(in.uv));
```

with no grade and no depth test, so every pixel of the file **is** the map lookup. Read straight
out of the files, nothing undone (`out/probe-three-skies.py`):

| frame row | raster, raw lookup | tracer | ratio |
|---|---|---|---|
| 0 | 0.4773 | 0.8234 | 0.580 |
| 20 | 0.4878 | 0.8106 | 0.602 |
| 40 | 0.5110 | 0.7837 | 0.652 |
| 60 | 0.5255 | 0.7418 | 0.708 |
| 80 | 0.5464 | 0.6669 | 0.819 |
| 90 | 0.5595 | 0.6504 | 0.860 |

**The ratio moves from 0.58 to 0.86 across ten rows.** A constant ratio is a scale and one number
explains it; a ratio that moves is the two frames placing the same image differently, and no field
on a light will close that.

## What the document hands the tracer

`out/now-traced-terraces.cycles.xml`, the whole of the world:

```xml
<film exposure="1" />
...
<background>
    <environment_texture name="world_texture" filename="sky.png" />
    <background name="world" strength="1, 1, 1" />
    <connect from="world_texture color" to="world color" />
    <connect from="world background" to="output surface" />
</background>
```

One point light, `sun`. **Nothing else touches the background**: no exposure on the film, a strength
of one, no second term mixed into the world.

## What the image itself holds

`out/sky.png`, 256x128 RGBA, the file the tracer was handed, in the file's own order
(`out/probe-exported-map.py`): row means run **0.2504 at row 0, 0.7008 at row 64, and a flat
0.3915 from row 96 down**. A conventional equirectangular — dark pole, bright horizon at the middle
row, flat ground half below. The rasteriser's values sit inside that range and rise towards the
horizon row; the tracer's do the opposite.

## Two things this cost to find

**the documents in `out/` share image filenames, so a re-trace can pick up the wrong sky.** `dunes`
and `foldyard` both declare an image labelled `air`, and the exporter writes it as
`out/air.png` — **last write wins**. `out/now-traced-dunes.cycles.xml` therefore names a file that,
by the time it was re-read, held `foldyard`'s sky. The control above had to re-export `dunes` into
a clean prefix first. A document and the image beside it are **a pair, and re-reading one without
the other is how a measurement becomes a different measurement.**

**and a finding that seemed to be an export fault was not.** For a while the reading here was that
the tracer's 0.8234 "exceeds the map's brightest row" and so cannot come from the map at all. That
compared a row **mean** against a pixel **value**, and 0.754% of this map's texels are clipped to
one — the peak is near one, so 0.8234 is inside the image. The claim was wrong, and it pointed the
search at the export instead of at the lookup.

## The control, and it is the reason this reading is safe

**One scene cannot change an exporter**, because a blanket mirror would break every scene that is
already right. So `dunes` — which reads 1.001, and is the only scene in the gallery that paints
nothing into its sky — had its exported map mirrored in `u` and was re-traced anyway.

| row | plain | mirrored | ratio |
|---|---|---|---|
| 0 | 0.5203 | 0.5203 | **1.000** |
| 40 | 0.5567 | 0.5567 | **1.000** |
| 80 | 0.6084 | 0.6084 | **1.000** |
| 120 | 0.6862 | 0.6862 | **1.000** |
| 160 | 0.5895 | 0.5895 | **1.000** |

**Every row identical to three decimals.** `dunes`' environment map is **uniform in azimuth** —
mirroring it changes nothing at all — so mirroring every map in `u` **cannot break it**, and the
fix is safe with respect to the one scene that currently looks perfect.

**and this is why the census never caught it.** "the four scenes that declare an environment light
read 1.001, 1.040, 1.059 and 1.085" was never evidence about the lookup. **An azimuthally uniform
map cannot detect an azimuth error**, because the map under the error is the same map. `dunes`
agrees at 1.001 and would agree at 1.001 with the sky turned inside out. `terraces` sees it because
its map carries the sun's bearing, and a scene whose weather has a direction is the only kind that
can.

## The fix, and what is left of it

**The rasteriser is right and the exporter's writer is wrong.** The rasteriser's own numbers match
the image: the map rises 0.250 at the pole to 0.701 at the horizon, and the raster's sky rises
0.477 to 0.560 down the frame, which is that same rise. The tracer's 0.823 to 0.650 is the rise and
fall of a **mirrored** map, and mirroring the file puts the two together.

So the change belongs where the image is written for the tracer — the equirectangular environment
image's `u` convention — and **not** in the camera and **not** in the rasteriser's lookup.

**What the mirror does not explain.** After it, the ratio sits at **0.955**, roughly constant across
the band rather than spread across it, so what is left is a scale and not a lookup: 8-bit
quantisation against an in-memory float map, and Cycles' pixel filter averaging a footprint the
rasteriser point-samples with `textureLoad`. Neither is settled here and neither is a lookup fault.

**What is left to do before this lands.** `foldyard` and `tidewater` also carry structured maps and
their skies will move under this change, and **neither has a clean sky measurement to check the
result against** — both paint ridges or fog into their sky, which is the composition problem
[the-raster-sky-band-is-not-the-tracers-sky-band.md](the-raster-sky-band-is-not-the-tracers-sky-band.md)
describes. `terraces` is the only scene in the gallery that can currently verify this.

## Not measured

- **Whether the residual 0.955 is quantisation, the pixel filter, or a colour space.** The
  `environment_texture` node carries no `color_space` attribute and therefore takes Cycles' default,
  and the writer's choice for an 8-bit image was not compared against it.
- **`landscape`, `foldyard` and `tidewater` under the mirror.** Their maps are structured and their
  skies will change; what they change *to* is not measured here.
- **A synthetic ramp world.** `axes.rs` describes one and the gallery does not have it. The
  experiment here does not need it — a real scene's own map, mirrored and re-traced, answered the
  question — but a ramp would say *which* axis is wrong without inferring it from one scene's sun.

## Why it matters more than its size suggests

`terraces` is now the worst-matched scene in the gallery — whole frame **0.531**, sky **0.857**,
ground **0.501** — and the ground is already accounted for (per-fragment albedo a tracer is not
handed). **This is the sky, and the census's headline is that skies agree**: the four scenes that
declare an environment read 1.001, 1.040, 1.059 and 1.085. `terraces` declares one and reads 0.857
with a shape correlation of +0.8732, which is not what that sentence says.

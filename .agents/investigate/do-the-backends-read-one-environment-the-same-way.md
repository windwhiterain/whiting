# Do the two backends resolve the same environment image to the same direction?

**Question.** `terraces` reads **0.781** on its sky against a tracer drawing the same law at
1.000, when the rasteriser's post stage is drawing nothing but `environment_sky(ray)` and no grade
at all. Its row profile says the two frames do not merely disagree in brightness — **their sky
gradients run in opposite directions**: the rasteriser's gets brighter towards the horizon and the
tracer's gets brighter towards the top of the frame. Both are handed one 256x128 image. **Do they
read that image the same way, and if not, which of them is wrong?**

**Answer, measured 2026-10-13 on `main` at `a1397e3`, 1024x1024, 32 samples on CPU.** They do not
read it the same way. **The fault is not a fixed convention to be corrected once** — mirroring the
exported map along `u` fixes `terraces` and breaks `landscape`, from the same generator, and the
reading that survives both is that **the tracer's azimuth is a function of the camera**. The
measurement is below, the control that made mirroring look safe is in its own section, and the
section that killed the obvious fix is the one after it.

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

## The mirror is NOT the fix, and landing it would have traded one scene for another

**The obvious reading of the section above is wrong, and the experiment that catches it is cheap.**
If the tracer reads every world image mirrored in azimuth, then mirroring on export fixes `terraces`
— and it must also leave `dunes`, `tidewater` and `foldyard` untouched, which it did. **Applied to
the exporter, it did exactly that and one more thing: it broke `landscape`.**

| scene | sky before | sky after | ground before | ground after |
|---|---|---|---|---|
| `dunes` | 1.001 | **1.001** | 1.048 | 1.048 |
| `tidewater` | 1.059 | **1.059** | 0.680 | 0.680 |
| `foldyard` | 1.089 | **1.089** | 0.982 | 0.982 |
| `terraces` | 0.857 | **1.048** | 0.501 | 0.490 |
| `landscape` | 1.040 | **0.704** | 0.948 | 0.951 |

`landscape` was given the same raw-lookup treatment `terraces` had — its post stage reduced to
`color = environment_sky(ray_through(uv))`, no grade, no painted ranges, no shafts — and against
that:

| `landscape`, raw lookup | against the unmirrored trace | against the mirrored trace |
|---|---|---|
| ratio across ten rows | **1.004 to 1.022**, gradients the same way | 0.595 to 0.682, gradients opposite |

**So `landscape` is right unmirrored and `terraces` is right mirrored, and both maps are written by
the same function.** `dawn::Sky::pixels` calls `crate::shaders::environment_pixels_of` and so does
`air.rs`'s `environment_pixels` — one generator, one projection, one exporter, two opposite
results. The hypothesis that the two generators disagreed was checked and **disproved**.

**The only reading left is that the tracer's azimuth is a function of the camera.** The correction a
scene needs would then depend on where its camera is looking, and **a mirror baked into the writer
is the wrong shape of fix**: it happens to be right for `terraces` and wrong for `landscape`, which
is a trade dressed as a repair. Their camera rotations are not close — the quaternions read
`terraces` 159.22/0.23/0.76/0.61 against `landscape` 83.31/1.00/-0.06/-0.07, and this note's first
version called that "a narrow band", which was not looked at closely enough.

**The exporter change was reverted and is not on `main`.** What it bought is the knowledge that the
fault is camera-dependent rather than a convention to be corrected once, which is worth more than
the fix would have been.

## What would actually settle it

**Sweep the map's azimuth and find the offset that matches.** Rotate `terraces`' exported world by a
fraction of a turn — a quarter, a half, three quarters — and re-trace each, against the raw-lookup
frame. If the required correction is a constant, the fault is a fixed convention error and a scene
whose camera sits at the matching azimuth is showing it; if the required correction differs between
`terraces` and `landscape` in the way their cameras differ, it is camera-dependent and the fix
belongs in the camera transform or in the world's own rotation, which is a node the format has and
this emitter does not write. **That sweep is four traces and has not been run.**

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

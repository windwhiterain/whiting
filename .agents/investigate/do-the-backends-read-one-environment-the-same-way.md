# Do the two backends resolve the same environment image to the same direction?

**Question.** `terraces` reads **0.781** on its sky against a tracer drawing the same law at
1.000, when the rasteriser's post stage is drawing nothing but `environment_sky(ray)` and no grade
at all. Its row profile says the two frames do not merely disagree in brightness — **their sky
gradients run in opposite directions**: the rasteriser's gets brighter towards the horizon and the
tracer's gets brighter towards the top of the frame. Both are handed one 256x128 image. **Do they
read that image the same way, and if not, which of them is wrong?**

**Answer, measured 2026-10-13 on `main` at `a1397e3`, 1024x1024, 32 samples on CPU.** They do not
read it the same way, and **this note does not establish which is right.** What follows is what was
measured, and then what was ruled out.

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

**One inference I had to withdraw.** For a while the reading here was that the tracer's 0.8234
"exceeds the map's brightest row" and therefore cannot come from the map at all. That compared a
row **mean** against a pixel **value**, and 0.754% of this map's texels are clipped to one — the
peak is near one, so 0.8234 is inside the image. The claim was wrong and is recorded because it
would have sent the search after the export rather than after the lookup.

## Ruled out, and how

- **A grade inversion artefact.** This is what the raw-lookup render exists to exclude. Both
  columns above are read straight out of their files.
- **The world's own settings.** `<film exposure="1">`, `strength="1, 1, 1"`, one background node,
  nothing mixed in.
- **A camera-specific fault.** All six scenes that declare an environment emit the same camera
  shape — `scale="-1, 1, 1"` on every one, and quaternions within a narrow band of each other
  (`dunes` 163.99/0.19/-0.79/-0.58, `terraces` 159.22/0.23/0.76/0.61, `foldyard`
  148.44/0.27/-0.67/-0.69, `tidewater` 179.62/0.004/0.74/0.67, `landscape` 83.31/1.00/-0.06/-0.07,
  `sunlight` 158.48/0.23/-0.75/-0.62). **The direction-to-texel mapping is therefore not varying
  with the camera**, which was the first thing to check and is now off the list.
- **A wrong `v` axis, as a completed argument.** `cycles_export/axes.rs` states the basis change
  `(x, y, z) -> (x, -z, y)`, which sends this engine's `+y` to Cycles' `+z`, so
  `v = acos(dir.z)/pi` is `acos` of this engine's `+y` and should be right. That is an argument from
  the code, not a measurement, and it is exactly the kind of argument that has already been wrong
  once in this chapter — `axes.rs` records that an axis ramp once showed the image's own row running
  along a *horizontal* line of a render, which is the symptom of this class of fault.

## Not settled

**Which backend is wrong, and where.** The two candidates the evidence has not separated:

1. **the `v` axis as Cycles resolves it for these rays**, in which case `dunes` at 1.001 with a
   shape correlation of +0.9898 is the scene whose sky is too flat to show it, and the fault is in
   the exporter's world handling;
2. **the colour space the 8-bit `environment_texture` is read with**, in which case the node carries
   no `color_space` attribute and takes Cycles' default, and the fault is in what the writer emits.

**The test that would separate them** is the one `axes.rs` already describes and is not repeated
here: a synthetic world whose image is a ramp along one axis. A ramp that comes back running
vertically is a correct lookup and a fault in the image or the colour space; a ramp that comes back
running sideways, or running the wrong way, is a fault in the lookup. That scene does not exist in
the gallery today and would have to be authored to answer this.

## Why it matters more than its size suggests

`terraces` is now the worst-matched scene in the gallery — whole frame **0.531**, sky **0.857**,
ground **0.501** — and the ground is already accounted for (per-fragment albedo a tracer is not
handed). **This is the sky, and the census's headline is that skies agree**: the four scenes that
declare an environment read 1.001, 1.040, 1.059 and 1.085. `terraces` declares one and reads 0.857
with a shape correlation of +0.8732, which is not what that sentence says.

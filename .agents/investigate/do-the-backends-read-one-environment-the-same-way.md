# Do the two backends resolve the same environment image to the same direction?

**Question.** `terraces` reads **0.781** on its sky against a tracer drawing the same law at
1.000, when the rasteriser's post stage is drawing nothing but `environment_sky(ray)` and no grade
at all. Its row profile says the two frames do not merely disagree in brightness — **their sky
gradients run in opposite directions**: the rasteriser's gets brighter towards the horizon and the
tracer's gets brighter towards the top of the frame. Both are handed one 256x128 image. **Do they
read that image the same way, and if not, which of them is wrong?**

**Answer, re-measured 2026-10-13 on `main` at `7a7fc5d`, 1024x1024, 32 samples on CPU.** **They
read the image the same way, and the reading is correct in every respect it can be measured in.** No
tone curve sits between the tracer and the file, the polar is the right way up, the azimuth is not
mirrored and is not rotated, and denoising is off. **The fault is not in the world at all: the two
backends are looking in different directions**, and the disagreement is a clean sideways
displacement across the frame rather than a difference in how a sky is drawn.

**The measurement that settles it is below. It took a map with no law in it**, because every
earlier experiment compared two skies that each had their own ridges, cloud, fog and sun painted
on them, and a scene's own painting is indistinguishable from a fault in the world it is painted on.

## What the projection actually is, measured against a map with nothing in it

Two synthetic maps, written by `out/probe-projection-map.py` and traced through the **same exported
terraces document** with only the world replaced. `latlong_direction` writes a direction from a
column and a row, so a channel carrying a known function of the column or the row reads back the
exact `u` or `v` the tracer looked up for that pixel. No ridge, cloud, fog, grade or sun law can
move the answer.

### There is no tone curve (`rows` map, `out/probe-read-projection.py`)

The map's red channel is **0.25 over its top half and 0.75 over its bottom half**. The traced sky
carries **0.2510** and **0.7490**: a ratio of **2.984** against the map's **3.000**. **The tracer
writes scene linear straight through.** This matters because the document carries
`<film exposure="1" />` and **no `<display_settings>` at all**, so Cycles takes its own default view
transform — and every ratio measured against an authored sky would have been partly a measurement of
that transform. It is not one.

### The polar is not the wrong way up

The sky reads red **0.2510**, which is the map's **top** half, and the map's top half is the zenith
half as `latlong_direction` writes it. The green channel, which ramps **0 at the zenith to 1 at the
nadir**, **rises down the frame by +0.0353** over the sky window. Both say the same thing: `v` is
read the way it was written.

### The azimuth is not mirrored and is not rotated (`columns` map, `out/probe-read-azimuth.py`)

The map's red channel ramps **0 to 1** from its left column to its right. Traced, over the sky above
the skyline:

| x | 64 | 192 | 320 | 448 | 576 | 704 | 832 |
|---|---|---|---|---|---|---|---|
| red | 0.2863 | 0.3020 | 0.3176 | 0.3333 | 0.3529 | 0.3686 | 0.3843 |

**Monotonically rising**: span **+0.1020**, least squares slope **+0.000135** a pixel. **The tracer
walks the map the way it was written.** This is the fault every earlier experiment kept half finding,
and it is not there.

### And denoising was never on

`<integrator seed="0" use_denoise="false" max_bounce="8" />`. Whiting writes it off, so a traced
frame carries no denoiser to tilt a gradient, and that candidate cost one read of the document to
kill.

## Where the fault is: the two backends look different ways

With the world cleared, the only thing the two backends each have their own is the camera. And the
disagreement has the **shape a camera makes and the world does not** — `out/probe-disagreement-shape.py`,
ratio of the rasteriser's raw lookup to the trace, in twelve columns across the sky:

| column | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| ratio | **1.2414** | 1.2432 | 1.2222 | 1.1875 | 1.1302 | 1.0608 | 0.9697 | 0.8796 | 0.8008 | 0.7352 | 0.7176 | **0.6980** |

**A clean monotone sweep from 1.2414 to 0.6980, span −0.5433.** That is a **sideways displacement**.
A fault in the world is the same at every pixel of a frame and shows up in a row statistic; a camera
pointing elsewhere puts the rasteriser brighter than the tracer on one side and darker on the other,
and the ratio across the frame **crosses one**. This one crosses one at column six, the middle of the
frame.

**So the sun's warm band is in a different part of the frame in each backend.** That is the whole of
what every experiment in this note was chasing, seen from the other end.

## Why the mirror looked like a fix, and why it was not one

Mirroring the exported map in `u` moved `terraces` from 0.58–0.86 to 0.945–0.983 and moved
`landscape` from 1.040 to 0.704. **Neither number was ever a measure of the projection**, and the
table below says why: the thing that needed correcting is a **displacement**, and a map mirrored in
`u` displaces its content sideways too. It moved `terraces`' warm band into the half of the frame
where the comparison wanted it, which raised the ratio, and it moved `landscape`'s out. **A
displacement and an azimuthal mirror both shift the sky sideways and are the same size to a frame
comparison**, which is why the correction looked like a repair for one scene and a breakage for the
other. The synthetic map separates them, because a mirrored map would have made its red **fall**.

**The azimuth sweep this note used to ask for is superseded and should not be run.** Rotating the map
a quarter, a half and three quarters was to distinguish a fixed convention error from a
camera-dependent one, but **mirroring twice cannot tell a mirror from a rotation** and neither can
rotating: both move content sideways, and the question is only answerable against a map that
measures the mapping instead of shifting it.

## Withdrawn: the camera-dependence claim this note used to make

An earlier version concluded that "**the tracer's azimuth is a function of the camera**", on the
grounds that `terraces` wanted the map mirrored and `landscape` wanted it whole. **That reasoning
was wrong and the claim is withdrawn.** The projection is camera-independent and correct; what
varies between the two scenes is where **each camera points**, which is a real fault but not that one.
The note reached the right neighbourhood by a route that could not distinguish "the world is turned
round" from "the camera is turned round", because every measurement it used had both in it.

## The camera fault, measured: one line of the exporter

Measured 2026-10-13 on `main` at `f962748` with `out/probe-camera-ray.py`, which builds **both**
frame-centre rays and compares them in Cycles' axes. whiting's ray is
`forward = normalize(target - eye)` (`standard/camera.rs:357-369`), carried through the same
`(x, y, z) -> (x, -z, y)` the exported `translate` is the evidence of. The tracer's is
`scale * rotate * (0, 0, 1)`, because Cycles looks down **+Z** and a translation does not move a
direction.

| | direction in Cycles axes | azimuth | elevation |
|---|---|---|---|
| rasteriser | `[0.5405, 0.8107, -0.2252]` | **−56.31°** | −13.01° |
| tracer | `[−0.5405, 0.8107, −0.2252]` | **−123.69°** | −13.01° |

**65.43 degrees apart, the elevation identical to two decimals, and the eye in the same place to
0.0075.** Only the `x` component of the direction is negated — which is exactly what a scale of
`-1, 1, 1` does to a direction vector and nothing else — and the two rays are strict mirror images
about the `YZ` plane. **It is a turn and not a move.**

**The rotation is not at fault.** The same ray with the scale left off is **0.0015 degrees** from
the rasteriser's. `camera_rotation` builds `yaw = atan2(fx, fz)` and a negated pitch, and those
already carry `+Z` onto `normalize(target - eye)`.

### Why the scale is there, and what it does instead

`cycles_camera_frame` (`cycles_export/scene.rs:503`) negates the local `X`, and its comment records
a real measurement: **Cycles' camera `+X` is its own right**, and with `+Z` forward that basis is
left-handed, so a camera with its own right-hand side needs `det = -1`. Markers on the raster
camera's right and left came back from Cycles exchanged, at `x = 0.240` and `x = 0.818`. **Negating
the local `X` is a correct answer to that question.**

**The error is one sentence in that comment**: *"a scale written last multiplies the rotation's
columns from the right and reaches the frame rather than the world."* The loader composes
`translate`, then `rotate`, then `scale` (`transform.rs:13-16`), so a written scale is **`S * R`** —
it multiplies from the **left**, in world space. `S * R` negates the `x` of **both** the forward
column and the right column, so it does not only swap the image: **it turns the camera to face a
different part of the world.**

| composition | forward azimuth | right azimuth | determinant | writable as `rotate` |
|---|---|---|---|---|
| `S * R` — written today | −123.69° | −33.69° | −1 | no, and it aims wrong |
| `R * S` | −56.31°, the raster's | 33.69° | −1 | **no** — see below |

**`R * S` aims correctly and still cannot be written down.** `diag(-1, 1, 1)` is a reflection, its
determinant is −1, and every product carrying it has determinant −1, so neither form is a rotation
the loader's Rodrigues composition can carry. That is what the next section turns on.

### The correction that holds, and the one that does not

**`R * S` was wrong, and the probe that proposed it refuted itself.** `diag(-1, 1, 1)` has
determinant **−1**: it is a **reflection**, not a rotation. The half turn about `+Y` is
`diag(-1, 1, -1)`. A product of a reflection with anything also has determinant −1 and **cannot be
written as a `rotate` attribute at all**, because the loader's Rodrigues composition is always a
proper rotation. The probe printed both determinants as −1, which was the refutation sitting in its
own output and not the support it was read as.

**What holds is the same correction the other way round.** The camera that is wanted is a basis `M`
whose determinant is **−1** — that is the whole reason for the mirror, since Cycles' `+X` is its own
right and with `+Z` forward that basis is left-handed. Build `M` from the rotation already in the
document: keep **its own** forward and **its own** up, and take the right axis that makes the
determinant −1, which for orthonormal columns and `det = -1` is `a = c * b`, so
`right = forward * up`. Then write **`rotate` as `S * M`** and leave the `scale` alone. `S * M` has
determinant **+1** and is a proper rotation, so the loader accepts it, and what reaches Cycles is
`S * (S * M) = M`.

Both the forward and the up must come from **`R`**, never from `S * R`: the scale negates `x` on
**every** column, so the up column of `S * R` is `S * (R * y)` and is not `R * y`. Taking one axis
from each matrix leaves them non-orthogonal, and `S * M` comes out with determinant **0.9818** —
which is how this probe's first attempt at the correction failed, and why the determinant is
printed rather than assumed.

### Measured end to end, on the document, with no source touched

For `terraces` the change to the written `rotate` is **two signs**:

| | axis |
|---|---|
| as written today | `159.2177, 0.2305913, 0.7615913, 0.6056454` |
| as corrected | `159.2177, 0.2305913, **-**0.7615913, **-**0.6056454` |

Traced at 1024x1024, 32 samples, against the rasteriser's raw lookup
(`out/probe-camera-basis-fix.py`, `out/probe-disagreement-shape.py`):

| | left edge | right edge | span | crosses one |
|---|---|---|---|---|
| as written today | 1.2414 | 0.6980 | **−0.5433** | yes, at column six |
| as corrected | 1.3740 | 1.3088 | **−0.0652** | no |

**The monotone sweep is gone and the ratio no longer crosses one, so the sideways displacement is
resolved.** What is left is a **constant factor of about 1.41** across the whole sky, and a constant
ratio is a scale rather than a model disagreement — **that is a separate open question and is not
settled by this.**

### What is not established

**The two orientation measurements were never in conflict, and the reading that made them look so
was wrong.** `out/probe-orientation-one-statistic.py` derives the ratio curve and both correlations
from **one profile per frame** — mean luminance down the sky rows, one number per column — so the
two cannot disagree by construction. For the corrected trace against the rasteriser's raw lookup:

| | raster's profile | traced profile | ratio, left to right |
|---|---|---|---|
| **as corrected** | falls −0.0166 | rises **+0.0457** | 1.4257 to 1.2853, span **−0.1404** |
| as written today | falls −0.0166 | rises **+0.3938** | 1.1915 to 0.7053, span **−0.4862** |

**One description, not two: the traced sky's left-right gradient runs the other way from the
raster's in this band.** The ratio falls by **9.8%** across the frame, and a falling ratio is not a
constant one — which is the whole of the confusion. A ratio of 1.4257 to 1.2853 was read as
"proportional" and a correlation of −0.5945 as "opposite", and the two were then treated as a
contradiction. **They are the same fact.** The uncorrected trace is the control that shows the
measure works: the same profile rises by +0.3938 instead of +0.0457 and the ratio falls by **40.8%**,
so the correction cut the horizontal mismatch by a factor of **8.6**.

**So the camera is right and something is still wrong with the sky, by a tenth of what it was.**
That remainder is **not measured**, and it is very likely not the camera: the rasteriser reads the
map with `textureLoad`, which takes the **nearest texel**, while Cycles samples the same 256x128
map **filtered**, and a difference between the two on a sky with a bright horizon band and a sun
with a bearing is asymmetric by nature. **That is a separate question and nothing here has answered
it.**

**What is owed for the image orientation is still a marker**: one unmistakable object on the
rasteriser's right and one on its left, geometry held still and only the transform changed. The ray
is settled exactly — 0.000000 degrees — and that is the part the camera answers.

## Two errors of mine this note should not let anyone repeat

**A window over the mountains is a window onto lit rock.** `probe-read-azimuth.py` first took the
sky as the top **45%** of the frame, measured a span of **−0.0608**, and printed "**red FALLS to the
right: the tracer walks the world the opposite way round**" — over rows whose own printed values rose
from `0.2863` to `0.3843**. `terraces` looks thirteen degrees down and its ranges reach into the
lower half of the frame; lit rock under a flat world reads about **0.6** red whatever its `u` is,
which swamps the **0.28 to 0.38** the sky carries. Both probes also printed their green summary over
the bottom of the frame, where there is no sky, and drew a decreasing trend from a mountain. **The
window was the measurement's error and the verdict was read off it anyway** — twice.

**A row median is a statistic that hides a displacement.** The original evidence for this whole
question was a ratio moving down the rows, and a sideways displacement produces exactly that as
well, which is the trap the column-wise comparison above exists to close.

**And a statistic chosen after seeing the answer is not a measurement.** The orientation probes were
written to settle left-for-right by correlating column profiles straight against mirrored, and they
were written **after the correction had already been made** — the position from which any number
looks like confirmation. One of them reported a verdict and the note recorded it as a contradiction
with the ratio probe. **Deriving both numbers from one profile showed there had never been a
contradiction: a falling ratio and a negative correlation are the same fact, and the ratio was
falling all along.** The lesson is not about correlation or about medians. It is that two readings of
the same data, made with two different reductions and no shared array, will be read as two findings
long before anyone checks whether they can be the same finding.

## What it cost to find

**The documents in `out/` share image filenames, so a re-trace can pick up the wrong sky.** `dunes`
and `foldyard` both declare an image labelled `air`, and the exporter writes it as `out/air.png` —
**last write wins**. A document and the image beside it are **a pair, and re-reading one without the
other is how a measurement becomes a different measurement.** The work above re-exported into a clean
prefix for exactly this reason, and `dn-terraces.png` came out byte-for-byte the size of
`now-traced-terraces.png`, which is how the pair was confirmed to be the same one.

**And a finding that seemed to be an export fault was not.** For a while the reading here was that
the tracer's 0.8234 "exceeds the map's brightest row" and so cannot come from the map at all. That
compared a row **mean** against a pixel **value**, and 0.754% of this map's texels are clipped to
one — the peak is near one, so 0.8234 is inside the image.

## Why it matters more than its size suggests

`terraces` is the worst-matched scene in the gallery — whole frame **0.531**, sky **0.857**,
ground **0.501** — and the ground is already accounted for (per-fragment albedo a tracer is not
handed). The sky is this fault, and it is **not** in the world: **every scene is mis-framed by it,
in its own direction, by its own angle.** A scene whose sky happens to be uniform in azimuth — which
is why the census read 1.001, 1.040, 1.059 and 1.085 for the others — cannot see a fault that is a
displacement, because the frame it would have to be mis-pointed within is the same frame either way.
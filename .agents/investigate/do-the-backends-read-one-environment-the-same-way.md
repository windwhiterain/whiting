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

**The sentence this section used to attack was right, and attacking it cost the gallery its
cameras.** *"A scale written last multiplies the rotation's columns from the right and reaches the
frame rather than the world."* **The loader right-multiplies** —
`tfm = tfm * transform_translate(..)`, `tfm = tfm * transform_rotate(..)`,
`tfm = tfm * transform_scale(..)` (`src/app/cycles_xml.cpp:694-710`) — so a transform written at
the top of a scope is `T * R * S`, and the scale lands on the **camera's own frame**, on the
right. `diag(-1, 1, 1)` negates the local `X` **column** and leaves `Y`, `Z` and the translation
alone. `S * R` — a world-side scale that turns the camera — is not what the loader computes.

**Measured 2026-10-13 at `afb0c0b`, on a document holding nothing but a camera with no rotation
and that one scale:**

| | raster `x` of the local `+X` marker | local `+Y` | local `-Y` |
|---|---|---|---|
| no scale | 0.9844, at the right edge | bottom | top |
| `scale="-1, 1, 1"` | **0.0146, at the left edge** | bottom, unmoved | top, unmoved |

**The frame came back mirrored left to right and otherwise identical.** The aim did not move at
all. A world-side `S * R` would have moved the aim instead, which is the distinction the whole
section turned on.

**What the wrong reading cost.** The rotation was rewritten as `S * R * S`, in the axis-angle form
`(w, v_x, -v_y, -v_z)`, to cancel an `S * R` that never happens. Under the loader that
composition is `(S * R * S) * S = S * R`, so the forward became `S * (R * forward)`: every camera
mirrored in `x` about its own axis, a turn of `2 * atan2(forward.x, forward.z)` away from its
target. Reverted in `afb0c0b`.

The tables below are kept because they are the record of the wrong turn, not because their
conclusion stands.

| composition | forward azimuth | right azimuth | determinant | writable as `rotate` |
|---|---|---|---|---|
| `S * R` — **assumed, not computed by the loader** | −123.69° | −33.69° | −1 | yes, and it aims wrong |
| `R * S` — **what the loader computes** | −56.31°, the raster's | 33.69° | −1 | it needs no `rotate` at all |

**`R * S` is the wanted basis and it needs no rewriting of the rotation.** `diag(-1, 1, 1)` is a
reflection, determinant −1, which is exactly what a camera whose own `+X` is its right-hand side
needs; the format carries it in `scale`, the loader applies it on the right, and the rotation the
document already holds reaches Cycles untouched.

~~**`R * S` aims correctly and still cannot be written down.** `diag(-1, 1, 1)` is a reflection, its
determinant is −1, and every product carrying it has determinant −1, so neither form is a rotation
the loader's Rodrigues composition can carry. That is what the next section turns on.~~ **This was
the first wrong turn.** The reflection is already written down, in `scale`, and the loader
multiplies it in; there was never anything to fold into `rotate`.

~~### The correction that holds, and the one that does not

**`R * S` was wrong, and the probe that proposed it refuted itself.** `diag(-1, 1, 1)` has
determinant **−1**: it is a **reflection**, not a rotation. The half turn about `+Y` is
`diag(-1, 1, -1)`. A product of a reflection with anything also has determinant −1 and **cannot be
written as a `rotate` attribute at all**, because the loader's Rodrigues composition is always a
proper rotation. The probe printed both determinants as −1, which was the refutation sitting in its
own output and not the support it was read as.~~ **The determinant reasoning here is sound and the
conclusion drawn from it is not.** `S * R * S` is a proper rotation and *is* writable as a
`rotate` — that was never in doubt. What was wrong is that it was needed at all, and writing it
turned every camera away from its target.

~~**What holds is the same correction the other way round.** ... Then write **`rotate` as `S * M`**
and leave the `scale` alone.~~ **Withdrawn.** The wanted basis is reached by `scale` alone; the
rotation needs no change, and changing it is the regression.

~~Both the forward and the up must come from **`R`**, never from `S * R`: the scale negates `x` on
**every** column, so the up column of `S * R` is `S * (R * y)` and is not `R * y`. Taking one axis
from each matrix leaves them non-orthogonal, and `S * M` comes out with determinant **0.9818** —
which is how this probe's first attempt at the correction failed, and why the determinant is
printed rather than assumed.~~ **The premise is false.** The scale negates `x` on the local `X`
column only, so the forward and the up columns of `R * S` are `R`'s own.

~~### Measured end to end, on the document, with no source touched~~ **The measurement is
sound and the comparison is not.** `out/probe-camera-basis-fix.py` composes the document the way
this file said the loader did, which is not the way, so both of its rays are computed on a
composition that does not occur. Two probes agreeing on a wrong composition is not a
cross-check.

~~For `terraces` the change to the written `rotate` is **two signs**: ... | as corrected |
`159.2177, 0.2305913, **-**0.7615913, **-**0.6056454` |~~ **Those two signs were the regression.**

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

~~### The fault is general, and the correction holds on a second scene~~ **Withdrawn.** The
fault was general — every camera was wrong — but the fault was the correction's, not the
original's, and both scenes were turned by it rather than repaired.

| scene | as written before `a81b203` | as written after | what changed |
|---|---|---|---|
| `sunlight` | **59.597%** of the frame drawn | **10.372%** | the room left the picture |
| `foldyard` | the courtyard is in the frame | bare hillside | the courtyard left the picture |
| `dunes`, `landscape`, `terraces`, `tiles` | the 2026-10-12 census figures | **the same figures to the digit** | nothing |

**`dunes` 1.030 / 1.001 / 1.047 / 1.048, `landscape` 1.125 / 1.040 / 0.848 / 0.948, `terraces`
0.531 / 0.857 / 0.521 / 0.501, `tiles` 1.184 / black / 1.134 / 1.285 and `sunlight`
0.850 / black / 0.959 / 0.957 are the numbers this file recorded on 2026-10-12, and the revert
reproduces every one of them exactly.** A census that had improved by turning five cameras away
from their subjects was not an improvement.

**A file timestamp was used as proof of what a document contained, and it was not.** The sweep
that narrowed `sunlight`'s dark frame to the field of view ran against an export written five
hours before the camera work began; the two documents differ in the camera's rotation and in
nothing else.

The same treatment was applied to **`dunes`**, which is a different camera, a different sky and a
different world (`out/probe-camera-ray.py`, `out/probe-camera-basis-fix.py`):

| scene | as written | rotation alone | as corrected |
|---|---|---|---|
| `terraces` | **65.43°** from the raster's ray | 0.0015° | **0.000000°** |
| `dunes` | **51.63°** from the raster's ray | 0.0000° | **0.0000°** |

**Both cameras were turned, both elevations matched to two decimals and both eyes to five, and in
both the rotation on its own already aims correctly.** The fault is not one scene's framing: it is
every camera, by an amount that depends on where that camera points.

**An objection was raised here and withdrawn.** `dunes` exports its rotation with the axis's `y` and
`z` already negative while `terraces` exports them positive, which looked like the correction
turning one scene one way and the other the other. **The sign of the axis components is not the
marker.** `probe-camera-ray.py` on the corrected `dunes` document composes to **−117.10°, the
raster's ray to four decimals**, so the correction is right there too.

**And the frame correlation that seemed to show the correction breaking `dunes` was measuring
something else.** Whole-frame correlation against `now-dunes.png` fell from **0.8729 to 0.1365**
after the correction, which reads as a catastrophe — but `now-dunes.png` is the **graded**
rasteriser frame, and `dunes`' grade paints in screen space: its own source says the cloud shadow
*slides across the picture as the camera moves*. **A grade that depends on where the camera points
cannot be used to judge where the camera points.** The comparison that bears on the camera is the
raw lookup against the trace; the one that bears on the grade is a different question.

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

### The filter explanation was predicted, tested, and is wrong

The obvious remaining candidate was the **filter**: the rasteriser reads the map with `textureLoad`,
which takes the **nearest texel**, while Cycles samples it **filtered**, and on a sky with a bright
horizon band and a sun with a bearing the difference between those two is asymmetric by nature.
That is falsifiable, because Cycles' equirect filter width is fixed **in texels**: on a 256-wide
map one texel is 1.4 degrees, on a 4096-wide one it is 0.088, so **a finer map should bring the
tracer's filtered read closer to a nearest-texel read** and the residue should shrink.

The map was replicated sixteen times by nearest neighbour — a latlong grid is separable, so this
samples the same function of direction more finely and changes nothing the map says — and the
scene was re-traced (`out/probe-map-resolution.py`).

| map | ratio span | traced profile | correlation |
|---|---|---|---|
| 256 wide | −0.1404 | rises +0.0457 | −0.5945 |
| **4096 wide** | **−0.1401** | rises **+0.0452** | −0.5993 |

**Sixteen times the sampling density moved the residue by three ten-thousandths. It is not the
filter**, and the prediction this note made a commit ago is withdrawn.

The reference frame is sound and was checked rather than assumed: the rasteriser re-renders
`terraces` **byte for byte**, so `baresky.png` is a different image by design — the ungraded sky —
and not a stale one.

### What is left, and it is the measurement rather than the world

The candidate list is now short and each entry has been measured out: the camera, the projection,
the tone curve, the denoiser and the filter are all ruled out, and the reference frame is verified.
**What has not been checked is whether the two frames hold the same picture in the band every
comparison here reads**, and checking it has failed twice in two different designs.

Every figure in this investigation — the 0.58 to 0.86 sweep, the 1.2414 to 0.6980 sweep, the
corrected 1.4257 to 1.2853 — is read from **the top fifth of the frame on the assumption that it is
sky in both images.** That assumption has never been verified, and it cannot be verified from these
frames by either of the two detectors tried:

- a **level** detector keys each frame against its own top brightness, and the frames sit at
  0.6998 and 0.4831, so a dimly lit ground stays above its threshold and reads as unbroken sky. It
  returned **no skyline at all** for the corrected trace (`out/probe-skyline.py`);
- a **gradient** detector keys on the difference between adjacent rows, which no exposure can move,
  and it is unstable: **the same corrected trace against the same sky came back at row 434 and at
  row 316**, 118 rows apart, purely on which end the search started from
  (`out/probe-skyline-gradient.py`).

**The reason neither works is that these frames contain two horizontal edges and the terrain's is
an order of magnitude the weaker.** The map's bright horizon band is an edge in the sky and the
terrain against the sky is the other, and their measured strengths are not close:

| edge | strength |
|---|---|
| the map's horizon band, in the rasteriser's frames | **0.039 to 0.043** |
| the terrain silhouette, in a trace against a sky with no edge at all | **0.0040** |

**So the terrain is a ten-times weaker edge than the thing competing with it**, and no threshold on
one statistic separates them reliably. That is not a detector written badly; it is what the picture
is. It is also why the map resolution moved the answer so far — a sixteen-times finer map softens
the band's edge just enough to change which one a threshold prefers.

A trace against a **synthetic sky of two flat halves**, which has no horizontal edge in it at all,
puts the corrected camera's terrain at **row 507** and the uncorrected one at **none found at
all** — and the step it found is the 0.0040 above, barely over the threshold rather than clear of
it.

**So the residue is not yet attributable to anything**, because the window it is measured over is
not known to hold the same content in both frames. The camera's part is settled exactly, and the
camera was the fault. What remains is either a real second fault or an artefact of a comparison
whose window was never checked — and the two are not distinguishable until the window is.

**What would settle the window is a scene whose foreground contrasts**, because the terrain's own
edge is an order of magnitude too weak to find against this sky, and no amount of care with a
threshold changes that. A marker on each side would settle the orientation at the same time. Both
are scene code, and neither has been written.

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
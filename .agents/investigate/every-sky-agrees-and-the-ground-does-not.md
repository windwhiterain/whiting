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
| `terraces` | 0.531 | **0.857** | 0.521 | 0.501 |
| `lanterns` | not comparable — see below | | | |
| `monoliths` | not comparable — it has no world to compare against | | | |

## ~~Re-measured 2026-10-13 at `7ddcef6`, with the camera's frame mirror balanced~~

**The whole of this section described a regression and was read as a repair.** `a81b203` turned
every camera in the gallery away from its target by mirroring it in `x` about its own axis; the
figures below are that fault's, and the census that appeared to improve had improved **because
the cameras were wrong**. Reverted at `afb0c0b` — see
[do-the-backends-read-one-environment-the-same-way.md](do-the-backends-read-one-environment-the-same-way.md)
for why the loader's scale is local rather than world.

~~The camera was turned away from its target by an angle that depended on where the camera
pointed — 65.43 degrees on `terraces`, 51.63 on `dunes` — and the fix is described in ...~~ **There
was no angle to fix: `probe-camera-ray.py` composes the document the way the file said the loader
did, which is not the way.**

~~| `dunes` | 1.249 | **1.001** | 1.583 | 1.649 |~~
~~| `landscape` | 1.112 | **1.024** | 0.839 | 0.954 |~~
~~| `tidewater` | 0.886 | **1.059** | 0.745 | 0.680 |~~
~~| `foldyard` | 1.017 | **1.089** | 0.982 | 1.010 |~~
~~| `terraces` | 0.471 | **0.966** | 0.456 | 0.443 |~~
~~| `tiles` | 1.184 | both black | 1.134 | 1.285 |~~
~~| `sunlight` | median 0 | both black | median 0 | median 0 |~~
~~| `spotlight` | median 0 | both black | median 0 | median 0 |~~

**What improved.** ~~`terraces` sky 0.857 to 0.966 ...~~ **`terraces`' sky reads 0.857 again,
which is the number it read on 2026-10-12 and the one this file later withdrew for a different
reason. Nothing about it was ever the camera's.**

## Re-censused 2026-10-13 at `afb0c0b`, after the camera correction was undone

Eight scenes, both backends rendered from the current tree, the rasteriser and the tracer to
**separate** filenames. That separation matters: for a single-camera scene both backends write the
same name, and a census run that reuses one prefix silently compares the tracer's frame against
itself. Four scenes reproduce their 2026-10-12 figures **to the digit**.

| scene | whole | sky | lower 62% | ground | 2026-10-12 |
|---|---|---|---|---|---|
| `dunes` | **1.030** | **1.001** | **1.047** | **1.048** | identical |
| `landscape` | **1.125** | **1.040** | **0.848** | **0.948** | identical |
| `terraces` | **0.531** | **0.857** | **0.521** | **0.501** | identical |
| `tiles` | **1.184** | both black | **1.134** | **1.285** | identical |
| `sunlight` | **0.850** | both black | **0.959** | **0.957** | identical |
| `spotlight` | whole black | both black | 1.177 | 0.971 | identical on the two it has |
| `tidewater` | 0.886 | 1.059 | 0.745 | 0.680 | identical, at camera 0 |
| `foldyard` | 1.042 | 1.089 | 0.963 | 0.982 | sky identical, the rest within 3% |

**`sunlight` is no longer a scene that goes dark.** It reads 0.850 whole, 0.959 on the lower
frame and 0.957 on the ground — the figures it read before `a81b203` — and the traced frame
draws **59.597%** of the picture against 52.757% in the rasterised one, which is the room, the
three boxes and their shadows.

**`spotlight` is still black over more than half its frame**, and that is open: its lower band
and its ground read 1.177 and 0.971, so what agrees is the ground and what does not is the sky,
which it does not have.

**What has been checked about it, and what has not.** Four things were verified and all four came
back correct:

| checked | how | result |
|---|---|---|
| the camera | `out/probe-camera-ray.py` | ray is the rasteriser's to four decimals, azimuth −124.22, elevation −11.23, eye exact to six places |
| the sun's direction | `out/probe-light-direction.py` | **0.00 degrees** from the direction the scene declared |
| the geometry | the emitted document | the floor is the full ±8.5 quad at `z = 0` with all normals `+Z`, and all four boxes are present with their own transforms |
| the materials | the emitted document | every `principled_bsdf` carries the scene's own `base_color` |

**Five more were tried on the emitted document and each is ruled out**, one variable at a time
(`out/probe-document-edit.py`, which edits the artefact and leaves the repository alone):

| edit | result | what it rules out |
|---|---|---|
| the sun's `angle`, zero to 0.05 | **byte for byte the same frame** | a **delta** sun being degenerate |
| the four boxes removed | `10.37%` lit, unchanged | the boxes darkening the floor |
| the floor's two triangles swapped | **byte for byte the same frame** | the loader drawing only the first triangle |
| the floor split into two `<mesh>` elements | **Cycles crashes**, `0xC0000409` | — and a loader that will not read two meshes in one state |
| the sun aimed **straight down** | max **0.4667 to 0.7373**, mean 0.0388 to 0.0641, and `10.37%` lit **unchanged** | shadowing, **by anything in the frame** |

**The sun test is the sharp one.** Aimed straight down, nothing in the scene can cast a shadow onto
the floor, so the lit **area** must be the whole of it. It is not: the same **10.37%** of the
frame is lit before and after, while the **brightness** of that area rises by a factor of 1.6. **The
region is fixed by something that is not the light.** And it is the same 10.37% in every variant,
including the one with the boxes deleted, which is what makes it a fact about the frame rather
than about any of these edits.

~~**What the region is.** In the trace it is a triangle in the lower left bounded by a straight
diagonal — which is what the floor's **own two far edges** look like from a camera standing almost
on its corner, and beyond them there is nothing at all to draw.~~ **This was wrong, and it was
wrong by reading the picture instead of projecting the geometry.** The rasteriser's frame of the
same camera shows the floor filling the lower half with all four boxes standing on it, and the
rasteriser re-renders **byte for byte**, so that frame is current and authoritative.

## `sunlight`'s dark ground — **closed 2026-10-13 at `afb0c0b`**

**It was the camera correction `a81b203` that darkened it, and this section's whole chain of
measurement was taken against the damaged document.** The last line below concluded that the
camera was correct and the tracer was drawing the wrong fraction of the scene. **Both halves of
that were measured on the regression's own output**, and every probe that fed it composes the
document the way the code comment said the loader did rather than the way the loader does.

`cycles_camera_frame` negates the local `X`, the camera is otherwise left alone, and `sunlight`
renders the room again: **59.597%** of the frame drawn against 52.757% in the rasterised one,
whole 0.850, lower frame 0.959, ground 0.957 — the figures of 2026-10-12, to the digit.

**Nothing in this section was a property of `sunlight`.** It is kept, because the argument is the
shape of a whole class of mistake and the numbers are the record of it:

1. **the narrowing to the field of view was made on a stale document.** The
sweep that produced "lit area rises monotonically with the field of view — 0.00% at 0.55, 10.37%
at 0.95, 19.79% at 1.40" ran against `out/now-traced-sunlight.cycles.xml`, written at **11:48 on
2026-10-13**. The camera investigation began at **16:51 the same day** and the correction landed
in `a81b203` at **17:57**. Re-exporting from the current tree gives a camera whose rotation differs
in the sign of its second and third components:

```text
now-traced-sunlight.cycles.xml  rotate="158.48303192786693,  0.23142489197372607, -0.751891754173714, -0.6173340338751173"
out/ft-sunlight.cycles.xml       rotate="158.48303192786693,  0.23142489197372607,  0.751891754173714,  0.6173340338751173"
```

**Every figure in that sweep was measured on a camera that is now known to be aimed away from its
target, and a misdirected camera is exactly what makes lit area rise with the field of view**: widen
the cone and it sweeps across the room until it catches something lit. The sweep is not evidence
about the field of view. **A timestamp is not proof of what a file contains, and it was used as
one.**

### What is now established, each by its own measurement

| checked | how | result |
|---|---|---|
| the camera's **forward** | `out/probe-camera-ray.py` | the rasteriser's to four decimals, azimuth −124.22, elevation −11.23; eye exact to six places |
| the camera's **roll** | `out/probe-camera-roll.py` | **0.0000 degrees** on both sides |
| the camera's **frame-centre ray**, re-checked on the corrected document | `out/probe-camera-ray.py` | **0.00 degrees** apart |
| the **field of view** | `out/probe-camera-calibration.cycles.xml` | `fov` is the **full** angle, `1/tan(0.5 fov)`; the mapping is isotropic to 1.5% — [cycles-raster-mapping-is-calibrated.md](cycles-raster-mapping-is-calibrated.md) |
| **clipping planes** | `C:\resource\cycles\src\scene\camera.cpp:136-137` | the sockets are `nearclip` and `farclip`, defaulting to `1e-5` and **`1e5`**. A room-scale scene cannot reach a far plane a hundred kilometres off. **Dead on the numbers, no render needed.** |
| the **floor is drawn where it should be** | `out/probe-floor-edges.py` against the calibrated mapping | **no** — on the damaged document. The floor's far edge projects to `(0.372, 0.443)`–`(1.312, 0.539)`, so it should fill the frame below about `y = 0.45`; the damaged frame has it in 10.37% of the picture. **The projection was right all along and the camera was not where the projection said it was**, because the projection was built on the wrong composition. |

**Every row above is a true measurement of a broken document.** The camera's aim, roll, stand and
field of view were all checked and all agreed — because every one of those probes composes the
document with a world-side scale that the loader never computes, so they confirmed the same wrong
model together and looked like agreement. **Two probes that share a wrong assumption are not a
cross-check**, and that is what the three of them amounted to here.

### The sharpest falsification: the black is not a shadow

The floor was given an emission so that nothing about lighting could move it, and the frame came
back **unchanged** — `out/ft-emit2.png` against `out/ft-sunlight-traced.png`. Then the floor's
`base_color` was set to pure red, and the lit wedge turned pure red, so the wedge **is** the floor.

An emitting surface cannot be in shadow, and the sun aimed straight down cannot either — the
earlier sun test moved the brightness of the region by 1.6 and left its area at 10.37%. **So the
10.37% is where the camera finds the floor and the other 89.63% is where it does not.**

### Two silent no-ops worth recording, because both look like measurements

- **`clip_start` / `clip_end` are not socket names.** The camera's are `nearclip` and `farclip`,
  and `xml_read_node` iterates the node's own inputs and **skips any attribute matching none**
  (`src/graph/node_xml.cpp:53-65`). An edit writing the wrong name produces a document that reads
  back correctly and a render identical to the baseline, which is indistinguishable from
  "clipping is not the cause".
- **`emission_color` alone emits nothing.** The closure needs `EmissionStrength != 0` as well and
  that defaults to **zero**. An edit writing only the colour produced a **byte-identical** render —
  the same trap, reached from the other side.

Neither was caught by looking. Both were caught by **hashing the output against the baseline**,
which is the only reason they are known at all.

### What is open

~~Why the tracer draws 10.37% of a floor the calibrated projection says fills the lower half. Not
yet checked: the `viewport_camera_border` term ...~~ **Closed by the revert.** The tracer draws
59.597% of a floor the calibrated projection says fills the lower half, which is the room. The
question was never a question about the floor: the projection and the camera disagreed because
the projection was composed the wrong way round, and it agreed with the camera because both were
computed the same wrong way.

`tiles` reproduces its figures to the digit because its rotation is a half turn about an axis in
the `YZ` plane, which **commutes** with `diag(-1, 1, 1)` — so the mirror is provably a no-op there,
and its being unchanged through both the correction and the revert is a check on the reasoning
rather than a coincidence.

**re-measured 2026-10-13 at `a1397e3`, after `tidewater`'s air moved into its post stage.** every
row not touched this session reproduces the 2026-10-12 figures to the digit — `dunes` 1.048,
`landscape` 0.948, `sunlight` 0.957, `spotlight` 0.971, `tiles` 1.285 — and the two that changed are
`tidewater` at **0.699 → 0.680** and `foldyard` at 0.982 unchanged. `terraces` is **new to the
table**: it became an environment-light scene when its sky law was given a map, and it is now the
worst-matched scene in the gallery on both halves of the frame.

**the sky column above is now wrong as a sentence, and `terraces` is why.** "the four scenes that
declare an environment light read 1.001, 1.040, 1.059 and 1.085" was true when it was written and
there were four of them. There are six now. **`terraces`' 0.857 is not a sky measurement in the
first place**: it paints a cloud deck, three ranges and the valley's fog into its sky, and
[the-raster-sky-band-is-not-the-tracers-sky-band.md](the-raster-sky-band-is-not-the-tracers-sky-band.md)
established that a sky figure is only meaningful for a scene that paints nothing there — of the
converted scenes, `dunes` is that one. Decomposed, `terraces`' sky is the map alone at **0.781**,
0.764 with the clouds and ranges added, and 0.857 once the fog is on top of it. **The number to
reason about is 0.781**, and that one is a lookup question rather than a composition one:
[do-the-backends-read-one-environment-the-same-way.md](do-the-backends-read-one-environment-the-same-way.md).

**one radiance can now be both the sky and the ground, which it could not on 2026-10-12.** The
asymmetry recorded in
[one-radiance-cannot-both-sky-and-ground.md](one-radiance-cannot-both-sky-and-ground.md) — the
rasteriser's post stage calling an analytic sky that reads no light, so its sky and its ground were
two independent quantities — **has been resolved**: every scene that declares an environment light
draws its sky from the map it declares, scaled by the declared radiance, which is what the tracer's
world is. That note is now the record of the asymmetry rather than of the code.

**a `shape` column was added to the measurement on 2026-10-13, and it should have been there from
the start.** a ratio says how bright and a correlation says how shaped, and the two fail
differently: a constant ratio across a region is a calibration one curve could fix, and a region
whose ratio moves from row to row is a model disagreement no exposure will touch. `terraces`' sky
reads 0.857 at a shape of **+0.8732** — close, and not close enough — and its ground reads 0.501 at
a shape of **+0.0375**, which is the per-fragment albedo story and no calibration at all. Reading
the ratio alone would have called both the same kind of thing.

**corrected 2026-10-13, `foldyard`, re-measured after the albedo was applied once.** the table
above read the ground at `0.274` and the whole frame at `0.342`; the cause was the scene's ground
body multiplying by `in.tint` a second time, which made the rasterised ground the tint squared
against a tracer that is handed the tint once. it now reads:

| scene | whole | sky | lower 62% | ground |
|---|---|---|---|---|
| `tidewater`, after | 0.886 | 1.059 | 0.745 | **0.680** |

**the other scene with air in the wrong place is `monoliths`, and it has no row in this table at
all.** it declares no environment light, so the tracer draws it black — traced sky median
`0.0000`, ground `0.0039` — while its raster paints a sky of post-only literals. a ratio of
`85.4` on the ground is a measurement of the absence of a world, not of agreement, which is why
it is not here and why the "both black" rows above are the ones that actually say something.

so the worst whole-frame scene in the table above became the best matched, and the only reason to
look at this table again is that its other rows may hide the same shape of mistake.

## Every ground row accounted for, 2026-10-13

the four rows that were not near one, each traced to a cause that is a decision rather than a
mystery. **two of the three are limitations the format already has; one was a bug and is fixed.**

| scene | ground | what it is |
|---|---|---|
| `dunes` | 1.048 | within two percent of the frame; nothing to explain |
| `landscape` | 0.948 | likewise |
| `sunlight` | 0.957 | likewise |
| `spotlight` | 0.971 | likewise |
| `tiles` | 1.285 | **the material image the tracer cannot be handed.** the raster's body is `color * (0.25 + 1.5 * pattern)` and the exporter reports the image and exports without it (`LossyMode::Degrade`), so the ratio is exactly that multiplier — the measured 1.285 says the pattern averages **0.69**. recorded at `cycles_backend.md`, materials. |
| `terraces` | 0.499 | **the albedo is decided per fragment** and a tracer is handed the material row, which says white. structural. |
| `tidewater` | 0.699 → **0.680** | **the air was in the wrong place, and moving it cost brightness.** the shore mixed toward `aerial_colour` at the lighting stage and the lake at the emission stage, by a law whose `slant = 1 + facing * 2.4` was worth up to 3.4x at grazing incidence. the post stage applies the real integral instead, which is about 1x on a horizontal ray, so the ground band fell. it was still **no discrete cause** for the residual: both bands' linear medians are near `0.02`–`0.03`, so this is a statistic over a very dark region rather than a level that is wrong by half. |
| `foldyard` | 0.274 → **0.982** | **the bug.** the ground body applied `in.tint` a second time, and the tint is `0.232`. |

re-measured 2026-10-13 against the current branch, all rows reproducing the 2026-10-12 figures to
the digit, which is the check that the table above is still about the code that is on `main`.

**Where both backends have a sky, they now hold the same radiance.** The four scenes that declare
an environment light read `1.001`, `1.040`, `1.059` and `1.085` — from a scene that was at `0.111`
and one that was at `2.817` an hour earlier. The four that declare none have a black sky on **both**
sides, which is the correct agreement: the tracer draws black for a scene with no world, and so
does the raster post stage once it stops painting one of its own.

**The ground does not follow, and two scenes are the reason.** `foldyard` at 0.274 and `tidewater`
at 0.699 were the two scenes carrying air the tracer cannot have. A path tracer draws no fog
unless it is given a volume, and the exporter carries none, so a scene that paints haze has
painted something the other backend cannot reproduce. That is the same class of difference as a
painted ridge silhouette, sanctioned by a decision that is otherwise right.

**Corrected 2026-10-13: the two were not the same case, and this paragraph said they were.**
`foldyard` painted its air in its **post** stage, over geometry, from a world position recovered
from the depth buffer (`foldyard.rs:1626`). `tidewater` painted its air in its **bodies** — the
shore at the lighting stage and the lake at the emission stage — and its post stage touched only
the sky, under `if depth >= 1.0`. Reading the code rather than the summary is what caught it: the
two scenes were both "the ones that paint air" and neither was in the other's place. Under the
stage rule decided on 2026-10-13 both are now in the post stage, and `tidewater`'s ground reads
**0.680**. The direction of that move was predicted in advance and it fell: the old law's
`slant = 1 + facing * 2.4` was worth up to 3.4x at grazing incidence, and an integral along the
line of sight is worth about 1x on a horizontal ray.

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
- The `sunlight` re-opening of 2026-10-13: `out/probe-camera-roll.py`,
  `out/probe-floor-edges.py`, `out/probe-regions.py`, `out/probe-silhouette.py`,
  `out/probe-channels.py`, and the edits `clip`, `emitfloor`, `floorcolor`, `noboxes` in
  `out/probe-document-edit.py`. Artefacts `out/ft-sunlight.cycles.xml`,
  `out/ft-sunlight-traced.png`, `out/ft-emit2.png`, `out/ft-red.png`, `out/ft-nobox.png`,
  `out/probe-camera-calibration.cycles.xml`, `out/calib.png`. The raster mapping these rest on is
  [cycles-raster-mapping-is-calibrated.md](cycles-raster-mapping-is-calibrated.md).
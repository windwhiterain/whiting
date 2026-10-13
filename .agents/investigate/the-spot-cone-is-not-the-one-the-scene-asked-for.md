# The spot cone, in both backends, was not the cone the scene asked for

Measured 2026-10-14, Cycles 5.3.0 at `C:\resource\cycles\install\cycles.exe`, against
`C:\resource\cycles` at `a456b761034dda42c32eef9f4aae0fa5a5c9f604`. Probes: `out/probe-spot-cone.py`
(arithmetic on the two formulas) and `out/probe-spot-profile.py` (the two formulas measured off two
renders). Documents: `out/rz-spotlight.cycles.xml` (what the emitter wrote), and
`out/sc-b0-written.cycles.xml` / `out/sc-b0-corrected.cycles.xml`, which are that document with one
number changed and the bounce depth set to zero.

## The question

`spotlight` is the one gallery scene lit by a cone, so it is the only place a spot light's falloff can
be read at all. Its cone did not fall off the way the document asked. A cone is two numbers, and the
question turned out to have to be asked of each of them separately **and of each backend
separately**, which is why this file is longer than the defect was: the band was wrong in the
emitter, and the shape of the ramp was wrong on both sides.

## What the two renderers compute

Both ramp from nothing at the outer half angle to full at the inner one, and both compare a cosine
against two cosines. They differ in **what the ramp is a fraction of**, and in **the shape of the
ramp**.

Ours, `crates/gallery/src/shaders.rs:1908-1925`:

```wgsl
let facing = dot(to_surface, cone.axis.xyz);
if facing <= outer { return 0.0; }
if facing >= inner { return 1.0; }
return (facing - outer) / (inner - outer);
```

A **straight line in the cosine**, over the band `cos(outer) .. cos(inner)`.

Cycles, `src/kernel/light/spot.h:27-31` with `src/scene/light.cpp:177-180`:

```c
return smoothstepf((ray.z - spot->cos_half_spot_angle) * spot->spot_smooth);
...
const float spot_smooth = 1.0f / ((1.0f - cos_half_spot_angle) * smooth);
```

A **smoothstep**, over the band `cos(outer) .. cos(outer) + (1 - cos(outer)) * smooth`.

## What the emitter writes

`crates/scene/src/cycles_export/light.rs:363-375`:

```rust
attributes.push(("angle", format!("{}", outer * 2.0)));
attributes.push(("smooth", format!("{}", ((outer - inner) / outer).max(0.001))));
```

`angle` is right: the document states half angles and the loader takes the whole cone.

**`smooth` is wrong, and it is wrong in kind rather than by a margin.** The kernel reads it as a
fraction of `1 - cos(outer)` — a band measured in the **cosine**. The emitter writes
`(outer - inner) / outer`, a fraction of the **angle**. The two are the same only in the limit where
the cone is narrow, because `cos(inner) - cos(outer) ~ (outer - inner) * sin(outer)` and
`1 - cos(outer) ~ outer^2 / 2`, and their ratio is `(outer - inner) / outer` only while `outer` is
small. **A narrow cone hides the mistake, which is why it survived.**

For `spotlight`, whose two half angles are 0.16 and 0.56 radians:

| | value |
|---|---|
| inner, outer | 9.17 and 32.09 degrees |
| what the emitter writes, `(outer-inner)/outer` | **0.714286** |
| what the kernel wants, `(cos inner - cos outer)/(1 - cos outer)` | **0.916379** |
| ratio | **1.2829** |
| where Cycles reaches full strength with what it was given | **16.99 degrees** off axis |
| where the scene asked for full strength | **9.17 degrees** off axis |

**Cycles' full strength core is 1.85 times as wide as the document's**, and its soft band is
correspondingly thinner. The largest disagreement between the two curves is **+0.2767** at 20 degrees
off axis — Cycles 0.937 where ours is 0.660.

## That it is the render, and not only the arithmetic

The arithmetic above is a claim about a formula. It was checked by rendering the same document twice
with **one attribute changed**, both at `max_bounce="0"` so that indirect light cannot fill the rim
of the cone and flatten the very thing being measured:

- `out/sc-b0-written.png` — `smooth="0.7142857142857143"`
- `out/sc-b0-corrected.png` — `smooth="0.9163789082272744"`

`out/probe-spot-profile.py` walks the camera's rays onto the floor plane, works out each floor
point's angle off the cone axis, and takes the median pixel in each one degree bin. The rasterised
frame is ungraded first (`c/(c+1)` then `pow(c, 1/0.4545)` inverted), and each curve is normalised to
its own value on the axis, because this build of Cycles writes scene-linear radiance straight into the
png and the two are not on the same scale.

Root mean square gap to the rasterised curve over 10 to 30 degrees off axis:

| | RMS |
|---|---|
| Cycles, with the number the emitter writes | **0.2744** |
| Cycles, with the number the kernel wants | **0.1721** |

A 37 per cent reduction from one number. The median profile agrees: at 20 degrees off axis the
written build reads 0.736 and the corrected one 0.569, against 0.350 in the rasterised frame.

## The control that made the number mean something

Comparing a render against a formula only says anything if the formula is the whole of what was
rendered. The first version of this comparison was not, and it produced a finding that was wrong.

A ring of a pool is **not one distance and not one angle of incidence**. The lamp stands 8.5 above
the floor, so along the ring the far side is both further away and more oblique, and a Lambertian
floor loses light to both: the predicted brightness goes as `cone(theta) * N.L / r^3` per pixel,
with `N.L = 8.5 / r`. Binning by angle and dividing only by `r^2` leaves the incidence term in, and
a curve that still carries it looks steeper than the cone alone.

With the two terms put back per pixel and binned exactly as the images are:

| | RMS against the measured rasterised curve, 10 to 30 degrees off axis |
|---|---|
| **predicted from `shaders.rs`, the rasterised path's own formula** | **0.1522** |
| **Cycles measured, with the corrected number** | **0.1721** |
| predicted from `spot.h`, with the corrected number | 0.1763 |
| predicted from `spot.h`, with the number as written | 0.2744 |
| **Cycles measured, with the number as written** | **0.2744** |

Three things follow, and the first two are the load-bearing ones.

**The traced render is following the kernel formula exactly.** Measured against the written number
it scores 0.2744 and the formula predicts 0.2744; measured against the corrected number it scores
0.1721 and the formula predicts 0.1763. The document and the render and the source agree to four
figures either way, which is what makes the corrected number a measurement rather than a reading.

**The rasterised path is drawing the cone its schema asks for.** Its own formula is the closest of
the four curves to its own image at 0.1522. **There is no second defect on this side**, and the
first version of this note said there was — it reported the rasterised curve as 0.2718 away from
its own model, which was the missing incidence and inverse square and nothing else.

**What is left is a shape, and it is smaller than the defect that was just fixed.** The residual
0.1522 is not the shadow page: inside the 26 to 30 degree band the rasterised brightness falls
smoothly and monotonically from 8.5 to 13.75 metres from the lamp with no step at any distance, and
the only cut anywhere — in the 22 to 26 degree band at about 12 metres — appears in both images and
is the cone's own edge. It is not the cone axis either, which is normalised at
`crates/scene/src/standard/emit.rs:1651-1657`. It is not the two angles, the band, or the angles'
cosines. **It is unexplained, and it is about half the size of the emitter defect that was fixed
here**, which is the honest way to leave it.

## What it did to the shipped picture

Re-censused 2026-10-14 on `main` at `dacf474`, `spotlight` only, because `spot()` is called from
one place in the gallery — `crates/gallery/src/spotlight.rs:223` — and the correction is behind
`light.kind == "spot"`, so no other scene can reach it. The other seven are unaffected by
construction, which is a fact about the code rather than a claim about a render.

The cross-check first: the document the emitter now writes renders **bit for bit identical** to
`out/sc-corrected.png`, the one built by hand from the old document with only `smooth` edited.
Zero one-sided pixels, median ratio exactly 1.0000. Nothing else in the emitter moved.

| | before | after |
|---|---|---|
| pixels only the tracer draws | 3.44% | **0.00%** |
| pixels only the rasteriser draws | 3.27% | **1.62%** |
| shape, pearson over the drawn region | +0.8846 | **+0.9844** |
| the probe's own verdict | *a difference of geometry* | *the same region to within two per cent* |

**The tracer's pool is now entirely inside the rasteriser's.** Before the correction it drew a rim
the rasteriser did not, in every direction, which is what a cone that is too wide at its rim and too
narrow at its core looks like from above.

On the floor, in raw radiance, against the rasterised frame's own values:

| degrees off axis | rasteriser | tracer before | tracer after |
|---|---|---|---|
| 0 | 0.6317 | 0.6458 | 0.6484 |
| 16 | 0.3675 | 0.5778 | 0.4915 |
| 20 | 0.2248 | 0.5242 | 0.3595 |
| 24 | 0.0704 | 0.4444 | 0.2039 |
| 28 | 0.0047 | 0.3229 | 0.0562 |

**The gain on the axis was never wrong** — the two agree to two per cent there, before and after.
The correction moves the ring only, and it moves it toward the rasterised frame at every angle.

## The census's `level` number cannot be read as the light's level

The census reports a level over the union of the two drawn regions, and on `spotlight` that number
went from 0.7131 to 1.4054 — which reads as the correction overshooting, and is wrong.

That region is not the pool. It is every pixel either backend drew, which here is dominated by
three coloured boxes and by the shadows they cast, not by the floor the cone is measured on. The
floor is where the numbers above come from, and there the tracer is brighter than the rasteriser at
**every** angle, before and after. A median over a region that includes brightly lit box faces and
deeply shadowed floor is a statement about which of those there are more of, not about the light.

**`out/probe-level-over-shared.py` is here because of it**: it reports the level over the shared
region and over the union separately, so a change that moves one backend's edge cannot silently move
the median as well. Read the pool, not the frame.

## The ramp, and what it cost to measure

Taken as `3bfccfd`: `cone_attenuation` ramps with `t * t * (3 - 2t)` over the same band, in both
copies of the function — `crates/gallery/src/shaders.rs` and the example scene's own lighting
shader in `crates/scene/src/standard/example.rs`. That is what the kernel does, and it is the whole
of the difference.

**On the floor, by angle**, every bin now lands within 2 to 7 per cent of the tracer, against 1.6
times off at 20 degrees and 2.9 times off at 24 before.

**Over the frame, like for like** — against `max_bounce="0"` so the tracer is direct only, and with
the rasteriser's grade taken off so both sides are radiance:

| | linear ramp | smoothstep |
|---|---|---|
| pearson | 0.2263 | **0.9994** |
| pixels only the tracer draws | 6.92% | **0.04%** |

**The census's own shape figure reads the other way** — 0.9838 before, 0.9241 after — and it should
not be read as a comparison of lighting. The rasterised frame is graded and this build of Cycles
writes scene-linear radiance straight into the png. A correlation is blind to a **gain** and not
blind to a **curve**, so correlating a graded frame against an ungraded one measures the grade as
much as it measures the light, and here the grade flatters the old frame and penalises the new one.

**That qualifier reaches back.** The 0.9844 recorded above with the emitter fix is a graded
correlation and is not a lighting comparison either. What survives it is the **one-sided-region
count**, which is a threshold test on two images and not a correlation between them — and that
number, 3.44% to 0.00%, still stands.

`out/probe-agreement-ungraded.py` prints both, so the two definitions can be read against each
other instead of one of them being quietly chosen.

## What the change does not reach

The rasteriser has no indirect light and the tracer has eight bounces, so the tall blue box is lit
in the traced frame and is not in the rasterised one, and its top is the only part of the picture
where the two still disagree visibly. **That difference predates this work and is not a cone
question.** It is the next thing on this scene, and it is a much larger one: it is the difference
between a renderer that draws direct light and one that draws light transport.

## What is left

The residual against the rasterised path's own formula is no longer a meaningful number, because
the formula and the render are now the same curve by construction — which is the point of the
change, and the reason the earlier 0.1522 cannot be compared with anything after it.
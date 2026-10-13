# The spot cone, in both backends, was not the cone the scene asked for

Measured 2026-10-14, Cycles 5.3.0 at `C:\resource\cycles\install\cycles.exe`, against
`C:\resource\cycles` at `a456b761034dda42c32eef9f4aae0fa5a5c9f604`. Probes: `out/probe-spot-cone.py`
(the arithmetic), `out/probe-spot-profile.py` (the profile off two renders),
`out/probe-agreement-ungraded.py` (agreement, both sides on the same scale).

## The question

`spotlight` is the one gallery scene lit by a cone, so it is the only place a spot light's falloff can
be read at all. Its cone did not fall off the way the document asked. A cone is two numbers, and the
question had to be asked of each of them **and of each backend separately**: the band was wrong in
the emitter, and the shape of the ramp was wrong on the rasterised side.

## What the two renderers compute

Both compare a cosine against two cosines, and both ramp from nothing at the outer half angle to full
at the inner one. They differ in **what the ramp is a fraction of**, and in **the shape of the ramp**.

Cycles, `src/kernel/light/spot.h:27-31` with `src/scene/light.cpp:177-180`:

```c
return smoothstepf((ray.z - spot->cos_half_spot_angle) * spot->spot_smooth);
...
const float spot_smooth = 1.0f / ((1.0f - cos_half_spot_angle) * smooth);
```

A **smoothstep**, over the band `cos(outer) .. cos(outer) + (1 - cos(outer)) * smooth`.

## What the emitter writes, and why `smooth` is wrong in kind

`crates/scene/src/cycles_export/light.rs` wrote:

```rust
attributes.push(("angle", format!("{}", outer * 2.0)));
attributes.push(("smooth", format!("{}", ((outer - inner) / outer).max(0.001))));
```

`angle` is right: the document states half angles and the loader takes the whole cone.

**`smooth` is wrong, and wrong in kind rather than by a margin.** `2 pi (1 - cos t)` is the solid
angle of a cone of half angle `t`, so the kernel's `1 - cos(outer)` means `smooth` is the share of
the outer cone's **solid angle** the band occupies. The emitter wrote the share of the **angle**.
`cos(inner) - cos(outer) ~ (outer - inner) sin(outer)` while `1 - cos(outer) ~ outer^2 / 2`, so the
two agree only while `outer` is small. **A narrow cone hides the mistake, which is why it survived.**

Blender's own manual says the same thing about its `Blend` slider: the value is "that amount of space
that the inner cone should occupy inside the outer cone Size". Space, not angle.

For `spotlight`, half angles 0.16 and 0.56 radians:

| | value |
|---|---|
| inner, outer | 9.17 and 32.09 degrees |
| written, `(outer-inner)/outer` | **0.714286** |
| wanted, `(cos inner - cos outer)/(1 - cos outer)` | **0.916379** |
| where Cycles reaches full strength with what it was given | **16.99 degrees** off axis |
| where the scene asked for full strength | **9.17 degrees** off axis |

Taken as `dacf474`. **How much the band was worth:** the same document rendered twice with only that
one number changed, both at `max_bounce="0"`, correlates **0.2648** with the grade taken off. The
band was not a rounding.

## The ramp, taken as `3bfccfd`

`cone_attenuation` now ramps with `t * t * (3 - 2t)` over the same band, in both copies —
`crates/gallery/src/shaders.rs` and the example scene's own lighting shader in
`crates/scene/src/standard/example.rs`. That is what the kernel does.

`t * t * (3 - 2t)` has derivative `6t(1 - t)`, zero at both ends of the band, where a straight line
has a constant slope. That buys two things in a picture: **no bright ring just inside the cone's rim**,
and **no ring at the edge of the full strength core**. Neither is geometry and neither is a shadow, so
both collapse under a threshold sweep, which is what tells them apart from geometry.

## Where the two backends stand, measured on files whose roles were hashed

| | pearson, grade off | pixels only the rasteriser draws |
|---|---|---|
| before: linear ramp, wrong band | 0.9667 | 2.36% |
| after: smoothstep, right band | **0.9994** | **0.26%** |

And on the floor, by angle, against the tracer's **direct** light:

| degrees off axis | rasteriser | tracer | gap |
|---|---|---|---|
| 0 | 0.61821 | 0.61830 | 0.01% |
| 12 | 0.55515 | 0.56209 | 1.2% |
| 20 | 0.34702 | 0.35163 | 1.3% |
| 28 | 0.05261 | 0.05490 | 4.2% |

**Within about one per cent across the core and the middle of the band.** Against the tracer at eight
bounces the gap is 3 to 5 per cent, which is the indirect light the rasteriser does not have.

## What the change does not reach

The rasteriser has no indirect light and the tracer has eight bounces, so the tall blue box is lit in
the traced frame and is not in the rasterised one, and its top is the only part of the picture where
the two still disagree visibly. **That difference predates this work and is not a cone question.** It
is the difference between a renderer that draws direct light and one that draws light transport, and
it is much larger than anything on this page.

## How to read a comparison between these two backends

Three of them, and they do not agree, which is the whole of the difficulty.

- **As written** — a graded rasterised frame against this build of Cycles' scene-linear radiance —
  measures the grade as much as the light. A correlation is blind to a gain and not blind to a
  curve, and the same pair can score 0.9838 graded and 0.2793 ungraded.
- **Ungrading the rasterised frame instead** is ill-conditioned. `c / (c + 1)` saturates: a display
  value of 0.95 comes back as 5.7 and 0.99 as 456. An eight bit png of a tone mapped frame does not
  carry the scene's radiance near white, so inverting it turns quantisation into radiance. Measured
  over the lower 62 per cent of `spotlight` it produced a ratio of **34.7**.
- **Grading both sides** is well conditioned, because `grade` is monotone and keeps both images off
  the saturation that makes the inverse blow up. That is what `out/probe-census-both-graded.py`
  does.

## Withdrawn, and why

**Every figure below was taken from two files that were both traced renders**, because the `rz-`
prefix in `out/` holds the **traced** frame and the `rr-` prefix the **rasterised** one — the
opposite of what the two letters suggest. That was found by hashing each file against the earlier
`now-` / `now-traced-` pair, whose names say what they are. The probes had the roles reversed, and
one of them also put the rasteriser's inverse tone map on a frame that was already linear radiance.

Withdrawn, all of them:

- the table reading **0.1522 / 0.1721 / 0.1763 / 0.2744 / 0.2744**, and with it the claim that the
  traced render follows the kernel formula "to four figures". Two traced frames were being compared
  against a cone model, one of them through an inverse tone map.
- the claim that **"the rasterised path is drawing the cone its schema asks for, there is no second
  defect on this side"**. It happened to be right, but it was argued from a table that was not a
  measurement of the rasterised path at all.
- the floor table reading **0.6317 / 0.6458 / 0.6484**, whose first column was a traced frame.
- **0.2263 to 0.9994** as the before and after of the ramp change. The before was measured against a
  bounced traced frame and a bounce-free one. The real pair is **0.9667 to 0.9994**.

**The historical gallery census is retired rather than reconciled.** Its figures were produced by a
measurement whose region definitions are written down nowhere, on files whose roles were recorded
nowhere, with one side graded and the other not. There is nothing to compare the new numbers with, and
a table that cannot be re-run is not a baseline.

## What the census looks like now

`out/probe-census-both-graded.py`, every pair hash-checked on each run and refusing to compare a file
with itself. Median ratio is traced over rasterised, both graded:

| scene | whole | shape | lower 62% | shape |
|---|---|---|---|---|
| dunes | 0.984 | 0.8670 | 0.980 | 0.8777 |
| landscape | 0.843 | 0.9112 | 1.033 | 0.8717 |
| terraces | 0.948 | **0.6633** | 0.948 | **−0.0014** |
| tiles | 0.782 | 0.8600 | 0.999 | 0.8600 |
| sunlight | 0.895 | **0.2588** | 0.892 | 0.2895 |
| spotlight | — | 0.9410 | 0.498 | 0.9445 |
| tidewater | **0.556** | 0.6363 | 0.579 | 0.4627 |
| foldyard | 0.924 | 0.8669 | 0.988 | 0.6135 |

**`sunlight` and `terraces` are now the two worst and neither has been looked at.** A shape of 0.26
and a shape of −0.0014 mean the two backends are drawing the same *levels* and almost nothing alike,
which is a different and much larger question than a level being out by ten per cent.
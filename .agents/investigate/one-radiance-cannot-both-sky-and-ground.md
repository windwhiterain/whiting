# Why one environment radiance cannot align both the sky and the ground

**resolved 2026-10-13, and this note is the record of the asymmetry rather than of the code.** the
diagnosis below describes a state that no longer exists: option 2 at the end of it **was taken**.
Every scene that declares an environment light now has its post stage draw its sky from the map it
already declares — `dunes`, `landscape`, `foldyard`, `tidewater` and then `terraces` — and both sides
therefore agree that **the sky is the world** and that one `radiance` sets it. The numbers in the
diagnosis are kept because they are what the asymmetry was worth, and a finding nobody can check is
a finding nobody should act on.

**what changed, so the reader does not have to guess which sentences are history.** the rasteriser's
post stage used to call `air_sky`, which reads the scene's atmosphere constants and **no light at
all**; it now calls `environment_sky`, which reads the declared map scaled by the declared radiance,
and the tracer's world is that same map scaled by that same number. Two independent quantities
became one. The mapping is written down under `brightness_field` in
[cycles_backend.md](../../SPEC/render/cycles_backend.md#a-lights-brightness-is-read-through-brightness_field-and-there-is-no-second-read),
where `foldyard`'s `0.26` is recorded as **the world's radiance** rather than as a fill strength —
which is what it always was, and what a comment describing only the fill had hidden.

**two statements below are therefore stale and are corrected in place**: the line saying `terraces`
and the other scenes without a world "are unaffected by this finding and still need one" — `terraces`
has a world as of 2026-10-13 — and the diagnosis's own "the rasteriser draws its sky from `air_sky`",
which was true when measured and is not now.

Measured 2026-10-12, on `landscape`, after `b4adcb5` made every environment light's declared
radiance reach the tracer for the first time. This is the finding behind the recalibration
work and it is why the recalibration is not a sweep.

## The question

`dunes` and `landscape` both declare an environment light over their own atmosphere. Both were
swept against their own rasterised sky. They came out with the optimum in **opposite
directions**:

- `dunes`: matching the sky improved the whole frame, -0.6585 → +0.2667.
- `landscape`: matching the sky made the whole frame **worse**, +0.5612 at radiance 0.55 down
  to +0.0807 at the radiance that matches the sky exactly.

A radiance is one number acting on one picture. If the optimum reverses, the reversal is in the
picture, not in the number.

## The answer

**The two backends do not use the number for the same thing.**

The rasteriser draws its sky in the post stage from `air_sky`, which reads the scene's
atmosphere constants and no light at all. Its ground is lit by the environment light. So on
that side the sky and the ground are **two independent quantities**, and only one of them is
the environment light's to set.

The tracer's sky **is** the world, and the same `strength` that scales that world is what
lights its ground. So on that side they are **one quantity**.

## The measurement

`landscape`, both sides re-rendered at each value rather than holding one still — the
rasterised frame depends on the radiance too, and sweeping against a fixed reference would
have measured a moving target against a still one.

| radiance | raster sky | raster ground | traced sky | traced ground |
|---|---|---|---|---|
| 0.55 | 142.66 | 175.21 | 61.87 | 120.53 |
| 1.24 | 142.66 | 182.30 | 139.48 | 145.56 |

What moved when the radiance went from 0.55 to 1.24:

| | change | what it is on that side |
|---|---|---|
| rasterised sky | **+0.00** | painted by the post stage |
| rasterised ground | **+7.08** | lit by the environment light |
| traced sky | **+77.61** | the world, scaled |
| traced ground | **+25.02** | lit by the same scaled world |

**A rasterised sky that does not move under a number that moves its ground is the whole
finding.** The two regions on that side are not one quantity, so the tracer's single world
strength cannot reproduce them: it is being asked to hit a moving target (the ground) and a
still one (the sky) with one setting, and which one it misses is decided by which is bigger
in the frame.

## Why `dunes` looked like it worked

It did work, and its number is the sky-matched one — but `dunes` is a frame where the sky is
most of the picture and the ground is a narrow band, so hitting the still target happened to
be close to hitting both. `landscape` is the same mechanism with the proportions reversed:
the sky is the top third and the ground is the rest, so the moving target dominates and the
still one gets abandoned.

**Neither scene has a radiance that is right.** They have a radiance that is right for whichever
region dominates their frame, and the sweep found that one, which is not the same as finding a
number that makes the scene agree.

## What would actually fix it

Not a better number. The two sides need the sky and the ground to be separately controllable,
which means one of:

1. **A radiance for the world's visible colour and a separate irradiance for what it lights.**
   This is what the rasterised side already has structurally — the sky is not the
   environment light's to set — and Cycles' own model supports both: `<film exposure>` is a
   multiplier on everything, while the world's own strength sets what it emits. The scene
   schema would need a second field, and the emitter would need to put it somewhere other than
   the one socket it has.

2. **Let the rasteriser draw its sky from the environment map it already declares**, so the two
   sides agree that the sky is the world. `landscape`, `foldyard` and `dunes` all already ship
   an `Atmosphere` and an `air_sky` that is term-for-term the same law; the post stage is
   re-deriving a sky the scene already states. That would make the sky one quantity on both
   sides, and the remaining disagreement would be the tone curve, which is a separate and
   already-documented matter.

Option 2 is the smaller change and it attacks the actual asymmetry. Option 1 is more general
but adds a field to the schema for a difference that one of the two backends should not have in
the first place.

## What this does not settle

- **The tone curve is still a separate difference.** With the sky matched by construction,
  `dunes`' ground still carries 99.81 against 152.94 and spans 29 levels of luma against the
  tracer's 112. That is [no view transform](../../SPEC/render/cycles_backend.md#no-view-transform)
  and no radiance reaches it.
- **`terraces` and the other scenes without a world** are unaffected by this finding and still
  need one. **Stale as of 2026-10-13:** `terraces` declares a world, and the scenes that declare
  none draw no sky on either side, which is the correct agreement rather than a gap.
- **No recalibrated radiance is committed.** `landscape`'s 1.24 and `foldyard`'s 1.30 are
  measured and they are the sky-matched values, but until the asymmetry above is decided they
  are the wrong shape of answer for `landscape` and this work stops here rather than committing
  a number chosen by the wrong rule.

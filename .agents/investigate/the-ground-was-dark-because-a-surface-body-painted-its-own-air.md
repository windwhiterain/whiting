# The ground was dark because a surface body was painting its own air

Measured 2026-10-12, on `dunes`, raster against Cycles at 32 samples on CPU.

## The question

Once the sky reads the scene's own environment at the light's own radiance, every scene's sky
agrees in level. No ground agreed, and not in one direction: `dunes` 49 levels dark, `foldyard` 47
dark, `landscape` 34 bright. `dunes`' ground correlated +0.9893 while sitting 49 levels away — the
shape was right and the brightness was not. What was wrong with the brightness?

## What it was

`dunes` passed a **surface body** to its lighting stage, and that body did two things:

```wgsl
color = color * mix(0.30, 1.0, shade);                                  // a cloud shadow
color = mix(color, air_colour(), clamp(haze * (0.5 + 0.5 * facing), 0.0, 0.62));
```

The second line is the one that matters, and `cycles_backend.md` has already ruled on it. At `:467`
the SPEC decides that **atmosphere belongs to the post stage**, that **no surface body is allowed to
reach for `air_colour()` or `air_closure()` and mix toward it**, and why: *a surface shader that
paints its own fog is a surface whose fog a post stage cannot correct, a tone curve cannot, and a
second backend cannot reproduce.* At `:435` the wider rule: *a scene that shades procedurally is a
scene whose picture cannot be reproduced by a path tracer, and that is the defect this chapter
exists to prevent.*

`landscape` had its body removed on that basis and the removal was measured at **+0.37**.
`dunes` was the last scene in the gallery still doing it — a survey of every `r#"` constant in
`crates/gallery/src` for `air_colour` or `air_closure` finds `dunes`' `SURFACE` and `foldyard`'s
`GRADE`, and the second of those is the post stage, which is where the decision puts it.

So this was not a mystery. It was a scene that had not been brought into compliance with a decision
the project had already made and already measured for another scene.

## What removing it measured

The body deleted outright — crest tinting, the cloud shadow and the aerial perspective all go with
it, because a tracer can run none of them.

| | before | after |
|---|---|---|
| whole frame, raw | +0.2669 | **+0.8765** |
| whole frame, curve matched | +0.3447 | **+0.9871** |
| sky, curve matched | +0.9198 | **+0.9944** |
| ground, curve matched | +0.9893 | **+0.9976** |
| ground mean luma, raster | 109.06 | **159.12** |
| ground mean luma, traced graded | 157.66 | 157.66 |

**The brightness gap closes on its own.** The raster ground was not mis-calibrated; it was dark
because the scene was mixing up to 62% of the air colour into it, and the tracer was drawing the
sand. Once both sides draw sand, both sides are at 158–159.

The whole-frame gain is +0.61 against the +0.37 the same removal was worth on `landscape`, and the
picture it leaves is a clean field of sand under a gradient.

## Why the obvious explanation was wrong

The first guess was aerial perspective flattening the field, because haze rises with distance and
that is what a contrast loss looks like. It was tested by measuring the ground in four bands from
the foot of the frame upwards, which is a direct reading of "does the compression grow with
distance".

| rows from the foot | traced span | raster span | ratio |
|---|---|---|---|
| 0%–10% | 58.70 | 19.84 | 0.338 |
| 10%–20% | 73.92 | 21.70 | 0.294 |
| 20%–28% | 82.29 | 23.34 | 0.284 |
| 28%–38% | 105.64 | 30.90 | 0.293 |

The ratio is flat at about 0.29 across the whole ground. A haze that rises with distance would put
its worst band at the back and its best at the front, and this does not. The compression was the
cloud shadow's `mix(0.30, 1.0, …)` multiplying the field down, which is everywhere at once, and the
shape disagreement underneath it was the air mix.

## What this says about the rule

The rule was already right and already paid for once. What was missing was a way to tell, cheaply,
whether a scene is inside it — a survey of the gallery's own source for a forbidden call answers it
in under a second and found the last violator immediately. It is at `out/survey-air-in-surface-bodies.py`.

## Not measured

- Whether `landscape` and `foldyard`, whose grounds still disagree in level, are wrong for the same
  reason. Neither has a surface body reaching for the air, so it cannot be, and their grounds also
  carry painted content that the tracer has no counterpart for.
- What the `sunlight` scene's flat, blown ground is. It names no environment and has no surface
  body, so it is a different defect: measured, its raster ground is a constant 193.13 across the
  bottom 28% of the frame — a span of **zero** — against a traced 99.63. That is recorded nowhere
  yet and is the next thing to look at.
- Whether the dune field wants its cloud shadow back by another route. It cannot come back as
  per-fragment arithmetic, which is the whole of this note.

## Sources

- The body and its removal: `crates/gallery/src/dunes.rs`, and the render log's own report.
- The decision this brings the scene into line with: `.agents/SPEC/render/cycles_backend.md:431`
  and `:467`.
- The survey: `out/survey-air-in-surface-bodies.py`.
- The distance test that dismissed the haze explanation: `out/probe-ground-contrast.py`.
- The measurement: `out/probe-agreement.py` and `out/probe-grade-matched.py`, on
  `out/env-sky.png` before and `out/gb-dunes-final.png` after, against the same
  `out/env-sky-traced.png`.
# Why tiles agreed with itself at +0.0221

measured 2026-10-11 on `main` at `1ed127e`. probe: `out/probe-agreement.py`, `out/probe-rows.py`,
1024x1024, 32 samples on CPU.

## The question

`tiles` was the worst scene in the gallery at **+0.0221**, and its lower 62% band was
**negative** at −0.1234. One image was almost entirely black with a thin bright strip
near the horizon; the other was a large lit surface. Two backends looking at the same
document were looking at different things. Which one was lying?

## The answer

Neither was lying about the renderer. **The scene named a light reach that did not reach.**

```rust
light_with_radius("sun", [-6.0, 12.0, 7.0], [1.0, 0.94, 0.86], 9_041.0, 16.0, 12.0)
//                                                                                 ^^^^ radius
```

The floor is `heightfield(48, EXTENT = 12.0)`, and `heightfield`'s own arithmetic puts
its origin at `-extent` with a step of `2 * extent / divisions`, so the floor spans
`x, z ∈ [-12, 12]`. The lamp stands **twelve units above it**, and its reach is **twelve
units**. The lamp's radius sphere is therefore *tangent to the floor plane*: it touches
at one point and contains none of it.

`kit.rs` says what that number means and what it wants:

> The value is the distance the light is meant to carry over, so it wants to be at least
> the distance from the light to the furthest surface it is supposed to light.

The furthest floor point from `[-6, 12, 7]` is `[12, 0, -12]`, at **28.8**. The scene
named 12.

## What that looked like

Measured per row on the rasteriser's own `depth` and `grade` channels. `drawn` counts
pixels the depth buffer holds something other than an untouched `1.0` for.

| y | drawn | lit > 8 | mean luma | depth p50 |
|---|---|---|---|---|
| 448–490 | 0.0% | 0.0% | 0.0 | — (sky) |
| 505 | 100.0% | 43.8% | 54.7 | 253 |
| 545 | 100.0% | 43.8% | 75.3 | 232 |
| 560 | 100.0% | 43.8% | 77.8 | 226 |
| 600 | 100.0% | **0.0%** | 0.0 | 206 |
| 900 | 100.0% | **0.0%** | 0.0 | 60 |

Three readings, each of which rules something out:

- **the floor is there.** `drawn` is 100% from y=505 to the bottom of the frame and the
  depth runs 253 down to 60, so this is a floor seen at a grazing angle and not a hole in
  the scene.
- **it receives no light at all.** the `grade` channel is the radiance *before* the tone
  curve, and it reads **exactly 0** below the band. Not dark: zero.
- **the lit band's right edge is a straight vertical line at 43.8% of the width.** That is
  the cluster grid's own culling boundary, which `kit.rs` describes as the reason a reach
  must cover the scene: *"a light culled out of one box and kept in the next has an edge
  that runs along a box boundary, which on the frame is a straight line at a multiple of
  the grid."* What survived was the cluster boxes' bounding slack, not any part of the
  floor being lit.

## Why the tracer drew it anyway

`radius` is the raster pipeline's own culling reach and has **no Cycles counterpart** — the
emitter does not map it, and should not. So the tracer read the same document, took the
lamp as a light that lights everything, and drew the floor. Both backends were faithful to
the document. The document was wrong.

## The fix and its measurement

One number: `12.0` → `30.0`, past the 28.8 the scene actually needs, which also promotes
the lamp to a global light (`kit.rs`: a reach that holds the whole scene changes no pixel).

| | reach 12 | reach 30 |
|---|---|---|
| whole frame | +0.0221 | **+0.9740** |
| lower 62% | −0.1234 | **+0.9390** |
| lower 38% | nan | **+0.3294** |

The largest single move in this project, and it was a scene lying about its own lamp.

## Why no other scene has it

Every other `light_with_radius` call site in the gallery names a reach far past what it
lights — `terraces` at 700 and states so in a comment, `foldyard` at 760 and 700,
`lanterns` at 300 three times, `monoliths` at 340, `nurbs` at 30. The one deliberate short
reach, `tidewater` at 1400 against corners at 2610, **says so** and records the decision in
a handoff. `tiles` at 12 is not a decision; it is a number that happens to equal the lamp's
height above the floor, which is what makes the failure total rather than partial.

## What is still different

The remaining gap in `tiles` is the known `image` refusal and nothing else: the rasteriser
samples the tile texture and shows the grid and its mortar seams, and the tracer draws the
material's tint because `principled_bsdf`'s `Base Color` cannot be connected by a loader
that splits a connect on whitespace. That is measured and recorded in
[cycles-emitter-dropped-material-fields.md](cycles-emitter-dropped-material-fields.md).

## Reproducing

```
cargo run --release -p whiting-device-rasterize --example render -- tiles out\td --debug depth
cargo run --release -p whiting-device-rasterize --example render -- tiles out\tg --debug grade
python out\probe-rows.py out\td.png out\td-depth.png
python out\probe-rows.py out\tg-grade.png -
```

# Physical light units, closed against a path tracer

## The question

`.agents/proposals/physical-light-units.md` decided that a scene's light states a physical
quantity — `power`, `irradiance`, `radiance` — and that the exporter maps straight through
"and does not judge". That decision was never checked against a render. Nine of the twelve
gallery scenes came back **completely black** through the subprocess: `max = 0` over every
texel, exit status zero, no `ERROR` on stderr, the full sample count spent.

Two more questions came out of chasing it:

- The rasterised lighting stage added an ambient term no document asked for.
- A scene that states its own `exposure` has it honoured by the rasterised path and ignored by
  the emitter.

## What was measured

Full measurements, with the sources and the arithmetic, are in
`.agents/investigate/cycles-light-units.md`. The three that decide this:

1. **Cycles writes scene-linear radiance, with no view transform.** A world of constant
   `color = 0.1, strength = 1.0` came back as 26 of 255 — `0.1 x 255 = 25.5`, not the 89 an
   sRGB encode gives. There is no `--view_transform` flag in this build and no OCIO
   configuration, so there is no curve at all.

2. **A point light's `strength` is radiant flux, and the identity mapping is right.** On `tiles`,
   with the lamp at `power = 3000`, `value x d^2 / cos` came out constant down the frame at 69
   to within 1%. `3000 / 69 = 43.5`, and the physical single-bounce constant is `1 / (4 pi^2) =
   1 / 39.5`. So the exporter was already correct, and the discrepancy was entirely ours.

3. **The gallery had been calibrated against a tone curve.** Seven of the eleven light-carrying
   values follow `power ≈ d^2` — dunes 1.57, foldyard 1.96, landscape 1.25, lanterns 0.86,
   monoliths 2.64, tidewater 1.56, spotlight 1.03 — which puts `power / d^2` at about 1.0 on the
   subject. The rasterised post stage ends in `(L / (L + 1))^0.4545`, and 1.0 through that is
   **186/255**. Cycles applies no such curve, so the same document came back at 1.0, which is
   1/255.

   `terraces` and `tiles` are not even on that rule, by five and seventy-five times.

## What is decided

### The `4 pi^2` goes into the shading model, and the scenes' values are raised by it

`scene::standard::photometry` owns `FLUX_TO_RADIANCE = 1 / (4 pi^2)`, and the gallery's
attenuation is `FLUX_TO_RADIANCE / d^2`. Every point, spot and directional value in the gallery
and in the example scene is multiplied by `4 pi^2`.

They cancel. `39.478 x P / (39.478 x d^2) = P / d^2`, so **the rasterised point-light term is
unchanged to the last bit**. What changes is that the number is physical, so a backend that
knows the physics — a path tracer, or any renderer with a real BRDF — lands on the same value
without being told this renderer's history. That is the outcome the earlier proposal was after,
reached by fixing the model rather than by patching the exporter.

A directional light is raised by `pi`, not `4 pi^2`: it has no sphere to spread over, so only
the Lambertian `1 / pi` was missing.

`terraces` and `tiles` were re-derived from their own light-to-subject distance rather than
scaled, because their values were never on the rule. `terraces`' two lights keep their 5.4 : 1
ratio — scaling both by `4 pi^2` would have made the fill brighter than the key, which is the
thing that scene's own comment says a fill must not do.

### The ambient goes

`environment_radiance` returned a two colour hemisphere for any scene naming no `Environment`
light. Nine of the twelve gallery scenes name none.

This was not a small term. On `tiles`, the rasterised floor sat at a linear radiance of **0.132**
and the scene's one point lamp accounted for **0.011** of it. **The rasterised image of that
scene was mostly a picture of a function no document asked for**, and a backend without a
stand-in renders the near-black frame the lamp actually makes.

It returns black now. A scene that wants a sky declares one. Nine gallery frames get
considerably darker, which is the honest picture of what they wrote.

### `example_standard_scene` names a camera and a flux

It named neither. The rasterised compiler falls back to `Camera::convention` — a **parallel**
camera framing bounds it has already computed. The emitter has no bounds, no padding and no
orthographic camera, so it fell back to the identity, and `studio` wrote 2 073 600 texels of
black. A document that does not say where it is drawn from is not a complete document, and the
backends cannot be left to disagree about what to assume. Its key light was carrying `56`
against a `d^2` of `56` — the old rule exactly — and is now `4 pi^2 d^2`.

## What is deliberately **not** changed

### A scene's `exposure` still does not reach the film

**This is a real gap and it is left open on purpose.** `scene_schema::EXPOSURE` documents the
field as "what the post stage multiplies the frame by before its own grade: a linear multiplier
on the radiance the grade reads". The rasterised post stage reads it. `EmitOptions::exposure` is
a separate number the caller passes, defaulting to 1.0, and the emitter never looks at the
scene's own field.

Wiring it up is a one-line read in the emitter and a public API change — `EmitOptions::exposure`
would have to become an override rather than a value — and that is not a change to make while
chasing a different bug. It also needs a decision the field's own documentation does not settle:
a scene's post-stage exposure is a number chosen against *that* stage's curve, and whether the
same number should drive a path tracer's film is a question about what a scene is claiming. See
"Open" below.

### Overexposure is left as the scene's own decision

`foldyard` writes `max = 255, mean = 240` and `tidewater` `255, mean = 131`. Both are lit by an
environment light *and* a point light, both now at their stated values, and their sum exceeds 1.0.
**Linear 1.0 is already the top of an 8-bit linear file**, where the rasterised path's Reinhard
curve would still be rolling off. With no view transform in this build there is no curve to roll
it off, and the only control is an exposure per scene — which is the question above, not a
defect.

`monoliths` (`mean 7.7`) and `terraces` (`mean 11`) are dark, and correctly so: a single low sun
leaves most of a plain in its own shadows, which is the scene.

## The two backends will not produce the same image, and should not

Even with the units closed, the frames differ, and the differences are not defects:

- **The world.** A scene naming no `Environment` light now renders a black sky in Cycles. The
  rasterised path still draws a sky for these scenes, from a **full-screen post pass** rather
  than from `environment_radiance` — `monoliths` carries `screen_material("post", "post")` and a
  `GRADE`, and its rasterised frame is a lit gradient that Cycles has no source for. Removing
  the hemisphere stand-in did not remove this; it is a separate, larger gap.
- **The grade.** The rasterised path ends in Reinhard and a gamma. Cycles ends in a clip. A
  scene exposed for the first is overexposed for the second by exactly the top of that curve.
- **`monoliths` reads as two different compositions.** The emitted camera matches the scene's
  own declaration (`translate="34, 7, 44"`), and Cycles draws the low, close, cropped view that
  declaration and its comment describe. The rasterised frame is a wide shot with the monoliths
  on the horizon. This was not chased further; it is a disagreement about a scene's framing and
  it is recorded rather than resolved.

## Open

- **Should `scene_schema::EXPOSURE` drive the emitted film?** If yes, `EmitOptions::exposure`
  becomes an override and `CyclesOptions::exposure` follows it. If no, the field should say it
  is the rasterised post stage's alone, so a scene author is not left believing it reached every
  backend. Either is defensible; the current state — silently honoured by one and ignored by the
  other — is not.
- **The rasterised full-screen post pass is a second world.** A scene's `screen_material` draws a
  background no path tracer can express. Either the format grows a way to express it, or a scene
  that wants a sky declares an `Environment` light and the post pass stops drawing one. This is
  the largest remaining difference between the two backends and it is not a units question.

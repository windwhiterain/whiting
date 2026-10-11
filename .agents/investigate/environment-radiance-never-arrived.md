# An environment light's radiance never reached the tracer

Measured 2026-10-11, on `dunes`, after the scene was given the sky its rasteriser already
drew. Supersedes the assumption in `.agents/investigate/cross-backend-sky.md` that a scene's
declared environment brightness is what the tracer sees.

## The question

`dunes` declared `environment_light("air", "air", [1.0; 3], 0.55)`. The exported scene
carried:

```xml
<background name="world" strength="1, 1, 1" />
```

A radiance of 0.55 had become 1.0. Where did it go?

## The answer

The exporter read the brightness out of a field the schema does not declare for this kind of
light, and defaulted when it found nothing.

`crates/scene/src/cycles_export/light.rs:117` already knows which field name each kind of
light states its brightness under:

```rust
fn brightness_field(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "Point" | "Spot" => "power",
        "Directional" => "irradiance",
        "Environment" => "radiance",
        _ => return None,
    })
}
```

`read_light` calls it and stores the result in `Light::brightness`
(`light.rs:190`, `light.rs:197`). That path was correct for every kind of light. But the
`Environment` arm read a **second** field as well:

```rust
strength: read::field(fields, "strength")
    .and_then(|value| read::number(value, "strength").ok())
    .unwrap_or(1.0),
```

`scene_schema.rs:869` declares an environment light as `shared_fields()` plus `radiance()`
and `image()`. There is no `strength` among them, so the read found nothing and the
`.unwrap_or(1.0)` supplied unit radiance — silently, and on a path that reported nothing.

The world writer then used that field, not the correct one:

```rust
fn environment_strength(environment: &Light) -> String {
    Writer::color(
        environment.color[0] * environment.strength,   // not .brightness
```

So `Light` carried both `brightness` and `strength`, populated by two different reads of two
different names for the same quantity, and only the punctual path used the right one.

## Why it went unnoticed

Three independent conditions had to hold at once.

**The default was a plausible value.** 1.0 is what a sky of unit radiance looks like, so a
world at the wrong brightness is still a sky. Nothing about the rendered image says "this
number was ignored".

**Nothing reported the miss.** The `.unwrap_or(1.0)` is the whole of the error handling, and
a field that is absent is not an error the exporter was looking for. The `Degrade` report
for `dunes` lists five entries and none of them mentions the radiance.

**The scenes that declare an environment are the ones nobody re-measured.** `landscape` was
last measured at `+0.7812` on `c8c7ec9`, before this field existed in the diff at all; its
`0.55` was carried from `5fb5c12` and had not been exercised since.

## The fix

`Light::strength` is removed rather than corrected. It has one reader, that reader was the
wrong one, and `Light::brightness` already holds the quantity under the name the schema
declares — so keeping both would leave a second way to ask the same question, and the second
way is the one that was already wrong once.

`crates/scene/src/cycles_export/scene.rs:647` now reads `environment.brightness`, which is
what every other light kind was already using.

## What it cost, measured

`dunes`, 1024x1024, 32 samples on CPU, against the same rasterised frame throughout. The
raster sky band carries 155.73 of mean luma; the radiance was swept by patching the `strength`
attribute in the exported XML and re-running the tracer, so the value under test is the value
the tracer reads.

| radiance | traced sky mean | gap against raster |
|---|---|---|
| 0.55 (the value landscape carried) | 79.58 | -76.15 |
| 0.90 | 131.49 | -24.25 |
| 1.00 (**what every scene actually rendered at**) | 146.09 | -9.65 |
| 1.02 | 149.00 | -6.73 |
| 1.05 | 153.39 | -2.35 |
| **1.06 (chosen)** | **153.21** | **-2.52** |
| 1.08 | 157.77 | +2.03 |
| 1.10 | 160.68 | +4.95 |
| 1.20 | 175.28 | +19.55 |
| 1.30 | 189.88 | +34.15 |

The response is close to linear in this range, about 145 of luma per unit of radiance, which
is what a linear radiance written straight into an 8-bit file without a view transform should
give. It is not exactly linear because the sky is an image of an atmosphere, not a constant.

## The scenes this changed

Every environment light in the gallery rendered at unit radiance until this fix. Their
declared values were:

| scene | declared | what it rendered at | measured before |
|---|---|---|---|
| `dunes` | 0.55 | 1.0 | -0.6585 |
| `landscape` | 0.55 | 1.0 | +0.7938 |
| `foldyard` | 0.26 | 1.0 | +0.1168 |
| `tidewater` | 1.0 | 1.0 | +0.3435 |

`tidewater` is the one scene whose number was right, and it was right by coincidence.
`foldyard` was the furthest from unity of the four and is the one whose picture should change
most.

## What this does not settle

The ground still disagrees, and it disagrees in a way no radiance can reach. With the sky
matched to -2.5, the ground band carries 99.81 on the rasteriser against 152.94 on the
tracer — and the rasterised ground spans 29 levels of luma across the band where the traced
one spans 112. That is the [no view transform](../../SPEC/render/cycles_backend.md#no-view-transform)
difference: the rasterised post stage applies Reinhard, a gamma of 0.4545, an S curve and a
saturation lift, and this build of Cycles writes scene-linear radiance into the file with no
curve at all.

A radiance is a scale and a tone curve is a shape, so no single number fits both. This fix is
about the sky being right on its own terms; the ground is a separate question and is recorded
as open.

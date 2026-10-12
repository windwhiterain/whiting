# A directional light's irradiance is used as a radiance, and it is pi too bright

Measured 2026-10-12, on `sunlight` and `dunes`, raster against Cycles at 32 samples on CPU.

## The question

With the traced frame put through the raster scene's own tone curve and the raster frame's curve
undone — so both sides hold a linear radiance and not a picture — `dunes` agrees at a ratio of
**1.00** and `sunlight` disagrees at **2.95**. Both scenes render through the same shader, the same
interface and the same emitter. What in the shading differs between a sun and a sky?

## What the two scenes have

`dunes` names an `Environment` light and no directional light. `sunlight` names a directional light
and no environment, and passes `""` as its surface body. So `sunlight` is the scene in the gallery
that isolates the sun.

## The measurement

Mean linear radiance, recovered by inverting each scene's own curve and compared with the tracer's
linear file. A uniform ratio across the frame is a units factor and is one number.

| region | `sunlight` raster | `sunlight` traced | ratio | `dunes` ratio |
|---|---|---|---|---|
| whole | 0.6410 | 0.2275 | **2.817** | 1.004 |
| sky | 0.0924 | 0.0353 | 2.618 | 0.927 |
| lower 62% | 0.9760 | 0.3449 | 2.830 | 1.049 |
| ground | 1.1651 | 0.3951 | **2.948** | 0.983 |

`dunes` at 1.00 says the environment path is calibrated. `sunlight` at 2.95 says the directional
path is not, and it says so uniformly — which is what a units factor looks like and is not what a
difference of shading models looks like.

## What the code does

`light_contribution` in `crates/gallery/src/shaders.rs:1795`:

```wgsl
return diffuse * lambert * radiance + fresnel * specular * radiance * 3.0;
```

There is no `1/pi` anywhere in it, and the two callers pass different quantities into the same
parameter:

- `directional_radiance` passes `uniforms.directional.colour.rgb * lit`. `brightness_field` maps a
  directional light to an **`irradiance`**, and `cycles_backend.md:125` records that mapping. A
  Lambertian surface under irradiance `E` returns radiance `albedo * E * cos / pi`, so the term above
  is missing exactly one `1/pi` and what it returns is an irradiance rather than a radiance.
- The environment term adds `sky * mix(...) * diffuse_color` with no `1/pi` either, and there it is
  **correct**: for a uniform hemisphere of radiance `L` the irradiance is `pi L`, so `albedo * E / pi`
  is `albedo * L`. The division cancels. `dunes`' measured 0.983 is that cancellation holding.

So one parameter carries two quantities of different dimension, and the shading is right for one of
them and pi out for the other.

## The confirmation

`sunlight` declares its sun at **3.927**, which is `pi * 1.25` to five figures — the author had
already folded a `pi` into the number by hand, in the wrong direction. Re-rendering it at
**1.25**, which is the same number divided by `pi`, moves the ground ratio:

| declared irradiance | ground ratio |
|---|---|
| 3.927 | 2.948 |
| **1.25** | **0.937** |

`pi` is 3.14159, and 3.927 / pi is 1.2499. The measured ratios are `pi` and `1 / 1.067`. The 6.7% the
raster is still short after the correction is the tracer's indirect bounce on a floor of that
albedo, which a rasteriser with no global illumination does not compute and is not expected to.

The scene was put back to 3.927. A number in a document that compensates for a bug in a shader is
not a fix, and leaving it would hide the bug behind a value nobody can read.

## What this says about the SPEC

`cycles_backend.md:501` records the remaining brightness gap as *the two lighting models — the
rasteriser's own Lambert and specular against the tracer's integral — and it is the same class of
difference as no view transform*. That reading does not survive the measurement. A uniform factor of
`pi` across a frame that contains one light and no other term is a conversion, not a model. The
correlations the chapter quotes (`sunlight` +0.9763) cannot see it, because a correlation is
insensitive to a gain — which is the same blind spot [the tone curve was
masking a ground brightness gap](the-tone-curve-was-masking-a-ground-brightness-gap.md) was read
through.

## Not measured

- Whether `spotlight` and `lanterns`, which carry point lights, are also out. Their lights go
  through `FLUX_TO_RADIANCE_DIGITS = 1 / (4 pi^2)`, which already carries the `1/pi` of a Lambertian
  lobe, so the asymmetry may be confined to the directional path — **not measured**.
- Whether `landscape` and `foldyard` are out for the same reason. Both carry painted content the
  tracer has no counterpart for, so their ratios mix this with something else.
- Whether correcting the shader would put every scene in the gallery pi darker, and what each
  declared irradiance would then have to become. That is a change to six pictures and is not
  measured here.

## Sources

- The term: `crates/gallery/src/shaders.rs`, `light_contribution` at `:1795` and
  `directional_radiance` at `:1818`.
- The quantity a directional light declares: `.agents/SPEC/render/cycles_backend.md:122`, the
  `brightness_field` table.
- The scene: `crates/gallery/src/sunlight.rs`, `directional("sun", …, 3.927)`.
- Recovery of the raster's radiance: `out/probe-linear-radiance.py`.
- Frames: `out/gb-sunlight.png`, `out/gb-sunlight-125.png`, `out/gb-sunlight-traced.png`,
  `out/gb-dunes-final.png`, `out/env-sky-traced.png`.
# Handoff: dunes declares its sky, and the environment radiance defect

Branch `feature/dunes-declares-its-sky`, from `4a1473f`. Two changes that turned out to be
one finding.

## Why this work exists

The cross-backend sky diagnosis (`.agents/investigate/cross-backend-sky.md`) found that a
scene's whole-frame correlation is a function of one number: the gap between the rasterised
sky band's brightness and the traced one's. `dunes` sat at -0.6585 with a gap of 155.7. Giving
it the sky its rasteriser already draws should move both, and testing that is what this branch
is for.

## What changed

### 1. `crates/gallery/src/dunes.rs`

- imports `environment_light` and `image`
- an `images` entry: `air`, 256x128 latlong, from `atmosphere().environment_pixels`
- the point light named `sky` is **replaced**, not joined, by
  `environment_light("air", "air", [1.0; 3], 1.06)`

A point light cannot stand in for a dome: its brightness is the square of its distance, and
this field is 420 units to the far corner, so a lamp bright enough to lift the near troughs
leaves the far plain black. That is the exact defect the original lamp existed to cure.

### 2. The environment radiance never arrived — `crates/scene/src/cycles_export/`

The exporter read an environment light's brightness out of a `strength` field the schema does
not declare for that kind, found nothing, and fell back to 1.0. Every environment light in
the gallery rendered at unit radiance regardless of what its scene declared — `landscape`'s
0.55, `foldyard`'s 0.26, `dunes`' 0.55.

`Light` carried both `brightness` (read correctly, through `brightness_field`) and `strength`
(read wrongly, defaulted). `Light::strength` is removed rather than corrected: one reader, and
it was the wrong one. `environment_strength` now reads `environment.brightness`.

Full investigation, including the sweep table and the reason three conditions had to hold for
it to go unnoticed: `.agents/investigate/environment-radiance-never-arrived.md`.

## The radiance was measured, not copied

`landscape`'s 0.55 is not a reference value — it never reached a renderer. The number here is
from a sweep of the `strength` attribute in the exported XML, which is the value the tracer
reads. Against a raster sky of 155.73:

| radiance | traced sky | gap |
|---|---|---|
| 0.55 | 79.58 | -76.15 |
| 1.00 | 146.09 | -9.65 |
| **1.06** | **153.21** | **-2.52** |
| 1.30 | 189.88 | +34.15 |

## Measured result

1024x1024, 32 samples on CPU, against the same rasterised frame throughout.

| | before | after |
|---|---|---|
| whole frame | **-0.6585** | **+0.2667** |
| lower 62% | +0.6268 | +0.7401 |
| lower 38% | -0.3941 | +0.8475 |
| sky band gap | 155.7 | **-2.5** |

The mechanism predicted by the sky diagnosis holds: closing the sky gap moved the whole-frame
correlation by +0.93, and it moved it in the predicted direction.

## What is still open

**The ground, and it is not a radiance.** With the sky matched, the ground band carries 99.81
on the rasteriser against 152.94 on the tracer. The rasterised ground spans 29 levels of luma
across the band; the traced one spans 112. The rasterised post stage applies Reinhard, gamma
0.4545, an S curve and a saturation lift; this build of Cycles writes scene-linear radiance
with no curve. A radiance is a scale and a tone curve is a shape, so no single number reaches
both. Recorded in SPEC under `no view transform` as open.

**`landscape` and `foldyard` need re-calibrating.** Their scenes are correct and their numbers
are now live; their pictures will change and their swept values do not exist yet. `landscape`
was +0.7938 at a radiance it never rendered at. This is the next job.

**`terraces` has no world** and its sky law lives in WGSL (`terraces.rs:895` `sky_colour`),
so it is a port rather than a declaration. Fog and procedural far ranges stay on the raster
side regardless.

## How to reproduce

```
cargo run --release -p whiting-device-rasterize --example render -- dunes ..\..\out\dunes-raster
cargo run --release -p whiting-device-cycles --example render_cycles -- dunes ..\..\out\dunes.png 32 CPU
python ..\..\out\probe-agreement.py <raster> <traced>
```

To sweep without a rebuild, patch the `strength` attribute in the dumped
`dunes.cycles.xml` and re-run `C:\resource\cycles\install\cycles.exe` directly; the parameter
order is `--device CPU --samples 32 --background --output <png> <scene.xml>`. Note that
Cycles exits 0 even when it reports `ERROR:`, so the presence of the output file is the check.

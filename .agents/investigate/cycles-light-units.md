# What a light's number means, measured against a path tracer

**Measured 2026-10-08 on `cycles.exe` 5.3.0**, built from `blender/cycles` at
`a456b761034dda42c32eef9f4aae0fa5a5c9f604`, at `C:\resource\cycles\install\cycles.exe`.
Sources are the built binary's own behaviour and the source tree it was built from; where a
claim is inference rather than measurement it says so.

## The question

`cycles_export` writes a scene's `power` straight to a light's `strength` with no scale factor,
on the argument that the document is in physical units and the exporter maps rather than judges.
`.agents/proposals/physical-light-units.md` made that decision. It was never checked against a
render.

Nine of the twelve gallery scenes came back **completely black** through the subprocess: `max = 0`
over every texel, exit status zero, no `ERROR` on stderr, the full sample count spent.

## What was measured

### 1. Cycles writes scene-linear radiance, with no view transform

A document whose only content is a world of constant `color = 0.1, strength = 1.0`:

| document | pixels |
|---|---|
| world 0.1 | **26** |

`0.1 x 255 = 25.5`, and 26 is that rounded. An sRGB encode of 0.1 would be `0.349`, which is 89.
A Filmic or AgX curve would be neither.

This build has **no view transform at all**: there is no `--view_transform` flag in `--help`, and
with `$OCIO` unset every run prints

```text
[OpenColorIO Info]: Color management disabled. (Specify the $OCIO environment variable to enable.)
```

Confirmed by reading `NODE_DEFINE(Film)` in `src/scene/film.cpp`: the sockets are `exposure`,
`pass_alpha_threshold`, `filter_type`, `filter_width`, `mist_*`, `display_pass`, and the
cryptomatte set. **No resolution socket either** — see §4.

### 2. A point light's `strength` is radiant flux, and the identity mapping is right

`tiles` has one white floor and one point lamp. With the lamp at `power = 3000`, the floor's
linear value was read down the frame, and `value x d^2 / cos(angle to the lamp)` was computed
from the known geometry of the camera at `(0, 0.7, 2.6)` and the lamp at `(-6, 12, 7)`:

| row | z | d² | measured | measured x d² / cos |
|---|---|---|---|---|
| 330 | −9.51 | 452.7 | 0.0863 | 69.3 |
| 430 | −0.67 | 238.8 | 0.2196 | 69.0 |
| 510 | 0.58 | 221.2 | 0.2510 | 68.8 |

Constant to 1%. `3000 / 69 = 43.5`, and the single-bounce physical value is `1 / (4 pi^2) =
1 / 39.478`. The 10% gap is the `principled_bsdf`'s 3.5% specular taking its share of a diffuse
albedo of 1, plus the film's filter and the lamp's own bias against the geometric cosine.

**So `strength` is watts, the falloff is `1/d^2`, the BRDF divides by `pi`, and the exporter's
identity mapping is correct.** The source agrees: `PointLight::copy_to_kernel` in
`src/scene/light.cpp` divides by the lamp's area and then by `M_1_PI_F`, and
`src/kernel/light/sample.h` multiplies `klight->strength` into the shader's own emission.

### 3. The `4 pi^2` was ours, and the gallery was calibrated against a tone curve

The rasterised lighting template evaluated `radiance = power / d^2` and fed that to a Lambert
term with no `1 / pi`. Physical single-bounce radiance is

```text
L = rho * Phi * cos / (4 pi^2 d^2)
```

— a lamp's flux spread over a sphere of area `4 pi d^2` to become an irradiance, and a
Lambertian lobe returning over a hemisphere, which is the other `pi`.

The gallery had been calibrated to that model by writing `power` as roughly `d^2`, so that
`power / d^2` came out at **1.0** on the subject. The rasterised post stage ends in a Reinhard
curve, `(L / (L + 1))^0.4545`, which maps 1.0 to **186/255**. Cycles applies no such curve, so
the same document came back at 1.0 — which is 1/255.

The measured powers confirm the rule, and show which scenes were never on it:

| scene | light | power | distance | power / d² |
|---|---|---|---|---|
| dunes | sun | 62 800 | ~200 | 1.57 |
| foldyard | sun | 314 000 | ~400 | 1.96 |
| landscape | sky | 450 000 | ~600 | 1.25 |
| lanterns | moon | 19 320 | ~150 | 0.86 |
| monoliths | sun | 19 100 | ~85 | 2.64 |
| tidewater | sun | 1 267 000 | ~900 | 1.56 |
| spotlight | lamp | 103 | ~10 | 1.03 |
| **terraces** | sun | **5.4** | ~429 | **0.000029** |
| **tiles** | sun | **3.0** | ~15 | **0.013** |

Seven scenes are on the rule. `terraces` and `tiles` are not, by five and seventy-five times.

**Resolution taken:** the normalisation went into the shading model
(`scene::standard::photometry::FLUX_TO_RADIANCE`) *and* every point, spot and directional value
was raised by the same factor. They cancel exactly — `39.478 x P / (39.478 x d^2) = P / d^2` —
so the rasterised point-light term is unchanged. `terraces` and `tiles` were re-derived from
their own light-to-subject distance instead. A directional light gets `pi` rather than `4 pi^2`,
because it has no sphere to spread over and only the Lambertian `1 / pi` was missing.

### 4. The document's own size is not the render's size

`xml_read_camera` in `src/app/cycles_xml.cpp` reads `width` and `height` off the `<camera>`
element and applies them with `set_full_width` / `set_full_height`. The **buffer** comes from the
command line:

| run | output |
|---|---|
| document camera says `1920, 1080`, no `--width` | **1024 x 512** |
| same document, `--width 320 --height 180` | **320 x 180** |

There is nowhere in the document to put it. `CyclesOptions` now carries a `Resolution` and
`scene::cycles_export::render_size` reads it through the same `read_camera` that writes it.

### 5. The rasterised image of `tiles` was not a picture of its lamp

The lighting template added `sky * diffuse_color` for every fragment, where `sky` is a two colour
hemisphere whenever the scene names no `Environment` light. Nine of the twelve gallery scenes
name none.

The rasterised `tiles` frame's floor sits at a linear radiance of **0.132**, and the scene's one
point lamp accounts for **0.011** of it. The other **0.12** was the stand-in. The rasterised
image of that scene was mostly a picture of a function no document asked for, and a backend
without a stand-in renders the near-black frame the lamp actually makes.

**Resolution taken:** `environment_radiance` returns black when the scene names no environment.
A scene that wants a sky declares one.

## What the emitter got wrong that was not a units question

### A light travels along its own `-Z`

`SpotLight::copy_to_kernel`, `src/scene/light.cpp`:

```text
const float3 dir = -transform_get_column(&object->get_tfm(), 2);
```

An identity transform's third column is `+Z`, so a light with no rotation travels along `-Z`.
`direction_from_z` built the rotation that carries `+Z` onto the scene's stated `direction`, which
aims the light **exactly backwards**. The module's own header said the negative Z was the way a
light travels; the code did the opposite.

Measured: `sunlight` (one sun, `irradiance = 3.927`) and `spotlight` (one spot, `power = 4066.3`)
each rendered `max = 0` over 2 073 600 texels, with correct geometry, correct `principled_bsdf`,
no error and a full sample count. After the negation: `255 / mean 58` and `255 / mean 23`.

### The camera's first Euler angle is negated

A right-handed turn about `+X` carries `+Z` to `(0, -sin a, cos a)`, so a positive first angle
tilts the view **down**. `tiles`' camera stands at `(0, 0.7, 2.6)` and looks at `(0, 0.1, -8)`,
3.24 degrees below the eye. Written un-negated it looked 3.24 degrees **up**: the floor occupied
the bottom two fifths and the horizon sat above centre, where the rasterised path put it below.

### The Euler-to-axis-angle composition had two faults

The scalar term was `cx*cy*cz - sx*sy*sz`, which is the scalar part of a **conjugate**, not of a
composition. A negative `w` takes the angle out through `atan2` the long way round, so a half
turn came back as 360 minus it.

The half-angle was read as `sin(x) * 0.5` and `cos(x) * 0.5`, which is not `(x * 0.5).sin()`. They
agree for the sine; for the cosine `cos(0) * 0.5` is `0.5` where `cos(0)` is `1`, so a rotation
about one axis was scaled by a factor that depended on how the other two happened to be zero.

## Where this leaves the gallery

Through the subprocess, 8 samples on CPU, `max / mean` of the 8-bit file:

| scene | before | after |
|---|---|---|
| dunes | 73 / 2.5 | 255 / 78 |
| foldyard | 255 / 161 | 255 / **240** |
| landscape | 91 / 0.01 | 255 / 2.4 |
| lanterns | 255 / 3.1 | 255 / 40 |
| monoliths | 12 / 0.2 | 255 / 7.7 |
| spotlight | 0 / 0 | 255 / 23 |
| stilllife | 136 / 1.1 | 255 / 27 |
| studio | 0 / 0 | **0 / 0** |
| sunlight | 0 / 0 | 255 / 58 |
| terraces | 0 / 0 | 255 / 11 |
| tidewater | 255 / 128 | 255 / 131 |
| tiles | 0 / 0 | 223 / 83 |

### `studio` is still black, and it is not a fault

`Gallery::build("studio")` returns `scene::standard::example_standard_scene()`, and that document
names **no camera at all**. The emitter falls back to the identity, so the emitted element is a
bare `<transform>` and the camera stands at the origin looking along `+Z`. The rasterised
compiler instead supplies a convention camera from `CompileContext { camera: 0 }`, which a
document cannot reach. The default lives in the compiler rather than in the scene, so one backend
has a camera and the other has an origin. The rasterised path renders it (`max 255`).

The fix is to give the example document a camera, which would make it self-contained for every
backend. Not done here: it is the fixture for the `standard` module's doctest.

### `foldyard` clips, and this build cannot help it

It is lit by both an environment light and a point light, both now at their stated values, and
their sum exceeds 1.0. **Linear 1.0 is already the top of an 8-bit linear file**, where the
rasterised path's Reinhard curve would still be rolling off. With no `--view_transform` flag and
no OCIO configuration (§1), the only control is an exposure per scene — a decision about what a
scene should be exposed for, not a defect to fix.

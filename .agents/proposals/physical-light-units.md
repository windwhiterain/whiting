# Physical light units

## The question

A scene's light `intensity` is documented as a **brightness multiplier** (`scene_schema.rs`,
`shared_fields`). The raster pipeline's distance falloff is a smoothed stand-in:

```text
1 / (1 + 0.0035 * r + 0.00002 * r^2)
```

which lives in the lighting template at `crates/gallery/src/shaders.rs:1664` and is mirrored
as a copy in `crates/scene/src/standard/emit.rs`, with `crates/scene/tests/scale_notes.rs`
holding the two in agreement.

Cycles wants a **physical** quantity. `Light::strength` and the `emission` node's `strength`
are radiant flux, and its point light attenuates by true inverse square. A document written
for one backend is therefore wrong for the other, and no amount of mapping in the exporter
fixes that, because the two disagree about what a number means.

The decision taken is: **the scene definition uses standard units, and nothing is normalised
to the scene's own size.**

## What is *not* true, and was worth checking first

There is **no scene-size normalisation to remove**. `scale_notes` in `emit.rs` reads like one
— it measures the scene bounding box's half diagonal and computes a fold — but it only pushes
a `CompileNote` telling the author their scene is large enough for the falloff gradient to
dominate. Its own text ends "this is drawn as written". The fallback path is untouched.

What does exist is a **falloff tuned to a scene size**: `FALLOFF_TUNED_SPAN` is 200 units and
the constants are chosen to be "the gentlest that still read as a falloff over a scene a
couple of hundred units across". That is the scene-size dependence to remove. A law that is
tuned to a preferred scene span is normalised, in everything but name.

## What changes

### 1. The falloff becomes the inverse square

`crates/gallery/src/shaders.rs:1664`, and the mirror in `emit.rs`.

The law is singular at zero, and the current comment says why the gentler one was chosen: "a
steeper one puts the light's own volume inside the scene, and every surface near the light
then reads as white no matter what its colour is." That objection is real and does not go away.
The answer is the softening term that physical renderers use — clamp the distance from below
by the light's own size — rather than a global constant fitted to a scene span.

**Open: how the near-field is clamped.** See below.

### 2. `scale_notes` and its constants go

`FALLOFF_LINEAR`, `FALLOFF_QUADRATIC`, `FALLOFF_TUNED_SPAN`, `tuned_fold`, `falloff_fold`
and the whole of `scale_notes` exist to describe the smoothed law. With the law physical
there is nothing left for them to say, and `tests/scale_notes.rs` becomes a test of nothing.

Note that `FALLOFF_LINEAR`'s own doc already anticipates this: "this is the place to delete
when the falloff becomes a field a scene sets."

### 3. `intensity` is documented in a unit

`scene_schema.rs`, `shared_fields`. A single `intensity` field serves four kinds of light, and
they are not the same physical quantity:

| kind         | quantity                                             |
|--------------|------------------------------------------------------|
| `Point`      | radiant flux                                        |
| `Spot`       | radiant flux                                         |
| `Directional`| irradiance, the flux a perpendicular surface receives  |
| `Environment`| radiance                                             |

Two questions follow: whether one field may carry four units without saying which, and
whether `Directional` and `Environment` should keep sharing a field whose meaning differs.

### 4. Every scene is recalibrated

Scenes are Rust generators under `crates/gallery/src`, not data files. The light
constructors are in `kit.rs`; the values are literals at the call sites. Four scenes
explicitly reason about the old constant and will need their comments rewritten as well as
their numbers:

- `stilllife.rs:17-23` — the scene is 100 units across *because of the attenuation*, and says
  so. Under a physical law that justification inverts: the scene picks its own scale and the
  light follows.
- `tidewater.rs:199-203` — the intensity is large "because the falloff below is a constant of
  the standard shading"; it will no longer be.
- `foldyard.rs:273-277`
- `sunlight.rs:248-251` — "the intensity is only 'bright enough that the floor is not mud'",
  which is a calibration against the old law.

### 5. The exporter maps straight through

With the document in physical units, `cycles_export::light` writes `intensity` to the
`emission` node and `color` to the light's `strength` with no scale factor of any kind. This
is the outcome that makes the exporter honest: it maps, and it does not judge.

## What is already correct and needs no change

`radius` stays unmapped. Our `radius` is **how far a light reaches** for cluster culling
(`scene_schema.rs`, `radius`), which is not a physical source size, and Cycles' `PointLight::radius`
is. A reach of sixty units would become a light a hundred and twenty units across. Cycles'
zero radius gives a delta light and hard shadows, which is the honest rendering of a document
that says nothing about source size.

## Decided

**The singularity is accepted.** The law is the inverse square with no softening term. The
template already floors the distance at `max(length(to_light), 0.0001)` before dividing, which
is there to keep `direction = to_light / light_range` from being `NaN` at zero, and it has the
same effect on the attenuation: the worst case is `1 / 0.0001²`, a hundred million, which is a
large number in `f32` and not an infinity. A light standing inside a surface is a scene's own
doing, and the picture it produces is the picture that scene asked for. If a scene turns out
to need a floor of its own, that is a field the schema can grow then, with a scene that wants
it in front of it.

The argument the old comment made — that a steeper law puts a light's own volume inside the
scene, so every surface near the light reads as white whatever its colour — was an argument
about the *old* constant's job, not about the law. Its answer was to flatten the law over one
preferred scene size; the actual answer is that a scene states how big its lamps are, which is
what `power` now is.

**The schema distinguishes the kinds.** One field carrying four units, whose meaning depends
on a tag two fields away, is the thing being removed:

| kind          | field        | quantity                                        |
|---------------|--------------|-------------------------------------------------|
| `Point`       | `power`      | total radiant flux                               |
| `Spot`        | `power`      | total radiant flux                               |
| `Directional` | `irradiance` | flux per unit area on a perpendicular surface     |
| `Environment` | `radiance`   | light leaving the image per area per solid angle  |

`LightKind::brightness_field` in `emit.rs` is the one place that knows the mapping, so the
compiler reads the right field from the tag it has already read and no call site restates it.

A document carrying the wrong one is refused with a missing-field error, which is the failure
this split exists to make possible: a number whose unit depends on a neighbouring tag cannot
be checked by reading it.

## Only point and spot lights move

A `Directional` light and an `Environment` light are **not** attenuated. `shaders.rs` says so
directly: a clustered light's brightness "arrives already scaled by its falloff and its
shadow, and the directional one arrives unscaled, because a light that reaches every distance
equally has no falloff to apply." So their numbers carry over unchanged and only the field
they are written under changes — which is also why `irradiance` and `radiance` are honest
names: those two kinds have no distance to be a power over.
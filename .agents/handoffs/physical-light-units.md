# Physical light units

Branch `physical-light-units`, branched from `main` at `f023ce3`.

## What is being changed

A light's brightness leaves the document. The single `intensity` field every kind of light
carried is gone, replaced by one field per kind in a real unit, and the standard shading
template's distance falloff becomes the inverse square law.

| kind          | field        | quantity                                          |
|---------------|--------------|---------------------------------------------------|
| `Point`       | `power`      | total radiant flux                                 |
| `Spot`        | `power`      | total radiant flux                                 |
| `Directional` | `irradiance` | flux per unit area on a perpendicular surface      |
| `Environment` | `radiance`   | light leaving the image per area per solid angle    |

The proposal and the reasoning are in
[`.agents/proposals/physical-light-units.md`](../proposals/physical-light-units.md). Two open
questions in it are now closed: the near-field singularity is **accepted** — the template's
existing `max(length, 0.0001)` already keeps `1/d²` finite — and the schema **distinguishes**
the kinds rather than letting one field carry four units.

## Why the falloff changed

The old law was `1 / (1 + 0.0035r + 0.00002r²)`, whose own comment placed its constants "in
a scene a couple of hundred units across". A law fitted to a preferred scene size is
normalised to that size, which is what this removes. Two proposals had already recorded it as
a defect — `photographic-scenes.md` item 6 calls it "a factor the author has to know about and
cannot set", and `agent-scene-iteration-loop.md` records that two agents inherited it and
applied it to scenes fourteen times its intended size.

## The calibration

Only `Point` and `Spot` are attenuated; `shaders.rs` states that a `Directional` light
"arrives unscaled, because a light that reaches every distance equally has no falloff to
apply". So directional and environment values are carried over unchanged.

Every point light's `power` is `old_intensity × f_old(d) × d²` at the light's distance `d`
from its subject, which reproduces the old brightness at that distance and nothing else —
the gradient across each scene is now steep where it was flat, and that is the change working.

| scene      | light   | `d`   | old     | new        |
|------------|---------|-------|---------|------------|
| `dunes`    | sun     | 217   | 3.6     | 62,800     |
| `dunes`    | sky     | 220   | 0.5     | 8,840      |
| `monoliths`| sun     | 82    | 4.0     | 19,100     |
| `lanterns` | moon    | 166   | 1.5     | 19,320     |
| `lanterns` | near    | 26    | 2.4     | 1,479      |
| `lanterns` | far     | 25    | 2.0     | 1,109      |
| `foldyard` | sun     | 400   | 11.0    | 314,000    |
| `tidewater`| sun     | 716   | 34.0    | 1,267,000  |
| `tidewater`| sky     | 578   | 13.0    | 447,500    |
| `spotlight`| lamp    | 10    | 1.15    | 103        |
| `stilllife`| window  | 388   | 1.22 ea | 34,222 ea  |
| `stilllife`| bounce  | 272   | 0.45    | 9,700      |
| `stilllife`| room    | 286   | 0.35    | 7,870      |

`landscape` is the one that is 38,761 units across, and its lamp stands about 23,000 units
above the plain. At that scale the old law's quadratic term is 10,816 against a linear term of
81 — **the old law was already inverse square here** — so its power is the constant `450,000`.

The first attempt wrote it as `9.0 * (1.2 * radius)²` on the reasoning that a family
parameterised by `radius` must scale with it. That was wrong by about ten thousand times, and
the render said so immediately: a frame ten thousand times too bright is not a bright frame but
a clipped one, and the picture came back washed to near-white with the far ranges dissolved into
the haze. A constant is correct here precisely *because* the law is already scale-free at this
scene's size.

## What the renders showed

Three scenes rendered on the new code and on unmodified `main`, same build, same machine.

- **`dunes`** — indistinguishable. The calibration reproduces the old brightness at the
  reference distance, which is the field's centre, which is what the picture is mostly made of.
- **`landscape`** — indistinguishable, once the power above was corrected.
- **`stilllife`** — **visibly different, and the difference is real rather than a rounding
  error.** The wall behind the jug is lighter and the jug's shadow on it reads more strongly.
  The cause is that the scene's three lights were calibrated against the *table*, and the wall
  sits nearer the two fills than the table does, so under a real falloff it gains what the
  table loses. The subject — bowl, cloth, lemons, jug — is close to unchanged.

That last one is the cost of removing the flat law, stated plainly: with a real falloff, which
distance a scene is calibrated at decides which parts of it move. A scene with a near subject
and a far background now has to name both.

## Files

- `crates/scene/src/standard/scene_schema.rs` — the field split, and the four unit builders
- `crates/scene/src/standard/emit.rs` — `LightKind::brightness_field`, and the deletion of
  `FALLOFF_LINEAR`, `FALLOFF_QUADRATIC`, `FALLOFF_TUNED_SPAN`, `tuned_fold`, `falloff`,
  `falloff_fold`, `half_diagonal`, `FALLOFF_FOLD_BAR` and `scale_notes`
- `crates/scene/src/standard/example.rs` — the built-in example light
- `crates/gallery/src/shaders.rs` — the attenuation itself
- `crates/gallery/src/kit.rs` — the four light builders
- `crates/gallery/src/{dunes,monoliths,lanterns,landscape,foldyard,tidewater,spotlight,stilllife,sunlight}.rs`
- `crates/scene/tests/scale_notes.rs` — **deleted**; it tested the two constants this removes
- `crates/scene/tests/{spot_lights,environment_light,directional_lights,camera_notes}.rs`

## What is left

- [ ] Confirm the pictures still read. The brightness at each scene's reference distance is
      preserved by construction, but the gradient across every scene is now real, and none of
      these frames has been rendered since.
- [ ] `stilllife` and `tidewater` set their own `exposure`, and those were left alone on the
      reasoning that a power calibrated to preserve scene brightness preserves the exposure
      with it. That is an argument, not a measurement.
- [ ] `SPEC.align.md` lines 793-805 still describe `intensity` as a field every kind carries.
      SPEC itself is human-editable and was not touched.
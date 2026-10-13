# The emitter writes a spot cone that is not the cone the scene asked for

Measured 2026-10-14, Cycles 5.3.0 at `C:\resource\cycles\install\cycles.exe`, against
`C:\resource\cycles` at `a456b761034dda42c32eef9f4aae0fa5a5c9f604`. Probes: `out/probe-spot-cone.py`
(arithmetic on the two formulas) and `out/probe-spot-profile.py` (the two formulas measured off two
renders). Documents: `out/rz-spotlight.cycles.xml` (what the emitter writes), and
`out/sc-b0-written.cycles.xml` / `out/sc-b0-corrected.cycles.xml`, which are that document with one
number changed and the bounce depth set to zero.

## The question

`spotlight` is the one gallery scene lit by a cone, so it is the only place a spot light's falloff can
be read at all. Its cone does not fall off the way the document asked: across the ring between the two
angles the traced frame is much brighter than the rasterised one. Is the emitter writing the wrong
cone, and if so which number is wrong?

## What the two renderers compute

Both ramp from nothing at the outer half angle to full at the inner one, and both compare a cosine
against two cosines. They differ in **what the ramp is a fraction of**, and in **the shape of the
ramp**.

Ours, `crates/gallery/src/shaders.rs:1908-1925`:

```wgsl
let facing = dot(to_surface, cone.axis.xyz);
if facing <= outer { return 0.0; }
if facing >= inner { return 1.0; }
return (facing - outer) / (inner - outer);
```

A **straight line in the cosine**, over the band `cos(outer) .. cos(inner)`.

Cycles, `src/kernel/light/spot.h:27-31` with `src/scene/light.cpp:177-180`:

```c
return smoothstepf((ray.z - spot->cos_half_spot_angle) * spot->spot_smooth);
...
const float spot_smooth = 1.0f / ((1.0f - cos_half_spot_angle) * smooth);
```

A **smoothstep**, over the band `cos(outer) .. cos(outer) + (1 - cos(outer)) * smooth`.

## What the emitter writes

`crates/scene/src/cycles_export/light.rs:363-375`:

```rust
attributes.push(("angle", format!("{}", outer * 2.0)));
attributes.push(("smooth", format!("{}", ((outer - inner) / outer).max(0.001))));
```

`angle` is right: the document states half angles and the loader takes the whole cone.

**`smooth` is wrong, and it is wrong in kind rather than by a margin.** The kernel reads it as a
fraction of `1 - cos(outer)` — a band measured in the **cosine**. The emitter writes
`(outer - inner) / outer`, a fraction of the **angle**. The two are the same only in the limit where
the cone is narrow, because `cos(inner) - cos(outer) ~ (outer - inner) * sin(outer)` and
`1 - cos(outer) ~ outer^2 / 2`, and their ratio is `(outer - inner) / outer` only while `outer` is
small. **A narrow cone hides the mistake, which is why it survived.**

For `spotlight`, whose two half angles are 0.16 and 0.56 radians:

| | value |
|---|---|
| inner, outer | 9.17 and 32.09 degrees |
| what the emitter writes, `(outer-inner)/outer` | **0.714286** |
| what the kernel wants, `(cos inner - cos outer)/(1 - cos outer)` | **0.916379** |
| ratio | **1.2829** |
| where Cycles reaches full strength with what it was given | **16.99 degrees** off axis |
| where the scene asked for full strength | **9.17 degrees** off axis |

**Cycles' full strength core is 1.85 times as wide as the document's**, and its soft band is
correspondingly thinner. The largest disagreement between the two curves is **+0.2767** at 20 degrees
off axis — Cycles 0.937 where ours is 0.660.

## That it is the render, and not only the arithmetic

The arithmetic above is a claim about a formula. It was checked by rendering the same document twice
with **one attribute changed**, both at `max_bounce="0"` so that indirect light cannot fill the rim
of the cone and flatten the very thing being measured:

- `out/sc-b0-written.png` — `smooth="0.7142857142857143"`
- `out/sc-b0-corrected.png` — `smooth="0.9163789082272744"`

`out/probe-spot-profile.py` walks the camera's rays onto the floor plane, works out each floor
point's angle off the cone axis, and takes the median pixel in each one degree bin. The rasterised
frame is ungraded first (`c/(c+1)` then `pow(c, 1/0.4545)` inverted), and each curve is normalised to
its own value on the axis, because this build of Cycles writes scene-linear radiance straight into the
png and the two are not on the same scale.

Root mean square gap to the rasterised curve over 10 to 30 degrees off axis:

| | RMS |
|---|---|
| Cycles, with the number the emitter writes | **0.2744** |
| Cycles, with the number the kernel wants | **0.1721** |

A 37 per cent reduction from one number. The median profile agrees: at 20 degrees off axis the
written build reads 0.736 and the corrected one 0.569, against 0.350 in the rasterised frame.

## The control that made the number mean something

Comparing a render against a formula only says anything if the formula is the whole of what was
rendered. The first version of this comparison was not, and it produced a finding that was wrong.

A ring of a pool is **not one distance and not one angle of incidence**. The lamp stands 8.5 above
the floor, so along the ring the far side is both further away and more oblique, and a Lambertian
floor loses light to both: the predicted brightness goes as `cone(theta) * N.L / r^3` per pixel,
with `N.L = 8.5 / r`. Binning by angle and dividing only by `r^2` leaves the incidence term in, and
a curve that still carries it looks steeper than the cone alone.

With the two terms put back per pixel and binned exactly as the images are:

| | RMS against the measured rasterised curve, 10 to 30 degrees off axis |
|---|---|
| **predicted from `shaders.rs`, the rasterised path's own formula** | **0.1522** |
| **Cycles measured, with the corrected number** | **0.1721** |
| predicted from `spot.h`, with the corrected number | 0.1763 |
| predicted from `spot.h`, with the number as written | 0.2744 |
| **Cycles measured, with the number as written** | **0.2744** |

Three things follow, and the first two are the load-bearing ones.

**The traced render is following the kernel formula exactly.** Measured against the written number
it scores 0.2744 and the formula predicts 0.2744; measured against the corrected number it scores
0.1721 and the formula predicts 0.1763. The document and the render and the source agree to four
figures either way, which is what makes the corrected number a measurement rather than a reading.

**The rasterised path is drawing the cone its schema asks for.** Its own formula is the closest of
the four curves to its own image at 0.1522. **There is no second defect on this side**, and the
first version of this note said there was — it reported the rasterised curve as 0.2718 away from
its own model, which was the missing incidence and inverse square and nothing else.

**What is left is a shape, and it is smaller than the defect that was just fixed.** The residual
0.1522 is not the shadow page: inside the 26 to 30 degree band the rasterised brightness falls
smoothly and monotonically from 8.5 to 13.75 metres from the lamp with no step at any distance, and
the only cut anywhere — in the 22 to 26 degree band at about 12 metres — appears in both images and
is the cone's own edge. It is not the cone axis either, which is normalised at
`crates/scene/src/standard/emit.rs:1651-1657`. It is not the two angles, the band, or the angles'
cosines. **It is unexplained, and it is about half the size of the emitter defect that was fixed
here**, which is the honest way to leave it.

## What is not proposed here

Nothing is proposed. The one-line correction in `light.rs` has been taken, as `dacf474`. Whether
the rasterised ramp should become a smoothstep to match Cycles' is still a decision about what this
engine is rather than a measurement, and it is not made here — the shape difference is worth about
0.096 near 29 degrees off axis, against the 0.2767 that the wrong band cost.
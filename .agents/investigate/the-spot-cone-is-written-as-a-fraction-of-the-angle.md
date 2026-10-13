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

## Two things this did not settle

**The residual is the shape of the ramp.** Cycles uses `smoothstepf`, `x*x*(3-2x)`, and ours is a
straight line. They agree at both ends and at the midpoint and differ in between, so matching the band
cannot make them the same curve. With the band corrected the largest disagreement is **0.096** near 29
degrees off axis — real, and about a third of what the wrong band cost. Which of the two is the
reference is a decision about what this engine is, not a measurement; it is not made here.

**The rasterised frame does not follow its own cone model either.** `predicted, ours` — the formula
in `shaders.rs` evaluated at these two angles — scores RMS **0.2718** against the measured rasterised
curve, which is as large a gap as the traced frame had before this was found. The measured curve is
already **0.917 at 4 degrees** off axis and **0.879 at 8**, inside a 9.17 degree full strength core
that the model says reads 1.00 at both. So the rasterised cone falls off faster than the code that
draws it says it should, and **this is unexplained**. It was checked and is not the cone axis:
`crates/scene/src/standard/emit.rs:1651-1657` reads the axis through `read_direction`, which returns
it already normalised. Nothing else was checked, and the profile above is the evidence to check
against.

## What is not proposed here

Nothing has been changed. The one-line correction in `light.rs` is available and is measured; whether
to take it, and whether the rasterised ramp should become a smoothstep to match, are decisions the
superior makes rather than facts this note establishes.
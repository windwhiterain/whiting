# Water as a material, not a shader somebody wrote

## What exists today

`tidewater` has a lake. It is drawn by this:

```text
mesh_material("water", "water", surface([0.0; 3], 0.0, 0.02, [0.0; 3]))
```

bound to `Stage::Emission` (`crates/gallery/src/tidewater.rs:325`, `:410`), and the appearance is
forty-five lines of WGSL pasted into the scene — `water_body()` at `tidewater.rs:651`, with
`WATER_NORMAL` at `:603` supplying the wave normal. What it computes, per fragment:

| term            | how                                                                   |
|-----------------|-----------------------------------------------------------------------|
| wave normal     | four crossing swells, analytic, hardcoded                             |
| Fresnel         | `0.02 + 0.98 * pow(1 - dot(N, V), 5)` — Schlick with water's own F0   |
| reflection      | `sky_colour(reflect(-V, N))` — the environment map, by reflected ray  |
| body            | a constant `vec3(0.010, 0.016, 0.021)`                              |
| sun glitter     | `pow(dot(N, H), 240) * 0.5` against a hardcoded `SUN_DIRECTION`       |
| aerial          | `aerial_penetration` mixed toward `aerial_colour`                     |

There is **no refraction and no depth read**. The body colour is a constant, so a shallow shore
and a deep channel are the same colour and nothing under the surface is visible.

## Why the path tracer has no water

Not because Cycles cannot draw water. `principled_bsdf` draws it correctly out of the box. The
Cycles exporter reads only the **Lighting** stage's materials and skips the rest
(`crates/scene/src/cycles_export/material.rs`), and the water is an **Emission** material, so it
is reported and skipped with its instance. `tidewater` emits 212 meshes and not one of them is
water.

The obstacle is therefore not the format and not the renderer. **It is that the water is a
bespoke WGSL blob living in a scene file, so there is nothing in the document for any backend to
read.** A second backend cannot have an opinion about a thing the document never says.

## The proposal

**Make water a row in `custom_data` on a Lighting material, and let both backends read it.**

### 1. The water moves from Emission to Lighting

A water surface is a transmissive dielectric, not an emitter. It should be shaded by the
lighting pass like any other surface, with a transmissive closure.

This is the load-bearing change. `crates/scene/src/cycles_export/material.rs` already emits
every Lighting material and already writes `roughness` and `metallic` from `custom_data`; a
lighting material that carries `transmission` and `ior` needs **no new code in the exporter at
all**. The special case disappears rather than being extended.

It also fixes the rasterised side on its own terms: the Lighting pass is where a surface is lit,
and the water is currently the one surface in the scene that no light touches.

### 2. New `custom_data` fields

The existing `surface()` row already carries `roughness`, which water needs and already has
correctly set (`0.02`). Add:

| field             | meaning                                                        |
|-------------------|----------------------------------------------------------------|
| `transmission`    | zero to one; `1.0` is water                                      |
| `ior`             | `1.333` for water; defaults to `1.45` for a dielectric          |
| `absorption`      | per-channel extinction coefficient, so depth means something    |
| `depth_tint`      | the colour of the body, for the no-depth fallback                |
| `wave_amplitude`  | how far the ripple normal is displaced                          |
| `wave_scale`      | the wavelength, in world units                                  |

`absorption` and `depth_tint` together replace the constant body colour, and the split matters:
`depth_tint` is what a surface shows when there is no depth to read, and `absorption` is what it
shows when there is.

### 3. The depth buffer is already bound — this is free

**The Emission pass already binds both `COLOR_TARGET` and `DEPTH_BUFFER`**
(`crates/scene/src/standard/pass_schemas.rs:464-469`), and the Lighting pass binds both as well
(`emit.rs:3822-3827`). `debug.rs:400-406` shows the read pattern: a `textureLoad` on the depth
binding, no sampler.

So a water material can compute real absorption — `exp(-absorption * path_length)` — with **no
pipeline change at all**. The path under the surface is recovered from the depth buffer the
depthing pass already produced, which is why the water's own mesh is in the depthing bucket
(`tidewater.rs:408`) as well as the emission one.

This is the single biggest quality difference available, and it costs nothing structurally.

### 4. What is deliberately **not** in this

- **No screen-space refraction.** It needs a colour copy, and a lake at eye level is nearly all
  environment reflection at grazing angles, which the environment lookup already gets right. Say
  so rather than build it.
- **No vertex displacement.** The current ripple is a normal only, and a still lake's silhouette
  does not need to move. See `REED_SWAY`'s own note at `tidewater.rs:700-708`, which already
  argues this.
- **No Caustics.** Cycles can do them; the rasteriser has no concept, and a lake's caustics are
  on the seabed, which is underwater and mostly invisible at these angles.

## What the rasteriser gains

The scene writes `mesh_material("water", "lighting", water(...))` and the water stops being 45
lines of WGSL in a scene file. It becomes:

- **reusable** — a second lake is a row, not a copy of a shader;
- **depth-aware** — a shore goes pale and a channel goes dark, for free;
- **readable by the path tracer** — the exporter picks it up with no special case.

## Open questions for the superior

1. **Wave normal: analytic or texture?** The current four swells are analytic in WGSL and are
   crisp at any distance. A generated normal map is more art-directable and would need a
   convention for scale across scenes. Recommend analytic, parameterised — it is what the scene
   already has and it needs no asset.
2. **Does `Emission` survive at all?** If water is the only user, the stage may be removable.
   Not assumed here; `monoliths` uses a screen material on `Post` and the grep for other
   `Emission` users was not exhaustive.
3. **Ordering.** A transparent surface in the lighting pass needs a draw order against opaque
   geometry. The lake is flat and below everything, so it may be trivially correct here and
   wrong in general. Worth deciding before the first scene that is not a lake.

# Terraces is out because its terrain is a WGSL material the document cannot carry

Measured 2026-10-14, Cycles 5.3.0 at `C:\resource\cycles\install\cycles.exe`, at `323322c` and
later in the tree. Probe: `out/probe-grade-matched.py`. Frames: `out/rlx-terraces.png`
`ff3dc11eb4695c8c` (traced, film exposure 0.25 with the exposure divided back out) against
`out/rrf-terraces.png` `1aa14dd4ec78b2ae` (rasterised, re-rendered from the same build as the traced
one and byte identical to the earlier `out/rr-terraces.png`).

## The question

`dunes` agrees with the rasterised backend to **1.003** whole frame with a pearson of **0.9940**,
and `landscape` to 0.981, both read each frame the way its own writer wrote it with nothing clipped.
`terraces` is 1.318 and 0.6496. Same method, same build, same day. What is different about it?

## Its terrain declares no albedo at all

`crates/gallery/src/terraces.rs:440-446`:

```rust
// The terrain's albedo is decided per fragment by the surface; the row only
// carries the light's own terms. A roughness near a half is wet earth's sheen
// under the sky, and the water's sharp glint is the reflected sun the surface
// adds, not this lobe.
surface([1.0, 1.0, 1.0], 0.0, 0.55, [0.0; 3]),
```

**White.** The material carries no colour because the colour is computed per fragment in
`TERRAIN_SURFACE`, `terraces.rs:809-897`: a per-fragment albedo mixing zones, and then the water —
`terraces.rs:881-896` —

```wgsl
let mirror = 0.035 + 0.965 * pow(1.0 - facing, 5.0);
let reflected = sky_colour(reflect(view_dir, water_normal));
color = mix(color, reflected * open, clamp(mirror * water * 1.02, 0.0, 0.96));
```

a Fresnel mix towards an **analytic sky function evaluated along the reflected ray**, with a rippled
normal and lanes of wind. What the picture's blue-green paddies and their blaze on the sun's quarter
are is that block of WGSL.

**The document can carry `base_color = 1, 1, 1` and nothing else.** So the traced frame is white
snow, and the rasterised frame has rock, trees and water. That is what 1.318 and 0.6496 are
measuring.

## It is not a dropped field, and the report says so correctly

`terraces` drops **675** fields and the reason list is fully accounted for: **405** instances of
`custom data 'reflect', which has no `principled_bsdf` socket to fold into, and **270** rasterised-only
shader stages. **Neither can whiten a material, and nothing is missing from the list.** The document
carries the rock at `0.55, 0.45, 0.33`, the snow at `0.76, 0.71, 0.62` and the trees at
`0.1, 0.11, 0.14` — those are the walls, the roofs and the firs. The terrain's three colours are
`0.086, 0.155, 0.076` and near neighbours in the document too, but they are the per-instance variants
that the terrain does not use, because the terrain's albedo is a fragment function and not a field.

**A `LossyReport` that lists only declared-and-unmappable fields cannot report a material that
declares nothing to map.** This is not a gap in the report; it is the report being about a different
question.

## It is a known limit of the emitter, already written down

`cycles-procedural-materials.md` established that this build registers and renders
`noise_texture`, `voronoi_texture`, `wave_texture`, `gradient_texture`, `mix_color`, `mix_float`,
`mix_closure`, `rgb_ramp`, `bump`, `math`, `vector_math`, `map_range`, `fresnel`, `layer_weight`,
`sky_texture`, `glossy_bsdf`, `diffuse_bsdf` and `transparent_bsdf`, with no OSL needed — a `<shader>`
graph can express what `terraces` computes per fragment.

It also named the obstacle exactly: **"whiting currently emits one constant `principled_bsdf` per
material and has no document-level vocabulary for a graph."** `terraces` is that sentence with a
frame attached.

So `terraces` is not a renderer disagreement, not a lost field and not clipping — **it is the one
scene in this comparison whose subject matter the target format cannot express.** Its per-band ratio
rising monotonically from 1.065 at the sky to 1.571 at the near ground is the mirror term: the near
paddies reflect the zenith and the far ones the horizon, and none of that reaches the document.

## What this does not settle

**Whether whiting should gain a graph vocabulary.** That is a design decision and belongs in a
proposal, as `cycles-procedural-materials.md` says. This file only establishes that `terraces` is the
instance that makes the decision necessary rather than theoretical.

**Which other scenes are in the same class.** `landscape` paints ranges, shafts and cirrus into its
sky and `terraces` fogs its whole frame, both in the post stage, and both are composition differences
rather than shading ones — but their *materials* do carry albedo, which is why they agree to within
two per cent. **Only `terraces` was found to have a white-declared terrain material.** The other
scenes were not checked for it.

## Not measured

**Whether the graph spelling that would carry the water mirror actually works in this build.** The
nodes are registered and render, per `cycles-procedural-materials.md`, but a Fresnel mix towards a
sky function along a reflected ray with a rippled normal is a specific graph and this was not built.

**The other four scenes** — `sunlight`, `tiles`, `spotlight`, `tidewater`, `foldyard`, `monoliths` —
carry none of the figures in this investigation.
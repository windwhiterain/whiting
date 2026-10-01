# Environment light: what the first version does not do, and what each item costs

Image based lighting landed as one environment per scene, sampled as a two dimensional octahedral
map with the box filtered level chain an uploaded image already has, driving the one ambient term
the clustered lighting pass computes. The reasoning is in `SPEC.align.md` under "Image based
lighting". This file is the list of what that leaves, what each item actually costs, and which of
them is a bug rather than a refinement.

Ordered by how much a scene would feel it.

## 1. No specular environment, and the diffuse term is the only one

The environment reaches the frame through the diffuse path alone. A smooth surface reflects it
nowhere: the specular term is still the single point light's `pow` lobe against `f0`, so a mirror
in an environment is a mirror of the lamps and not of the sky. This is visible as soon as a scene
has a reflective material and no lamp in the direction it is reflecting.

A real fix is a **split-sum approximation**: a prefiltered specular mip chain (the same box
filtered chain, with the roughness widened rather than narrowed) plus a small irradiance term for
the `DFG` term, and a second uniform vector carrying the map's average. None of that needs a
schema change and all of it needs shader work this stage deliberately did not do.

**Cheaper first**, and worth doing before the split-sum: sample the environment in the existing
specular path using the reflection vector and the same roughness-derived level, with the lobe's
`strength` and `fresnel` already computed. That is wrong by a constant factor and by the missing
`D` and `G` terms, and it is visibly better than a mirror of nothing for about twenty lines.

## 2. No rotation, so the map is pinned to world axes

The octahedral map's centre is the world's `+z` and its top row is `+y`. An author who wants the
sun's glow at a bearing other than `+z` cannot put it there, and a scene whose sky is authored
from a reference photograph will be rotated by however much the world's axes disagree with the
photograph's.

A `rotation` on the light — a single `f64` about `y` — is the cheapest fix and is pure data. A
full `direction` is more useful and costs the same in the shader (a `rotateY` before the fold).

**This is the one gap in the list that a scene author will hit first**, because a sunset map is
the obvious first thing to author and a sunset has a bearing.

## 3. One environment per scene

A second `Environment` light is refused by name rather than ignored, which is the right failure
and the wrong ceiling. Supporting two means a second binding point and a second uniform field
(`environment2_map`, `environment2`), and a fragment adds two terms. Additive and reversible in
one place, but it doubles the fixed per fragment cost of the term for a feature nothing uses yet.

## 4. The environment light still occupies a light record

96 bytes and one index, for a light that is in no cluster list and gets no page, so no fragment
ever loads its record. Kept because the record's index is what the cluster lists point into and
because the invariant that an exempt light keeps its record is what a `Directional` light's own
path will need. Not worth the index shift for one light, and it becomes worth revisiting the day
a scene can name a thousand of something.

## 5. ~~A scene naming no light at all does not render~~ — fixed

**This was a bug, not a refinement, and it was pre-existing.** A scene with an empty `lights`
list packed a light record buffer of four bytes, and the shader's `array<LightRecord>` wants a
whole 96 byte record, so wgpu refused the draw with a validation error naming a buffer size.

**Fixed.** `lighting::buffers::light_records` now writes one placeholder record when the light
list is empty: a zero colour, the origin, a zero radius, and `NO_PAGE` with the identity
projection. No cluster points at it — with an empty list every cluster's count is zero, so the
loop that would load it never runs — which is the same reason an environment light's own record
carries `NO_PAGE`. The workaround this was found through, a point light of **zero intensity** in
the environment render test, is gone, and that test now exercises a scene whose only light is
the environment and a scene with no light at all.

## 6. The seam clamp is argued, not proved

A direction on the square's edge computes a texel index one past the end, and the shader clamps
it. On this machine the clamp is **not observable**: wgpu's out of range `textureLoad` returns
the last texel anyway, so deleting the clamp leaves the pixel test green. The clamp stays
because an out of bounds read is undefined and a different backend need not be forgiving, but
the test does not hold it. A test that could would need a backend that faults, which is not a
thing to build for this.

What the pixel tests *do* hold: the fold's vertical axis, by swapping the two probed surfaces
completely, and the roughness to level mapping, by the ground's value moving from the map's
brightest row to the map's single averaged texel.

## 7. The map has no colour space of its own and no exposure

The map is whatever format the scene's image declares, read as `rgba8unorm` or `rgba8unorm-srgb`
through `textureLoad`, which on an sRGB format the hardware decodes. There is no exposure, no
`intensity` curve and no units, so a map authored at one exposure and a scene at another need
the author to match them. An `exposure` on the light is a fifth uniform number; a scene that
wants it should say so rather than bake it into pixels it cannot inspect.

## 8. Anisotropy and seams in the level chain

The octahedral map's level chain is a plain box filter, so the seam is averaged **across** it at
coarse levels: at the last level the whole map is one texel and the seam is a line of blended
texels running through it. At level zero the seam is invisible because the read is point sampled.
A seam-aware filter would weight each texel by its solid angle over the destination texel, which
is the same filter a cube map's seam handling needs and is about thirty lines in
`filtered_level` — but it changes every mipped texture in the renderer, not just this one, so it
is a change to the device and not to a shader.

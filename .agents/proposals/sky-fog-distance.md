# Fog on sky pixels uses a far plane's worth of air

Found while building `bamboo` (worktree `.worktrees/k3`). **Not fixed.** The scene it was
found in was not merged, so nothing below ships; this is the record of a defect that is still
in the tree. Verified against `terraces.rs:1015` at `7f5ff03`, where the recovery still reads
`eye + ray * min(t, 2400.0)`.

A post stage that fogs by distance recovers a world position from the depth buffer, and
for a sky pixel nothing was drawn: the recovery (`terraces`' `world_position`) runs the
ray out to its clamp — 2400 units — and the aerial-perspective term then integrates
**2400 units of air**. In `bamboo` an aerial coefficient of 0.00028 gave the sky an
optical depth of 0.67: half the zenith's colour replaced by the haze grey, which is why
every wide frame came out milky until the grade was adjusted around it.

`terraces` carries the same latent wash (0.00055 × 2400 ≈ 1.32 optical); its frames are
bright enough that it reads as haze rather than as milk, but the number is an accident of
the clamp, not a choice about weather.

## The scene-side workaround, which shipped in `bamboo` and did not land

```wgsl
var air = world;
if depth >= 1.0 {
    air = uniforms.camera.position + ray * 300.0;
}
color = fog(color, ray, air);
```

A sky pixel fogs by a representative air path (300 units — a few times the mist's own
scale depth) rather than by the far plane. Geometry pixels are untouched, and the mist on
the valley floor below a sky pixel still comes from the real depth.

Kept as the shape of the fix rather than as a diff, because **the choice of 300 is the
problem this file is about**: it is a number standing in for a distance the framework has
no way to name. A scene that adopts the workaround has chosen its weather twice, once in
the aerial coefficient and once in that literal.

## The framework-level question

The clamp is needed because a depth of one has no position; the question is what distance
*means* for a pixel that shows air. Options: a `sky_distance` the post uniform carries
(scenes set it beside `frame`), or the aerial term dividing height fog and Rayleigh air
so each is integrable along an open ray in closed form. The second is the honest one and
is a real piece of work; the first is a field.

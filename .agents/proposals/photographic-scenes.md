# What a scene cannot say, and what it costs the picture

Status: proposal, ready for review. The evidence is from building `tidewater` — a lake at
first light — against the standard pipeline, and every claim below is a thing that was tried
and measured rather than one that was reasoned about in advance.

`dunes`, `lanterns` and `terraces` were written before most of this vocabulary existed and
work around all of it. The line that separates a diagram from a photograph runs through
these gaps, and the four below are the ones that block a scene outright.

The first four are the ones a reviewer should rule on. Three more turned up while building
and are lower priority; they are at the end, and the first two of them are small.

## Do not overlap `feature/lighting`

`.worktrees/lighting` owns the lighting path. Nothing here asks for a per light pass, a
cluster grid, a shadow page or an environment map, and the one finding that touches the
lighting path — a mirror drawn once per light — is answered by the **emission stage that
already exists**, not by anything stage 3 or 4 changes. Stage 5's environment light would
also answer part of finding 6, and it is left there deliberately.

## 1. A surface body is spliced in after the shading, so it cannot displace the normal

**What stands in the way.** [`shaders::lighting`](../crates/gallery/src/shaders.rs) computes
`normal`, then `lambert`, `fresnel`, `specular` and the offset the shadow is looked up at,
and *then* splices the scene's body. A body spliced there can add to a colour it did not get
to influence. It cannot say "this surface is lit by a different normal", and the normal is
the input to every term in the list.

**What it costs.** Water, foliage, cloth, brushed metal, a beach — anything whose shading is a
function of a normal that is not the geometric one — are unreachable. Not hard to reach:
unreachable without reimplementing the whole lighting stage, shadow lookup, uniform layout and
all, by hand, from a template the scene did not choose.

**The change.** One splice point, before the shading, where the normal arrives and before
anything reads it. `var normal = normalize(in.normal); {displace}` — the body may reassign,
and an empty body changes nothing. Implemented as
[`lighting_with_normal`](../crates/gallery/src/shaders.rs) and used by `tidewater` for the
sway on its reeds; a scene that does not displace a normal compiles to the same `WGSL` as
before, which is the additive property every other change here also holds.

**The part that is still missing.** A displaced normal is a per-fragment lie about geometry. A
leaf that turns, a coat of cloth, a wave — all of those want the *silhouette* to move too,
which is a vertex-stage hook and a different decision. `tidewater`'s reeds sway in shading
only, and the file says so rather than pretending otherwise.

## 2. No value reaches two stages, so the same air is written twice and disagrees

**What stands in the way.** The air is one thing. The post stage draws the sky, the lighting
stage hazes every surface towards it, and a water surface reflects it — and there is no
binding that spans the stages, so each is a separate `WGSL` string the scene writes out.

**What it costs.** Where the far water meets the sky there is a seam, and a seam is the single
most reliable way a rendered landscape announces itself. This is not hypothetical:
[`dunes`](../crates/gallery/src/dunes.rs) already has the bug and
[SPEC.align.md](../SPEC.align.md) already records the symptom — *"the horizon has a bright
seam where the field's far edge meets the sky"*. The two values are
`haze_colour = vec3(0.58, 0.62, 0.72)` in the lighting body and
`horizon = vec3(0.70, 0.66, 0.60)` in the grade. They are the same physical quantity and they
differ by a fifth.

**The change.** The scene document carries the atmosphere once — a sky, a ground, a density,
a height falloff — and the compiler supplies it to every stage that needs it, the way it
already supplies `time` and the projection. A scene that names no atmosphere is unchanged, and
a scene that names one cannot disagree with itself.

**What it buys beyond the seam.** Everything a scene currently hand-rolls per surface. Every
outdoor scene in the gallery has its own aerial perspective, each with its own haze colour and
its own distance law, and each is a place for the bug above. A scene should say "the air" and
get it everywhere.

## 3. A mirror drawn in a lighting pass is added once per light

**What stands in the way.** The lighting stage runs once per light and its passes add, so
anything a lighting body computes is computed and added per light. A surface whose colour
comes from a reflection rather than from a light — water, a mirror, a window — therefore
comes out once per light.

**What it costs.** `tidewater`'s lake was drawn in a lighting pass and came out **twice as
bright as the sky above it**: white, clipped, no depth, no Fresnel gradient. The fix is not a
workaround, it is the stage the framework already has — the **emission** stage exists for
exactly this, "added once rather than once per light", and drawing the water there is both
correct and a third of the passes. The trap is that nothing says so. The name "emission" is
about light a material gives off, and a mirror gives off nothing, so a scene author draws
water where everything else is drawn.

**The change.** The stage is right; the discoverability is wrong, and the underlying gap is
that the shading model has **no unlit concept**. A material says what colour it is, how
metallic, how rough, and what it reflects — there is no way to say "none of that is a light's
business". Worth a schema field and a documentation pass rather than a new pass kind.

**The second half of this is `feature/lighting`'s.** The glitter path on `tidewater`'s lake is
computed from the sun's *direction*, which it can only get because the scene wrote the sun
into its own `WGSL` as a constant. With a directional light the uniform would carry it. That
is stage 2's `Directional`, and it is already in the schema.

## 4. The specular is not normalised by roughness, and the Fresnel goes to one at grazing

**What stands in the way.** In the standard shading, `specular` is
`pow(dot(n, h), exponent) * mix(0.02, 1, (1 - roughness)²)`. Roughness moves the *lobe width*
and barely moves the *peak*, and `fresnel` reaches one at a grazing angle, so a thin object
seen edge on takes the whole peak. A microfacet BRDF divides by the lobe width, so a rough
surface is dimmer at its peak; this one is not.

**What it costs.** The reed bed of `tidewater`'s first attempt rendered as 340 pale sticks
whatever tint it was given — and a black tint proved the point, because the diffuse term is a
fiftieth of the specular at those angles and the tint is not in the specular at all. The only
lever a scene had was to scale the whole shaded colour by `0.06`, which is a scene reaching
around the shading model to undo something the model did.

A reed is the extreme case, but a wire, a blade of grass, a branch, a rope, a wet edge, a
railing and a horizon-lit rim are all the same failure, and none of them can be fixed by a
material.

**The change.** Normalise the specular by the lobe, and soften the grazing Fresnel toward
what a rough surface actually does. Both are in the gallery's template rather than in the
framework, so this is the cheapest item on the list and the one that improves every frame in
the gallery at once.

---

## Found while building, lower priority

### 5. MSAA is unreachable from a scene

`samples` is a field of a rasterize pass in the graphics schema
([`graphics::pass`](../crates/graphics/src/pass.rs)) and the device builds multisampled
attachments and resolves them ([`raster.rs`](../crates/device/src/wgpu_renderer/raster.rs)).
The standard compiler writes `BLEND` and `DEPTH` on a pass and **never `SAMPLES`**
([`emit.rs`](../crates/scene/src/standard/emit.rs:2158)), so no standard scene can turn it on.
Every frame in the gallery is hard-aliased; the dune ridge and the terrace contours show it.

One scene field, stamped by the emitter onto every pass that shares a target — they have to
agree, which is why it cannot be a per-pass field in practice. Cheap, and it is a real
renderer capability sitting behind a schema the emitter does not fill in.

### 6. A photograph is exposed once, and a lens is not a field of view

Three things a scene cannot say:

- **Exposure.** Every light carries its own hand-tuned `intensity`, and adding a light
  changes the brightness of the frame. `tidewater` needed the sun at `34` and the sky fill at
  `13`, numbers that mean nothing until you know the falloff.
- **The falloff itself.** `1 / (1 + 0.0035r + 0.00002r²)` is a constant of the template
  *"tuned for a scene a couple of hundred units across"*. `tidewater` is a thousand units
  deep, so its "sun" is attenuated by more than twenty and its intensity is twenty times a
  radiance. Doubling a light's distance changes the picture by a factor the author has to
  know about and cannot set.
- **The lens.** A camera carries a field of view and nothing else — no focal length against a
  sensor, no roll, no shift. A photographer chooses a lens; a wide lens close to a reed bed
  and a long lens on a ridge are different pictures from the same geometry, and the
  difference is the difference between the first version of this scene and the second.

### 7. The harness writes both cameras to the same file

A scene naming two cameras is compiled and drawn twice, and
[SPEC.align.md](../SPEC.align.md) states that this writes `-camera0` and `-camera1`. It does
not: the suffix is appended *after* the extension, so `with_extension` strips it again and
the second camera silently overwrites the first
([`render.rs`](../crates/device/examples/render.rs:162)). Found by rendering a two-camera
scene and getting one image. One line, and the documented behaviour is what happens.

---

## What `tidewater` shows that the framework gets right

Worth recording, because it is the reason the rest was worth building.

- **The emission stage was the right answer** before anybody knew a scene would need it. A
  mirror drawn there is correct and costs a third of the passes. Finding 3 is a discoverability
  failure, not a design failure.
- **The normal matrix is right and it is load-bearing.** A bank scaled unevenly is lit wrongly
  by exactly the amount it is uneven by, and the gallery's scenes use it.
- **The uniform layout check caught nothing it should not have**, across a scene that declares
  a 3 by 2 frame, two cameras, five shaders, two stages and four materials. The interface
  verification held while everything else around it was being rewritten.
- **The same pass-bucketing decision that is awkward to use is what let the lake work.** What
  casts a shadow is a decision, not a consequence: the water is left out of the `shadowing`
  bucket because a flat plane a thousand units across at two degrees off the light is the
  worst case for acne there is, and excluding it costs nothing and buys the far shore's
  shadow lying across the lake.
- **A render is a pure function of `(documents, time)`.** The mist burning off over
  twenty-four seconds is reproducible, and the frame at `t = 12` is the frame at `t = 12`.

## Touched

Framework files this branch changes, for whoever merges it. `tidewater.rs` is a pure
addition. The rest are small and are listed so the merge is not a surprise; none of them
touch the lighting path.

| file | change |
| --- | --- |
| `crates/gallery/src/shaders.rs` | the `{displace}` splice point, `lighting_with_normal`, `mesh_shaders_with_normal`, and a `helpers` slot on the emission stage |
| `crates/gallery/src/kit.rs` | `instance` takes `impl Into<String>`, because a label names one instance in the whole scene and a scene placing one mesh in several passes has to qualify them |
| `crates/gallery/src/lanterns.rs` | the one call site of `emission_shader` |
| `crates/device/examples/render.rs` | the camera-suffix fix, item 7 |

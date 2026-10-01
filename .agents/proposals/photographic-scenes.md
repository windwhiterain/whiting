# What a scene cannot say, and what it costs the picture

Status: **items 1, 2, 3 (the gallery half), 4, 6 (the lens) and 7 are implemented** on
`feature/photographic`. Items 5, 6's exposure and 6's falloff are not, and the reason in each
case is recorded beside the item. Two claims below were wrong when first written and are
corrected in place; both corrections are the kind that only measuring the thing produces.

The evidence is from building `tidewater` — a lake at
first light — against the standard pipeline, and every claim below is a thing that was tried
and measured rather than one that was reasoned about in advance.

`dunes`, `lanterns` and `terraces` were written before most of this vocabulary existed and
work around all of it. The line that separates a diagram from a photograph runs through
these gaps, and the four below are the ones that block a scene outright.

The first four are the ones a reviewer should rule on. Three more turned up while building
and are lower priority; they are at the end, and the first two of them are small.

## What implementing this did to the measurements

Recorded here rather than only in the commit, because two of these **invert a claim above**,
and a reader who took the original at face value would be misled.

- **Item 4 is a near no-op on the gallery as it stands, and the proposal said it would not
  be.** It claimed the change "improves every frame in the gallery at once". Measured across
  the gallery: a mean absolute difference of 0.0009 (`monoliths`), 0.0020 (`dunes`), 0.0006
  (`terraces`) and 0.0263 (`lanterns`) per channel out of 255, the largest single-channel
  difference anywhere being 20. That is a rounding difference, not a fix.
  The change is not wrong. **The gallery no longer contains the case that motivated it** —
  the reed bed of 340 thin edge-on sticks was removed from `tidewater` precisely because it
  was diagnosed as the bug. What is left is poles and spires a few pixels tall, too small on
  screen for a grazing term to dominate. The fix is kept because it is correct, because the
  class it fixes is large, and because it costs one multiply and one divide. But anyone who
  built a scene to *see* the fix would have concluded it did not work, and that is worth
  knowing before the next reviewer tries it.
- **Item 5 is not "one scene field".** The device's multisample path is a stub with three
  separate defects, two in `raster.rs` and one in `graphics::PipelineState::read`. See the
  item.
- **Item 3's second half is not settled.** The proposal said a directional light "is already
  in the schema" and left it there. It is in the schema and `read_lights` **refuses** it.

## Do not overlap `feature/lighting`

`.worktrees/lighting` owns the lighting path. Nothing here asks for a per light pass, a
cluster grid, a shadow page or an environment map, and the one finding that touches the
lighting path — a mirror drawn once per light — is answered by the **emission stage that
already exists**, not by anything stage 3 or 4 changes. Stage 5's environment light would
also answer part of finding 6, and it is left there deliberately.

The implemented half of this branch is deliberately the half that is **not** in that path:
the lighting template in the gallery, the camera in `standard`, and a helper module in the
gallery. What was held back, and why, is stated against each item.

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
differ by a fifth. The lighting body's own comment says *"the colour a distant surface tends
towards is the colour of the sky behind it"*, immediately above a value that is not.

**The change.** The scene document carries the atmosphere once — a sky, a ground, a density,
a height falloff — and the compiler supplies it to every stage that needs it, the way it
already supplies `time` and the projection. A scene that names no atmosphere is unchanged, and
a scene that names one cannot disagree with itself.

**What it buys beyond the seam.** Everything a scene currently hand-rolls per surface. Every
outdoor scene in the gallery has its own aerial perspective, each with its own haze colour and
its own distance law, and each is a place for the bug above. A scene should say "the air" and
get it everywhere.

**What landed, and what did not.** The framework half of this — a compiler-supplied uniform
spanning the stages — needs a field in the **lighting** uniform, which is precisely what
lighting stage 1/2 is replacing. It is deferred.

What landed is the gallery half, as [`air::Atmosphere`](../crates/gallery/src/air.rs): a
scene describes its air once and `wgsl()` hands the same string to the lighting stage and the
post stage. The rule it enforces is deliberately narrow — **what must not disagree is the
colour, not the law.** Every outdoor scene has its own aerial perspective, a height falloff
over water and a flat distance wash over a dune field being different things, and forcing one
law on all of them would be a worse mistake than the seam.

`dunes` is migrated. `tidewater`, `terraces` and `monoliths` are not, and each of them carries
the same duplicate pair that `dunes` did. Measured on `dunes`: a mean absolute difference of
1.728 per channel and 6.2% of channels differing by 8 or more — the largest change anything
in this branch made to any frame — and the cold ridge along the horizon, which was the seam,
is gone.

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

**The second half of this is `feature/lighting`'s, and the original text here was wrong about
it.** It said a directional light "is already in the schema", which reads as though the
question were closed. It is in the schema — and `read_lights`
([`emit.rs`](../crates/scene/src/standard/emit.rs):623) **refuses** anything that is not a
`Point`, with `CompileError::UnsupportedLightKind`. So `tidewater` still has to write the sun
into its own `WGSL` as a constant to get a glitter path at all, and a directional light is
lighting stage 4b's, not a settled fact.

**What landed.** The gallery half: [`kit::instances`](../crates/gallery/src/kit.rs) now takes
`Vec<(Stage, Vec<Value>)>` rather than `Vec<(&str, Vec<Value>)>`, and the rule above is stated
on it. That does not make an author choose correctly, but it puts four choices side by side in
one signature where the fourth is a different *word*, rather than four interchangeable strings
that a typo turns into a different pass.

The schema field did **not** land, and adding it without the compiler acting on it was
rejected on purpose: a field nothing reads is exactly the state the lighting handoff calls out
as one to avoid ("data that nothing reads yet"), and the part that would act on it is
`groups`/`read_instances` in the emitter, which is the lighting path.

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
framework, so this is the cheapest item on the list.

**What landed, and the measurement that qualifies it.** Two changes in
[`shaders::lighting_with_normal`](../crates/gallery/src/shaders.rs):

- The Fresnel gets a **ceiling of `max(1 - roughness, f0)`** rather than saturating at one.
  The rise towards one at a grazing angle is a property of a *smooth* interface; a rough
  surface is a field of microfacets tilted in every direction and reflects about as much edge
  on as head on. Schlick's term hands it the whole difference whatever the roughness, and that
  is what made a thin object seen edge on render as a white stick whatever tint it carried.
- The specular peak is **divided by the lobe's own width**, normalised against the smoothest
  lobe the template builds, so a mirror is exactly as bright as it always was and only the
  roughness moves. Widening a lobe is half of what a microfacet distribution does; the other
  half is that the peak falls as the lobe widens.

The qualification is at the top of this file: on the current gallery this changes almost
nothing, because the gallery no longer holds the case that motivated it. It is measured, it is
correct, and it is cheap. What it is not is the gallery-wide improvement the original text
claimed.

---

## Found while building, lower priority

### 5. MSAA is unreachable from a scene — and the device's multisample path is a stub

`samples` is a field of a rasterize pass in the graphics schema
([`graphics::pass`](../crates/graphics/src/pass.rs)) and `graphics::PipelineState` reads it
([`pipeline.rs`](../crates/graphics/src/pipeline.rs):154). `emit.rs:2158` writes `BLEND` and
`DEPTH` and never `SAMPLES`, so no standard scene can turn it on.

The original text said that was the whole of it, and it is not. Turning it on would reach a
path that does not work, and there are three defects between the schema field and a frame:

1. **`samples` is never read for a pass that declares no depth state.**
   `PipelineState::read` returns at [`pipeline.rs`](../crates/graphics/src/pipeline.rs):128
   when `DEPTH` is absent, before it looks at `samples` at all. The shadowing and depthing
   passes use the default state, which declares neither — so a sample count stamped on them
   would be silently dropped rather than refused.
2. **A pass that loads its target reads uninitialised memory.**
   [`raster.rs`](../crates/device/src/wgpu_renderer/raster.rs):173 builds a *fresh*
   multisample view per pass and never loads the single-sample target into it. Every lighting
   pass uses `load: Load` on the colour target, because there is one per light and they add.
   With more than one sample the second light would add to whatever was in a texture nobody
   wrote.
3. **The depth is never resolved.** Line 203 takes the multisample depth view and discards the
   resolve target — `let (view, _) = drawing_into(...)`. The post stage reads the depth buffer
   with `textureLoad`, so it reads a texture no pass wrote.

And fixing all three is not sufficient, which is the part that makes this a renderer change
rather than an emitter change: **the multisample colour and depth have to persist for the
whole frame and resolve once.** Resolving between the per-light lighting passes makes the
depth test fuzzy — a fragment on a coverage edge can pass for one light and fail for the
next, so different lights contribute at the same pixel and the frame fringes. A multisample
attachment that lives across the passes sharing a target, and is resolved before anything
samples it, is the actual requirement.

**Not implemented.** This is the `device` crate, and `feature/lighting` is editing
`raster.rs` and `bindings.rs` right now (5d76434, "Give a texture a layer count, and a pass
the layer it draws into"). It does not collide with the lighting *path* — it is attachment
lifetime — but it collides with a file being written, and the superior ruled that the
collision-free items only. The emitter half (one scene field, stamped on every pass that
shares a target) is small and can land with it or before it.

### 6. A photograph is exposed once, and a lens is not a field of view

The original text filed three things as one. They are three things at three different layers,
two of which can be built without touching anything `feature/lighting` owns and one of which
cannot.

- **The lens** — [`camera.rs`](../crates/scene/src/standard/camera.rs) and
  [`scene_schema.rs`](../crates/scene/src/standard/scene_schema.rs). **Implemented.**
  `focal_length` and `sensor_width` in millimetres, and `roll` in radians. The derivation is
  the thin-lens one, `2·atan(sensor / 2·focal)`, and the one subtlety is that the angle a
  focal length buys is **horizontal** while `field_of_view` is the **vertical** angle the
  projection is built from — dividing the horizontal tangent by the frame's aspect is what
  turns one into the other, and doing it the other way round quietly widens every frame that
  is not square. A document carrying both an angle and a focal length takes the angle, stated
  rather than inferred: the alternative is a scene whose picture changes when a field it never
  read is added to it.

  `roll` turns the **frame**, not the world: the eye and the target are untouched, so a rolled
  camera still looks at exactly what it looked at. That is the whole difference between it and
  `up`, and it is why `up` cannot express a roll — a camera whose up is not world-up has a
  tilted frame *or* a horizon that is not the world's, and a photograph wants the first.

  Verified on the adapter, not by reading: 35 mm and 85 mm on `tidewater` produce different
  frames, and 0.25 rad of roll turns the horizon, the treeline and the sun's reflection column
  together while the geometry stands still.

  **`shift` did not land**, and it is the honest gap here. A shifted principal point has to
  reach the post stage too — `ray_through` reconstructs a ray from `half_extent` and the
  shift is not in it, so a shifted camera draws a sky in the wrong place. That is a field on
  the post uniform, which is the same emitter change exposure wants, and that was ruled out of
  this stretch.
- **Exposure.** Every light carries its own hand-tuned `intensity`, and adding a light changes
  the brightness of the frame. `tidewater` needed the sun at `34` and the sky fill at `13`,
  numbers that mean nothing until you know the falloff. **Not implemented** — it is a scene
  field the emitter stamps into the post uniform, and the superior kept it out of this stretch.
  It is small and it collides with nothing the lighting work is rewriting; the post uniform is
  not what stage 3b replaces.
- **The falloff.** `1 / (1 + 0.0035r + 0.00002r²)` is a constant of the template
  *"tuned for a scene a couple of hundred units across"*. `tidewater` is a thousand units
  deep, so its "sun" is attenuated by more than twenty and its intensity is twenty times a
  radiance. Doubling a light's distance changes the picture by a factor the author has to
  know about and cannot set. **Not implemented, and it belongs to `feature/lighting`**: the
  constant lives in the per-light uniform, which is what stage 1/2 is replacing. Making it a
  field now means editing a struct that 4b deletes.

### 7. The harness writes both cameras to the same file

A scene naming two cameras is compiled and drawn twice, and
[SPEC.align.md](../SPEC.align.md) states that this writes `-camera0` and `-camera1`. It does
not: the suffix is appended *after* the extension, so `with_extension` strips it again and
the second camera silently overwrites the first
([`render.rs`](../crates/device/examples/render.rs:162)). Found by rendering a two-camera
scene and getting one image. One line, and the documented behaviour is what happens.
**Implemented.**

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

Framework files this branch changes, for whoever merges it. `air.rs` and `tidewater.rs` are
pure additions. The rest are small and are listed so the merge is not a surprise; none of them
touch the lighting path.

| file | change |
| --- | --- |
| `crates/gallery/src/air.rs` | **new** — `Atmosphere`, the air a scene is drawn through, and the `WGSL` that describes it to every stage |
| `crates/gallery/src/shaders.rs` | the `{displace}` splice point, `lighting_with_normal`, `mesh_shaders_with_normal`, a `helpers` slot on the emission stage, and the roughness-aware Fresnel ceiling and lobe normalisation |
| `crates/gallery/src/kit.rs` | `instance` takes `impl Into<String>`; `instances` takes a `Stage` rather than a string; `camera_placed_on_lens` is new |
| `crates/gallery/src/dunes.rs` | reads its air from `Atmosphere`; **the horizon seam is closed** |
| `crates/gallery/src/tidewater.rs` | both cameras name a lens instead of an angle |
| `crates/gallery/src/lanterns.rs` | the one call site of `emission_shader` |
| `crates/gallery/src/{monoliths,terraces,tiles}.rs` | bucket keys only |
| `crates/scene/src/standard/camera.rs` | `focal_length`, `sensor_width` and `roll`; the thin-lens derivation; the roll applied to the basis |
| `crates/scene/src/standard/scene_schema.rs` | the three new optional camera fields |
| `crates/device/examples/render.rs` | the camera-suffix fix, item 7 |

**Not touched, deliberately:** `emit.rs`, `pass_schemas.rs`, anything under
`crates/scene/src/lighting/`, and the `device`'s rasterize path. Those are `feature/lighting`'s
or, in the rasterize case, a file that branch is writing to right now.

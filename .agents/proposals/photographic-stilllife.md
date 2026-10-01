# A still life, and the six things the framework could not say while drawing one

`stilllife` is a lake's opposite: the first gallery scene whose subject is **light on
material** rather than geometry, and the first that is lit by something the pipeline does not
have. Building it found one defect that makes any scene's occlusion a matter of luck, one that
made the first four frames look like a tile floor, and four gaps that cost the picture
something a still life cannot do without.

Everything below was measured on the adapter. The scene is
[`crates/gallery/src/stilllife.rs`](../../crates/gallery/src/stilllife.rs), the harness is

```text
cargo run --release -p whiting-device --features wgpu --example render -- stilllife out/stilllife
```

and the frame is 1536 by 1024, nine window lights and two fills, 105 passes, about 0.4 s.

---

## 1. The lighting stage does not test depth, and says that it does

**This is a defect and not a gap.** An occluded surface is not discarded in the lighting
stage; it is **added** to whatever is in front of it. Which surface wins a pixel is decided by
the order the passes and the instances happen to be drawn in, and the order is an artefact of
how a scene lists its instances.

### The evidence

The emitter gives a lighting pass a depth *state* and not a depth *target*
([`emit.rs`](../../crates/scene/src/standard/emit.rs), `lighting_state`):

```rust
// "The comparison is `LessEqual` and the pass does not write. A depth prepass records the
//  depth of every opaque fragment, and a later pass drawing the *same* geometry lands on
//  exactly those depths ..."
(depth, {write: false, compare: LessEqual})
```

but the pass it builds names one target, and it is the colour one:
`rasterize(..., vec![COLOR_TARGET], ...)`. The depth buffer is instead **bound as a sampled
texture** — it is the binding the gallery's shader leaves a deliberate gap for.

The device builds a pipeline's depth-stencil state **only** from a depth-format *attachment*
among the pass's targets ([`raster.rs`](../../crates/device/src/wgpu_renderer/raster.rs)):

```rust
let depth = attachments.iter().find(|entry| entry.depth);   // from bindings.targets
depth_stencil: depth.map(|entry| wgpu::DepthStencilState { .. })
```

With an instrumented `run_rasterize` the two kinds of pass print this:

```text
depth/depthing/bowl    targets ["depth_buffer"] depth attachment true   compare Less       write true
lighting/ceramic/jug   targets ["color_target"] depth attachment false  depth stencil false  compare LessEqual  write false
```

So on the lighting passes the declared state is inert: the render pass has no depth attachment
and the pipeline has no depth-stencil state. (The depth prepass itself is real — it writes the
buffer, and the post stage reads it, which is what the defocus needs. Nothing is wasted; the
shader that *claims* to test it simply never does.)

### The measurement

One frame's pixels are not enough to see it, because a scene's instance order usually happens
to agree with its depth order. The measurement that cannot be explained away is **the same
mesh, in the same pass, twice, 0.3 m apart along the camera's own line of sight**, the far one
drawn after the near one:

| frame | the near jug's belly, red channel |
| --- | --- |
| one jug | 66 |
| two jugs | **201** |

A `LessEqual` test rejects the far one and leaves the pixel alone. It went up by a factor of
three: the far jug was added. Over the whole frame 6.5% of the pixels change, by up to 453 of
765.

**Why the gallery never noticed.** Every earlier scene is either a field of small separated
objects (`dunes`, `tiles`), a set of boxes at clearly different depths (`lanterns`,
`monoliths`), or a landscape whose instance order happens to run near-to-far. The defect is
invisible until two surfaces *overlap on screen* and the order disagrees. It is also why the
ceramics of this frame are so bright and flat: every closed pot is drawn with its own far inner
wall added under its near wall. Removing the jug's inside changes 0.3% of the frame — the mouth
only, which is the one place the inside is legitimately seen.

### What to do about it, in increasing order of usefulness

- **Refuse it.** A pass that declares a depth state and names no depth target is a document
  that cannot mean what it says. Refusing it in `graphics::pass` turns a silent wrong frame
  into a compile error, and costs nothing.
- **Attach and do not bind.** The lighting pass names `depth_buffer` among its **targets** and
  drops it from its binders. wgpu forbids one texture being sampled and attached in the same
  pass, so the two cannot both stay; the pass does not read the buffer, so the binding is doing
  nothing but holding a binding number. The consequence is that everything numbered after it
  moves down by one, so `lighting_bindings` in the gallery loses its deliberate gap and
  `light_record`/`shadow_pool` move from 4/5 to 3/4. That is the whole change, and it is the
  one that makes the renderer mean what its own comments say.
- **A depth-only pass per group** would also work and costs a pass per group, which is the
  wrong direction in an effort that has been removing passes.

**Decided: recorded, not built.** The superior's ruling on this stretch is that the framework
is not edited here, and `.worktrees/lighting` owns the emitter.

---

## 2. A scene with a wide depth range loses its lights, and the frame goes blocky

The first four frames of this scene were covered in **rectangular patches about a cluster
wide**, each a flat tone, and the patch size moved with `clustering.width_extents` — which is
how the cluster grid was identified as the cause rather than the shadows.

### What is happening

A cluster of the **last** slice is the slab at its slice's far boundary, which is the *far
plane* of the projection, where a perspective view is widest. Its columns therefore partition a
rectangle tens of metres across that is well off to one side of the subject, and a light
standing over the table is further from that rectangle than its own `radius`. The cluster is
assigned no lights at all and the fragment is lit by the ambient only.

A projection's depth is a fraction of `1/d`, so a far plane far enough away collapses the whole
scene into the last slice and every cluster becomes such a slab. This scene's first camera was
`near 0.06 m, far 40 m` — a ratio of 667 — and the dumped counts were:

```text
counts by value: [(0, 73), (1, 9), (2, 4), (6, 1), ... (37, 3332)]
cluster 3312 (slice 23): count 0
```

73 clusters lit by nothing, and every one of them a rectangle in the frame.

### The workaround, which is not a fix

A scene that names its **depth planes close to what it draws** has shallow slices and tight
boxes. This scene now uses `near 0.25 m, far 8 m`, the frame's patches are gone, and the same
dump shows every cluster the frame reaches holding all eleven lights.

Two things are wrong with leaving it there:

- The scene author has to know it, and the failure mode is a *picture* rather than an error.
  Nothing in the dump above is visible from the scene's own source.
- Tight planes are good practice for depth precision anyway, and being *forced* into them by
  the cluster grid is the wrong reason to arrive at them.

### What to do about it

The box a light is tested against should be the slab at the slice's **far** boundary only in
the direction the slab grows; the *columns* of a deep slice should be taken at a boundary the
fragment can actually be near, or the box should be clipped to the slab's own near half for
the across/up test. A cluster that cannot be reached by a light standing in front of the view
is a cluster whose box does not describe the view.

---

## 3. There is no environment, and a still life is mostly environment

A still life is what things show of one another: the window in a glaze, the table in the
underside of a bowl, the wall in a jug, the room in a spoon. The pipeline has no image based
lighting, no probe and no environment light — `read_lights` refuses `Environment` — so this
scene **writes the room as a function**, `room_environment`, and every surface body samples it.
That is the honest workaround and it produced the best thing in the frame: the brass spoon is a
dark object with the window burning along it, which is what a metal in a dark room is.

Two consequences are worth recording because they are not obvious from inside the scene:

- **A reflection's radiance and the lights' irradiance share no scale.** The window's radiance
  is `26` in the environment function and the window's lights put out `11` of irradiance
  between them; nothing derives either from the other, and the ratio between them is a number
  the author picked. A glaze shows four percent of the first and all of the second, so the
  picture's highlights are as bright as the author's guess. With one environment per scene this
  is a tuning problem; with the `Environment` light of the lighting effort's stage 5 it
  would be a physical one, because the same number would light the scene and appear in it.
- **The room's light is not the stage's ambient.** See finding 6.

`Environment` is lighting stage 5 and is untouched. This scene is evidence for what it is for
rather than a request to move it up.

---

## 4. The falloff is a constant in absolute units, so a tabletop has to be 100 units across

The attenuation is `1 / (1 + 0.0035 r + 0.00002 r²)`, and its own comment places it in *"a
scene a couple of hundred units across"*. At a tabletop's true size — a metre, one unit — that
is flat to within a percent: there is **no gradient across the table at all**, which is the
single thing a window does to a table.

So this scene is written in metres and multiplied by a hundred. At 100 units to the metre the
near corner of the table is lit three times as hard as the far one, the shadows behave, and the
picture has the falloff it needs. Two notes:

- This is the *deferred* half of the previous effort's finding 6 ("the falloff"), which was
  held back because the constant lives in the per-light uniform that lighting stage 1/2
  replaces. It is now measured from the other end: it is not only that a scene cannot set the
  falloff, it is that a scene **cannot be at human scale without losing the light**.
- A scene's scale is a lighting decision, and nothing says so. `UNIT` at the top of the scene
  file carries the reason, and the reason is four paragraphs long.

---

## 5. A window is thirty-five point lights, and an area light is what a still life is lit by

The pipeline has `Point`, `Spot`, `Directional` and `Environment` in its schema and emits only
`Point`. To get the light a still life actually has, this scene stands **nine point lights in
the window's opening**, scattered inside their own cells rather than set on a lattice. What
that buys and what it costs:

| bought | cost |
| --- | --- |
| a penumbra that is a *stack* of shadows rather than a blurred edge, which is the closest this pipeline gets to a soft shadow | nine steps in every penumbra, and the only lever on their number is the number of lights |
| a highlight on a glaze that is the window's own shape rather than a dot | nine delta lobes per fragment, each a ninth as bright as one would be: the glaze's specular is *dimmer and flatter* than a lamp's would be |
| PCF softness on top: the shadow kernel is three by three texels of a 1024 page over a 0.62 m box, so about 4.5 mm | 88 of the frame's 105 passes are shadowing passes: nine lights plus two fills, times the scene's eight groups |

The number of lights is therefore a *composition* decision — how many steps a soft edge is
allowed — and it belongs in the same class as the cluster grid. A `SoftLight` with a rectangle,
a couple of samples and one pass is what this picture would use if the pipeline had one.

**The 3×3 kernel and the 1.6-unit normal offset are literals in the gallery's shader**
([`shaders.rs`](../../crates/gallery/src/shaders.rs), `shadowed`). The offset is a *world*
constant and this scene is a hundred times smaller than the scenes it was tuned for, so it had
to be checked rather than assumed; it is 1.6 cm here, which is right. A scene that is a
thousandth of the gallery's size would have to change a number it cannot reach.

---

## 6. The ambient is a fixed hemisphere of sky, and a body can only add to it

The lighting stage adds `mix(vec3(0.014, 0.017, 0.026), vec3(0.055, 0.072, 0.110), normal.y)`
before a surface body runs. Outdoors that is a decent stand-in for the sky. **Indoors it is the
wrong colour, the wrong size and the wrong shape** — a room's ambient is warm and comes off the
table, and a blue-grey floor of light under every shadow side is the difference between a
still life and a diagram.

A body is spliced *after* that term, so it cannot choose it. What this scene does is subtract
it and put the room's own in:

```wgsl
color = color - ambient * diffuse_color + room_bounce(in.world_position, normal) * diffuse_color;
```

That is a scene reaching around the model to undo something the model did — the same phrase the
previous effort used for the specular, and the same shape of problem. Measured across the
frame: 15% of the pixels move by more than two of 255, the largest by 60. It is a small change
and it is the right one, but the fact that every body in the scene carries that line is the
finding. **A material field saying "the ambient is mine" is what it wants**, and it belongs
with the environment of finding 3 rather than beside it.

---

## 7. The camera — exposure, defocus and the transfer — is all in one post pass, because two would clip

The post stage is the only place a scene has for exposure, a tone curve, a vignette, grain and
defocus, and it produced the picture's two most photographic qualities: a circle of confusion
taken from the depth buffer, and one exposure through a film curve.

It is written as **one** pass, and the reason is not taste. A post process's intermediate
target is `rgba8unorm` ([`emit.rs`](../../crates/scene/src/standard/emit.rs),
`post_target_entry`), so a chain of two passes stores the first pass's output **clamped to one
and quantised to eight bits before any tone curve exists**. Anything above one — every
highlight a photograph has — is gone at the join. Doing the whole camera in one pass avoids it:
the gather sums **radiance** (unclamped, in registers), a depth weight keeps a sharp edge off a
defocused background, and the film curve is applied once, to the sum, which is the order the
light takes.

Two things follow:

- **A post chain is not a general tool until its intermediate is not eight bits.** The scene
  wanted three passes: defocus, bloom, grade.
- **The vignette, the grain and the S-curve are applied after the sum and not per tap**, and
  that is a decision the single pass forces: applied per tap they would smear across the blur.

`focal_length`/`sensor_width`/`roll` from the previous effort are used as they were designed to
be, and one line of the derivation is worth repeating for the next author: an 85 mm on a 36 mm
frame looked slightly down is the whole of the composition, and a *fitted* camera pulls back
until the table is in frame, which is the one composition a still life cannot have.

---

## What landed, and what was held back

| item | state |
| --- | --- |
| the lighting stage does not test depth | **recorded, not built** — proposal only, by ruling |
| the cluster grid loses lights on a wide depth range | worked around in the scene (tight planes); the cause is recorded |
| no environment | the scene writes its own room; `Environment` is stage 5's |
| the falloff is absolute | worked around (the scene is 100 units across); recorded |
| no area light | worked around (nine scattered point lights); recorded |
| the sky ambient indoors | corrected in every surface body, at the cost of a line per body |
| no tone mapping or exposure field | the post stage, in one pass, for the 8-bit reason above |

Landed in code, all additive:

| file | change |
| --- | --- |
| `crates/gallery/src/stilllife.rs` | **new** — the scene |
| `crates/gallery/src/geometry.rs` | `lathe`, a surface of revolution about the y axis, with normals from the profile's own tangent. Every object in a still life is a profile turned, and the gallery had no generator for one |
| `crates/gallery/src/kit.rs` | `light_with_radius`: a light that says how far it reaches, because the default is one box radius of *what the scene draws* and a window two metres outside the subject reaches nothing under it |
| `crates/gallery/examples/clusters.rs` | **new** — dumps the cluster counts, the light records and the pass order of a gallery scene. It is how finding 2 was found, and the framework has no other way to see what the assignment did |
| `crates/device/examples/render.rs` | prints `CompileOutput.notes`. The note channel has had a filler and no printer since the lighting effort, and this is the exact host the lighting handoff named |
| `crates/scene/tests/clustered_lights.rs` | the new scene added to the pass count table, which the test's own comment requires |

## Still owed

- **The depth test.** Everything in finding 1 is a change to `emit.rs` and to the gallery's
  binding numbers, and none of it has been built.
- **`shift` on the camera** (the previous effort's gap) is still a field the post uniform would
  have to carry, and a still life is exactly the kind of picture that uses one.
- **A normal map, or any texture that is not albedo.** Every surface detail in this scene is
  arithmetic in a fragment body, which is why there are six shaders for six materials and why
  none of them can be combined. A material that carried an albedo and a normal would collapse
  most of this file.
- **Glass.** There is no refraction, no transmission and no absorption, so the one object a
  still life is most often *about* is not in the picture.

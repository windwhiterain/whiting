# A still life, and the things the framework could not say while drawing one

`stilllife` is a lake's opposite: the first gallery scene whose subject is **light on
material** rather than geometry, and the first that is lit by something the pipeline did not
have when it was begun. Building it found a second sighting of a defect that makes any scene's
occlusion a matter of luck, one gap that made the first four frames look like a tile floor, and
four more that cost the picture something a still life cannot do without.

**This was written against `b998f89` and corrected after merging `main` at `cf125c4`.** The
lighting effort's checkpoints 4, 5 and 6 landed in between and brought `Spot`, `Directional`
and `Environment` lights, an octahedral environment map, and the printer for
`CompileOutput.notes`. Two of the findings below were written before that and are **corrected in
place**, marked where they stand: the framework gained, during this stretch, exactly what items
3, 5 and 6 said it lacked. What the scene predates, and what is still true, is stated in each.

Everything below was measured on the adapter. The scene is
[`crates/gallery/src/stilllife.rs`](../../crates/gallery/src/stilllife.rs), the harness is

```text
cargo run --release -p whiting-device --features wgpu --example render -- stilllife out/stilllife
```

and the frame is 1536 by 1024, nine window lights and two fills, 105 passes, about 0.4 s.

---

## 1. The lighting stage does not test depth: a second sighting, and a measurement

> **Already recorded, and better, by the lighting effort**:
> [lighting-pass-has-no-depth-test.md](lighting-pass-has-no-depth-test.md), found from the
> `spotlight` scene. That proposal has the mechanism, the three places the design says the
> opposite, the two coupled changes a fix needs, and what turning occlusion on would do to every
> image the renderer produces. **Nothing here should be read as a second claim**, and the
> mechanism is not repeated. What this scene adds is an independent confirmation from a different
> pipeline stage and a measurement that is the cleanest form of the same fact.

### Why it is worth a second entry

The two sightings were made independently, on different scenes, in the same week, and neither
session knew about the other's. That is worth recording: this is not one scene's oddity, it is a
trap the next author will fall into as well. The lighting effort found it in a scene whose
*purpose* is a shadow.

### The measurement

A frame's pixels are not enough to see the defect, because a scene's instance order usually
happens to agree with its depth order — which is why this scene rendered eighteen times before
anything looked wrong. The measurement that cannot be explained away is **the same mesh, in the
same pass, twice, 0.3 m apart along the camera's own line of sight**, the far one drawn after
the near one:

| frame | the near jug's belly, red channel |
| --- | --- |
| one jug | 66 |
| two jugs | **201** |

A `LessEqual` test rejects the far one and leaves the pixel alone. It rose by a factor of three:
the far jug was added. Across the frame 6.5% of the pixels change, by up to 453 of 765.

The same fact, stated in a way that matters for a still life: **a closed pot is drawn with its
own far inner wall added under its near wall**, which is why the ceramics of this frame are
bright and flat and why nothing has any modelling to it. Removing the jug's inside changes 0.3%
of the frame — its mouth, the one place the inside is legitimately seen.

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

## 3. There was no environment, and a still life is mostly environment

> **Corrected after the merge.** This was written when the pipeline had no environment at all,
> and it does now: checkpoint 4/5 carries an `Environment` light — one per scene, an image the
> scene generates, sampled as a two dimensional octahedral map — and the lighting stage's
> ambient term is `environment_radiance(normal, roughness)`, which reads that map when a scene
> names one and falls back to the two-colour hemisphere when it does not. **This scene predates
> it and still hand-writes its room.** What follows is therefore not a request but a record of
> what the workaround cost, and the comparison it makes available.

A still life is what things show of one another: the window in a glaze, the table in the
underside of a bowl, the wall in a jug, the room in a spoon. With no environment to name, this
scene **writes the room as a function**, `room_environment`, and every surface body samples it.
That is the honest workaround and it produced the best thing in the frame: the brass spoon is a
dark object with the window burning along it, which is what a metal in a dark room is.

What the workaround costs, and what an `Environment` light would have bought:

- **A reflection's radiance and the lights' irradiance share no scale.** The window's radiance
  is written as `11` in the environment function and the window's lights put out `11` of
  irradiance between them, and nothing derives either from the other. A glaze shows four percent
  of the first and all of the second, so the frame's highlights are as bright as the author's
  guess. An `Environment` light is a *light*: the same number that lights the scene appears in
  it, which is the whole reason it is a kind and not a field.
- **The room's light and the stage's ambient were two things to keep in step.** See finding 6.
- **One environment per scene, from one image, is exactly the shape this scene wants** — a dark
  room with one bright rectangle in it. The scene's function is that image; the image is the
  function's four constants plus a fold.

**Still owed here:** migrating `stilllife` onto an `Environment` light, which is mechanical
(`room_environment` becomes a generator of octahedral pixels using the same folding the shader
uses, and the bodies call `environment_radiance` instead) and needs a re-tune of the exposure,
because the ambient stops being the scene's to choose.

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

## 5. A window is nine scattered point lights, and a window is what a still life is lit by

> **Corrected after the merge.** The kinds are no longer the gap. When this stretch began the
> emitter refused everything but `Point`; it now emits `Spot`, `Directional` and `Environment`.
> What none of the four kinds is, and what a still life is actually lit by, is a **rectangle** —
> an area light with a shape and a size. So the nine scattered point lights stay, and so do
> their consequences.

The pipeline has `Point`, `Spot`, `Directional` and `Environment`, and not one of them has an
extent. To get the light a still life actually has, this scene stands **nine point lights in the
window's opening**, scattered inside their own cells rather than set on a lattice. What that
buys and what it costs:

| bought | cost |
| --- | --- |
| a penumbra that is a *stack* of shadows rather than a blurred edge, which is the closest this pipeline gets to a soft shadow | nine steps in every penumbra, and the only lever on their number is the number of lights |
| a highlight on a glaze that is the window's own shape rather than a dot | nine delta lobes per fragment, each a ninth as bright as one would be: the glaze's specular is *dimmer and flatter* than a lamp's would be |
| PCF softness on top: the shadow kernel is three by three texels of a 1024 page over a 0.62 m box, so about 4.5 mm | 88 of the frame's 105 passes are shadowing passes: nine lights plus two fills, times the scene's eight groups |

The number of lights is therefore a *composition* decision — how many steps a soft edge is
allowed — and it belongs in the same class as the cluster grid. A `SoftLight` with a rectangle,
a couple of samples and one pass is what this picture would use if the pipeline had one; a
`Spot` is closer than a `Point` in that it has an axis, and it is the wrong axis — a cone cannot
be a window.

**The 3×3 kernel and the 1.6-unit normal offset are literals in the gallery's shader**
([`shaders.rs`](../../crates/gallery/src/shaders.rs), `shadowed`). The offset is a *world*
constant and this scene is a hundred times smaller than the scenes it was tuned for, so it had
to be checked rather than assumed; it is 1.6 cm here, which is right. A scene that is a
thousandth of the gallery's size would have to change a number it cannot reach.

---

## 6. With no environment named, the ambient is a fixed hemisphere of sky, and a body can only add to it

> **Corrected after the merge.** The design is now the right one: the hemisphere is a **stand-in
> that steps aside**, and a scene that names an `Environment` light gets its own environment in
> that term instead. What is left of the finding is what happens to a scene that names none —
> which is every gallery scene except `spotlight` — and the one line of apology such a body owes.

The lighting stage adds `environment_radiance(normal, roughness)` before a surface body runs. Outdoors that is a decent stand-in for the sky. **Indoors it is the
wrong colour, the wrong size and the wrong shape** — a room's ambient is warm and comes off the
table, and a blue-grey floor of light under every shadow side is the difference between a
still life and a diagram.

A body is spliced *after* that term, so it cannot choose it. What this scene does is subtract
it and put the room's own in:

```wgsl
color = color - ambient * diffuse_color + room_bounce(in.world_position, normal) * diffuse_color;
```

Measured across the frame: 15% of the pixels move by more than two of 255, the largest by 60. It
is a small change and it is the right one, but the fact that six surface bodies each carry that
line is the finding — a scene reaching around the model to undo something the model did, the
same phrase the previous effort used for the specular. **Naming an `Environment` light removes
the need for it entirely**, which is why that migration is the first thing this scene owes.

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
| the lighting stage does not test depth | **recorded by the lighting effort**; this scene is a second sighting with a measurement, not built |
| the cluster grid loses lights on a wide depth range | worked around in the scene (tight planes); the cause is recorded |
| no environment | **superseded during the stretch** by the `Environment` light; the scene still hand-writes its room, and the migration is owed |
| the falloff is absolute | worked around (the scene is 100 units across); recorded |
| no area light | worked around (nine scattered point lights); `Spot`, `Directional` and `Environment` landed during the stretch, and a *rectangle* is still not a kind |
| the sky ambient indoors | corrected in every surface body; an `Environment` light removes the need for the correction |
| no tone mapping or exposure field | the post stage, in one pass, for the 8-bit reason above |

Landed in code, all additive:

| file | change |
| --- | --- |
| `crates/gallery/src/stilllife.rs` | **new** — the scene |
| `crates/gallery/src/geometry.rs` | `lathe`, a surface of revolution about the y axis, with normals from the profile's own tangent. Every object in a still life is a profile turned, and the gallery had no generator for one |
| `crates/gallery/src/kit.rs` | `light_with_radius`: a light that says how far it reaches, because the default is one box radius of *what the scene draws* and a window two metres outside the subject reaches nothing under it |
| `crates/gallery/examples/clusters.rs` | **new** — dumps the cluster counts, the light records and the pass order of a gallery scene. It is how finding 2 was found, and the framework has no other way to see what the assignment did |

Two things this stretch needed were landed by the lighting effort's checkpoints 5 and 6 while
it was running, and are no longer this branch's to do: the printer for `CompileOutput.notes`,
which the lighting handoff had asked for and which
[`render.rs`](../../crates/device/examples/render.rs) now has as `print_notes`, and the pass
count table's entry for a new scene, which the test's own comment requires.

## Still owed

- **Migrating this scene onto an `Environment` light.** Findings 3 and 6 both end there.
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

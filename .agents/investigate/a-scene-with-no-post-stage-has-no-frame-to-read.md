# What is a standard scene that declares no post stage supposed to return?

Measured 2026-10-16, on machine B, on `main` at `95b2ba4`.

## The question

`crates/device-rasterize/tests/gallery_render.rs::every_gallery_scene_renders` fails on one scene:

```text
`studio` does not render: the frame does not render: the frame is drawn in `rgba16float`, and a
frame is read as an 8 bit rgba image
```

`studio` is `scene::standard::example_standard_scene()` and it declares **no** `post_processes`.
The question is not why it fails — it is **what the tree intends a scene with no post stage to
be**, because three records answer it and they now contradict one another, and nothing in SPEC
answers it at all.

## What is measured

### the render succeeds; only the read back refuses

`cargo test -p whiting-device-rasterize --test gallery_render`, 2026-10-16: **1 passed, 1 failed**
in 30.66 s, the panic above. `a_scene_that_samples_an_image_renders_more_than_one_colour` passed.

The harness on the same build, 2026-10-16, `render studio out\studio`:

```text
studio: the scene's own time drawn, 3 passes, 0 operations of delta, cloned 0 ms,
        validated 0 ms, compiled 26 ms, rendered 173 ms
error: the frame does not encode: the frame is drawn in `rgba16float`, and a frame is read as an
       8 bit rgba image
```

exit status 1, **no png written**. So nothing about the gpu work fails: three passes compile,
validate and draw in 173 ms, and the refusal is at `present.as_frame()`.

### the four scenes the failing loop never reached do render

`every_gallery_scene_renders` aborts at the **first** failure and `studio` is the tenth of
fourteen, so nine scenes were ever checked. Measured 2026-10-16 through the harness, same
build:

| scene | passes | render | exit | png |
|---|---|---|---|---|
| `sunlight` | 16 | 727 ms | 0 | 41 467 bytes |
| `terraces` | 19 | 1699 ms | 0 | 991 612 bytes |
| `tidewater` camera 0 | 15 | 1292 ms | 0 | 2 795 798 bytes |
| `tidewater` camera 1 | 15 | 980 ms | 0 | 3 007 762 bytes |
| `tiles` | 4 | 676 ms | 0 | 82 148 bytes |

All four render, all four write a picture, and each is far from a one-colour frame by file size
alone. **The previous agent's statement that all four are fine is confirmed.**

### `studio` is the only scene in the gallery without a post stage

`crates/gallery/src`, 2026-10-16: thirteen of the fourteen names in `Gallery::names()` carry a
`post_processes` field — `dunes`, `foldyard`, `hermite`, `hermite_massif`, `landscape`,
`lanterns`, `monoliths`, `spotlight`, `stilllife`, `sunlight`, `terraces`, `tidewater`, `tiles`.
Only `studio` does not, because it is not a gallery scene at all: `Gallery::build("studio")`
returns `scene::standard::example_standard_scene()` (`crates/gallery/src/lib.rs:151`).

This matters for reading `0257706`'s evidence: it measured four scenes — `landscape`,
`spotlight`, `sunlight`, `tiles` — and all four have a post stage. **Its claim "all five gallery
scenes render" was never a claim about a scene with no post stage.**

## The three records, and the contradiction

### 1 — a scene with no post stage is a legal document

`post_processes` is **optional** in the scene schema (`crates/scene/src/standard/scene_schema.rs:302-311`),
and the compiler carries three separate branches for the empty case rather than refusing one:
the exposure note (`emit.rs:1108-1118`, *"said rather than refused"*), the frame target
(`emit.rs:1183-1187`), and the pass loop itself. Nothing in the tree treats the document as
malformed.

### 2 — its frame is the shaded image, by decision

`emit.rs:1183-1187` names the point the frame is read from:

```rust
let frame_target = if post.is_empty() {
    COLOR_TARGET_ENTRY
} else {
    PRESENT_TARGET_ENTRY
};
```

and `debug_outputs` says why in the same file, at `:3593-3597`:

> The frame's own type is `vec4<f32>` because that is what every shader in this pipeline returns
> at location zero: the post stage's colour, **and a lighting pass's shaded image when there is
> no post stage at all**.

This is a decision, not an accident. It landed with `2e7644a` (2026-10-01) and is restated in
`.agents/proposals/typed-outputs.md` under *What is deliberately not proposed* — *"A change to
how the frame is chosen. `present` is still the one called `present`"* — and in
`.agents/proposals/agent-debug-scenes.md:235-241`.

### 3 — the shaded image is a radiance, so it is `rgba16float`

`0257706` (2026-10-11) split the two constants (`pass_schemas.rs:320` and `:329`):

```rust
pub const COLOR_FORMAT: &str = "rgba16float";
pub const PRESENT_FORMAT: &str = "rgba8unorm";
```

on measured evidence: at `rgba8unorm` 18.0% of `landscape`'s red pixels arrived at the tone
curve already flat, and `exposure` cannot reach them because a multiplier applied to one is
still one. Its stated rule is *"the shaded image is the one image in the pipeline that is not a
display value"* and *"the frame keeps `rgba8unorm` under its own name and the read-back is
untouched."*

### where they break

`0257706` changed the constant that record 2 points the frame at, and **did not revisit record
2**. When both were written, `COLOR_FORMAT == PRESENT_FORMAT` and the two agreed. Now, for a
scene with no post stage:

```text
frame target  = color_target   (emit.rs:1183)
color_target  = rgba16float    (pass_schemas.rs:320)
render()      requires as_frame(), which accepts only rgba8unorm / rgba8unorm-srgb
                (crates/device/src/render.rs:65-69, refused at :180, message at :552)
```

So the tree simultaneously holds *"the frame of a scene with no post stage is the shaded image"*
and *"the frame is a display image read as eight bits a channel"*. For this scene those cannot
both be true.

### the contradiction was noticed once, and filed as an observation

`.agents/investigate/cycles-mesh-normals.md:515-518`, measured on `aeb4ced`:

> two scenes have no raster frame to compare against, and did not have one before this either,
> measured on `aeb4ced` itself: **`studio`** renders its frame in `rgba16float` and the harness
> reads a frame as an 8 bit rgba image […] neither is a winding matter and neither is fixed here.

That is the whole record of the conflict. It is called a defect, it was left unfixed, and it was
never carried into SPEC. Per `AGENTS.md` an investigate note is "never a basis for
implementation on its own", so nothing since has been obliged to resolve it.

### SPEC is silent

`.agents/SPEC/` searched 2026-10-16 for `rgba16float`, `display image`, `present_target`,
`present`, `frame`, `post stage`, `no post stage`, `8 bit`: **no SPEC file decides whether a
scene must have a post stage, nor what the frame is when it does not.** The only SPEC text about
an eight-bit ceiling (`SPEC/render/cycles_backend.md:102`, `:166`) is about the **Cycles**
output file, not the rasterised frame. Both live decisions are recorded only in code comments
and in `.agents/proposals/`, which is where a finding is *not* yet a decision.

## The three ways out, and what each costs

Nothing here was implemented. These are the options with what was checked against the tree,
not a preference.

### A — give the standard example a post stage

**Measured 2026-10-16, then reverted.** A `Post` shader built on `post_pass_schema()`, a
`screen_materials` entry and a one-entry `post_processes` were added to
`example_standard_scene()` in an uncommitted edit, built, and run:

| | before | after |
|---|---|---|
| `render.exe studio out\studio` | 3 passes, rendered 173 ms, **exit 1**, no png | **4 passes, rendered 415 ms, exit 0, a 25 501 byte png** |
| `cargo test -p whiting-device-rasterize --test gallery_render` | 1 passed, 1 failed | **2 passed, 0 failed** |

So it works, and it is not a guess. **What it costs is three red tests in one file, not the two
this note first predicted**, measured on the same build:

| test | what it asserts | what option A does to it |
|---|---|---|
| `an_all_point_scene_still_emits_the_passes_it_emitted_before` (`clustered_lights.rs:56`) | the example scene's exact pass list, `["shadow/key/shadowing/triangle", "depth/depthing/triangle", "lighting/lighting/triangle"]` | gains `"post/post"`. **This one is not a number that moves — the test's whole subject is the example scene's pass list**, and its name claims it emits what it emitted before |
| `every_gallery_scene_still_emits_the_pass_count_it_emitted_before` (`:394`) | `studio` = `{shadowing 1, depthing 1, lighting 1, other 0}` | `other` 0 to 1 |
| `the_totals_shrank_where_the_scene_had_more_lights_than_groups` (`:425`) | `studio` total `<= 3` | 4 |

`cargo test -p whiting-scene --test clustered_lights` on that build: **10 passed, 3 failed**.

Option A also would have to:

- **Contradict the example's own doc.** `crates/scene/src/standard/example.rs:1-6` says the
  scene exists so that *"the pipeline it compiles to is the whole three stage pipeline"*, and
  the harness now measures four.
- **Write a fourth WGSL source in `scene`.** `example.rs:192-199` builds mesh shaders and
  `pass_schemas::post_pass_schema()` exists, but a `ScreenShader` is a different variant: it
  carries a `stage` string and takes no `point_schema` (`scene_schema.rs:1322-1337`), and the
  emitter refuses one that declares an interface nothing supplies. So this is a new builder,
  not one line added to the existing one. `example.rs:543-544` already records that
  duplication into `scene` is the standing practice here.
- **Stop the gallery exercising the no-post case at all**, which the three `post.is_empty()`
  branches in `emit.rs` were the only cover for.
- Six test files use `example_standard_scene()` as a fixture (`directional_lights`,
  `spot_lights`, `environment_light`, `image_channel`, `clustered_lights`, `camera_notes`) plus
  the `standard` module doctest. Measured: only the three above move; the other three and the
  doctest were not run against the experiment.

### B — make the read back handle a scene with no display image

Three distinct shapes, and two of them re-create the defect `0257706` exists to remove:

- **B1, convert in `as_frame()` / `render()`.** The device would decide what an ungraded
  radiance becomes as a picture — clamp, Reinhard, exposure then clamp. Whichever rule is chosen
  either clips at one, which is the plateau `0257706` measured, or puts a tone curve in the
  device when the scene layer owns the grade (`0257706`: *"the tone curve runs on what it
  reads"*). It also contradicts `as_frame`'s own documented invariant (`render.rs:58-69`). This is
  a design decision, not a fix.
- **B2, emit a copy pass `color_target` → `present_target` when there is no post stage.** The
  frame then always has a format of its own, and the split reads cleanly. But the copy is a
  linear-to-unorm conversion: the same clip at one, in a new place.
- **B3, refuse a scene with no post stage as out of contract.** The strongest reading of "the
  frame is a display image", and a small diff. Against it: the field is optional, three
  branches handle the empty case on purpose, `debug_outputs` documents the answer, and the tree
  refuses a document only when it is *malformed* — `emit.rs:1108-1118` is the statement of that
  rule, *"said rather than refused"*. Note that B3 would make the no-post case an error rather
  than a path, so it is the one option that makes the current example's document wrong rather
  than its picture.

### C — declare the example out of contract and change the test

The test says what it is for: *"a binding numbering that shifts by one still renders, and still
renders the wrong thing, so what this holds is that none of them stops rendering at all"*
(`gallery_render.rs:60-62`). `studio` **does** render — measured above, three passes, 173 ms.
`Renderer::render_outputs` is the call that returns every declared image including a float one,
and it would hold the stated contract for all fourteen.

Against it: it removes the only automated notice that studio has no readable frame, and it does
not close the user-visible gap — `examples/render.rs` still exits 1 for `studio`, which is how
`studio` has had "no raster frame to compare against" in every cross-backend survey since
(`cycles-mesh-normals.md:515`).

## Which one I would take, and the case against it

**A.** It is the only option that makes the tree state more than it already does:

- It is the only one where `COLOR_FORMAT` and `PRESENT_FORMAT` mean what their own doc comments
  say. Every sentence `0257706` wrote — *"the shaded image is the one image in the pipeline that
  is not a display value"*, *"the frame is what comes back to the host"*, *"widening the present
  target with the shaded image … the read-back is untouched"* — describes a pipeline with a
  post stage. A makes that universal; B1 and B2 make it a special case.
- B1 and B2 both re-create the plateau `0257706` measured, in a different place. B3 refuses a
  document the schema accepts, against the tree's own stated rule.
- `emit.rs:1183`'s `post.is_empty()` branch does not have to disappear under A. It stays correct
  and is still reachable from a hand-written document; what changes is that the gallery stops
  being the one scene with no picture.

**The case against it, which is why this is still the superior's call**: A rewrites a test whose
name claims the example emits what it emitted before, and rewrites a second whose numbers are
pinned on purpose ("*a scene added to the gallery now has to be added to the table and cannot be
forgotten*"). Editing tests to accommodate a scene change is the thing `AGENTS.md` reserves, and
it removes the gallery's only exercise of the no-post path — which is, per
`cycles-mesh-normals.md:515`, exactly what makes `studio` the one scene the cross-backend survey
cannot compare. If the survey is more valuable than a readable `studio` frame, C is the answer
and A is the wrong one.

## What would settle it

One question, and it is not a technical one:

> **Is a standard scene with no post stage required to produce a picture?**

The answer picks the option and nothing else does:

- **Yes** → the compiler should say so where `post.is_empty()` is already handled
  (`emit.rs:1183`), which is B3. A scene author gets a compile note naming the field to add,
  the same way the exposure field already gets one.
- **No, but a picture is still wanted** → A for `studio` specifically, and the two contract
  numbers in `clustered_lights.rs` move with it.
- **No, and a radiance is the answer** → B1, and the device owns a tone curve, which is a larger
  statement than this failure.

## Not measured

- What the picture would look like under B1 with any given curve. Not computed, because which
  curve is the decision.
- Whether `nurbs`, the other scene named in `cycles-mesh-normals.md:517`, is still failing to
  compile. That row was replaced by `hermite` on 2026-10-15 and was not checked here.
- Whether a hand-written document with no post stage reaches `render()` anywhere else. The
  device's own doctest (`crates/device/src/lib.rs:82-89`) goes through `present()` on a
  `RenderedFrame` rather than `as_frame()`, so no existing test asserts an eight-bit frame for a
  no-post document. Searched 2026-10-16.

## Sources

- `git show 0257706` — the format split, its measurements, and its five-scene evidence, all of
  which are post-stage scenes.
- `git log -1 --format=%B 2e7644a` — the frame-target decision and why `present` did not move.
- `crates/scene/src/standard/emit.rs:1108-1118`, `:1146-1160`, `:1183-1187`, `:3244-3348`,
  `:3593-3597`.
- `crates/scene/src/standard/pass_schemas.rs:309-342` — `COLOR_FORMAT`, `PRESENT_FORMAT`,
  `IMAGE_FORMAT` and their three doc comments; `:689-717` — the post interface.
- `crates/scene/src/standard/scene_schema.rs:302-311` — `post_processes` is optional;
  `:1322-1337` — what a `ScreenShader` variant carries, and why it is not a mesh shader.
- `crates/device/src/render.rs:58-69`, `:174-183`, `:552-553`.
- `crates/scene/tests/clustered_lights.rs:56`, `:139-175`, `:339-347`, `:394`, `:412-429`.
- `crates/scene/src/standard/example.rs:1-6`, `:110-117`, `:192-199`, `:543-544`.
- `cargo test -p whiting-device-rasterize --test gallery_render` and `render.exe` for five
  scenes, both 2026-10-16 on `95b2ba4`, at
  `C:\resource\whiting\.worktrees\studio-post`.
- The option A measurement above, 2026-10-16, applied and then reverted with
  `git checkout -- crates/scene/src/standard/example.rs`. Nothing from it is in the tree.

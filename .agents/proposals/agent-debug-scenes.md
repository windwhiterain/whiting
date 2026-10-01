# Agent debug outputs: extra images a scene can ask for

Status: **landed.** The graphics document declares its outputs, the standard compiler draws a
position image when its caller asks for one, the device reads back every declared output in the
format it was drawn in, and the render harness writes each beside the picture with a sidecar of
the numbers. The four parts below are what was built; what the building **found** is under
[What the first render found](#what-the-first-render-found), and that is the part worth reading.

The ask, in the superior's words: the framework should be able to **render an extra position
image on request, so an agent knows which position in the scene a pixel of the picture is**.

## The decisions

Taken on review:

1. **The switch is the harness's, not the document's.** No scene schema field. A scene document
   does not carry a debugging concern, and the emitted documents differ for a scene only when
   the caller asked them to.
2. **One channel in the first landing: `Position`.**
3. **A position image is a float image** (`rgba16float`), not a value squeezed into 8 bits. There
   is no decode key, and the numbers an agent reads are the answer.
4. **The graphics document declares its outputs.** A new optional `outputs` field, so which
   image is the frame is a declaration rather than a consequence of pass order.

### The consequence of 1, stated once

The channel is an input to the **compile**, so a render's outputs are a function of the
documents *and the compile's inputs* rather than of the documents alone. That is not new — the
camera index is already such an input, because a compile is a compile for one camera — but this
one is a consumer's preference rather than a fact about the scene.

What it costs: an agent driving the **device over RPC** cannot *ask* for a position image,
because the switch is a field of the compiler that a scene does not carry. What it does not cost:
the device returns every output a document declares and the RPC hands them all back, so such an
agent receives a channel a document was compiled with, and closing the last gap is an argument on
the RPC surface alone. This is a low-regret choice, and it is recorded here rather than argued.

## The gap

A compile produces two documents and a render produces **one** image: `Renderer::render`
returns the colour target of the *last pass that drew into one*
([`mod.rs:344`](../../crates/device/src/wgpu_renderer/mod.rs), [`render.rs:80`](../../crates/device/src/render.rs)).
An agent that authored a scene looks at that image and has nothing else to look at. "The dark
band across the middle" is a position in a picture, not a position in the scene, and the only
bridge between the two today is to author another scene and render *that* — which changes the
frame being diagnosed.

What is missing is not another image. It is the **mapping** between a pixel and the scene. A
world position per pixel is the general form of that mapping, because every other question an
agent asks about a frame — where is that, what is in front of it, which mesh is it, is that
band a shadow or a gap — is a function of it or a sibling of it.

## What the framework already has, which is most of it

- **The depth buffer survives to the post stage.** Every post pass already binds `depth_buffer`
  and the camera's projection ([`emit.rs:2694`](../../crates/scene/src/standard/emit.rs)), and
  `terraces` already recovers world position from them in WGSL
  ([`terraces.rs:1006`](../../crates/gallery/src/terraces.rs)). That recovery is written
  against rows two and three of the whole view-projection rather than against a perspective's
  coefficients, so it holds for a perspective and an orthographic camera alike. **A position
  image is one fullscreen pass away**: no new geometry pass, no change to any material, no
  second draw of the scene.
- **A pass can already name a target of its own.** `add_targets`
  ([`emit.rs:2649`](../../crates/scene/src/standard/emit.rs)) creates `depth_buffer`,
  `color_target` and `present_target`; a pass renders into whatever its `targets` name, and the
  instance holds the payload. A debug image is one more entry and one more pass.
- **A compiler can already supply a shader.** A scene's shaders are instance entries
  ([`emit.rs:690`](../../crates/scene/src/standard/emit.rs)), and nothing stops the compiler
  inserting one of its own. That is what makes the channel a facility of the standard pipeline
  rather than a screen material every scene has to write out by hand.

## What it cannot do, and should not pretend to

- **A normal image is not a sibling of a position image; it is a different feature.** The
  depthing pass writes depth and nothing else, and a rasterize pass has **exactly one colour
  attachment** ([`raster.rs:85`](../../crates/device/src/wgpu_renderer/raster.rs)). A normal, an
  identifier or an albedo image needs a G-buffer — several colour attachments written by one
  geometry pass — which is a graphics-schema and renderer change of its own. It is the obvious
  next channel and it is deliberately not this one.
- **`read_back` reads four bytes per pixel and `Frame` is RGBA8**
  ([`raster.rs:305`](../../crates/device/src/wgpu_renderer/raster.rs),
  [`render.rs:11`](../../crates/device/src/render.rs)). A float image cannot be read back today,
  and no target in the workspace is a float format (`COLOR_FORMAT` is `rgba8unorm`). This is the
  one real cost of decision 3 and it is the bulk of the device half of the work.
- **Nothing in the workspace has ever rendered into a float target.** `rgba16float` is a legal
  format, is blendable and is already accepted by `format_of`, but rendering into one is
  unverified until it is measured on the adapter — so it is the first thing the handoff checks,
  before anything is built on it.

## The design

Four parts, in the order they land. Each is additive: a scene compiled with no channel asked for
produces exactly the documents it produces today.

### 1. The graphics document declares what it produces

The graphics document has two fields today, `passes` and `instance_schema`
([`graphics/lib.rs:150`](../../crates/graphics/src/lib.rs)). It gains a third, optional:

```ron
outputs: {
    present: "present_target",
    position: "debug_position",
}
```

A name is the key and a binding point is the value, so a caller looks an output up by name. The
frame is the one called `present`.

**Why the document must say this**, rather than the renderer keeping the rule it has: today's
rule is *the colour target of the last pass that drew into one*, which is a convention about
position in a list rather than a declaration. A debug pass is written after the post chain and
would therefore **become the frame**, and the beauty image would silently stop coming back. That
is exactly the class of failure this project refuses everywhere else — a document that is valid,
a render that succeeds, and a result nobody asked for.

**Absent means today's rule**: the map is derived from the passes, being `present_target` when
the scene has a post chain and `color_target` when it does not, which is what the last colour
pass resolves to in each case. No existing document changes meaning, and the standard compiler
always writes it explicitly.

### 2. The compiler asks the channel, and answers with a pass

The switch is a field of the concrete compiler — `StandardScene::with_debug([Position])` — rather
than the generic `CompileContext`, because a channel is a concept only the standard pipeline has
and the generic trait should not learn it. A field rather than a context argument also keeps the
device's own recompile identical to the compile it is checking, since the device holds the
compiler.

For each channel the emitter appends, **after the post chain**:

- an instance entry for the target image, `debug_position`, `rgba16float`, the frame's size;
- an instance entry for the channel's own shader, supplied by the compiler;
- one fullscreen rasterize pass that binds `depth_buffer` and the camera's projection, and writes
  its target.

Every channel writes a target of its own, so **turning one on cannot move a pixel of the beauty
image**. A channel that tinted the frame in place would make the image being diagnosed a
different image.

### 3. What a position image *is*

Per pixel, the world position of **the nearest surface that pixel shows**, which is the only
position a picture can be said to correspond to. A pixel no surface covers is **marked as none**
rather than given a position: the depth buffer is cleared to one, so `depth >= 1` is background,
and a pixel reported at the far plane would tell an agent the sky is six hundred units away. The
marker is the alpha channel — `1` where a surface was drawn, `0` where none was.

### 4. The device returns every declared output, in the format it was drawn in

`Renderer` returns the named outputs a graphics document declares rather than one frame, and
`read_back` becomes format aware: the bytes per pixel are the format's, and a float image is
decoded to `f32` rather than handed over as raw texels. `Frame` stays the RGBA8 frame a png is
made of; a float output is a different thing and is typed as one.

`render` — the single frame — stays as a convenience over the same execution, so the gallery
tests and every existing caller keep working unchanged, and no caller runs the pass list twice
to get one image.

## How the agent reads it

The image is the general mechanism; a picture of positions is not readable as numbers. What the
harness does with an output the device returned:

1. **The png**, written beside the beauty image (`out/dunes.png`, `out/dunes-position.png`), with
   the camera suffix where it already goes, so a position image can be *looked at* and compared
   with the frame pixel by pixel. A float image has no natural 8 bit form, so the harness
   normalizes it for viewing and that is a display decision rather than part of the channel.
2. **A sidecar of the numbers** (`out/dunes-position.txt`): the world position at a nine by nine
   grid of pixels, with `-` where the scene drew nothing, and the mapping the png beside it used
   in the header. This is what actually answers "where is that band", and it is why a float image
   is worth its cost. It is a file rather than a line on standard output because a range of
   moments would otherwise print eighty one positions per frame.
3. **A note** from the compiler when the image cannot cover the picture at all — a scene that
   lists no instance under `depthing` — so what a caller was *not* shown is said rather than
   discovered.

## What is deliberately not proposed

- **A G-buffer, and with it normal, identifier and albedo channels.** A separate feature, and the
  one that would pay for itself next.
- **A picking endpoint on the device.** "What is at pixel (x, y)" is a question about a *frame*,
  and the device holds no frame: a render is a function of the documents and reads back what it
  drew. A probe belongs in the harness, over the image the device returned.
- **Carrying the channel in the scene document.** Decided against above; the reasoning is
  recorded there rather than repeated.

## What the first render found

### The measurement that was the whole risk

`rgba16float` as a render target was checked first and alone, before anything was built on it, on
the adapter this repository is developed against. A one pass shader wrote `(1.5, 0.25, -2.75,
1.0)` into an `rgba16float` target built with `BlendState::REPLACE`, and the readback held exactly
`[0x3E00, 0x3400, 0xC180, 0x3C00]` at the first texel and at a texel on a later row, with the
backend reporting nothing. Two things are settled by that and they were both open:

- a colour target of `rgba16float` builds a pipeline with the blend state `raster.rs` builds for
  every pass, so no pipeline path had to change for a float image;
- **the values are not clamped**, so the target really is the float image decision 3 assumed
  rather than an 8 bit one wearing a float name.

### The channel covers what the **depth** bucket drew, and a scene can disagree with itself

The first render of the shipped example scene — `studio`, one mesh, one instance under
`depthing` and two under `lighting` — produced a position image showing **one** triangle where
the picture showed two, in the middle of the frame rather than where either was. Measured rather
than read: the picture's mask is `(280, 454)-(743, 569)` covering 26912 pixels, the position
image's is `(396, 454)-(627, 569)` covering 13572, and `-` is reported at every sample outside it.

That is not a defect in the channel. A position is recovered from the **depth buffer**, and the
depth buffer holds what the `depthing` bucket drew; the example scene deliberately lists one
instance there and two in its lit stage. **The channel reported the depth buffer faithfully, and
the depth buffer genuinely did not describe the picture.** This is a contract on the scene rather
than on the channel, and it is recorded in three places:

- `crates/scene/src/standard/debug.rs`, where the module says what a channel covers;
- a **note** whenever a channel is asked for and the scene lists no instance under `depthing` at
  all, because that is the case where the image is empty and nothing says why;
- the harness's sidecar, which names what `-` means.

A scene whose lit stages draw an instance its `depthing` bucket does not is already a scene whose
lighting pass tests depth against a surface that is not there, so no channel could have named a
position for those pixels honestly. Measured on `dunes`, whose buckets agree: the geometry is
covered to the last row and the sky is `-` for the top 335 rows.

### The picture does not move

`studio` and `tidewater` were each rendered with a position channel and without one, and the
beauty images are **byte for byte identical** (SHA-256 equal in both cases). `tidewater` is the
stronger of the two: it has a post chain, so the frame is `present_target` and the debug pass
comes after the pass that writes it, two cameras, and a perspective camera. A channel that moved
a pixel of the picture would have made the image being diagnosed a different image.

### Where the frame comes from when there is no post chain

The compiler declares `present` as `color_target` for a scene with no post stage and
`present_target` for one with it, which is what the old rule — the last colour pass — resolved to
in each case. The declared name is therefore not a change of picture, and the doctest that
asserted the placeholder's size for a hand written document still holds, because a document that
declares nothing is still answered by the old rule.

## The verification, and what it answered

- **`rgba16float` as a render target works on this adapter**, checked first and alone: **yes**,
  and it does not clamp.
- A scene with a channel and without render the **same beauty image**: **byte for byte**, on
  `studio` and on `tidewater`.
- A position image of a scene that names its own positions reports them: **yes** — `dunes` at
  `(511, 511)` reports `(7.32, 19.14, 18.80)`, and `tidewater` spans
  `x ∈ [-500.8, 465.8], z ∈ [-729.0, 220.6]`, which is the valley it draws.
- A perspective camera and the convention orthographic one both produce one: **yes** —
  `tidewater` is perspective and `studio` is not.
- Background pixels report none rather than the far plane: **yes** — `alpha` is exactly `0.0` for
  1035004 texels of the example scene and exactly `1.0` for the 13572 it drew.
- The no-operation renderer still answers with the right size and format: **yes**, from the
  instance's own payloads, and it is exercised by the device doctest.
- The whole workspace suite, and the device suite on the adapter, are **green**; `clippy` reports
  nothing in anything this change adds.

## Still owed, and deliberately not this stretch

- **A G-buffer**, and with it `Normal` and `Identifier` channels, which is the change that would
  make the coverage contract above go away — a channel that ran its own geometry pass would cover
  exactly what it drew.
- **A permanent test** of the two invariants that matter most here: the picture is unchanged by a
  channel, and a background pixel reports none. They were verified by hand and by scratch
  programs, and a scratch program is not a regression test.
- **Exposing the channel over RPC.** The device now returns every declared output, so an RPC
  caller *receives* a position image a document declares; it still cannot *ask* for one, because
  the switch is the compiler's field.

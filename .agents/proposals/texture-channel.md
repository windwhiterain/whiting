# A sampled texture channel: a binding that names an image and the sampler that reads it

Status: proposal, **not started**. The design below was reached by building the naive version
first and rendering it; the section *What the built version got wrong* is the part worth
reading, because the rule it broke is the whole content of this proposal.

## The gap

A standard scene can express geometry, placement, lighting, shadows, time and a fullscreen
grade, and it cannot sample an image. A texture entry in a `GraphicsInstance` binds a *size and
a format* and holds no pixels, and there is no sampler in the binding model at all.

So every piece of surface detail in a gallery scene has to be either geometry or arithmetic in
the fragment stage. That is a real limit, and it is the last structural one between the pipeline
and a photograph: the standard compiler already writes a noise function, a sky and a Fresnel
term, all of which are cheap, and none of which is a photograph of a surface.

## The shape of the problem

A binding carries **one** resource. A sampled image is **two**: the texture and the sampler that
reads it. That is not a gap that can be filled inside a binding, because the backend allocates
one number per resource and a bind group layout entry holds one thing.

There are three ways out, and the choice is the whole design:

- **A `Sampler` binder kind.** The schema grows a fourth variant beside `Buffer` and `Texture`,
  and a sampling pass declares both. Explicit, and it makes a pass that binds a sampler with no
  texture expressible — which nothing wants.
- **Sampler settings on the texture's own payload**, and the number the sampler takes is
  derived. Two passes that bind one texture then bind one sampler, which is what they should
  do, and there is no fourth kind to learn.
- **A combined binding.** Not available: the backend's layout entry is one resource.

The second is the one built, and it is right about the payload. The mistake was in *how the
number is chosen*, below.

## What the built version got wrong

The rule was: **a bound texture is followed by the sampler that reads it, at the next binding
number.** Counting in one place, so the layout and the bind group cannot disagree, and the
depth exception handled — a depth texture is read with `textureLoad` and gets no number.

It broke every scene in the gallery, and the failure is instructive. Five of the six passes in
`dunes` bound only depth textures, so their numbering was unchanged and they built fine. The
sixth — the post stage — binds the shaded image, which is a **colour** texture, so the rule
gave it a sampler and pushed the depth buffer from binding 2 to binding 3. The post shader names
`@binding(2)`. Every scene failed:

```text
Validation Error
  In Device::create_render_pipeline, label = 'post/post'
    Shader global ResourceBinding { group: 0, binding: 2 } is not available in the pipeline layout
      View dimension D2 (is array: false) doesn't match the binding Sampler(Filtering)
```

The rule cannot tell a texture that is **sampled with a sampler** from a texture that is
**read by integer coordinate**. The post stage does the latter: it reads the shaded image at
the coordinates of the pixel it is writing, and needs no sampler. The rule inferred the need
from the *format*, and format does not carry it.

Worth recording: the agent that built this wrote a texture-atlas scene on top, rendered it, and
produced a frame. It never re-rendered an existing scene, so the regression was invisible until
someone else ran `dunes`.

## The corrected rule

> A bound texture is followed by the sampler that reads it, **when the document says it is
> sampled.**

The need is declared, not inferred. A texture payload carries an optional `sampled`, defaulting
to **absent**, and:

- `sampled` present — the entry gets a texture binding and a filtering sampler at the next
  number. The shader is expected to call `textureSample`.
- `sampled` absent — the entry gets a texture binding and nothing after it. The shader is
  expected to call `textureLoad`.

Absent is the right default rather than `false`, for the same reason `data` is absent rather
than empty: a render target is read, a sampled texture is sampled, and the two are told apart
by the key being there or not.

This is a smaller change than it looks. It keeps every existing scene's numbering byte for
byte, because every existing texture is absent-sampled, and it puts the decision where it can
be seen — in the document, in the same place as the `format` that was being used to guess it.

## What to keep from the built version

The payload design was sound and should be carried over:

- **`data`, optional, on the texture payload**, as RGBA8 bytes. A texture a pass renders into
  carries none — there is nothing to upload into a map the shadow pass is about to write.
- **`filter`, `address` and `mip_filter`, all optional**, defaulting to `Linear`, `Repeat` and
  the filter. They belong to the image rather than to the pass: two passes that bind one
  texture bind one sampler.
- **A mip chain generated on upload** when the declared size is above one texel, by a box
  filter in linear light. Without it any tiling detail aliases into moiré the moment the
  surface is smaller than a texel.
- **The renderer places the inner mip levels itself** rather than copying the whole chain,
  because `COPY_BYTES_PER_ROW_ALIGNMENT` is 256 and a texture whose row is not a multiple of
  it — 300, 100, 90 — is a plainly wanted shape.
- **An attachment's sample type stays `Float { filterable: false }`**, while a read texture
  gets the sample type its format allows, with `is_filterable` stating which those are.

## What makes this landable now

The rule is a numbering rule, and numbering rules are what the device just stopped trusting.
`crates/device/src/wgpu_renderer/shader_interface.rs` now compares a shader's declared
bindings against the layout the pass built, and reports a disagreement as an error naming the
pass and the shader's own variable. The failure above becomes:

```text
the shader for `post/post` declares `depth_buffer` at binding 2 as a texture, and this pass
binds a sampler there.
```

Which is to say: a wrong numbering rule is now a one-line message naming the document to open,
instead of a backend validation error naming a view dimension. That is the difference between a
rule that can be changed safely and one that cannot.

## Decisions

- **Sampler settings on the texture payload, not a `Sampler` binder kind.** See *The shape of
  the problem*.
- **The sampler is allocated by declaration, not by format.** The built version's rule is the
  rejected alternative, and the post stage is why.
- **Absent means not sampled.** The same "absent rather than empty" rule `data` follows.
- **A depth texture is never sampled.** It is read by `textureLoad`, and a filtered depth
  comparison is a comparison sampler, which is a different resource from what a shader that
  compares depths by hand wants. This is unchanged from the built version and was right.

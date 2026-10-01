# An image is scene data, and the compiler is what binds it

Status: **landed.** Everything below is code, with the amendments recorded under *What the build
changed*. The rule held: a scene names its own image data and never names a gpu resource, and the
compiler sorts a material's custom data by kind — packing the numbers into the instance row and
lifting the image into a texture with a sampler. Both a mesh material and a screen material name
one, by the same rule and through the same code.

The scene that shows it is `tiles`, which generates its own pixels and samples them.
`crates/device/tests/gallery_render.rs` draws it on a real adapter and reads the frame back,
because a texture that was declared and never arrived still produces *a* picture, and only a frame
that varies proves one was sampled.

## What the build changed

The image is written in `custom_data` as the design says. What the design did not say is that it
cannot **stay** there: custom data is packed into the instance row as vertex attributes, so every
field of it has to flatten into four byte numbers, and a name does not. So the sort is not a
tidying step — it is what makes the value expressible at all. The kind comes from the shader's
`custom_schema` rather than from the value, which is what lets the compiler decide before it reads
anything an author wrote.

Two consequences were found by the checks rather than by review, and both are worth knowing before
touching the numbering:

- **A binding point's number depends on the field's name.** A pass binds its resources in name
  order, so a field called `image` lands between `depth_buffer` and `shadow_map`, and every number
  after it shifts by one. A shader that counts as though the image came last is refused by
  `check_bindings` with a message naming the point and the kind.
- **The image binder is optional in the interface.** A material need not name an image, so a
  required binder would make every material of that shader carry one. The same correction was
  needed twice, once for the mesh stages and once for the screen stage, which is what an
  invariant that is only stated in one place looks like when it is not stated at all.

One stage is not grouped the way the others are. A post process runs once over the frame rather
than once per object, so a screen material carries its own images rather than sharing a group's,
and the image-emission half of the emitter is a function both callers share.

## The gap

A standard scene can express geometry, placement, lighting, shadows, time and a fullscreen
grade, and it cannot sample an image. The device can now: a texture payload carries `data`,
`sampled`, `filter`, `address` and `mip_filter`, a sampled texture takes a sampler at the
binding after its own, and the pixels and their level chain reach the gpu. What is missing is
the scene side.

## The rule this has to obey

> **The scene layer does not distinguish a gpu resource from an ordinary uniform. The
> compiler is responsible for handling the difference behind the scene's back.**

That rules out a material naming a *texture*, and it rules out a scene listing *gpu
resources*. It does not rule out a scene naming its own **image data**, and the distinction is
not a lawyer's. It is already the framework's own:

> `meshes` is a list of labelled geometry. An instance names one by label. A mesh may be
> written in the scene or referenced from another definition. Nothing in the scene says the
> word "buffer", and nothing in the scene picks a vertex layout — the compiler derives the
> layout from the schema the points were packed with.

A scene names its geometry and the compiler decides what that becomes on the gpu. An image is
the same shape of thing: **data the scene owns and the compiler binds.** So images get the
same treatment, for the same reason meshes already do, and the rule is kept rather than bent.

## One image, one copy

An image is **scene data named once and shared by every material that uses it**, which is what
the mesh list already gives. Every material that names one image binds the one texture the
compiler emitted for it, so the "two passes that bind one texture bind one sampler" that the
device half established holds here by construction rather than by care.

Following the mesh pattern exactly means an image can also be **referenced from another
definition**. That is not a convenience: it is what makes a real image possible at all, because
a definition is sent to the device as json over rpc, and a megabyte of pixels inlined into a
scene document is a megabyte re-sent on every reload. A scene that wants a large image names it
out of a definition of its own; a scene that wants a small procedural one writes it inline.

## What an image is, in the scene

A value with a **size, a format and pixels**: the same fields the device payload already has.
`data` is optional for the same reason the device's is — an image does not have to carry
pixels to be an image — but a scene image with no pixels is an image nothing can sample, so the
compiler says so rather than emitting a texture that reads as garbage.

## How a material uses one

A material's `custom_data` is the one slot a scene already has for "data this material
defines". Names are looked up in the scene's image list, exactly as a mesh label is looked up
in the mesh list and a material label in the material list — so an unknown name is an error at
compile time, naming the material and the name, and not a texture that silently reads black.

## Who notices that it is an image

The compiler, by **kind**. It sorts a material's custom data into two piles:

- every scalar and vector is packed into the instance row, as it is today;
- every image becomes a **texture payload** in the emitted instance, bound at a point derived
  from the name it is written under, with a **sampler**.

The author wrote one `custom_data`; the emitted documents hold one row and one texture, and the
difference is made up behind the scene. Nothing in the scene document changes shape to
accommodate this: no new material field, no second slot.

This is how the row already works. `merged_row_schema` flattens a shader's custom schema into
the row schema, so a shader reads `tint` as a field and never learns an offset. An image is
the one kind of custom value that cannot be flattened into a row of four byte numbers, so it is
lifted out and bound instead — and the shader still reads a field of its own data.

## Where the shader's binding number comes from

This is the part that has to be got right, because a shader names its bindings by hand.

The pass schema is **not** a fixed per-stage constant today. It is a field of the scene
document (`pass_schema`), the emitter reads it, it builds the pass's binder declarations
**from** it, and the instance schema is then *derived* from those declarations
(`derived_instance_schema`). So "the pass declares what its materials actually need" is the
existing mechanism rather than a new layer.

A shader says what it wants where it already says what its custom data is: its `custom_schema`.
One more kind is added to that vocabulary:

- a field of the image kind is **not** packed into the row;
- it becomes a texture plus a sampler at a binding point derived from the field's name;
- the compiler adds that point to the pass schema and the matching entry to the shader's
  interface, so no number is written by hand and no number can drift.

**A shader does not count to its image and does not name a binding for it.** It reads a field
of its own data, and the number is the compiler's business in the same way the row's field
offsets already are.

## Two kinds that stay apart

The custom payload is not pure arithmetic today, in a way worth noting rather than hiding: it
already carries a **shader label**, which is a resource and not a number, and the compiler
already knows to resolve it. An image is the second such kind, and the shader label is the
precedent that makes it unsurprising. The rule "a value carries a kind and the compiler
dispatches on it" is being extended, not invented.

## The type question

`definition` is a general data representation and knows nothing about graphics. So the image
kind should be expressed as a **schema attribute** — the mechanism `components_schema(3)`
already uses to say how many components a field has — declared in `graphics` and read by the
compiler, rather than as a new variant in `definition`'s `SchemaKind`. Adding a variant would
put a graphics concept into the general representation, which is the one thing that crate is
built not to carry.

## What this does not close

- **A screen material's image is bound but never drawn**, because no gallery grade names one yet.
  The mechanism is built and the tests hold the shape; what is missing is a grade that uses it.
- **A texture a pass renders into and a later pass samples.** That is the shadow map and the
  depth buffer, which are pipeline internals rather than scene data, and they already work.
- **A file.** Nothing in this framework reads the filesystem; a shader carries its own source
  and an image carries its own pixels. Pixels are generated in the scene or named from a
  definition, and neither is a path the renderer follows.

## Decisions

- **An image is scene data in a labelled list, named by a material**, following `meshes`
  exactly. A scene names its image data; it does not name a gpu texture. The compiler still
  decides every resource behind the scene.
- **A shared image is emitted once**, so every material naming it binds the one texture.
- **An image may be referenced from another definition**, which is what keeps large pixels out
  of the document that is re-sent on reload.
- **The compiler sorts custom data by kind**, packing numbers into the row and lifting images
  into texture payloads with samplers.
- **The binding number is derived from the name and written by nobody**, and both the pass
  schema and the shader interface are generated from it, so they cannot disagree.
- **The kind lives in `graphics` as a schema attribute**, not in `definition`'s `SchemaKind`.

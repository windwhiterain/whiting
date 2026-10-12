# A post stage can read any image its document names except the environment map

Measured 2026-10-12, while looking for the mechanism behind
[one radiance cannot both sky and ground](one-radiance-cannot-both-sky-and-ground.md). This is
the mechanism underneath it, found by reading the emitter rather than by measuring frames.

## The question

`landscape` and `dunes` disagree about what an environment radiance should be. The reason is
that the rasteriser's sky is painted by its post stage from `air_sky`, which reads no texture
and no light, while its ground is lit by the environment light. Before proposing to change
that, the question is what it would take: **can a scene's post stage read a binding it needs?**

## The answer

**Almost. A post stage reads any image its document names — and the environment
map is the one thing it cannot.**

A screen shader carries a `pass_schema`, and `post_shader` fills it in
(`crates/gallery/src/shaders.rs:2804`). The emitter builds the post pass's bindings from a
list of its own and does not consult that field for them — it reads it once, at `:1130`,
to refuse a scene that names an exposure its post shader does not declare:

```rust
// crates/scene/src/standard/emit.rs:3262-3318
let mut uniform = vec![
    (CAMERA.to_owned(), camera_value(projection)),
    (PROJECTION.to_owned(), projection_value(projection)),
    (TIME.to_owned(), Value::Float(time)),
    (DEPTH_RANGE.to_owned(), ...),
];
if let Some(exposure) = exposure {
    uniform.push((EXPOSURE.to_owned(), Value::Float(exposure)));
}
let pass = rasterize(
    ...,
    images.iter().cloned().chain([
        (VERTICES.to_owned(), ...),
        (COLOR_TARGET.to_owned(), ...),
        (DEPTH_BUFFER.to_owned(), ...),
        (PRESENT_TARGET.to_owned(), ...),
    ]),
    vec![PRESENT_TARGET],
    ...
);
```

The bindings are the frame, the depth, the output, and whatever images the scene declared.
That last clause is the exception, and it is a real one: a post stage already reads **any image
its document names**. `post_processes` carries `images`, and `declare_read_images` turns each
into a binding on the pass.

## The asymmetry, stated precisely

The lighting stage has the opposite rule. `lighting` is emitted with

```rust
// crates/gallery/src/shaders.rs:2773
light_pass_schema_over(&extras.binding_points())
```

— a base list **over** whatever binding points the scene declared. `light_pass_schema_over`
(`crates/scene/src/standard/pass_schemas.rs`) is that function, and its own comment says why
the base is fixed and the extras are not:

> A binding that came and went with the document would move `shadow_pool` and `spot_cones` off
> the numbers the shader declares them at.

So the compiler solved this for one stage and not the other. A scene can declare its own
outputs and the lighting pass honours them; the post stage's `pass_schema` is written into the
document and not read back by the pass that uses it.

**The uniform is not part of this.** A pass's uniform is uploaded with the layout its own shader
declares, not the one the emitter assembles:

```
// crates/scene/src/standard/emit.rs:1123-1124
// A pass's uniform is uploaded with the layout its **own shader declares**
// (`graphics::inputs::uniform_schema`), so a field the interface does not declare is
// not a field the frame disagrees about: it is a field the upload drops.
```

`graphics::inputs::uniform_schema` reads the pass's interface and returns its `uniform` fields.
The list `post_pass` builds supplies **values**; the schema supplies the **layout**. A scene
that declares `exposure` in its post shader's `pass_schema` already gets the field, and the
`exposure` block at `:1130` is a check that it declared it rather than a second way of adding
it.

So the consequence for this work is specific and narrow: **a post stage can read any image its
document names except the environment map.** The map is bound through the lighting stage's own
path — `environment_binding` at `:3938`, entered at `:3940` from the scene's
`Environment` light rather than from the image list — so there is no route from a post
stage to it. The document has no way to express it, and the fix is a route rather than a
redesign of what a post stage is.

## What is not wrong here

Two things that look like part of this and are not:

- **The device does not restrict it.** `crates/device-rasterize/src/wgpu_renderer/bindings.rs`
  builds groups generically from whatever a pass declares — `read(source, payload, max_layers)`
  at `:302`, `layout_entries` at `:618`, `group_entries` at `:673`. A pass that declared an
  environment map would be bound one. Nothing in the device stands in the way either.

- **The projection is not the obstacle it appears to be.** `environment_radiance` in
  `crates/gallery/src/shaders.rs:1962` already samples `environment_map` by a direction, and
  `environment_texel` at `:1805` is that lookup. The obstacle noted in
  `pass_schemas.rs:605` — that the map is documented as octahedral while `dunes` declares a
  latlong one — is a real mismatch, but it is a second question. The first is that the post
  stage cannot be given the map at all.

## What fixing it means

**One route, not a rebuild.** `post_pass` already takes `images` and turns each into a binding;
the environment map is missing because it never entered through that door. Letting a post stage
name the scene's environment in its `images` list, the way it names any other image, is the
whole of the fix — the binding, its entry and the shader's lookup already exist, and
`environment_radiance` at `shaders.rs:1962` is already the shader-side half of it.

This is far smaller than the change this document first described. An earlier draft said the
post stage's interface would have to stop being the compiler's and become the document's, which
would have meant rebuilding `post_pass` and re-deciding what a post stage is. That was wrong,
and the reason it was wrong is worth keeping: **the post stage's interface is already mostly the
scene's**, and the compiler's part of it is the four bindings that make it a pass — the
frame, the depth, the output and its own triangle. Those are not a restriction on what a scene
can read. They are what makes the pass run.

# The post stage's interface is written by the compiler, not declared by the scene

Measured 2026-10-12, while looking for the mechanism behind
[one radiance cannot both sky and ground](one-radiance-cannot-both-sky-and-ground.md). This is
the mechanism underneath it, found by reading the emitter rather than by measuring frames.

## The question

`landscape` and `dunes` disagree about what an environment radiance should be. The reason is
that the rasteriser's sky is painted by its post stage from `air_sky`, which reads no texture
and no light, while its ground is lit by the environment light. Before proposing to change
that, the question is what it would take: **can a scene's post stage read a binding it needs?**

## The answer

**No. The compiler writes the post stage's interface and the scene's declaration of it is
decorative.**

A screen shader carries a `pass_schema`, and `post_shader` fills it in
(`crates/gallery/src/shaders.rs:2804`). The emitter then builds the post pass from a
hardcoded list and **never reads that field back**:

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

The uniform is a fixed list with one conditional field, and the bindings are the frame, the
depth, the output and whatever images the scene declared. There is no path by which a binding
a scene's post shader asks for reaches the pass.

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

So the compiler already solved this problem for one stage and not the other. A scene can
declare its own outputs and the lighting pass honours them; a scene cannot declare anything the
post stage reads, and the post stage's `pass_schema` is written into the document and then
ignored by the pass that uses it.

The consequence for this work is specific: **a scene cannot fix its own sky by reading its own
environment map in its post stage**, which is the change the decoupling finding asks for. It is
not that the change is awkward — it is that the document has no way to express it.

## What is not wrong here

Two things that look like part of this and are not:

- **The device does not restrict it.** `crates/device-rasterize/src/wgpu_renderer/bindings.rs`
  builds groups generically from whatever a pass declares — `read(source, payload, max_layers)`
  at `:302`, `layout_entries` at `:618`, `group_entries` at `:673`. A pass that declared an
  environment map would be bound one. The limit is entirely in the emitter's `post_pass`.

- **The projection is not the obstacle it appears to be.** `environment_radiance` in
  `crates/gallery/src/shaders.rs:1962` already samples `environment_map` by a direction, and
  `environment_texel` at `:1805` is that lookup. The obstacle noted in
  `pass_schemas.rs:605` — that the map is documented as octahedral while `dunes` declares a
  latlong one — is a real mismatch, but it is a second question. The first is that the post
  stage cannot be given the map at all.

## What fixing it means

`post_pass` has to take its uniform and its bindings from the shader's own `pass_schema`, the
way `light_pass` takes its from `light_pass_schema_over`, and the post interface stops being a
compiler-fixed list. The `post_pass_schema` and `post_pass_schema_exposed` pair at
`pass_schemas.rs:685` and `:706` would become the *default* rather than the answer, and a scene
that wants its sky from its environment map would say so by declaring the map.

This is a change to what a post stage **is** — from "a stage the compiler gives a frame" to "a
pass whose interface the document states, with the frame among its bindings" — and it is
recorded here rather than made here.

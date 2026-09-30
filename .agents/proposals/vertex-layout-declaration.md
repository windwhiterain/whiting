# Vertex layout declaration

## The problem

A rasterize pass can clear its target and read it back, but it issues no draw calls. The
reason is not that drawing is hard: it is that no document says what a vertex buffer
contains in a form a renderer can hand to `wgpu`.

A buffer payload currently says `size_bytes`, `usage` and `data`. Two things are missing:

- **How many vertices.** `size_bytes` divided by the vertex stride would do, but the
  stride is exactly what is not declared.
- **What each vertex looks like.** `wgpu` needs an array stride and a list of attributes,
  each with a format, a byte offset and a shader location. None of that is anywhere in
  the graphics document, and none of it can be recovered from a shader without
  reflection, which the workspace has no source for.

## What a declaration would look like

The point a shader reads is already declared in the pass, so the layout belongs beside the
size, inside the same declaration:

```text
Resource(Buffer(
    source: "quad_vertices",
    size_bytes: 192,
    layout: VertexLayout(
        stride_bytes: 32,
        attributes: [
            Attribute(shader_location: 0, offset_bytes: 0, format: Float32x3),
            Attribute(shader_location: 1, offset_bytes: 12, format: Float32x2),
        ],
    ),
    vertex_count: 6,
))
```

`vertex_count` and `stride_bytes` are redundant with each other and with `size_bytes`, and
that redundancy is the point: a compiler that packs a mesh knows all three, and a renderer
that draws must not have to infer any of them.

## Why it is not in this change

It is a new field on the binder declaration, which every shader interface and every
compiler has to agree on, and the superior should choose it rather than an agent. It also
lands in the same place as the uniform layout question below, so doing both at once
avoids two rounds of the same churn.

## The uniform question, which is the same question

A pass's `uniform` is an arbitrary `Value` and the renderer does not upload it at all
today. The scene compiler packs a uniform the way it packs points: field order from the
schema, each field four little endian bytes, lists flattened. That convention lives in
`scene/src/standard/packing.rs` and is documented there, but it is a convention between a
compiler and a shader author with nothing checking it.

The fix is the same shape as the vertex layout: the uniform's own declaration says its
layout, the way a vertex buffer's would. See the decision in
[SPEC.align.md](../SPEC.align.md) about where that belongs.

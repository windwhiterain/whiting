# The identity a building's frame can carry

Started 2026-10-16. Branch `feature/building-identity` (from `main` at `b24faa8`, which closed the floor
slice). The design is [`agentic-building-generation.md`](../proposals/agentic-building-generation.md).
**This file and the proposal are the whole record.**

## The debt this slice pays

`building::identity` (P2) answers *"what is this element called"* from a document and its mesh. What it
cannot do is answer it **from a picture** — and a picture is all an agent looking at a frame has. The
mountain side of the gallery already has the join (`gallery::identity`: an `authored_id` output, a packed
point buffer, and `locate` to turn a pixel back into a belt); a building has none of it.

This slice is the **CPU half** of that join, and it is where the crate's own vocabulary ends: the numbers
per corner, and the scene's declaration that it writes them.

## What is here

`crates/gallery/src/building_identity.rs`:

- **`building_extras()`** — one output, `authored_id` at location 1, read from the points and carried
  **flat** (an element is a property of a face, not a place on it, so interpolation is the wrong operation
  and flat is the right one).
- **`outputs_field()`** — the scene's own `outputs` declaration, `f32` because `r32float` is exact where an
  `rgba16float` is exact only to 2048.
- **`ids(built, identity)`** — one id per point, three per face, and **the definition `packed` writes** so a
  probe or a tool can read the numbers without decoding a buffer.
- **`packed(built, identity)`** — the building's points with that id beside the corner normals, the
  positions and the plan `texture`, laid out through `packed_layout` on the schema's own order.

`crates/gallery/src/shaders.rs` gained `Extras::outputs()` (and made `Extra`'s fields readable), because a
tool that numbered a target off a second list would be reading somebody else's picture.

## Invariants

- **The id is written per corner and identical at all three.** The mesh is not indexed, so a value at one
  corner is interpolated across the face; a differing corner is a pixel pulled between two elements.
- **A face whose element has no row is refused, not written as zero.** Zero means "not an authored element"
  — the sky, the ground — and a wall answering zero would be a picture lying about what it shows. The table
  is built from the same mesh, so a miss is a bug, and it panics with that sentence.
- **The layout is the schema's own**, read with the same `packed_layout` the renderer uses.

## Acceptance so far, measured

`examples/building_identity_probe.rs` on a document holding a two-storey shell, a plate, a courtyard wall
and a part in a bay:

| measurement | value |
| --- | --- |
| the table | **100 faces, 50 elements numbered `1..=50`** |
| points per face | `300` for `100` faces, and **0 faces whose corners disagree** |
| the numbers written | **50 distinct ids against 50 rows — equal**, and zero is **never** written |
| what a pixel would answer | `id 1 → mass 0, outer wall over edge 0`; `id 44 → wall 0, its top` |
| the declaration | `authored_id` at location 1, `f32`, one output |
| an empty document | 0 faces, 0 elements, 0 ids |

## What is *not* in this slice, and where it belongs

**The frame half.** A scene must declare `outputs_field()` and draw its meshes with `building_extras()`,
and the renderer must keep the extra target and hand it back — the mountain's `landscape` scene is the
template for the first part (`stage_set` is its own private helper, so either lift it or write the building
equivalent). Nothing in this slice touches a pixel; what it establishes is the numbers a pixel will carry
and the sentence a missing one is refused with.

## Status log

- **2026-10-16** — branch from `b24faa8`; module, extras accessor, probe, and the measurements above.
- **2026-10-16** — **the frame half, and the loop closes.** `building_identity` gained
  `not_authored_extras()` (the reserved zero as a **constant**, so a surface that is not an element carries
  no id field at all), `stage_set(prefix, body, helpers, point, extras)` (the four stages of one mesh, with
  the lighting one declaring the outputs, filtered by `Stage::tag()` rather than by a second stage table),
  and **`locate(id) -> Option<String>`**, which rebuilds the identity scene's own document and answers with
  the row's path.

  The scene is `crates/gallery/src/authored.rs` (registered as **`identity`**, 7 passes): a two-storey shell
  with a plate, a courtyard wall and a shutter in a bay, one mesh, one material body, and the ground drawn
  with the constant zero. `cargo run -p whiting-device-rasterize --example render -- identity out/identity`
  writes **three** files: the frame, `identity-authored_id.png`, and `identity-authored_id.txt` — the values,
  because a png cannot hold floats.

  Measured, from the rendered frame's own sidecar:

  | fact | value |
  | --- | --- |
  | the image | `1024x1024, r32float`, red over **`[0.000, 44.000]`** |
  | building pixels | ids `3, 4, 16, 20, …` |
  | sky and ground pixels | **`0.000`** — the reserved zero doing its job |
  | the join | `id 3 → mass 0, outer wall over edge 2`; `id 4 → edge 3`; `id 16 → roof 0, the skin of plane 1`; `id 20 → roof 0, the fascia over the skin's boundary edge 1` |
  | an id the table does not hold | `0` and `9999` both answer **`None`** |

  Three things the wiring taught, all of them measured rather than reasoned:

  - **A material names its shader by the label `stage_set` built** (`<prefix>_<stage>`), so a scene with two
    meshes needs two labels and the two cannot read each other's point layouts.
  - **A constant is not a field.** `not_authored_extras().point_schema()` is the **three**-field shaded schema,
    so the ground must be packed as a heightfield — packing it over the building's four-field layout is
    `the points are packed as normal, position, texture but the mesh's point schema lays them out as
    authored_id, …`, which the compiler caught at the first attempt.
  - **The pipeline needed nothing.** `render_outputs` already reads a scene's `outputs` and hands back one
    image per declaration, and the render example already writes each as `<out>-<channel>.png` plus a sidecar
    of the values. The whole frame half was the scene's declaration and the points' packing.

  One accident worth recording: a failed scripted edit wrote an **empty** `landscape/mod.rs` (a `Set-Content`
  on a null variable after a `Substring` threw), and the build named it immediately; it was restored from the
  previous commit with `git checkout --`, which is the reason the file was committed before it was touched.

- **Next**: nothing owed in the crate. Possible next slices, in the order this session would take them: a
  scene drawing **instances** (`Template::mesh` + `PartFrame`, so a gallery scene stops uploading one copy
  per placement); **`build` caching** a template's expansion per document; and the **shadow terminator**,
  which belongs to another subsystem.

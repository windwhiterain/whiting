# The renderer draws a scene document, and the gallery writes one

**The gallery generates the scene def directly, and the renderer renders a scene def.** The two
used to meet at a name, and the name was the coupling: every renderer-side target asked the
gallery for a scene, and a change to a scene relinked the renderer. This is the split, and what
it cost and what it bought.

## The new invocation

```text
# the producer: a gallery scene written out as the document a renderer reads
cargo run -p whiting-gallery --example scene_document -- <scene-name> <path.json> [--census] [--bounds]

# the renderer: a document drawn, with no gallery anywhere in it
cargo run -p whiting-device-rasterize --example render -- --document <path.json> <out> [times] [mode]
cargo run -p whiting-device-cycles  --example render_cycles -- --document <path.json> <out.png> [samples] [device]
```

The render harness takes the name of the scene either from the document's own `name` field or
from the command line, and the second is what the manifest records:

```text
render --document hall.json out/hall              # the document's own name
render hall out/hall --document hall.json         # the name the caller gives it
```

`--census` is no longer a flag of the renderer. The census reads the document, and the document
on this path is the one that was read from the file, so the producer is where it is printed:
`scene_document -- <name> <path> --census`.

## The two couplings, which are different jobs

The renderer-side targets that named the gallery were six, over forty references:

| target | refs | what it asked the gallery for |
| --- | --- | --- |
| `crates/device-rasterize/examples/render.rs` | 21 | `Gallery::{build,census,bounds}`, `gallery::{locate,identity,building_identity}` |
| `crates/device-rasterize/examples/shadow_probe.rs` | 5 | `Gallery::build`, `gallery::{geometry,kit,shaders}` |
| `crates/device-rasterize/tests/{clustered_shadow,environment_map,multi_attachment}_render.rs` | 12 | `gallery::{geometry,kit,shaders,attributes}` |
| `crates/device-cycles/examples/render_cycles.rs` | 2 | `Gallery::build` |

They were two different jobs, and only one of them was the name registry.

### 1. `Gallery::{build, census, bounds}` is a scene-name registry, and it belongs to the producer

A name is the gallery's own fact: it knows every scene and what it is called. A renderer that
asks for a scene by name is asking the content for content, and a change to a scene then costs
the renderer a link. The producer is `crates/gallery/examples/scene_document.rs`: it builds a
scene, writes it as the json `facet_json` writes, and prints the census and the bounds of what
it wrote.

The census and the bounds were extracted into `Gallery::census_of` and `Gallery::bounds_of`,
which take the document rather than a name, so the producer reports over the document it just
built instead of building the same scene twice. `Gallery::census` and `Gallery::bounds` are the
name registry's wrappers over the two, and they are still there for a caller that has a name.

### 2. `gallery::{identity, locate, building_identity}` are document utilities, and they move

They answer "given this id, what is it" and "given this row, which handle", which are
questions about a *document*, not about a scene. They are now in `crates/document`:

- `document::identity` — the identity table, the reserved-id contract, and the mesh writers
  that put an id in a frame.
- `document::building_identity` — the same for a building. `id_in` and `locate_in` take the
  `&Building` rather than a scene name, because which building a scene draws is the content's
  fact and a document crate that carried the list would be carrying the gallery with it.
- `document::naming` — the naming contract: the table of ids and handles, the sentence about
  what an id means, and the words around a handle.

The content's half stayed in the gallery, in `crates/gallery/src/naming.rs`: the sentences each
scene says about its own ids, the list of buildings the gallery draws, and the tables those ids
are named by.

## How the naming reaches a renderer that cannot see the gallery

**The scene document carries it.** The standard scene schema gained one optional field,
`elements`: the handle each id names, the sentence about what an id means in this scene, and
the sentence about the reserved zero. The table is built where the ids were numbered — the
plant builder for the plant scenes, the gallery's building list for the buildings and the
landscape — so a pixel and a name are the same number by construction rather than by two
orderings that have not diverged yet.

This is also the *more correct* arrangement, and not only the possible one. A table rebuilt by
the content on every question answers about the content *now*, which is a different question
from the one a frame asks: a document written before an edit and a frame drawn from it are one
picture, and the name that goes with the picture is the name the document carried when it was
written.

The compiler does not read the field, so the documents it emits are the ones it emitted before:
the frames are unchanged, and the manifest says so in hashes.

## The measurement

The win is felt as a rebuild, so it was measured as one.

```text
touch crates/gallery/src/hall.rs
cargo build --release -p whiting-device-rasterize --example render
```

| artefact | before | after |
| --- | --- | --- |
| `target/release/examples/render.exe` | length 11407360, LastWriteTime 2026-10-20T07:13:52.8520230Z | **unmodified** — same length, same LastWriteTime |
| `target/release/examples/scene_document.exe` | length 4228096, LastWriteTime 2026-10-20T07:08:22.0124908Z | relinked, LastWriteTime 2026-10-20T07:14:07.7657291Z |

A document edit costs zero cargo: the renderer is not in the gallery's dependency graph at
all, so cargo had nothing to do (the build finished in 0.29 s), and the producer relinked
because the scene it compiles into it changed.

The frames are the other half of the claim, and the harness's own manifest is what says so:

| scene | documents | pixels |
| --- | --- | --- |
| `hall` | `e74fcca4725153fa9281a8f1600950f1e49f7cd4438733a7d824efc237f4afbf`, identical | `72717cd0d81ccce351b1ab4d7ee1cd088148c9f6c1c6bbadaf3f5cb0bab75c34`, identical |
| `alpine_oak` | `d2139a459c99faa912c91fb69e2659d25edf081a71c0c9215afe4d7727d008f8`, identical | `77b331f2f9a13f79fff9564126f59daa97b6e5b0d5d2787bc1ade8cf83659ddd`, identical |

`alpine_oak` is the one that matters: its document is 652 794 206 bytes of json, and a round
trip that lost or reordered anything in it would show there first.

## The old invocation spells, to be fixed

These are in the gallery's own scene files, and they are **left as they are** rather than edited
here: each one names a renderer invocation that no longer exists, and each of them needs a
producer in front of it. The fix is a producer call and a document, in that order.

| where | the old spell |
| --- | --- |
| `crates/gallery/src/authored.rs:22` | `cargo run -p whiting-device-rasterize --example render -- identity out/identity` |
| `crates/gallery/src/shed.rs:10` | `cargo run -p whiting-device-rasterize --example render -- shed out/shed` |
| `crates/gallery/src/vegetation_laws.rs:18` | `cargo run -p whiting-device-rasterize --example render -- garden_wall out/wall` |
| `crates/gallery/src/vegetation_picture.rs:13` | `cargo run -p whiting-device-rasterize --example render -- bamboo out/bamboo` |
| `crates/gallery/src/southwest_valley.rs:150` | `cargo run -p whiting-device-rasterize --example render -- …` (the same spell, in prose) |
| `crates/gallery/src/sweep_picture.rs:21` | `cargo run -p whiting-device-rasterize --example render -- sweep out/sweep` |
| `crates/gallery/src/workshop.rs:27` | `cargo run -p whiting-device-rasterize --example render -- workshop out/workshop` |
| `crates/gallery/src/yard.rs:18` | `cargo run -p whiting-device-rasterize --example render -- yard out/yard` |

The new spell for each of them is two commands:

```text
cargo run -p whiting-gallery --example scene_document -- <scene-name> <path.json>
cargo run -p whiting-device-rasterize --example render -- --document <path.json> <out>
```

## What moved where, and what was left undone

**Moved into `crates/document`:** `air`, `attributes`, `building_identity`, `geometry`,
`identity`, `kit`, `naming` (the old `locate`), `shaders`. The gallery depends on the document
crate; the renderer depends on the document crate and not on the gallery.

**The shadow probe moved the other way**, to `crates/gallery/examples/shadow_probe.rs`. Its
control is the gallery's `spotlight` scene — "the one scene in the tree whose virtual shadow
was measured before this work" — so a renderer that cannot see a scene cannot be reconciled
against one. It is the only renderer-side target that still names the gallery, and it lives
with the content rather than against it. The gallery's dev-dependencies on the renderer and the
device are a cycle that exists only for probes, which cargo allows and the published library's
dependency graph never sees.

**The gallery's dev-dependency is gone from `crates/device` and `crates/device-cycles`.** The
device's probes never named a gallery scene, so the dependency was there for nothing. The
cycles harness now reads a document like the rasterising one does.

**Left undone, with the reason:**

- **The landscape's long answer is shorter.** `gallery::identity::locate` answered a pixel with
  the face it showed, the belt's parameters and the edit that would change it — the massif's own
  numbers, which are not in the document. The renderer now answers with the handle alone, which
  is what the document carries. The prose is content, and a content-free renderer cannot have
  it without a link to the content.
- **A plant's handle is spelled "planting N, …" only when the scene draws more than one
  plant.** A scene that draws one plant used to answer with the bare handle and now answers
  with the bare handle still; a scene that draws several answers with the planting beside it,
  as it always did. The rule is the one the old `handle_of` functions followed by hand.
- **`crates/scene/tests/shadow_pages.rs` fails on `main`**, before this branch: it applies
  `with_one_more_light` twice and so asks for two lights labelled `added`, which the compiler
  refuses as a duplicate. The file is untouched here.

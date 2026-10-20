# An incremental scene document: parts, a manifest, and the boundaries between

> Status: **proposed; nothing built, and the first step is a measurement rather than a design.**
> This note records why the pipeline is *cached* but not *incremental* in the one place a caller
> sees: the scene document. It does not reopen the stage decomposition of the lichen vertical
> ([plant-pipeline-into-lichen](plant-pipeline-into-lichen.md),
> [building-pipeline-into-lichen](building-pipeline-into-lichen.md)); it takes the cells as given
> and asks what has to change **inside** the document stage and after it.
>
> Context: the generation itself is the larger cost (8.6 s for one plate, decomposed in
> [where-a-vegetation-plate-spends-its-build](../investigate/where-a-vegetation-plate-spends-its-build.md)).
> A document that is cheap to *regenerate* is still worth nothing if the *write*, the *compile* and
> the *upload* all redo themselves on every edit. This note is about those three.

## 1. What the words mean here

A cache answers *"is this result still here"*. Its unit is whatever the caller marked — a `@cache`
cell, a slab intern, an artifact. Incrementality answers *"how much of the output must change"*,
and it needs three things at once:

1. **a stable identity per part** — the part is recognised by a name, so inserting one element does
   not renumber the rest;
2. **a key per part** — "this part did not change" is decidable without comparing its bytes;
3. **an indirection at every boundary the output crosses** — value, file, compile product, device
   buffer — so an unchanged part is *named* rather than inlined.

The pipeline satisfies (2) only at cell granularity (a stage's key), and satisfies (3) nowhere on
the producer side. That is the whole of the diagnosis; the sections below are the specifics.

## 2. Where each condition breaks

### 2.1 The cell boundary is drawn across change rates

`lichen-stages::stages::document(drawn, key_position)` reads the *drawing* and the *key light* in
one cell, and its body is `plant_scene_drawn`, which packs every mesh. So a lighting edit re-runs
the whole document stage — measured at **1.86 s** against a fresh 12.0 s, with five of six cells
reused — and every one of those seconds is spent rebuilding geometry whose inputs did not move.

The building vertical already has the right shape, and it is the template: `hall::parts` reads only
`built` and produces the four packed point lists plus the building's own dimensions;
`hall::document` reads `parts` + the two presentation dials. `HallParts`' own invariant is the
sentence the plant is missing:

> **Nothing here is a function of a dial of presentation.**

A lens edit is therefore 4–5 ms, because the geometry is in a cell the lens does not reach.

So the first defect is not "the cache missed". It is that the plant's document stage mixes a
geometry input with a presentation input, and the cache can only act at the granularity the cell
boundary gives it.

### 2.2 One value, and a diff that addresses by position

The document is one `definition::Value`. Whiting has a sparse patch mechanism and it is used on the
*emitted* documents (`assemble` → `Delta`, applied by `Device::reload` with a cross check). Two
properties of it matter here:

- `diff::list_delta` replaces a whole list whenever its length changes, and `Path` addresses list
  elements by index — so inserting or removing one element moves every later element's path and the
  diff can only say "the list changed".
- A changed `Packed` run becomes a whole `Replace(clone)` of that run.

The document already carries identity — a mesh has a `label`, an instance has its own `id`, an
element has its `id` — but the diff does not use any of it. The identity exists and is unused,
which is the cheapest possible place to start.

### 2.3 Every downstream step copies the whole thing

- `CompileContext::scene_value()` resolves every reference into one `Value` before a compiler reads
  it.
- `emit` copies each mesh's point bytes into the instance document
  (`buffer_payload(… points.bytes().to_vec() …)`), one entry per mesh.
- A changed run reaches the registry as a whole-value replace.

So compile and render both scale with document size: **compile 5.74 s, render 2.43 s** for
`alpine_oak`; **compile 13 ms** for `hall`, whose document is 4.27 MB. That is one curve, not two
mechanisms.

### 2.4 The measured shape

Release, one machine, from the session that built the vertical:

| step | `alpine_oak` | `hall` |
|---|---|---|
| document build (the stage) | 2.11 s | 3.3 ms |
| an edit that spares geometry | 1.86 s (lighting) | 4–5 ms (lens) |
| write the document | 2.6–4.2 s for 481 MB | 4.27 MB |
| read it | 0.63 s | 16.5 ms |
| compile | 5.74 s | 13 ms |
| render | 2.43 s (21 passes) | 75 ms (13 passes) |

87.7 % of the plant document's 481 MB is packed triangle runs. So the document is not "large and
occasionally slightly edited": it is one value whose weight is the geometry, and every boundary
above moves all of it.

## 3. The mechanism: parts and a manifest

```
parts   = { wood: id, blades: [id], knots: [id], floor: id,
            rows: id, lights: id, elements: id }      // each interned, each with its own key
manifest = [ (label, part key, digest) ]               // small: what the document *is*
```

The document becomes a *manifest* of parts, and each boundary learns to name them:

| boundary | today | incremental form | what already exists |
|---|---|---|---|
| cell | one document cell | one cell per part family (or per element) | `slab::intern`; the building's `parts`/`document` split is the worked example |
| value | one `Value` | a manifest (part ids + keys) | `Held`; `slab::get` is a deep clone, so a manifest stores **ids, not values** |
| file | the whole document every time | chunks plus a manifest, or one file with a section table and in-place patching | the binary codec; `PackedList` compares by `memcmp` |
| compile | resolve, then copy | keep the reference; do not resolve | the schema's `reference` slots for `meshes`/`images`/`cameras`; the instance document's `entries` already admit references |
| device | buffers rebuilt per render | patch by delta | `Device::reload` applies a delta today, with a cross check |

One convenience that costs nothing: **the previous document is still in the slab.** `intern` never
evicts, so a host that keeps the previous document's id can diff the two without lichen exposing
any stateful operator.

## 4. The four routes, and what each buys

**A — split the document stage along change rates.** Give the plant the split the building has:
an `oak_parts` cell keyed on the drawing alone, and an `oak_document` cell that adds the lighting
and assembles. Small, no format change, provable against today's bytes.
*Buys*: the lighting edit's 1.86 s → the non-geometry remainder (unknown until measured; the
document stage's own split is §6). *Does not touch*: the 2.6 s write, the 5.74 s compile.

**B — parts plus a manifest, on disk.** Content-addressed chunks plus a small manifest; only
changed chunks are rewritten. *Buys*: the write becomes proportional to the change, and a reader
can leave unchanged chunks in the page cache. *Requires*: stable part identity (§5) before it helps
a scene with many elements.

**C — the consumer stops copying.** `emit` references the scene's point runs instead of copying
them, and the delta learns a keyed list operation. *Buys*: potentially the largest single step —
if the 5.74 s compile is dominated by the copy, it becomes proportional to the number of parts.
*Costs*: `emit.rs` is ~5 700 lines and four consumers (rasterize, cycles, RPC, standalone) have to
be shown to follow a reference.

**D — element cells in lichen.** Generalise `@cache` to a collection so a program can retain per
element. Not recommended now: it needs float and bytes in the lowlevel (the critical path in
[floating-point](../../../lichen-vm/docs/notes/floating-point.md)), and a 481 MB document must not
become a lichen value at all (§5).

Recommended order: **measure (§6) → A → B → C**, with D recorded as the language-level form of the
same idea and left unbuilt.

## 5. What lichen needs, and what it must not do

**Must not**: turn the document into a lichen value. `lichen_highlevel::ValueExt` is
`Debug + Copy + PartialEq`, and the vertical's boundary carries a `usize` id precisely so that
megabytes of curves never enter the value layer. A 481 MB document in the module arena would also
be walked by the deep pass and copied by every freeze. The opacity is the design.

**Needs, in the general case** (recorded, not scheduled): element-keyed cells — a cell whose
identity is `(occurrence path, element key)` — and a `K_out` per cell, since "did this part
change" without a key means comparing values, and lichen's array equality is allocation identity.
Both are in [incremental-update](../../../lichen-vm/docs/notes/incremental-update.md) §2.5 and
[heavy-value-cache](../../../lichen-vm/docs/notes/heavy-value-cache.md) §3.

For this application, neither is on the path: the host's slab already interns per part, and a part
is compared by `PackedList::matches` (a `memcmp`) rather than by a lichen value comparison.

## 6. The first step is a measurement

Two numbers order everything above, and neither exists:

1. **`plant_scene_drawn` by part** — wood points, blade points, knot points, floor, materials,
   instances, naming. That is route A's ceiling, and it is also what says whether the lighting
   edit's 1.86 s is mostly packing.
2. **`emit` by phase** — resolving the scene, reading and copying each mesh, building the passes,
   validating, diffing. That is route C's ceiling.

Until they exist, A and C are guesses about their own size. The probe prints and asserts nothing,
the way `vegetation_cost_probe` does.

## See also

- [where-a-vegetation-plate-spends-its-build](../investigate/where-a-vegetation-plate-spends-its-build.md) — the generation's own decomposition
- [the-cost-of-the-binary-form-of-a-scene-document](../investigate/the-cost-of-the-binary-form-of-a-scene-document.md) — the document's on-disk sizes and read times
- [scene-def-the-renderer-draws-a-document](scene-def-the-renderer-draws-a-document.md) — why the seam is a document at all
- [plant-pipeline-into-lichen](plant-pipeline-into-lichen.md), [building-pipeline-into-lichen](building-pipeline-into-lichen.md) — the stage decomposition this note takes as given
- `lichen-vm`: [incremental-update](../../../lichen-vm/docs/notes/incremental-update.md), [heavy-value-cache](../../../lichen-vm/docs/notes/heavy-value-cache.md), [incremental-evaluation](../../../lichen-vm/docs/notes/incremental-evaluation.md)

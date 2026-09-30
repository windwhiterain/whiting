# The compile, not the render, is what costs

Status: **partly implemented**, partly open. The measurements and the three changes that came out of
them are below; the larger structural fix is still open and is stated at the end. Every number was
measured on an RTX 3060 through `wgpu`, with the harness in `crates/device/examples/render.rs`, which
reports the phases of a frame apart.

## The observation

A range of times over the dune field, each frame rendered from the previous frame's output rather
than from scratch:

| | t=0 | t=1 | t=2 | t=3 |
| --- | --- | --- | --- | --- |
| operations in the emitted delta | 0 | 6 | 6 | 6 |
| compile | 1724 ms | 2400 ms | 3690 ms | 3538 ms |
| render | 21 ms | 22 ms | 32 ms | 23 ms |

**The delta is six operations and the render is twenty-five milliseconds.** Six passes, 1.7 million
vertex invocations, three 1024² targets and a nine-tap shadow lookup per lit fragment: the gpu is
not the problem, and never was. Ninety-nine percent of the frame is the compile that *produces* the
delta.

## Why the delta does not help here

A delta is small. Producing it is not.

`document_delta` is given the previous output and the new one, and decides which paths changed. To
decide that a path did not change it has to look at it. There are 96×96×6 = 55,296 points in the
field, each a `Value::Struct` over a `BTreeMap<String, Value>` with three heap-allocated keys, and
the diff walks all of them, compares all of them, and finds six leaves different.

The delta then describes the change compactly, and `Delta::apply` is correspondingly cheap. So the
mechanism does exactly what it is for: **it makes applying a change cheap, and finding the change
expensive.** For an agent editing a document by hand, that is the right trade — an agent changes a
few paths and does not care that the compiler read a million. For a *sequence* of frames, where
nothing changes but one number, it is the wrong trade entirely, because the same million is read
once per frame.

## Where the time actually went

Instrumenting the compile rather than reasoning about it found three things, none of which was the
diff. On the warm frames, of 690 ms:

| | ms |
| --- | --- |
| `scene_value` — resolving the scene through the store | 200 |
| verifying every point against the mesh's schema | 92 |
| `pack_points` — growing a 1.7 MB buffer from empty | 28 |
| `bounds_of_instances` | 38 |
| verifying every point against the shader's schema, **per instance** | ~250 |
| schema validation of the emitted documents, and `document_delta` | < 5 |

The last two rows are the interesting ones. **The diff is free**, because the bulk of an emitted
document is one `Value::Bytes` of packed vertices, and comparing two of those is a `memcmp`.

## Three changes, all sound

  - **Point verification is per mesh, not per instance.** A point was verified against its mesh's
    own schema once, and then against the drawing shader's schema *once for every instance that
    draws it*. A field of three identical instances verified the same fifty-five thousand points
    three times over, and the answer is the same every time because it depends only on the points
    and the schema. The check is now done once per **distinct** point schema the scene's shaders
    declare, which is one per shader rather than one per instance.
  - **The packed buffer is sized from the schema before it is filled.** Every point occupies the
    same number of bytes, and the schema declares the widths, so the reservation is exact rather
    than a guess. Growing a 1.7 MB `Vec` from empty is a few hundred reallocations and a copy of
    everything written so far; writing into a sized one is a single allocation.
  - **The walk for a reference stops at a schema that constrains nothing.** `find_reference` was
    descending into every value it met, which for a mesh means every one of its points — and a
    point's schema is `Any`. A reference is *declared*: a slot is marked and the mark names the
    kind the target must have, so nothing beneath an unconstrained slot can be one, because there
    is no declaration down there to mark it. Stopping there is as much a correctness point as a
    speed one, and it removed two thirds of `scene_value`.

### What it bought

| | before | after |
| --- | --- | --- |
| compile | 690 ms | 310 ms |
| compile at 240 divisions | ~3.5 s | — |
| per frame, clone + validate + compile + render | 3690 ms | 368 ms |
| compile over a sequence | 690 → 3538 ms, growing | 307 → 313 ms, flat |

The growth across a sequence is gone as well as the size, which is the part that mattered: a range
of frames is a range of frames rather than something that gets slower the longer it runs.

## Still open

The remaining 310 ms is the honest cost of rebuilding everything from scratch every time, and
removing it needs the change this proposal originally pointed at.

### Do not re-read a value that has not changed

Every document the workspace holds is immutable once written, and the workspace already has an
identity for a value: a hot reload makes a new version and the old one stays alive while anything
refers to it ([SPEC.align](../SPEC.align.md), *Generations are immutable*). If the compile
compared **node identity** before it compared contents, an unchanged subtree would cost one pointer
comparison instead of a walk.

This is the change with the best ratio of work to payoff, and it is the one the existing design
almost already has. What it needs is a path from a `Value` to the node it lives in — the
[`Loaded`](../../crates/definition/src/loaded) generation store is exactly that — and a compiler
that reuses the nodes it read last time rather than building fresh ones. A document that is the
previous document with six leaves replaced then shares every other node, and everything above
becomes O(changed) rather than O(size).

It also makes the contract better rather than merely faster: "this delta changes six paths" would
become a fact about identity rather than about equality, and two documents that are equal but
distinct nodes would correctly produce a delta.

### Let bulk geometry live somewhere else

The deepest fix is the one the schema already allows and nothing uses. `meshes` is a reference slot
([`scene_schema.rs`](../../crates/scene/src/standard/scene_schema.rs)), so a mesh may live in a
definition of its own and be *named* by any number of scenes. A compiler that read such a mesh once
and carried the packed buffer forward, re-reading only when the mesh's own definition took a delta,
would make a scene with heavy geometry cheap to animate however many times — because the geometry
is not in the document that changes.

That is a real change to what a compiler is, and it is the one that scales to a real asset.

## What this is not

This is not an argument against the value tree. A tree is the right *interface* language for a
document an agent reads and edits, and the schema over it is the best part of the framework. It is an
argument that a tree is a poor *storage* format for a hundred thousand floats, and that the
mechanisms which make editing cheap — immutable generations, node identity, versioned deltas —
should be the ones that make reading cheap too.

## The number to beat

Ten milliseconds of render against three hundred and ten of compile, where a dune field is
fifty-five thousand points. An animation is still thirty times away from being limited by the thing
it is actually for.

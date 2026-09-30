# The compile, not the render, is what costs

Status: proposal, for review. Not implemented. Every number here was measured on an RTX 3060
through `wgpu`, with the harness in `crates/device/examples/render.rs`, which reports the two halves
of a frame apart.

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

The same shape appears in the other two places the pipeline re-derives rather than reads:

- `emit` re-packs every point of every mesh on every compile, even when the points are the same
  points.
- `read_instances` re-composes every model matrix, and now every normal matrix, on every compile.

## Three ways out, cheapest first

### 1. Do not re-read a value that has not changed

Every document the workspace holds is immutable once written, and the workspace already has an
identity for a value: a hot reload makes a new version and the old one stays alive while anything
refers to it ([SPEC.align](../SPEC.align.md), *Generations are immutable*). If `document_delta`
compared **node identity** before it compared contents, an unchanged subtree would cost one pointer
comparison instead of a walk.

This is the change with the best ratio of work to payoff, and it is the one the existing design
almost already has. What it needs is a path from a `Value` to the node it lives in — the
[`Loaded`](../../crates/definition/src/loaded) generation store is exactly that — and a diff that
walks the two documents *together* rather than the new one alone. A document that is the previous
document with six leaves replaced shares every other node, and a walk that notices that is O(changed)
rather than O(size).

It also makes the contract better rather than merely faster: "this delta changes six paths" would
become a fact about identity rather than about equality, and two documents that are equal but
distinct nodes would correctly produce a delta.

### 2. Cache the packed buffer against the points it came from

`pack_points` turns points into bytes, and the bytes are a function of the points. Memoised against
the node identity of the point list, an unchanged mesh packs once and never again — which removes
the packing from the per-frame cost without touching the diff.

### 3. Let bulk geometry live somewhere else

The deepest fix is the one the schema already allows and nothing uses. `meshes` is a reference slot
([`scene_schema.rs`](../../crates/scene/src/standard/scene_schema.rs)), so a mesh may live in a
definition of its own and be *named* by any number of scenes. A compiler that read such a mesh once
and carried the packed buffer forward, re-reading only when the mesh's own definition took a delta,
would make a scene with heavy geometry cheap to animate however many times — because the geometry
is not in the document that changes.

That is a real change to what a compiler is, and it is the one that scales to a real asset. Options
1 and 2 are both ways of getting most of the benefit without it.

## What this is not

This is not an argument against the value tree. A tree is the right *interface* language for a
document an agent reads and edits, and the schema over it is the best part of the framework. It is an
argument that a tree is a poor *storage* format for a hundred thousand floats, and that the
mechanisms which make editing cheap — immutable generations, node identity, versioned deltas —
should be the ones that make reading cheap too.

## The number to beat

Twenty-five milliseconds of render against three and a half seconds of compile. A frame is four
orders of magnitude away from being limited by the thing it is actually for.

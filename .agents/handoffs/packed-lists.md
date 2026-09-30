# Handoff: packed lists

Branch `feature/showcase-scenes`, worktree `.worktrees/showcase`. Follows on from
[proposals/compile-cost.md](../proposals/compile-cost.md), whose *Still open* section this
implements.

## What this is

A list whose elements all have one structure is stored as a flat run of bytes rather than as
`Value::Struct` over `BTreeMap<String, Value>`, one map per element. The uniform structure is
guaranteed by the schema: a list declares one element schema, and a struct is closed, so every
element has the same fields in the same order with the same widths.

`Value::Packed` is the new variant, holding a `PackedList`. It is framework-wide, not a mesh
feature: any list whose element schema is a struct of fixed-width numbers can be carried in it, and
every `match` on `Value` handles it. `Packed` and `List` are the **same kind** — `kind_name` reports
`"list"` for both and a `List` schema accepts either — so packing is a representation and never a
change of meaning.

## What it bought

Measured on an RTX 3060 through `wgpu`, release, `dunes` at 96x96 = 55,296 points.

| | before | after |
| --- | --- | --- |
| clone | 40 ms | 0 ms |
| validate | 9 ms | 0 ms |
| **compile** | **305 ms** | **10 ms** |
| render | 10 ms | 8 ms |

A frame went from being 97% compile to being split evenly. **All seven frames are byte-identical to
the pre-change baseline** (SHA-256 over the PNGs), so nothing about the picture moved.

The compile cost now scales with *bytes*, not with tree nodes: `monoliths`, at 182k points against
`dunes`' 55k, compiles in 32 ms against 10 ms — 3.3x the data for 3x the time, which is the linear
behaviour the change was for.

The five rows that accounted for ~275 of the 305 ms, and what each became:

| | was | is now |
| --- | --- | --- |
| `scene_value` — clone the document | 53 ms | a `memcpy` |
| verifying every point against the mesh's schema | **100 ms** | `len % stride == 0` |
| `pack_points` | 30 ms | a copy of bytes that are already packed |
| clone the point list again | 45 ms | nothing |
| `bounds_of_instances` | 43 ms | a strided read, no map lookup |

## The second proposal, and why it was dropped

"Do not validate anything in release mode" was implemented, measured, and then **reverted**.

It bought nothing. With validation on: 10, 10, 10, 10, 11, 11 ms. With it off in release: 10, 10,
10, 10, 10, 11 ms — both at the noise floor. The reason is the point above: the 100 ms *was* the
point verification, and packing made it O(1). The checks that remained were the ~2 ms ones, so the
gate cost three real contracts for no measurable gain:

- a bad document became a bad frame rather than an error naming the field that is wrong;
- `Delta::apply` lost atomicity against a *schema-breaking result* (it keeps it against a malformed
  operation, which the applier refuses itself);
- a server's `put` accepted documents its schema would have rejected.

`Schema::validate` therefore runs in every build, and its doc comment states the three contracts
that depend on it. **Packing is what makes validation affordable, not what makes it optional** —
that is the sentence to keep if this is ever reopened.

## The constraint that shaped the API

`Path::resolve` returned `&Value`. A byte run cannot hand out a reference to a decoded element, so
`Path::resolve` now returns `Resolved<'_>` — `Borrowed(&Value)` or `Owned(Value)` — with
`Deref<Target = Value>`, so most call sites are unchanged.

Mutation could not return a borrow into a temporary at all, so `Path::resolve_mut` is **replaced**
by `Path::update`, which decodes the element, hands `&mut Value` to a closure, and re-encodes.
`Value::resolve_mut` / `Value::at_mut` are gone; use `Value::update_at`. Editing
`points[42].position[1]` of a run of a hundred thousand points costs one point.

`MeshRead.points` is a `PackedList` and is read by nothing but `bounds_of_instances`, so the
compiler never needs the tree form of a mesh's points at all.

## Invariants

  - A packed list's byte length is always a whole number of strides. A partial element is a
    malformed list, not a short read, and `PackedList::new` refuses to build one.
  - Field order is schema order, which is sorted by name, which is the order
    `graphics::layout::vertex_layout` derives offsets from. `graphics::layout::packed_layout` and
    `vertex_layout` read the same schema the same way, so the compiler that packs and the renderer
    that draws cannot disagree about a stride.
  - Every component is four little endian bytes, the same rule a buffer already follows. A field of
    one component may be a bare number in a document; a field of several is a list. Reading back is
    the mirror of that rule, which is why a document round-trips.
  - A packed list round-trips: `to_list` of a `from_list` equals the original.
  - Packing is never done behind an author's back. Nothing expands a run to answer a question about
    its shape, because that would undo the reason for having it.
  - The layout is *not* compared against the element schema in `definition`, which is below the
    layer that knows what a field's width means. Whoever packs from a schema checks the two agree.

## What is deliberately not done

  - **A packed list has no element cache.** `Path::resolve` into one decodes every time, so a
    caller walking a whole run element by element pays a decode per element. That is the correct
    default — such a caller wants a tree and should call `to_list` — but it means the run is fast
    for *bulk* reads and not for random ones.
  - **`Loaded::resolve` refuses a path that goes *through* a packed index.** A run holds no nodes,
    so there is no node for `points[42].position`. `Value`-level `Path::resolve` and
    `Path::update` both handle it by decoding; the loaded graph does not. A reference *to* a
    packed list resolves to the run.
  - **Not every list is packed.** Instance rows, shader sources and pass lists stay trees, because
    three elements are not worth a run. The choice is per list and belongs to whoever writes it.
  - **The three gallery scenes now emit packed points**; a scene authored by hand may still write a
    tree of points, and the compiler packs it once on the way in.

## Where to look

  - `crates/definition/src/packed.rs` — `PackedLayout`, `PackedList`, `PackedBuilder`.
  - `crates/definition/src/path.rs` — `Resolved`, `Path::update`, `walk_owned`.
  - `crates/graphics/src/layout.rs` — `packed_layout`, and `value_bytes` appending a run whole.
  - `crates/scene/src/standard/emit.rs` — `read_mesh` packs once; `packed_point_position`.
  - `crates/scene/src/gallery/geometry.rs` — the generators write runs and never build a point.

## Order, for whoever picks this up

The build is green, `cargo test --workspace` passes (28 tests), and the change is uncommitted on
`feature/showcase-scenes`. Docs to sync before merging: SPEC.align (the `Value` and `Path` terms,
and the validation paragraph), and [proposals/compile-cost.md](../proposals/compile-cost.md) (its
*Still open* section is now answered — by this for the first of the two, and by making the second
moot for the dune field, since the geometry is no longer in the document that changes).

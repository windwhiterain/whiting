# Handoff: reference slots

Branch `feature/reference-slots`, stacked on `feature/pass-schema-standard-scene` (the
reference design needs the schema machinery that branch carries). The design is recorded in
[proposals/reference-slots.md](../../../../.agents/proposals/reference-slots.md).

## Decided

- A schema declares which slots may be a reference, and such a slot holds a **union**: a
  reference or a value of the declared kind. The declared kind describes both ends, so an
  inlined value must satisfy it and a reference must resolve to something that does.
- A reference is recognised by its tag, so a malformed reference is reported as one rather
  than as a kind mismatch, and a value that happens to look like a reference in an unmarked
  slot is just data.
- Traversal is transparent; re-pointing a slot is a `Replace` of the slot.
- A loaded version is a **generation**: an arena of nodes addressed by index, held behind one
  reference count, freed whole when its last holder drops, so a cycle is free and there is no
  collector. Generations are immutable, so a hot reload makes a new version and an old one
  lives while anything refers to it. `History` is a legitimate strong holder.
- The **sharing** of a node decides a `Replace`: a node another slot also names is written in
  place so every referrer observes it, and a node this slot alone names may be swapped.
  Sharing is a property of the node, not a live count, so a transient clone cannot change the
  answer.
- `PrimitiveDelta` has three variants, each acting on a union of the shapes a value can have.

## Done

- `reference.rs`: `Reference` (`Internal(Path)`, `External(Path)`, `File(FilePath)`), its
  value encoding and inverse, and `Reference::is_tag`. A path is written as a list of string
  keys and integer indices, which is unambiguous because a dict is keyed by strings and a
  list is addressed by position.
- `path.rs`: `Path::to_value` and `Path::from_value`.
- `schema.rs`: `Schema::reference()`, `is_reference()`, `without_reference()`, the union rule
  in `validate_at`, and the marker's round trip through a schema document.
- `delta.rs`: `PrimitiveDelta::{Replace, Append(Appended), Remove(Removed)}` with builders
  `append_values`, `append_entries`, `remove_indices`, `remove_keys`, and application with the
  inverse algebra unchanged. The scene's emitter needed no change because it used the
  builders.
- `loaded/`: the loaded generation. `Node`, `Slot` (a target plus the spelling it was written
  with), `NodeId`, `Loaded` with `resolve`, `is_shared`, `node`, and `to_value`. Loading reads
  the tree into a raw arena and then resolves references to a fixpoint, so a forward reference
  and a reference to an ancestor both work, and a cycle of references holding no value is an
  error. A reference's target is marked shared, which is what a patch consults.
- Doctests pin the union rule, the lossless round trip, a cycle through the root, and the
  value-less cycle.

## Next

1. **A store**, so an `External` or `File` reference can resolve. `LoadError::NoStore` is the
   seam: the loader needs something that hands it another definition's `Loaded` generation and
   reads a file, which is also where the client's root for relative paths belongs. Decide
   whether an external path's first segment names a definition.
2. **Patching a loaded generation**, and `Delta::apply` against it: the sharing flag decides
   whether a `Replace` writes in place or swaps the target, and a removal of a shared target
   needs a rule.
3. **`History` inverses recorded at the resolved location** rather than the spelling the
   forward delta used, or undo replays through a reference that may since resolve elsewhere.
4. **Packers and emitters** resolve before packing, since a GPU buffer cannot hold a path, and
   the graphics-to-instance link becomes a checkable reference edge rather than a `source`
   string that happens to match an entry key.

## Flags for the superior

- A `Remove` of indices is a contiguous range, so an arbitrary non-contiguous set is not
  representable. That is deliberate: the operations map holds one primitive per path, and no
  `Append` can invert a scattered removal. The general fix would be multi-position inserts on
  both sides, which is more vocabulary.
- `Append` carries a position, so a middle insertion is expressible and invertible.
- A loaded generation is a plain value; whether the client holds it in an `Rc`, behind a lock,
  or in a single owning task is the client's call, and the single-owner reading keeps the graph
  `Send`.

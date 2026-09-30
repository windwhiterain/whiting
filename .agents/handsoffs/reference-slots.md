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
- `loaded/`: the loaded generation. `Node`, `Slot` (a `Target` plus the spelling it was written
  with), `NodeId`, `Loaded` with `resolve`, `is_shared`, `node` and `to_value`. Loading reads
  the tree into a raw arena and then resolves references inside it to a fixpoint, so a forward
  reference and a reference to an ancestor both work, and a cycle of references holding no
  value is an error.
- `loaded/store.rs`: `Store`, `Origin` and `Wanted`. An external path's first segment names a
  registry definition and the rest addresses inside it; a relative file path is resolved
  against the document that spelled it, which is what `Origin` carries. A file wanted as an
  asset becomes the bytes it holds, and one wanted as a document becomes a generation of its
  own — which is what makes a path inside it relative to the file.
- `loaded/mod.rs`: `Target::{Local, Foreign}`, where a foreign target holds the generation it
  lives in, so a reference out of a document keeps its target alive.
- Doctests pin the union rule, the lossless round trip, a cycle through the root, the
  value-less cycle, the store contract (external definition, file as asset, spellings
  preserved), and a patch through a reference being visible to every referrer.
- `loaded/patch.rs`: `Loaded::patch`, which applies operations to this generation in place.
  A path is resolved transparently, so a patch through a reference writes the node the
  reference names and every referrer observes it, including a reference from another document.
  A path that arrives in another definition is refused, because a delta belongs to one
  definition at one version. Planning does everything that can fail — resolving paths, checking
  kinds and bounds, reading introduced content — so writing cannot fail and a patch is atomic.
  Inverses are keyed by the path they were applied at, which is correct because undo is a
  stack: a reference is restored before the change that ran through it is undone.
  The schema is what tells a patch which slots may hold a reference, so introduced content is
  read with the same rules a load uses (`field_schema`, `element_schema`, `variant_schema` and
  `spelled` are shared rather than repeated).

## Next

1. **A compiler that emits a reference.** The device resolves one and the instances it holds may
   name their payloads elsewhere, but no compiler emits such a name yet: the standard scene puts
   each mesh's bytes in the instance output, so a second scene does not share them. Emitting a
   definition per shared part is what the design was for, and it is a compiler change rather than
   a mechanism one.
2. **A cache, if the load per render matters.** A document that spells a reference is loaded per
   render call; the store is rebuilt each time. A document that names nothing is handed over as
   it is, so only a sharing document pays.
3. **`History` and the registry on the loaded form.** `Definition` still holds the serialized
   value and applies deltas to it, so `Delta::apply` and `Loaded::patch` now exist side by side;
   the wire path refuses a document that spells a reference, so the two cannot disagree silently,
   but the registry itself is still value-based.

## Flags for the superior

- **The graphics/instance edge is where the spec puts it**, on the instance
  (`referencing_graphics`), and the instance is now a struct of a name and a payload map rather
  than a bare map, because a dictionary schema carries one value schema and so had nowhere to put
  a name. An instance that does not say which graphics it belongs to fails validation, and a
  render of a pair that disagrees is refused.
- **A slot must be marked before a reference in it means anything**, which the device's own
  doctest proved the hard way: the entry schema was `Dict(Any)`, so an entry that spelled a
  reference was read as data and silently never resolved. Both instance schemas now mark their
  entry values as able to hold a reference. The same is true of a dictionary in general, where
  the loader had been reading entries as open rather than as the dictionary's value schema — a
  bug that only a document *using* a reference could have found.
- **A field whose only meaning is a pointer cannot be required to be one.** The union lets a
  marked slot hold a reference *or* a value, so a field that must be a pointer is only enforced by
  the reader. If that should be expressible in the schema, it needs a marker that *requires* a
  reference alongside the one that permits it.
- **A delta's paths only mean what they say on a loaded generation**, so `Delta::apply` refuses a
  document that spells a reference and the loaded applier is the authoritative one.
- **Definitions refer to each other in a graph, not in a cycle** — confirmed, and the store in the
  device enforces it by refusing a definition already being loaded.
- **A delta mutates the live generation in place** — confirmed, so the sharing flag is a
  diagnostic and a packer hint rather than a correctness input.
- A `Remove` of indices is a contiguous range, so an arbitrary non-contiguous set is not
  representable, because the operations map holds one primitive per path and no `Append` can
  invert a scattered removal.
- A loaded generation is a plain value; whether the client holds it in an `Rc`, behind a lock, or
  in a single owning task is the client's call, and the single-owner reading keeps the graph
  `Send`.

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

1. **The material's custom data never reaches the gpu.** The emitter verifies it against the
   shader's custom schema and then drops it: the instance row is the model matrix and nothing
   else. Since the material carries it and a material belongs to an instance, it is per-instance
   data, so it belongs in the instance row — which is also what makes it work under batching,
   where a uniform is read by every instance of the pass. This is the next thing to write.
2. **A compiler that emits a reference.** The device resolves one and an instance entry may name
   its payload in another definition, but no compiler emits such a name yet: the standard scene
   puts each mesh's bytes in its own output, so a second scene shares nothing.
3. **A cache, if the load per render matters.** A document that spells a reference is loaded per
   render call; one that names nothing is handed over as it is.

## Flags for the superior

- **The renderer already draws instanced, and the compiler now feeds it.** A pass binds one
  vertex buffer and one instance buffer, and the draw is `draw(0..vertices, 0..rows)`, so
  instances whose passes would be identical now share one pass: the same stage, shader, mesh and
  light. The example scene gained a second lighting instance to demonstrate it — one pass, one
  buffer, two rows — because its original three instances were in three stages and could never
  merge. The consequence for the format is that anything varying per instance must be per-instance
  *data*, which is where the material's custom data belongs and where a uniform cannot.
- **The instance carries the payloads under `entries`, and names its graphics**, because a
  dictionary schema had nowhere to put a name. A pair that disagrees is refused rather than
  rendered.
- **A slot must be marked before a reference in it means anything**, and a dictionary's entries
  are read with the dictionary's value schema rather than as open — a bug that only a document
  using a reference could have found.
- **A field whose only meaning is a pointer cannot be required to be one.** The union lets a
  marked slot hold a reference *or* a value, so a pointer field is only enforced by the reader.
- **A delta's paths only mean what they say on a loaded generation**, so `Delta::apply` refuses a
  document that spells a reference and the loaded applier is the authoritative one.
- **Definitions refer to each other in a graph, not in a cycle** — confirmed, and the device's
  store enforces it.
- **A delta mutates the live generation in place** — confirmed.
- A `Remove` of indices is a contiguous range, so an arbitrary non-contiguous set is not
  representable.

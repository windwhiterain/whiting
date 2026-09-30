# Reference slots: design record

Status: design settled, implementation started against it. Written against the spec revision
where a value slot is inlined by default and may be a `Reference` (`InternalReference` by
`Path`, `ExternalReference` by `Path`, `FilePath`), every reference resolves to a Rust
reference at load, `FilePath` is absolute or relative and holds a definition or an asset, and
the schema decides which slots are references.

## Decided

- **A schema declares which slots are references**, and the value at such a slot carries only
  the spelling. This is what keeps the runtime fast: validation checks the reference itself
  rather than dereferencing, the loader has an exact work list instead of scanning for
  markers, an inlined slot stores its value with no runtime tag, and a packer knows whether to
  follow the indirection or read inline.
- **Traversal is transparent.** Two slots naming one value are aliases, so a change through
  either is a change to the value, and re-pointing a slot is a `Replace` of the slot.
- **Resolution is eager and to refcounted references.** In memory every slot resolves to the
  same representation, so "inlined" versus "referenced" is a serialization distinction rather
  than a runtime one.
- **A loaded version is a generation: an arena of nodes addressed by index, held behind one
  reference count.** References inside it are indices, so a cycle is free and a leaked cycle is
  impossible: when the last holder drops, the arena is freed whole and there is no collector to
  write. A long lived generation that churns is the only case that wants an intra-generation
  mark and sweep, which is one arena with no cross-generation edges.
- **Generations are immutable.** A reload makes a new version, and an old version stays alive
  while anything refers to it, so a reader sees a consistent version rather than a stale or
  half-updated one, and hot reload needs no invalidation pass. `History` is a legitimate strong
  holder, which is what undo needs.
- **The sharing of a node decides how `Replace` is applied**: a node another slot also names is
  written in place so every referrer observes the change, and a node this slot alone names may
  be swapped. Sharing is a property of the node — a flag set when a reference resolves to it —
  rather than a live count, so a transient clone during traversal cannot change the answer.
- **Validation uses one rule for both forms.** A marked slot validates its reference spelling,
  and the loader validates the resolved target against the same schema with the marker
  ignored, so a locally valid document can still fail to load when a target is missing, and
  that failure is its own error class rather than a schema failure.
- **`FilePath` holds a definition or an asset**, absolute or relative to the referencing
  document's root, and the slot's schema says which of the two it must produce: a bytes schema
  wants the raw asset, a structured schema wants a parsed definition.

## Still open

1. Whether a marked slot may *alternatively* hold an inlined value, which would need a union
   and put a tag back on that field. The fast reading, and the one this record assumes, is that
   a marked slot is always a reference.
2. Whether the reference kind belongs in the schema as well as the value: the schema currently
   says only "a reference to an `S`", leaving `Internal`, `External` or `File` to the value.
3. `Append` carrying a position; multiple `Remove` indices simultaneous against the pre-removal
   list; dict `Append` refusing a present key; `Remove` on a struct left to validation. The
   defaults in the plan hold unless overridden.

## Cost in the code

- `definition` gains the reference spelling, the `Path` value encoding, the schema marker and
  the validation branch; `PrimitiveDelta` goes from five variants to three.
- The loaded generation, its loader and writer are new, and `Delta::apply` gains the sharing
  decision.
- Packers and emitters resolve before packing, since a GPU buffer cannot hold a path.

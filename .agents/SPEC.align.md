# Agentic Creative Content Creation Framework

rust multi-crates workspace, each crate has publish name prefixed with "whiting" while in-workspace name use the un-prefixed one.

> This is a proposed alignment of [SPEC.md](SPEC.md) for human review and merge.
> It records the decisions taken during spec alignment and fills in the terms the
> original spec left undefined.

## Principles

- The domain has **no Rust types**. `Graphics`, `Pass`, `Binder`, and `Scene` are
  schema documents and dynamic values, never `struct` or `enum` definitions.
- A **schema can change at runtime without recompiling rust**. Schemas are data.
- Agents read and write documents; every other layer is a pure function of them.

## Dependencies

- `definition`, `graphics`, `scene` depend on `facet` for `(de)serialization only`.
  Schema validation does not use `facet` reflection.
- `device` additionally depends on `axum` with `facet-axum`, `tokio`, and `wgpu`.
- the `facet` stack is pinned to the `0.44` line: `facet 0.44`, `facet-json 0.44.7`,
  `facet-axum 0.44.0`. `facet-axum` has no stable release past `0.44.0`, so this is the
  newest combination in which all three crates are stable and mutually compatible.
- `wgpu` is an optional feature of `device` so the workspace builds and tests on
  machines with no gpu adapter.

## Terms

### `definition` crate

- `Value`: the leaf-and-container value tree. `.ron`-shaped.
  - enum of:
    - `Unit`
    - `Bool(bool)`
    - `Int(i64)`
    - `Float(f64)`
    - `String(String)`
    - `Bytes(Vec<u8>)`
    - `Struct`: ordered map of named fields to `Value`.
    - `Enum`: a variant tag plus a map of named fields to `Value`.
    - `List`: ordered sequence of `Value`.
    - `Dict`: ordered map of string keys to `Value`.
  - `Struct` and `Dict` are distinct: `Struct` is closed by its `Schema` and
    validates field presence, `Dict` is open.
  - `Packed(PackedList)`: a list whose elements all have one structure, carried as a flat
    run of bytes, one stride per element, described by a `PackedLayout`.
    - **decision**: `Packed` is the *same kind* as `List`, not a kind of its own.
      `kind_name` reports `list` for both and a `List` schema accepts either, so packing
      is a representation and never a change of meaning. A document that a reader cannot
      tell apart is a document an agent can edit with the same tools.
    - **invariant**: a packed list's byte length is always a whole number of strides. A
      partial trailing element is a malformed list, and `PackedList::new` refuses to build
      one, so no accessor ever has to cope with reading past the end. This is what makes a
      run indexable by offset arithmetic at all.
    - **invariant**: field order is schema order, which is sorted by name, which is the
      order `graphics::layout::vertex_layout` derives offsets from. `packed_layout` and
      `vertex_layout` read the same schema the same way, so a compiler that packs and a
      renderer that draws cannot disagree about a stride.
    - **invariant**: every component is four little endian bytes — the rule a buffer
      already follows. A field of one component may be a bare number in a document and a
      field of several is a list; reading back is the mirror of that rule, which is why a
      document round-trips.
    - **invariant**: packing is never done behind an author's back. Nothing expands a run
      to answer a question about its shape, because that would undo the reason for having
      one. The choice to pack is the writer's, per list.
    - **why it exists**: a tree is the right *interface* language for a document an agent
      reads and edits, and a poor *storage* format for a hundred thousand floats. Fifty-five
      thousand `Value::Struct` values over `BTreeMap<String, Value>`, each with three
      heap-allocated keys, is walked per pass; a run of `N * stride` bytes is one
      allocation that a `memcmp` compares and a strided read indexes. The uniform structure
      is guaranteed rather than hoped for: a list declares one element schema and a struct
      is closed, so every element has the same fields, order and widths. A list that does
      not satisfy that cannot be packed and is a `List` instead.
    - **not done**: a run has no element cache, so a path walk into one decodes per element.
      That is the right default — such a caller wants a tree and should say so — but it
      means a run is fast for bulk reads and not for random ones.
- `Schema`: a schema document used to verify a `Definition` and the `Delta`s
  referencing it.
  - a recursive node of:
    - `kind`: a `SchemaKind` describing the shape.
    - `attributes`: constraints and metadata, see `Attributes`.
    - `documentation`: optional prose.
  - `SchemaKind` enum of:
    - `Unit`, `Bool`, `Int`, `Float`, `String`, `Bytes`
    - `Struct`: a map of field name to `Schema`.
    - `Enum`: a map of variant tag to a map of field name to `Schema`.
    - `List`: an element `Schema`.
    - `Dict`: a value `Schema`.
    - `Any`: accepts any `Value` of the container kind expected at that position.
  - **invariant**: a `Struct` is closed. A field present in the `Value` but absent from
    the `Schema` is an error. A `Dict` is open, which is the distinction that lets an
    agent add entries without editing a schema. `Int` and `Float` are distinct: an
    `Int` value does not satisfy a `Float` schema.
  - **decision**: the schema of an `Enum` variant is a `Struct`, never another `Enum`. A
    variant payload in this language is a struct of named fields, so a spec written as an
    enum of an enum has nowhere to put the inner tag. The vocabulary still nests — a
    `MeshShader` is a shadowing, depthing or lighting shader — but the document says so
    with the tag alone, which is what the value tree can carry.
- `Attributes`: per-node constraints and metadata.
  - `optional`: the field may be absent.
  - `default`: value substituted when the field is absent.
  - `extensions`: open map for domain-specific constraints.
- **decision**: a `Schema` is a document, so a `Schema` must be embeddable in a `Value`.
  A shader declares the interface of the data it reads, and a mesh declares the shape
  of its points; both are schemas living inside another document rather than a
  separately registered name. `Schema` therefore converts to and from `Value`, and the
  conversion is the same `.ron`-shaped language as every other document.
  - `fn Schema::to_value(&self) -> Value`: the schema written as a document.
  - `fn Schema::from_value(value: &Value) -> Result<Schema, SchemaError>`: reads it back.
  - `SchemaError`: a value is not a readable schema document. This is a read error, not
    a `ValidationError`, because it is the *schema* that is malformed rather than the
    value being wrong.
  - **invariant**: the round trip is exact. `Schema::from_value(&schema.to_value())`
    equals the original schema, so a document may be carried, stored and sent without
    losing anything.
- `DefinitionIdentifier`: unique name of a `Definition` in a `Registry`.
- `SchemaIdentifier`: unique name of a `Schema` in a `Registry`.
- `Version`: monotonically increasing counter of applied `Delta`s on a `Definition`.
- `Definition`: a versioned data value that references a `Schema`.
  - `definition_identifier: DefinitionIdentifier`
  - `version: Version`
  - `schema_identifier: SchemaIdentifier`: references a `Schema` held by the `Registry`.
    It is a reference, not an embedded copy, so one `Schema` is stored once no
    matter how many `Definition`s and `Delta`s use it.
  - `value: Value`
  - `fn validate(&self, registry: &Registry) -> Result<(), ValidationError>`:
    resolves `schema_identifier` then checks `value` against it.
- `Path`: the chain of keys and indices that accesses a certain value in a `Definition`.
  - a sequence of segments, each either a `String` key or a `usize` index.
  - invariants: an index segment only addresses a `List`; a key segment only
    addresses a `Struct`, `Enum`, or `Dict`.
  - `Resolved<'value>`: what a path answers with, either `Borrowed(&Value)` or
    `Owned(Value)`, with `Deref<Target = Value>`.
    - **decision**: a path walk returns this rather than `&Value`. A packed list carries a
      run, and an element of it has to be decoded to be a `Value`; a decoded element is a
      temporary with no address in the tree, so a walk that reached into a run cannot hand
      back a borrow of it. Telling the two cases apart in one type, rather than at every
      call site, is what keeps the readers reading like the borrows they usually are.
    - **invariant**: reaching into a packed run decodes exactly one element, so the cost of
      a path is bounded by an element rather than by the length of the list it names.
  - `Path::update(&mut Value, FnOnce(&mut Value) -> R)` **replaces** `resolve_mut`.
    - **decision**: an edit into a run decodes the element, hands it to the closure, and
      encodes the result back, because a borrow of a decoded element could not outlive the
      call. Doing it through a closure is what lets the two representations share one edit
      path: a caller writes the same code whether the value it is changing is a map in a
      tree or a stride in a run. The edit runs at most once, and a path that does not
      resolve never runs it.
    - `Value::resolve_mut` and `Value::at_mut` are removed with it; use `Value::update_at`.
- `PrimitiveDelta`: a change to the value a `Path` selects.
  - enum of:
    - `Replace`: schema must be the same with the `referencing_value`.
      - can replace a structural value.
    - `Append`: values to insert into a `List` at a position.
      - `index` is the position the values are inserted before, so `index == length`
        is a tail append. An `index` beyond the length is rejected.
    - `AppendEntries`: key-value pairs to append to a `Dict`.
    - `RemoveIndices`: a contiguous range of a `List` to remove, as `start` and `count`.
    - `RemoveKeys`: keys to remove from a `Struct` or `Dict`.
  - **invariant**: `Append` and `RemoveIndices` each describe a contiguous range, so
    they are exact mutual inverses and every list mutation inverts minimally. An
    arbitrary non-contiguous set of indices is deliberately not representable, because
    no append primitive can invert it.
  - an indexed `Append` shifts every later index and therefore invalidates the `Path`s
    of other `Delta`s. This is safe because a `Delta` carries the `version` it was
    computed against and is rejected as a conflict once that version is stale, so a
    shifted list can never be mutated by a delta that assumed the old indices.
  - the applier rejects an out-of-bounds `index`, an out-of-bounds `count`, and a
    `Path` that does not resolve.
- `Delta`: how to change a `Definition` that a `PrimitiveDelta` references.
  - `definition_identifier: DefinitionIdentifier`: the `Definition` this delta applies to.
  - `base_version: Version`: the `version` this delta was computed against.
  - `operations`: a map from `Path` to `PrimitiveDelta`, a sparse subset of the
    `referencing_definition` where each value is a `PrimitiveDelta` to its
    `referencing_value`.
  - a `Delta` is not a `Definition`, for simplicity.
  - `fn apply(&self, definition: &mut Definition, registry: &Registry) -> Result<Delta>`:
    applies the delta and returns the inverse `Delta` that undoes it.
    - the inverse is produced from values observed **at apply time**, so `RemoveIndices`
      and `RemoveKeys` can return the removed data.
    - **invariant**: list inverses are exact and minimal. The inverse of
      `Append { index, values }` is `RemoveIndices { start: index, count: values.len() }`,
      and the inverse of `RemoveIndices { start, count }` is
      `Append { index: start, values }` carrying the removed values. A whole-list
      `Replace` is never needed to undo a list mutation.
    - apply is atomic: a `Delta` either applies completely and leaves the `Definition`
      at `version + 1`, or leaves it untouched. This is achieved by applying to a
      clone, validating the clone against its `Schema`, and only then committing.
- `History`: a `Delta` replay system that generates an inverse `Delta` for each `Delta`
  applied, to help building an undo-redo system.
  - `entries`: an ordered log of applied deltas paired with their inverses.
  - `fn record(&mut self, applied: Delta, inverse: Delta)`
  - `fn undo(&mut self, definition: &mut Definition, registry: &Registry) -> Result<Delta>`
  - `fn redo(&mut self, definition: &mut Definition, registry: &Registry) -> Result<Delta>`
  - invariants: `undo` pops the newest entry and applies its inverse, pushing that
    inverse onto the redo log; `redo` is the mirror. The two logs are never both
    non-empty, because a new `record` clears the redo log.
- `Registry`: the store of `Schema`s and `Definition`s.
  - resolves `SchemaIdentifier` to `Schema` and tracks the `Version` of each `Definition`.
  - schema and definition changes take effect at runtime with no recompilation.
- `ValidationError`: a `Value` does not satisfy its `Schema`.
  - **validation runs in every build.** It is not a `debug_assertions` check, and the
    three contracts below depend on that:
    - a bad document is an error naming the path that is wrong, rather than a bad frame an
      agent has to diagnose from a picture;
    - `Delta::apply` is atomic against a schema-breaking result and not only against a
      malformed operation, because it validates a clone before committing it;
    - a server's `put` rejects a document at the boundary rather than storing one no schema
      admits.
  - **measured and rejected**: gating validation on `debug_assertions` was implemented and
    measured at *zero* gain — 10, 10, 10, 10, 11, 11 ms with it on against 10, 10, 10, 10,
    10, 11 ms with it off, both at the noise floor. The 100 ms that made validation look
    expensive was per-point verification, and packed lists made that `len % stride == 0`.
    **Packing is what makes validation affordable, not what makes it optional.** See
    [proposals/compile-cost.md](proposals/compile-cost.md).
  - a packed list satisfies `List { element }` in constant time, so a packed mesh is
    still verified rather than trusted.
  - **not validation**: packing, unpacking, path resolution and delta inversion are
    checked in every build regardless, because a run is read by offset arithmetic and its
    whole-number-of-strides invariant is what makes it readable.
- `ConflictError`: a `Delta` was computed against a `Version` that is no longer current.

### `graphics` crate

provides a schema of `Definition` that represents an instruction set of lowlevel
graphics computation, as data.

- `Graphics`: a `Definition` in the graphics schema.
  - `instance_schema`: a `Schema`, authored independently and deliberately
    permissive so an agent can read and edit it easily.
  - `Pass`:
    - enum of:
      - `Rasterize`
      - `Compute`
    - `uniform: Uniform`
    - `shader: Shader`
    - `binders`
    - `binders`: an open dict of binding point name to a `Binder` declaration, holding
      everything besides the uniform that the pass binds.
    - **decision**: the consistent part is one struct, the varying part is the enum. A
      pass is an enum whose variants each carry a struct built by merging the shared
      fields with that variant's own fields. The shared fields are declared once and
      merged in, so a field common to every pass is added in one place.
      - shared fields: `label`, `uniform`, `shader`, `binders`.
      - `Rasterize` adds `targets` and `clear_color`.
      - `Compute` adds `workgroup` and `entry_point`.
      - **invariant**: a pass kind that is not in the enum fails validation as an
        unknown variant, so a typo in a tag can never be mistaken for a pass.
    - `uniform`: the uniform data of the pass, as a `Value`. The renderer uploads it
      verbatim; its shape is the shader's business, not the graphics schema's.
    - `shader`: the name of the instance entry holding the pass's shader. A rasterize
      pass binds one shader that must expose both the vertex and the fragment entry
      point, so there is no separate fragment binder name.
    - `targets`: the binding point names of the colour attachments, in attachment order.
      A target is a binding like any other, so its format and its size live with the
      payload in the instance rather than being written twice.
    - **invariant**: a pass carries declarations, never payloads. A `Binder` names the
      instance entry its data comes from, and the size and format it needs; the bytes
      themselves live in the instance. That is what keeps a graphics document cheap to
      diff and an instance document cheap to reload.
  - `Binder`: the declaration a pass binds, naming the instance entry its data is in.
    - enum of:
      - `Resource`: GPU resources.
        - enum of:
          - `Buffer`
          - `Texture`
          - `TextureArray`
        - each carries `source`, the instance entry the payload is in.
      - `Uniform`: uniform data, carried by `source`.
      - `Shader`: verify schema against `uniform` and `binders`.
    - **decision**: `Texture` and `TextureArray` are two kinds, not one kind with a layer
      count. `texture_depth_2d` and `texture_depth_2d_array` are two WGSL types and a
      `textureLoad` on an array takes a layer index a flat texture has nowhere to put, so a
      document whose payload reads `Texture` while meaning an array names something no
      shader can read, and a reader of that document cannot tell which it holds. A reader
      *can* tell a flat texture from an array of sixty four, so the document says which,
      which is the same argument the light kinds made. The count is required rather than
      defaulted, so there is no absent case for a reader to interpret.
    - **decision**: the layer a pass draws into is a field of the `TextureArray`
      *declaration*, not of the payload. A pool is one texture with many layers and one
      pass per light: `layers` says how many the image has and `layer` says which of them
      this pass writes, and every other pass that binds the same entry still sees the whole
      array. A payload naming the layer would name one page for the whole image, which is
      the addressing a pool exists to replace. A `Texture` declaration has no `layer` at
      all, because a flat texture has no layer to choose between.
    - **decision**: how many layers is a payload field and which one is a declaration
      field, so the two halves of an array's addressing sit on opposite sides of the
      binding. They answer different questions — the first is a fact about the image, the
      second a fact about the pass — and only the second is per pass.
    - **decision**: the shader's `schema` lives on the instance entry, not in the
      graphics document. The interface travels with the shader that declares it, the
      pass keeps to naming its shader, and the instance schema is open so the new field
      needs no schema edit.
    - an instance entry of kind `Shader` carries `path`, `entry_point` and `schema`,
      where `schema` is a schema document declaring the shape the shader reads.
    - **invariant**: the declared kind and the kind of the entry it names must agree, and
      a payload must be at least as large as the declaration reads. A `Buffer` declared
      over a texture, or a read past the end of a buffer, would otherwise bind
      successfully and produce nonsense. This is what forces a pass that binds a pool to
      declare a `TextureArray`, rather than a `Texture` that quietly resolves to one.
- **decision**: the interface check compares a texture's **shape**, not only its resource
  kind. `ResourceKind::Texture` carries the shape, the layout entry's `view_dimension` is
  read rather than discarded, and `naga`'s image `dim` and `arrayed` are read rather than
  discarded, so a shader declaring `texture_depth_2d_array` where the pass bound a flat
  texture is a render error naming the pass, the variable and both sides. Before this there
  was one texture shape and the check's invariant that a resource kind is compared rather
  than a resource's shape was sufficient; the moment a second shape exists it is not,
  because the shape is part of the *type* a shader names rather than a property of its
  size.
  - the previous invariant stands for buffers. A buffer is a buffer whether it is a uniform
    or a storage buffer, and a shader that asks for a storage buffer where the pass bound a
    uniform still passes the check and fails in the backend. That gap is recorded, not
    fixed, because the address space is a level of access rather than a different type.
  - **invariant**: a shape the binding model never builds is left alone on both sides. A
    one dimensional image is a WGSL type no document here declares and the device never
    creates, so there is nothing to compare and forcing it into another shape would be a
    lie in the other direction.
  - the comparison is a parsed module and a list of layout entries, with no device in it, so
    it is checkable on a machine with no adapter. That is the honest form of a test for it.
- `verify_pass_inputs`: resolves a pass's `shader` entry and checks the pass's
  `uniform` and `binders` against that shader's `schema`.
  - the pass writes its binding points as an open dict, because the set of points is the
    pass's own business, and the interface writes them as a struct, because a shader needs
    to name each one and a dict carries a single entry schema. This check is where that
    difference is bridged, and it is the only place.
  - a shader with no `schema` is unchecked, which keeps an instance editable before a
    shader has declared anything.
  - a failure is a binding error rather than a `ValidationError`, on the same grounds as
    a missing binder: both documents validate against their own schemas and still fail
    to agree with each other.
- `derived_instance_schema`: the schema a graphics document implies for its instance,
  built from the binders its passes declare.
  - **invariant**: an entry name bound by two passes must be declared the same way by
    both. A disagreement fails the derivation rather than widening to `Any`, because a
    silent widening would turn an author mistake into an unchecked binding.
- **decision**: the vertex layout is derived from the point schema, and is not declared
  again. A field of the schema is an attribute, a list field is as many components as it
  declares in a `components` extension, and the fields are laid out in schema order, which
  is sorted by name. A field wider than four components is split into consecutive
  attributes of four, which is how a matrix is read. The compiler that packs and the
  renderer that draws both read the layout from
  [`layout`](../crates/graphics/src/layout.rs), so they cannot disagree about a stride.
  - **invariant**: every component is four little endian bytes. A buffer was packed at
    eight bytes per component while the layout said four, which every stride and every
    draw then disagreed about; the check that caught it was rendering a triangle on a real
    adapter, not any schema validation.
  - **invariant**: a field's `components` extension is what the layout reads, so a bare
    `List` of floats has no layout and a buffer packed from it cannot be drawn.
- **decision**: the uniform layout is the shader's own interface, not a second
  declaration. The shader declares what it reads, the pass supplies it, and the renderer
  lays the value out by that schema: a three component vector starts on a sixteen byte
  boundary and a matrix on one too, with zeroed padding between fields.
  - **invariant**: a schema sorts its fields by name, so a shader declares the fields of
    its uniform in that order and the two lay out the same. This is the rule a shader
    author has to know, and getting it wrong is a render that reads the wrong bytes rather
    than an error.
  - a shader that declares no interface has declared no layout, and its uniform is packed
    tightly.
- **decision**: a vertex buffer is not a binding. A buffer whose usage is `vertex` or
  `instance` becomes a vertex buffer, and its attribute locations continue from where the
  previous one's ended; per vertex buffers come first and per instance buffers follow, so
  a shader reads its point at the low locations and the instance data above. Every other
  buffer and every texture is a binding, numbered from one in name order, with the uniform
  at binding zero.
  - **invariant**: the number belongs to the **binding point**, not to what the pass does
    with it. A point the pass renders into takes its number and emits no layout entry, so a
    pass that makes one of its own binders a target renumbers nothing. The earlier rule
    counted only the resources a pass *bound*, which made a pass's `targets` an input to
    every shader's numbering: naming the depth buffer as a target moved the environment map
    one number down, into the slot the lighting shader had reserved for the depth buffer.
    That is the defect recorded in
    [proposals/lighting-pass-has-no-depth-test.md](proposals/lighting-pass-has-no-depth-test.md).
- **decision**: a buffer whose usage is `storage` is a binding bound as a storage buffer,
  and one that also declares `storage_write` is bound so a shader may write it. The
  writable case is a **second word rather than a third kind**, because it is a permission on
  a storage buffer and not a kind of buffer: a uniform at sixteen kibibytes is the only
  other binding a buffer can be, and thousands of light records do not fit in it.
  - **invariant**: a read only and a writable storage buffer are created with the same
    `STORAGE` usage, because a backend tells them apart at the bind group and not at
    creation. One buffer therefore serves a compute pass that writes it and a later pass
    that reads it, which is the whole point of the word existing.
  - **invariant**: one instance entry is one gpu resource for a frame, for a buffer exactly
    as for a texture. A compute pass that writes a storage buffer and a rasterize pass that
    reads it must share the buffer, or the reader sees the bytes that were uploaded and not
    the bytes that were written. A per frame cache keyed by the instance entry is what makes
    that hold, and it is the buffer analogue of the texture cache, not an optimisation.
  - **invariant**: an unrecognised usage word is an error, not a uniform. The list is
    unconstrained by schema, so a misspelled `storage` validates and would otherwise bind a
    uniform: a frame that is quietly wrong rather than a message naming the entry. An
    *absent* usage list is still a uniform, so no document changes meaning. The same
    position the module already takes for an unfilterable format and for mip levels.
  - **invariant**: one entry cannot be two kinds of buffer within a frame. A backend grants
    a buffer its usages once, so an entry that is a uniform in one pass and a storage buffer
    in another is a document asking for two buffers under one name, and it is reported
    rather than served by quietly making a second one.

- **decision**: a texture whose payload declares `sampled` takes a sampler at the binding
  after its own, and a texture that declares nothing takes nothing. The need is **declared,
  never inferred from the format**: the post stage reads a colour texture with
  `textureLoad` and wants no sampler, so inferring one from the format pushes the depth
  buffer from binding two to three and stops every scene's pipeline from building. The key
  is a unit, because its presence is the whole declaration and a boolean would give
  `sampled: false` a meaning. The numbering is computed once, in `Bindings::binding_numbers`,
  and both the layout and the bind group read it, because a rule written twice is a rule
  that can be applied once.
- **invariant**: `sampled` on a depth format is an error, and so is `sampled` on a format
  that cannot be filtered. A filtered depth comparison is a comparison sampler, which is a
  different resource from the one this binding model has; and the device requests
  `Features::empty`, so `R32Float` and `Rgba32Float` genuinely cannot be filtered here. A
  document that asks for a linear filter of a float lookup table is refused by name rather
  than silently given a blocky one.
- **invariant**: a texture payload that carries `data` is uploaded, and the level chain its
  size implies is built by a two by two box filter in linear light — an sRGB channel is
  decoded before it is averaged and re-encoded after. A payload with no `data` gets one
  level, because a pass is about to write its base level and a chain built here would be
  stale from the first frame. Levels are written with `write_texture`, which stages
  internally, so a row need not be a multiple of `COPY_BYTES_PER_ROW_ALIGNMENT`.
- **invariant**: a binding point that is one of a pass's targets is an attachment, not a
  binding. A backend treats an attachment as an exclusive use of the texture, so a pass
  that both wrote and sampled one would be rejected. The declaration is how a pass says
  what it renders into, so the same point serves as the attachment. An attachment is
  filtered by nothing whatever its payload says.
- **invariant**: a texture whose format is a depth format is declared to a shader with the
  depth sample type, not the float one, and a pass whose targets are all depth formats is
  built with no fragment stage. A bound colour texture is filterable exactly when its
  payload asked for a sampler, so the sample type follows the texture's *role* rather than
  its format.
- **known gap**: `sampled` is a property of the payload and not of the declaration, so
  `graphics::verify_pass_inputs` cannot check it — a shader's declared interface says it
  reads a texture and carries no opinion on how it is read. `device`'s `check_bindings`
  compares the two against the layout it actually built, and reports a disagreement as a
  message naming the pass and the shader's own variable.
- **decision**: the scene's `Mesh` declares its `Topology`, as the spec says, and the
  compiler carries it into the packed buffer: a vertex buffer declares how its points are
  read, in a `topology` on the payload rather than a field on the pass. The topology belongs
  to the run of points rather than to the instruction, and a buffer that does not say is
  read as triangles, so a `Segments` mesh is drawn as lines rather than as a triangle fan.
- **decision**: a document is a graph of references, not a tree of owned values, and
  ownership is decided when a document is serialized rather than while it is held. A part
  names another part by label, and the emitted documents keep that shape: a mesh
  contributes one vertex buffer however many instances draw it, and a shader one entry
  however many materials name it. Emitted entry names are checked, because two labels that
  generate one name would silently overwrite each other.
- **decision**: a shader carries its source, and no part carries a path. An instruction has
  to be able to travel to whoever runs it, and the device renders over RPC: a document
  naming a file on the author's disk could not be drawn by a server that cannot see that
  disk. Nothing in the framework reads the filesystem, and `WgpuRenderer::new` takes no
  shader root. An author who keeps shaders in files resolves that in their own tooling,
  which is the only place the file's existence is known.
- **decision**: the scene declares its shaders once, and a material names one by label, the
  same way an instance names a mesh. The shader's stage comes from the shader, so a
  material's stage is whatever the shader it names belongs to, and a label no shader
  carries is a compile error rather than a pass that draws nothing.
- **decision**: the `Shader` binder carries an interface and no entry point. A rasterize
  shader exposes `vertex_main` and `fragment_main`, which is the backend's convention rather
  than data, and the entry point a compute pass invokes is already a field of the compute
  pass. The shader payload used to carry a second, unused `entry_point` that said `main`
  while the renderer called `vertex_main`.
- `GraphicsInstance`: a `Definition` that provides data to its `referencing_graphics`
  in the corresponding schema.
  - `referencing_graphics`: the `Graphics` this instance serves, by `DefinitionIdentifier`.
- Binder resolution failure: because `instance_schema` is authored independently of
  the binder names a `Pass` uses, a missing or ambiguous binder is **not** a
  `ValidationError`. It is a distinct `BindingError` raised by the renderer when it
  builds a pass. `Graphics` and `GraphicsInstance` can each validate against their
  own schema and still fail to bind.
- `outputs`: an optional `Dict` of the graphics root, mapping the **name** a caller looks an
  image up under to an **output declaration**. The frame is the one named `present`.
  - a declaration is a closed `Struct` of three fields, all required:
    - `point`: the **binding point** a pass renders the image into.
    - `location`: the `@location` of the writing shader's fragment output that carries it.
    - `type`: the type of a texel, spelled `f32` or `vec4<f32>`.
  - **decision**: a document declares what it produces, rather than the renderer inferring it
    from pass order. The rule it replaces — the colour target of the last pass that wrote one —
    is a convention about position in a list, and any pass appended after the last one would
    silently become the frame. This is the same position the project takes everywhere else: a
    valid document that produces a result nobody asked for is the failure worth spending a field
    to remove.
  - **invariant**: an **absent** field is the old rule, so no document written before the field
    existed changes meaning, and a document that declares an output some pass does not write is
    an error rather than an image quietly missing from the result. See
    [proposals/agent-debug-scenes.md](proposals/agent-debug-scenes.md).
  - **decision** (2026-10-02, supersedes the mapping-to-a-bare-point shape above): the entry is
    a declaration rather than a point, from
    [proposals/typed-outputs.md](proposals/typed-outputs.md). Without a type there is nothing to
    check, and the value in an image was previously something a reader had to infer from the
    bytes.
  - **invariant**: **the `location` is stated and never inferred** from a name's position in the
    map. A pass numbers its colour targets from zero in attachment order, so a location derived
    from map order would mis-bind the instant a name was inserted or removed, and the failure
    would be a wrong picture rather than an error.
  - **decision**: the type vocabulary is **closed at three spellings** — `f32`, `vec3<f32>` and
    `vec4<f32>` — and the unit of the type-to-format table is *a format this device can create
    **and read back***. A real WGSL type that fails either half is **refused by name with its
    reason** rather than accepted and then failing later:
    - a whole number (`u32`, `i32`) has a format (`r32uint`, `r32sint`) that this device does
      not create, and `RenderedImage::texel` returns four `f32`, which a number above 2^24 is
      not one of. Supporting it is a change to that method's contract and belongs with the
      identifier work [handoffs/gbuffer-channels.md](handoffs/gbuffer-channels.md) scoped.
    - **decision** (2026-10-02, supersedes both the refusal of a narrower vector and the
      "at least as wide" rule that replaced it): **two questions, asked against two different
      numbers.** A declaration states how many numbers a texel **is**; the row it resolves to
      decides how wide the image holding them is drawn, and an image is filled one whole texel at
      a time. So *can the shader's fragment output fill that image?* is asked against the
      **resolved format's width** (`TexelType::image_width`) and the answer must be **exact**, and
      *how many components does the declaration claim carry meaning?* is asked against the
      **declared count** (`TexelType::components`), which is never more than the image holds.
      Everything from the declared count onward is **padding**: written, present in the file,
      meaning nothing. A `vec3<f32>` normal is therefore declared with three numbers, drawn four
      wide, and its fourth channel is padding. The reason is the one that made `vec3` a row at
      all: a scene says what its value is, and a rule about how an image is written belongs to
      the layer underneath.
    - `vec2<f32>` is **not** given a reason from underneath: it is answered as an unknown
      spelling, which is true, and adding it is one row. It is the one width the vocabulary
      does not carry, and the reason is that the table's rows are claims about values the
      emitted passes actually write.
    - **decision** (2026-10-02, closing the open question the width rule above left): the
      fragment output is measured against **the width of the image the declaration resolves to**,
      and not against the declared count. The defect that forced it was found on the adapter
      rather than reasoned out: a `vec3<f32>` declaration filled by a `vec3<f32>` fragment output
      satisfied the old check and then failed at pipeline creation, because the image is drawn
      four components wide. So "declare three, write three" is not a document that draws; the row
      that draws is "declare three, write four", and the fourth is padding.
  - **invariant**: the device **checks every declared output against the shader that writes
    it** — `check_outputs` in `wgpu_renderer/shader_interface.rs`, a free function over a
    `naga::Module` so it is checkable with no adapter, as `check_resources` is. It refuses
    (1) a declared type no format can hold, (2) a location the fragment entry point does not
    write, (3) a fragment output that is not **exactly** the width of the image the declaration
    resolves to, (4) a declared count larger than that image, and (5) a type that is not a float
    at all. The order is the order of what each makes moot, and **the declaration's own facts are
    asked first**: whether a format can hold the type, whether the declared count fits the image
    it resolves to, and where the point sits in this pass's attachment order.
  - **invariant**: a declaration's `location` is **cross-checked against the pass's attachment
    order**, and the mapping is established rather than assumed. A pass reads its `targets` list
    in the order the document wrote it — no sort, no deduplication, only the depth attachment
    dropped — and `raster.rs` builds one `ColorTargetState` per attachment in that same order,
    which is the numbering a shader's `@location(i)` resolves against. So **a point's index in
    that list is the index its declaration must claim**, and two attachments of the same type
    whose declarations have their locations swapped are now refused. A point more than one pass
    writes has to sit at the same location in each of them, which is stated in the message.
  - **invariant**: the surplus component of a `vec3<f32>` declaration is **padding and never a
    marker**, and the sidecar machinery says so. Marking is a reading convention rather than a
    value, so nothing in a declaration can ask for one; the built-in `Position` and `Depth`
    images keep `vec4<f32>` **because** they own a marker, and their own description is what
    tells a reader what their fourth channel means. The harness therefore measures a
    scene-declared image over every texel and prints a `-` line only for a built-in.
  - **invariant**: every message names **the shader's own variable and the location**. A bound
    shader entry has no name and its source is a wall of text; the variable and the number are
    the two things a reader can go and look at. **No message names a rule from underneath**: the
    single place a format appears is the pre-existing refusal for a type this device cannot
    read back, which is phrased about the device.
  - **not done**: the **format an image is read back in** is the format its target was created
    with, which the document's own instance payload declares. The table governs the format a
    **compiler-created** target is built from, and refuses a type no format can hold. A
    document that created an `rgba8unorm` target and declared `vec4<f32>` renders an eight bit
    image and reports `rgba8unorm`; that is a payload decision, not a declaration one.

### `device` crate

a server that can hot reload `Graphics`, `GraphicsInstance`, incrementally apply
`Graphics` and `GraphicsInstance`'s `Delta`, do headless rendering using http RPC.

- state is in-memory in a `Registry`, with explicit snapshot endpoints that write it
  to disk on demand. There is no filesystem watcher.
- `Renderer`: a trait the hot-reload path depends on, so the gpu backend is an
  injection point.
  - `fn render_outputs(&self, graphics: &Value, instance: &Value) -> Result<Vec<RenderedImage>>`:
    every image the document declares, each in the format its target was drawn in.
  - `fn render(&self, graphics: &Value, instance: &Value) -> Result<Frame>`: a provided
    convenience returning the image declared as `present`, over the **same** execution, so a
    caller that wants the picture and a caller that wants a debug image do not run the pass list
    twice.
  - `fn available(&self) -> bool`
  - `RenderedImage`: a name, the format a document spells, and the texels. `Frame` stays the
    eight bit RGBA image a png is made of; an image whose values are **read** is a different
    type from one that is looked at, and quantising a position into eight bits a channel would
    answer which region a pixel is in rather than where it is.
- `WgpuRenderer`: executes `Rasterize` and `Compute` passes headlessly and reads the
  framebuffer back. Behind the optional `wgpu` feature.
  - a rasterize pass binds one shader for both stages, because a pass has one `shader`
    field and a shader that exposes both entry points is the only thing that field can
    mean. The two entry points are `vertex_main` and `fragment_main`: WGSL has one
    namespace for functions, so a shader cannot name both `main`.
  - every pass runs in the order the document lists them, into textures shared by the
    instance entry they come from, and what comes back is what the document **declares** —
    each named output read back in its own format. A document that declares nothing is answered
    by the old rule, the colour target of the last pass that wrote one, named `present`; a
    declared output no pass wrote is refused rather than dropped from the list, and a document
    with no colour pass reports that rather than returning a placeholder.
  - the bytes copied back are the **format's** own: eight byte texels are read as eight byte
    texels, and a float image is decoded to `f32` rather than handed over as raw bytes. A
    readback that assumed four bytes would ask the backend for a copy narrower than the texels
    it is copying, which the backend refuses rather than truncates.
  - a target is cleared by the first pass that writes it and loaded by the rest, or the
    second pass into a shared target would erase the first pass's contribution.
  - the device is created with an error handler and drains it into the render result, so a
    shader that does not compile, a bind group that does not match its layout, and a draw
    with no buffer arrive as errors carrying the backend's own words. Without it a silent
    render is indistinguishable from one that drew nothing.
  - **not shipped**: the workspace has no shader sources. The standard compiler emits
    paths such as `shaders/quad.wgsl`, so a scene renders once an author supplies them.
    The whole path was verified on a real adapter with temporary shaders: a triangle
    rasterized with a vertex layout derived from a point schema, and the standard scene's
    three passes ran in order, sharing a shadow map and a depth buffer between them, with
    the lighting pass shaded by what the earlier passes wrote.
- `NoopRenderer`: the fallback when no gpu adapter is present, so the server and the
  reload logic stay testable on machines without a gpu.
- `Frame`: a rendered frame, returned as png bytes and as raw readback bytes.
- the renderer reads the dynamic `Value` tree directly. There is no intermediate
  strongly-typed representation of `Graphics`.
- `Compiler` cross-check: in debug and test builds the device also performs a full
  recompile and asserts the result matches applying the compiler's `Delta`, so a
  faulty incremental compile is a test failure rather than silent state corruption.

### `scene` crate

provide utility to build compilers to compile `Definition` with certain schema into
`Graphics` and corresponding `GraphicsInstance`, and its `Delta` version for
incremental compile.

- `Scene`: the `Schema` describing the shape a `Compiler` accepts. It is a schema
  document supplied by the compiler, not a Rust type.
- `Compiler`: trait.
  - `fn scene_schema(&self) -> &Schema`: the schema to compile.
  - `fn compile(&self, context: &CompileContext) -> Result<CompileOutput>`
- `CompileContext`: the inputs of a compile.
  - `scene`: the current `Scene` `Definition`.
  - `previous_graphics` and `previous_instance`: the last outputs, or none on first compile.
  - `limits`: the `Option<DeviceLimits>` the compile is made against, `None` where no machine
    is behind it and the floor is the answer.
- `CompileOutput`: the outputs of a compile.
  - `graphics` and `instance`: the full new `Definition`s.
  - `graphics_delta` and `instance_delta`: the compiler's own emitted `Delta`s, each
    carrying the `base_version` of the output it was computed against.
  - **invariant**: applying the emitted `Delta` to `previous_graphics` must yield
    exactly the emitted `graphics`. The device cross-checks this in debug and test builds.
  - `notes`: what the compiler wants the author to know about what it produced, as a
    path and a sentence each.
    - **decision**: a note is not an error and not a field of either document. An
      error stops the compile, and some things a compiler learns are not reasons to
      refuse: a scene lit less than it was drawn is still worth drawing, provided the
      author is told. The documents hold what a shader reads, and a fact about what a
      shader cannot read is not theirs to carry, so the report travels beside them.
    - **invariant**: a compiler that may drop something fills this. A host that
      ignores it renders a scene that is quietly wrong, which is the one outcome this
      project treats as worse than a refusal.
- the compiler is not re-diffed centrally; producing the `Delta` is the compiler's
  own responsibility.
- `document_delta`: the shared part of that responsibility, comparing two whole
  documents field by field.
  - **invariant**: a top level field is diffed on its own, so a change to one field never
    widens into a replace of the whole document. A field that already exists is replaced
    in place, which is what keeps a small edit a small delta.
  - a change of shape is a whole value replace rather than a composition, because the
    operations map holds at most one primitive per path and no primitive covers every
    shape change: a list that changes length needs a removal and an append at the same
    path, and a struct that gains a field has no add-field primitive at all. This is
    coarser than necessary and always correct, and a compiler that knows better can emit
    the operations itself.
- `StandardScene`: a compiler over the conventional lit scene, and the schema that
  describes it.
  - the scene document: a `name`, a list of `cameras`, a list of `lights`, a list of `shaders`,
    a list of `meshes`, a list of `mesh_materials`, a list of `screen_materials`, a list of
    `post_processes`, and the `instances` bucketed by pass.
  - `DebugChannel`: a **registry of built-in outputs** — an extra image the pipeline can be
    asked to draw without the scene's help — and **not** a field of the scene document: a
    channel is a question the *caller* is asking rather than a fact about the scene, so it
    travels as `StandardScene::with_debug`, beside the camera, which is the other input a
    compile has that the scene does not express by itself.
    - **decision** (2026-10-02, from [proposals/typed-outputs.md](proposals/typed-outputs.md)):
      the set is **built-in**, not closed. A scene may declare an output of its own beside these,
      type-checked against its own fragment shader, and a caller looks both up the same way out
      of one `outputs` map and one list of images. The enum was not melted into a table of
      constants: it already *is* a registry of named, scene-independent entries, and melting it
      would touch six landed test files and buy no behaviour. **This is the decision a reviewer
      should look at hardest.**
    - each entry carries its own `ty()`, its own `location()`, its own compiler-supplied WGSL and
      its own coverage contract, and `format()` is **derived** from `ty()` rather than matched on
      the variant — the format-inferred-from-the-channel that the proposal superseded.
    - **invariant**: a built-in is **never compared against a shader by the type check**. Its
      WGSL is the compiler's own, so there is nothing a scene can get wrong about it and nothing
      to compare. It still *states* its type, because the declaration is what a caller and the
      sidecar read.
    - enum of: `Position` — the world position of the nearest surface each pixel shows, and none
      where it shows none. `Depth` — the depth buffer itself: the value the depth test compared
      against at each pixel, and none where the scene drew nothing.
    - **decision**: the image is a **float** target (`rgba16float`), not a position normalized
      into eight bits with a key to decode it. The numbers a caller reads are then the answer,
      and there is no encoding for a caller to get wrong. `Depth` is drawn in the same format for
      a different reason: a depth value in zero to one quantised to eight bits steps by four
      thousandths, which is enough to see *that* a fragment lost a depth test and not enough to
      say how near it was. The format is now reached through `TexelType::format`, which is where
      that reasoning lives.
    - **invariant**: every channel writes a target of its own, so asking for one cannot move a
      pixel of the picture. Verified by rendering: `studio` and `tidewater` produce byte for byte
      identical frames with a channel and without.
    - **invariant**: `Position` and `Depth` cover **what the `depthing` bucket drew**, because a
      position is recovered from the depth buffer and `Depth` *is* the depth buffer. A scene whose
      lit stages draw an instance its `depthing` bucket does not has pixels no channel will name a
      position for, and a scene with no `depthing` instance gets a compiler note rather than an
      empty image with no explanation.
    - **invariant** (2026-10-02): **the coverage contract has two sources of truth and they are
      kept distinguishable in code, not only in prose.** A **built-in** is bounded by *what the
      pipeline has* — a buffer — so its header and its compile-time note name that buffer. A
      **scene-declared** output is bounded by *which buckets the writing pass enumerated*, so
      there is no buffer to name: `RenderError::UnwrittenOutput` says "not a missing buffer",
      names the passes that **did** draw (which is what separates an emitter that never
      enumerated a bucket from a shader that does not write the location), and
      `scene::declared_description` writes the enumeration's contract into the sidecar header.
      The harness's `describe_outputs` is the one place the two are chosen between, by name.
    - **invariant**: `Depth` reports the buffer verbatim, including `1.0` at a pixel nothing was
      drawn on, and marks the background in **alpha** rather than by zeroing the colour — the
      same marker `Position` uses, so a caller reading two channels together reads one rule.
      Zeroing it would report the *near* plane where there is no surface, which is the same class
      of wrong answer as giving a background pixel a position at the far plane.
    - **a channel that does not read the depth buffer does not inherit that contract.** One drawn
      by a geometry pass of its own covers exactly what it drew and owes a different statement:
      which buckets the pass enumerated. Recorded in `standard/debug.rs` so the day a G-buffer
      channel is built the contract is written rather than discovered. **`Grade` is the third
      case and it is a buffer case, not a drawn case**: it copies the shaded image, so it covers
      exactly what the lighting and emission passes accumulated into it, and `reads_depth()` is
      the predicate that keeps the empty-image note off it. The predicate is on the channel
      rather than on one call site, which is what keeps the note true as channels are added.
    - **invariant**: `Grade` is the frame **before** the post stage's grade, copied verbatim —
      no camera, no matrix, no test, no branch, and `textureLoad` rather than a sampler so the
      copy moves no pixel of its own. It is `rgba16float` for the **sidecar** and not for
      precision: the shaded image is itself `rgba8unorm`, so a wider target cannot recover what
      the lighting stage clipped. Three things are therefore stated on the file rather than left
      to be discovered — **the sky is black** because a post stage draws it, **a value at one
      was clipped** before any grade saw it, and **alpha marks nothing**, because the lighting
      stage writes one everywhere and the pipeline clears it to one. The `depth` channel is the
      one that says which pixels have a surface at all. Measured on `tidewater`: the beauty is a
      blue-hour lake and the grade image is a black sky, a thin treeline and a glitter path, so
      the sky, the lake's blue and the glow column are all the grade's — which is the comparison
      two runs each paid ninety minutes and a pixel-sampling script to make by hand.
    - **invariant**: what a channel's image *is* is said by the channel, through
      `DebugChannel::description()`, and not by whatever program writes the file out. Every float
      image a render produced may be written with a header, and a header written by the harness
      was a claim about the **format** — which cannot make it, because `Position` and `Depth` are
      the same `rgba16float` with the same marker and nothing in the bytes tells them apart. That
      is not a hypothetical: the harness shipped one line reading "world position of the nearest
      surface … in scene units" above every float image, so a depth file claimed world positions
      and a caller reading it put the sky at 1.0 units instead of the far plane. A file that is
      valid, a render that succeeds, and a result nobody asked for. The description answers the
      three questions a reader brings to a file of numbers — what the numbers are, what the alpha
      means, and what the normalization ranges below it stand for — and an image no channel drew
      is described as exactly that rather than with the nearest channel's words.
    - **invariant**: the numbers a sidecar prints are the numbers the shader wrote, and changing
      what it says about them changes no number in it. Verified by diffing a real `dunes` render
      before and after: the 81 data lines and the png are byte for byte identical, and only the
      header prose differs.
    - **why reconstructed from depth**: a depth buffer for the frame already exists and every
      post stage already binds it, so a channel is one fullscreen pass with no second geometry
      pass and no change to any material. A channel that cannot be answered this way — a normal,
      an identifier — needs a G-buffer. See
      [proposals/agent-debug-scenes.md](proposals/agent-debug-scenes.md).
    - **a G-buffer needs no graphics-schema change, only a renderer one.** `pass.targets` was
      already a list and the binding layer already carried several; `raster.rs` collapsed them in
      two places. A pass now builds one `ColorTargetState` per colour attachment in attachment
      order, and a shader's `@location(i)` is that order. See
      [handoffs/gbuffer-channels.md](handoffs/gbuffer-channels.md).
    - `Light`: an enum of the kinds of light, because what a light is is its kind rather
      than a field it may or may not have.
      - `Point`: `label`, `position`, `color`, `intensity`, `radius`,
        `shadow_extent` — the shape every light had before kinds existed, plus the
        distance the light's light falls off over.
      - `Spot`: a point light with a `direction` and the two cone angles, `inner_angle` and
        `outer_angle`.
      - `Directional`: a `direction` and no position, because a direction is not somewhere.
      - `Environment`: the `image` a material samples, and an `intensity`. It shadows
        nothing, so it declares no volume.
      - `label`, `color` and `intensity` are what every kind carries; `position`,
        `direction`, the two angles, `image`, `radius` and `shadow_extent` belong to the
        kinds that have them.
      - `radius` is **required on a `Point`** and optional on a `Spot`. It is a field of
        the light rather than of the scene because a scene that mixes a lantern with a sun
        has to say which is which, and it is separate from `shadow_extent` because the
        two answer different questions: how far a light's *light* falls off, and how much
        of the scene its shadow map covers. A radius of zero or less is refused rather
        than read as a light that reaches nothing, which is a scene lit by nothing.
      - **decision**: a `Point` light's `radius` has no default. It used to be optional and
        to mean one box radius of what the scene draws, which is right for a light standing
        **inside** the scene it lights and wrong for one standing outside it — and a wrong
        answer here is a picture, not an error, because the light is culled per cluster by
        its own radius and the clusters it is missing are just never lit. The symptom is
        documented twice in this repository, once in `emit.rs` for a spot light and once in
        `proposals/photographic-stilllife.md` for a still life: **rectangular patches about
        a cluster wide**, because the boundary between a cluster that has the light and one
        that does not is a plane through the eye. Which distance is right is a fact about
        where the scene's own surfaces are, so a document carries it rather than a compiler
        working it out. A `Spot` may leave it out and is culled by `SpotCone::reach`, which
        its own two angles already determine; a `Directional` declares none and takes no
        cluster slot, so there is nothing for the number to cull.
      - **decision**: the kind is the variant tag and **a missing kind is a validation
        error with no default**. In an offline render a wrongly classified light is a wrong
        image that nobody sees until the render finishes, and statelessness removes the
        previous frame in which the mistake would have been visible, so a refusal is the
        only thing that catches it. This is the shape a pass already has:
        `graphics::pass_schema` gives `Rasterize` a `targets` list and `Compute` a
        `workgroup`, and neither declares an optional field for the other's work.
      - **invariant**: a variant declares only the fields its own kind has, so a
        `Directional` light cannot carry a `position` and an `Environment` light cannot
        carry a volume. A document cannot say more about a light than that light has.
      - **not emitted yet**: only `Point` is rendered. The emitter builds one shadowing
        pass and one lighting pass per light through a projection made from the light's
        position, so a light of another kind is refused by name at read time rather than
        drawn as a point — a light quietly drawn as a point is a wrong image nobody sees
        until the render finishes, which is the position this project takes everywhere else
        in both code and comments.
    - `Shader`: source, stage and the four schemas, declared once per scene and named by
      every material that draws with it.
      - `source`: the shader text. Nothing in the framework reads a file, because an
        instruction has to render wherever it is sent.
    - `Mesh`: geometry, and nothing about where it is drawn.
      - `point_schema: Schema`: the shape of a point, authored per mesh so that a mesh
        carrying only positions is not forced to carry texcoords.
      - `Point`: one point, verified against the owning mesh's `point_schema`.
      - `Topology`: enum of `Triangles` and `Segments`. The topology says how the
        points are read, and is what tells a rasterize pass to draw indexed or line
        primitives.
    - `Instance`: a mesh placed in the scene, listed under every pass that draws it.
      - `mesh`: the label of a mesh in the same scene. A label no mesh carries is a compile
        error rather than a pass that draws nothing.
      - `transform: Transform`: `translation`, `rotation` in radians, `scale`. The three
        compose into the model matrix the instance row carries, in that order: scale, then
        a turn about x, then y, then z, then the translation.
      - `material`: the label of a mesh material this scene declares.
    - `instances`: **a map from pass to the instances drawn in it**, keyed by the stage
      suffix — `shadowing`, `depthing`, `lighting`, `emission`. `post` is not a key: a post
      process draws a fullscreen triangle rather than a scene instance and has no instances
      to bucket.
      - **decision**: the bucket is the pass, and the material it names must have a shader of
        that pass's stage. Before this, the stage was a consequence of which shader a
        material happened to name, so "a lighting material listed under `shadowing`" could
        not be written down at all — and neither, therefore, could the check that refuses it.
        The bucket makes the stage a fact the document states and the compiler verifies.
      - **decision**: the duplication this costs is explicit and grouped. One mesh drawn in
        three passes is three entries with the same transform and a material per pass, which
        is what the old document carried implicitly and scattered; here they sit in three
        named buckets. **This replaced a material naming the stage its instance was drawn
        in**, which read as less duplication and was the same duplication with the reason for
        it hidden one indirection away.
    - `MeshMaterial`:
      - `label`: the name an instance refers to it by.
      - `shader`: the label of a shader this scene declares, whose stage is the only pass
        this material can be drawn in.
      - `custom_data`: an open dict, verified against the shader's `custom_schema`.
      - **decision**: a material is declared once and named by every instance drawn with it,
        which is what `meshes` and `shaders` already did and what lets forty stones drawn in
        three passes each carry a handful of materials rather than one apiece.
    - `ScreenMaterial` and `post_processes`: a screen material is a `label` and the label of a
      `ScreenShader`, and `post_processes` lists the screen materials to run, in order.
      - **decision**: a `post_process` names a screen material by label rather than inlining
        one, for the same reason `meshes` and `mesh_materials` are declared once and referred
        to: the spec writes the type inline, and this is the one place the referencing design
        was ours to choose.
      - **decision**: the passes chain. The first reads the shaded image and the last writes
        the frame, and every process between reads what the one before it wrote, each writing
        an image of its own rather than reading and writing one. A chain of two is two passes
        over the frame rather than two passes reading the same image.
      - both lists are optional, because a scene that grades nothing is a legitimate scene.
      - **not used yet**: `ScreenMaterial.custom_data`. Every screen shader the standard
        pipeline emits declares the unit schema, so a grade whose parameters live in its own
        source has none to carry, and requiring the field would be requiring an author to
        write `{}` to say so.
    - `Shader`: what a material is drawn with, and the interface it expects.
      - the four mesh stages and the screen stage are the variants of one enum. The spec
        nests a `MeshShader` around the mesh stages; a variant payload in this language is
        a struct of named fields, so the stage is the tag and the nesting is vocabulary.
      - `MeshShader`: a shader over mesh points, in one of the four stages.
        - enum of `ShadowingShader`, `DepthShader`, `LightShader` and `EmissionShader`.
      - `ScreenShader`: a shader over the screen rather than over mesh points, so it
        declares no `point_schema`.
      - every variant declares three schemas, and each one is verified against the data
        it describes, so a shader that outgrows its data is a compile error:
        - `pass_schema`: verified against the emitted pass by `verify_pass_inputs`.
        - `instance_schema`: verified against the instance row, and it is also the layout
          that row is packed with, so it has to be a struct.
        - `custom_schema`: the `CustomSchema`, verified against the material's
          `custom_data`. It is the one schema an author writes for their own effect.
        - `point_schema`, on a mesh shader only: verified against every point of the
          mesh an instance draws. A mesh also declares its own `point_schema`, and **both**
          must accept the points, so a mesh is never described in a shape its own shader
          rejects.
      - **decision**: a pass draws one instance. A pass binds one shader and a shader
        declares one interface, so a pass that drew several instances would have to bind
        several shaders and the pass model has no room for it. More passes than a merged
        pipeline needs, in exchange for a binding model where what a shader declares is
        exactly what the pass supplies.
      - **decision**: an instance row is the model matrix, the normal matrix, and the
        material's custom data. Per instance data beyond a placement is not needed, because
        the transform places an instance and the material's `custom_data` carries what else a
        shader reads.
      - the emitted instance carries one vertex buffer per mesh, shared by every instance
        that draws it, and one instance buffer per instance, holding that instance's row
        alone.
    - `clustering`: how the view frustum is divided for clustered lighting, and how
      many lights one box keeps. Four optional counts, `width_extents`,
      `height_extents`, `depth_extents` and `lights_per_cluster`, defaulting to
      16 x 9 x 24 and 64.
      - **decision**: the grid and the cap are fields of the scene, on the same grounds
        as every other number here: a number in the source is a value someone chose
        once, and a number in a document is a value an author chooses per scene. A
        number in a document also has to be *readable*, which is what makes the cost
        visible: the box list is `clusters * cap` four byte indices whatever the scene
        holds, and 3456 clusters of 64 is 864 KiB at the defaults.
      - **invariant**: every count is at least one, refused when it is not. A grid of
        nothing is a scene whose lights reach no cluster, and that is a wrong image
        rather than a fast one.
    - `Clustering`: the grid a scene declares, and the three buffers the assignment
      produces from it.
      - **decision**: the assignment is computed on the cpu in `emit`, not in a compute
        pass. `emit` returns the instance and `render` consumes an instance that is
        already built, so for a given scene and camera the frustum, the lights and the
        boxes are all fixed and the assignment is a pure function of the documents. The
        textbook gpu path needs a clear dispatch, an `atomicAdd` count pass, a prefix
        sum for the offsets and a scatter pass, over a grid that
        `max_compute_invocations_per_workgroup` cannot cover in one workgroup; here
        there is no clear, no counter, no prefix sum and no dispatch.
      - **the three buffers are read only `storage`**: one packed record per light, one
        count per cluster, and one slot per cluster per cap. `max_storage_buffers_per_
        shader_stage` is 4, so these are three of the four and the fourth is the last
        one available. Nothing writes them, because there is no pass to write one with.
      - **invariant**: a cluster's row is a fixed `cap` wide, not packed end to end. A
        fragment reading one cluster's lights computes `cluster * cap` and stops; a list
        packed end to end would need a prefix sum of the counts to find the row, which
        is the sum this stage exists to avoid computing. The unused tail of a short row
        is padded with the first light and the count beside it is what says how many
        entries are real.
      - **the slices divide the projection's depth, which is uniform in one over
        distance, rather than dividing the distance.** Slices of equal world thickness
        pile up at the eye and vanish in the distance, so a grid of them spends most of
        its boxes on the clusters that fill the screen; and a fragment already carries
        its own depth, so this is the division a shading pass indexes with no arithmetic
        at all. The cost is stated in `clusters.rs`: the slices are not even in world
        space, so a light spanning one is listed in a box larger than the part of the
        scene it lights.
      - **invariant**: a cluster is the slab at its slice's **far** boundary, so the box
        contains the whole slab. The tighter box at the near boundary would exclude a
        light that reaches the slice's far half, and a light missing from a cluster is a
        light no shading pass can find. The slack is spent where it costs work rather
        than where it costs an image.
      - **decision**: the cap keeps the lights that contribute most to their cluster and
        **reports what it dropped**, through `CompileOutput.notes`, one note per
        overflowing cluster with the labels named up to eight and counted beyond that.
        The rank is `brightness / (1 + distance_squared)` to the nearest point of the
        box, and the only property it needs is to be monotone in distance: a falloff of
        the form `1 / (a + b*d + c*d*d)` with positive coefficients decreases with
        distance whatever its coefficients, so the nearest lights are the ones a scene's
        own falloff would keep. No scene's falloff constants are written into the
        compiler, because they are a scene's decision.
      - **invariant**: a kind that covers the whole view takes no cluster slot. A
        `Directional` and an `Environment` light are in every cluster by definition, so
        listing them spends the cap on lights the culling was never going to remove; they
        are in the record buffer and in no cluster list. A scene whose lights are only
        those two kinds therefore has empty cluster lists, which is the correct answer
        rather than a missing one.
  - `PassSchema`: the schema of a pass the standard pipeline emits.
    - `ProjectionSchema` is the schema of a projection: a view projection matrix and the
      point it was made from. It is data on the pass, not arithmetic in the renderer,
      because a reload can change the scene a projection covers and the pass has to say so,
      and because a shader reads it from the uniform it declares.
    - `ShadowSchema`, `DepthSchema`, `LightSchema` and `EmissionSchema`, one per stage, each
      carrying a `projection` of that schema.
      - **the functions keep a `_pass_` in their names** — `shadow_pass_schema` rather than
        `shadow_schema` — because the scene already has a `light_schema` for a light, and a
        pass schema and a light are different things that would otherwise share a name.
    - each is the interface of that stage: a `uniform` and a `binders` block naming a
      buffer at `vertices` and `instance_data` and the textures the stage writes or reads.
      A material that wants the standard stage writes that schema, and the compiler is on
      the hook to supply exactly it.
    - the projection is a real matrix: the camera stages look at the scene's bounds from
      three box widths in front along the negative z axis, and a shadowing stage looks at a
      box of the light's `shadow_extent` centred on the scene from the light's position,
      with the near and far planes derived from how far that is. The scene document has no
      camera field, so that framing is a stated convention rather than an author's
      decision; an author who wants a camera writes one and the convention goes away.
    - **a fitted perspective camera's depth planes bracket the corners' own view depths**,
      rather than the eye's distance to the target plus and minus the bounds' half diagonal.
      The two look equivalent and are not: a wide flat scene's diagonal is an order of
      magnitude deeper than the scene is, which holds the near plane a whole diagonal in
      front of anything drawn, and depth mapped from a near plane at zero is depth that
      cannot tell two close surfaces apart. The near plane is additionally floored at two
      thousandths of the eye's own distance, because a camera that crops in stands *inside*
      the box and no near plane brackets a box the eye is inside. Framing is unchanged; only
      the z mapping moves.
  - a buffer is a flat run of little endian four byte floats, in the field order of the
    schema that describes it, which is sorted by field name. Every point of a mesh
    occupies the same number of bytes, because a shader reads a fixed stride; a ragged
    point list is refused rather than packed into a buffer the shader cannot walk.
  - the compile emits three stages in order: one shadowing pass per light and per shadow
    casting instance, one depthing pass per instance, then one lighting pass per instance
    and per light. The order is the pipeline: nothing in a document says a lighting pass
    runs after a shadowing pass. Each stage clears its target to what that stage means:
    depth maps clear to the far plane, a shaded image clears to unlit.
  - the camera frames what is drawn rather than what is authored: the box is over the
    instanced points, so a mesh an instance places somewhere is projected where it is drawn
    and a mesh no instance names does not widen the view.
  - the emitted instance carries the shaders with their schemas, the vertex and
    instance buffers, and the shadow map, depth and colour targets. The graphics
    document declares each of them through a `Binder`, and its `instance_schema` is the
    schema derived from those declarations, which is what makes the spec's
    "its binders define a corresponding schema for `GraphicsInstance`" true of every
    compiler rather than only of one.
  - a material whose shader cannot be read, an instance whose `mesh` label names nothing,
    or a mesh whose points do not satisfy its own `point_schema`, fails the compile rather
    than the render, because each is a fact about the scene document and is known before any
    gpu work starts. The compiler also verifies every pass it emitted against the interface
    that pass's own shader declares, so a compiler bug is a compile error rather than a bad
    frame.
  - the default point and instance schemas declare their component widths, because a
    schema is also the layout a buffer is read with: a point is a position of three, and a
    model matrix is sixteen, which the renderer reads as four attributes of four.

## What the standard pipeline gained, and what it got wrong

The spec describes a pipeline that could draw a scene and not compose one. Four changes make the
difference, and three defects surfaced underneath them.

### A camera, a perspective, a time, and a real shadow

  - `math::perspective` and `math::perspective_of` sit beside the orthographic path, which
    remains and is what a camera asks for by name.
  - **A scene may name the `frame` it is drawn into.** The frame used to be a constant of the
    pipeline — 1024 square, for the shaded image, the present target and the depth buffer
    alike — and that was a fact about the pipeline rather than about the picture. A valley
    drawn into a square is cropped at the sides, and the same valley drawn wide keeps the
    ridges that were outside it.
    - **a scene that names no frame is drawn into the one it was always drawn into.** Both
      fields are optional and both default to the square, so every scene that existed before
      a frame was a field draws the frame it always drew.
    - a dimension of zero or less is the default as well: a texture a backend cannot
      allocate is a render that cannot happen, and a compile error about a pixel count is
      not a useful one.
    - the three images are the size of the frame **together**, because a post stage reads the
      shaded image and the depth that produced it and writes the present target, and a
      sample of one read at the coordinates of another is a frame of the wrong picture rather
      than a wrong picture.
    - the frame is read before the camera, because the field of view is widened by its
      aspect. **A wide frame is wider rather than cropped**, which is the rule the camera
      already followed and the reason a scene can now choose one.
  - A scene carries an optional `camera` of `eye`, `target`, `up`, `field_of_view`,
    `projection` and `padding`. **A scene with no camera is drawn by the convention it was
    always drawn by** — parallel, dead ahead, framing everything that is drawn — so every
    scene that existed before a camera existed still draws as it did.
  - **A scene names a list of `cameras`, and every one of them is rendered.** `camera` was one
    optional field, so a scene had one view.
    - **a camera is an input to the compile rather than something a renderer picks
      afterwards**, because it decides the projection every pass draws through — and, for a
      fitted camera, how far back the eye stands, which changes what the passes frame.
      `CompileContext` therefore carries a camera index, and a harness compiles and renders
      once per camera. A scene that names none is drawn by the convention, exactly as before.
    - a scene with one camera writes where it always wrote, so every existing invocation of
      the harness names the same files; a scene with several writes `-camera0`, `-camera1`.
    - **not done**: the device server holds one compiled output per definition and so draws
      the first camera. Making it hold one output per view is a change to what it publishes
      rather than to what a camera means, and it is not made.
  - **Emission is a stage of its own, and it runs once.**
    - the lighting stage runs once per light and its passes add, so a term a surface body
      added there was added once per light: `color = color + in.emissive` made an emissive
      surface twice as bright under two lights as under one. A lamp is as bright under three
      lights as under one.
    - the emission pass reads **no light**, which is the whole of what makes it run once, and
      it adds into the shaded image against the depth the earlier passes wrote, so an
      emissive surface behind an opaque one is still occluded.
    - **it declares no pass schema of its own.** A projection, the two buffers every mesh
      pass reads and the depth it tests against are the whole of it, which is why the spec
      adds no `EmissionSchema` beside the other three.
    - it is opt-in by the same mechanism every other stage is: a material names an emission
      shader and an instance is listed under `emission`. A material that names none draws no
      emission pass, so a scene that emits nothing costs nothing. `lanterns` is the scene it
      was built for, and it is lit by three lights so that the pass count says which happened:
      the lanterns are added by **one** pass while everything lit is added once per light.
  - **A camera may also name `near` and `far`, and then it is drawn from the eye it wrote.**
    Fitting is the right default for a diagram, where the whole scene has to be in frame by
    definition; it is the wrong one for a landscape, where the subject is a view and the
    extent of the ground decides nothing about where a photographer stood. A camera on a
    hillside looking down a valley cannot have its viewpoint pulled back until the valley
    fits a lens.
    - **the two are read as a pair or not at all.** What a projection needs is where the
      depth range starts and ends, and a near plane on its own is a number with nothing to
      be measured against.
    - **a camera that names neither is fitted exactly as before.** The corner-based bracket
      for the fitted path is unchanged, so a scene that says nothing about its depth planes
      draws byte for byte what it drew before either field existed.
    - the `Projection` carries the two planes, and a post pass receives them as
      `depth_range`. Turning a depth back into a distance divides by a quantity the stage is
      not otherwise given, and `WGSL` cannot recover it for the same reason it cannot invert
      the matrix the camera's basis stands in for. The post template gained
      `distance_to(depth)` for it, and a stage that walks through air along a ray divides the
      result by `dot(ray, forward)`.
  - `time` is a field of the scene, on every pass uniform. A render is a pure function of
    the documents and that one number, so a frame taken at `t = 2.5` is reproduced by
    rendering the same documents at `t = 2.5` again.
  - `light_projection` and `light_shadow_texel` join the lighting uniform. **A lighting pass
    needs both projections and they are not the same one**, and this is the finding that
    matters: the example that shipped before this had only the camera's, and sampled the
    shadow map at the fragment's own screen coordinate. That is a screen-space lookup of a
    light-space texture, which is not a shadow. With the light's own matrix a fragment can
    carry its world position into the light's clip space, and the shadow map becomes a shadow
    map.

### Pipeline state as data

`blend`, `depth` and `samples` are fields of a rasterize pass, and a field that is absent is
the behaviour the backend always had: `Replace`, depth written, `Less`, one sample. These are
statements about how a frame is put together, so they belong on the pass that makes it rather
than in the code that runs it. Two uses in the standard pipeline would otherwise have been
impossible, and both were found the hard way:

  - **The lighting passes add.** There is one lighting pass per light and they all draw into
    the same image, so a pass that replaced what was there left only the last light standing.
    A scene with a sun and a sky fill was lit by the fill alone. Adding is what makes several
    lights a sum rather than a choice; the first pass to write the target clears it, which
    added to nothing is the same as replaced.
  - **The lighting pass does not write depth, and compares `LessEqual`.** A depth prepass
    records the depth of every opaque fragment and a later pass drawing the *same* geometry
    lands on exactly those depths, so `Less` rejected every one of them and the pass drew
    nothing at all. The dune field was invisible until this was fixed.

### A post stage

`ScreenShader` is no longer refused. The compiler emits a final pass that draws over the
whole frame, reads the shaded image and the depth, and writes a third image which is what
the frame is read back from. It carries the camera's basis — `forward`, `right`, `up`,
`half_extent` — beside the projection, because a ray through a pixel is recovered from a
matrix by an inverse and `WGSL` has no matrix inverse. **A depth of one is where nothing was
drawn**, so a post stage is also where a sky belongs: it draws over the frame and the pixels
the scene never reached are the ones left to it.

### Three defects this surfaced

  - **`point_position` read the first three numbers of a point in field order.** A point
    schema is free to declare whatever a mesh needs, and one whose fields sort as `normal`,
    `position`, `texture` has its normal first — so a mesh carrying a normal was one unit
    across as far as the camera was concerned, and the camera framed a box around the origin
    instead of the scene. The position is the field *named* `position` now.
  - **The camera fit a sphere around the scene's box.** A wide flat scene fills a box whose
    height is a tenth of its width, and a sphere around that box is as wide as the box and
    as tall as its half width, so fitting it puts the eye far enough out that the scene's own
    edges fall inside the frame. The fit is now a bound per corner: a corner is inside the
    frame when its offset from the axis is no larger than the frame is wide **at that
    corner's own depth**.
  - **A normal was carried through the model matrix.** A transform that rotates and
    translates does carry a normal through unchanged, and an instance twenty times taller
    than it is wide is lit wrongly by exactly the amount it is uneven by. A shader that
    declares a `normal_matrix` in its instance schema has one filled in beside the model, and
    the gallery's does.

## The gallery, and what it does not do yet

There is a sixth crate, `crates/gallery`, holding the builders a standard scene document is
assembled from and the scenes built with them. It was the `scene` crate's `gallery` module until
the builders turned out to be the reusable part: a scene document is the same shape at every call
site, so `kit` writes each field once, and a new scene is then a file beside the others rather
than an addition to the compiler.

Nothing in `scene` depends on it and it depends on `scene`, so the direction is the same as
`scene`'s towards `device`: frameworks below, the things built with them above. `device` takes it
as a development dependency, because nothing in the device itself knows a scene by name — only
the render harness and the render test do.

`crates/gallery/` holds the builders a scene document is assembled from — the WGSL library, the
geometry generators and the per-field builders — and the scenes `dunes`, `lanterns`, `monoliths`,
`terraces`, `tidewater`, `studio` and `tiles`. They are rendered by the harness in
`crates/device/examples/render.rs`:

```text
cargo run --release -p whiting-device --features wgpu --example render -- <scene> <out> [times] [mode]

  render dunes out/dunes                    one frame, at the time the scene carries
  render dunes out/dunes 0 5 0.25           twenty-one frames, a quarter second apart
  render dunes out/dunes 0 5 0.25 --sheet         the same range as one contact sheet
  render dunes out/dunes 0 5 0.25 --sheet-full    the same sheet, at every pixel of each frame
  render dunes out/dunes 0 5 0.25 --video         the same range as an mp4, encoded with ffmpeg
```

`--release` matters, though less than it did: the compile is no longer the bottleneck, but a
debug build is still several times it. The harness reports clone, validate, compile and render
apart, which is the measurement [proposals/compile-cost.md](proposals/compile-cost.md) rests on.
`ffmpeg` must be on the path for `--video`; the frames are written either way.

### The render manifest

Every run of the harness writes `<out>-manifest.json` beside the images it wrote — one file per
camera, named through the same `with_stem_suffix` a channel image is named by — holding, per
output: the scene, the camera, the `time`, a SHA-256 of the two emitted documents, a SHA-256 of
the frame's pixels, the pass count, and a SHA-256 of the executable that drew it. The next run
reads that file **before** it renders and says what changed.

- **three hashes, because one cannot answer the question.** The build, the documents and the
  image are three separate facts, and they give four answers that are four different situations:
  nothing ran and nothing changed (a stale build); the build took and the edit still did not
  reach the documents; the documents changed and this camera's draw does not read what changed;
  or it worked, which says nothing. The image hash is the deepest of the three and the one no
  earlier version had — it is what makes "the document changed and this picture did not" a
  statement the framework makes rather than one every caller re-derives.
- **It is harness state and not a `CompileNote`.** A note is a fact about one compile of one
  document. This is a comparison against the caller's *previous run*, which is history, and
  history is the harness's. No library crate is involved and none was added for it.
- **The build id is the hash of the executable, not a timestamp.** The scenes are compiled *into*
  the example, so the binary is the only provenance there is, and a clock that moved is not
  evidence of anything.
- **Nothing about the picture moves.** The manifest is read and written beside the render, never
  into it, and a render with it is byte-identical without it — checked by SHA-256 over the PNGs.
  A run naming no debug channel writes the files it wrote before, plus this one.
- The first run of a scene has no previous and **says so** rather than warning: silence about a
  missing manifest is wrong, and a warning on a fresh scene is noise.
- The document hash covers content, not provenance: identifier, schema and value, with a float
  hashed by its bits. The definition's version counter is excluded, because it is bookkeeping
  about how a document arrived rather than something the document says.

### The contact sheet

`--sheet` puts a range of frames in one grid, and it is the only view in which the range itself is
visible: three frames of a range composed alike, or one that drifts off and comes back, cannot be
seen one frame at a time. It samples every **fourth** pixel. `--sheet-full` asks for every pixel
instead. A sheet is a composition of frames that were already drawn, and nothing about any
individual frame's rendering changes either way.

- **The step is a choice and not a consequence.** A tile is the frame divided by the step and
  nothing else, so its size does not move when a range gets longer: the same frame is drawn at the
  same size in a six-frame sheet and a thirteen-frame one. Scaling the sheet to land inside a
  target size would be elegant and would give that up — the tile would become a function of how
  many frames were asked for, and a range read across runs would change resolution under the
  reader.
- **The quarter stays the default, and the objection to dropping it is sound.** A dozen frames of a
  thousand pixels each is a sheet nobody can see, so full resolution is a flag rather than the
  default: six 1024² frames are a 3072×2048 sheet, and thirteen are 4096×4096. Every invocation
  that does not name the flag writes the sheet it wrote before it, byte for byte.
- **A tile is a sample of the frame's own pixels, never a resize.** Nothing is interpolated, so a
  sheet cannot say something about a frame the frame does not say.
- **The line that reports a sheet says how far apart it read**, and the size it reports is the size
  of the **file**: it used to report the grid the frames would have made unsampled — a 3072x2048
  number beside a 768x512 png — which is the resolution of a thumbnail claimed as the resolution of
  a picture.

### A surface may displace the normal it is lit by

The lighting stage's fragment body now offers a second splice point, immediately after the normal
arrives from the vertex stage and **before** anything reads it, where a scene's body may
reassign `normal`. An empty body produces the `WGSL` the stage produced before, so a scene that
displaces nothing is unaffected.

- **The existing `{surface}` splice could not do this.** It lands after `lambert`, `fresnel`,
  `specular` and the offset the shadow is looked up at, all of which are functions of the
  normal, so a body there can only add to a colour it was not allowed to influence. Water,
  foliage, cloth and anything else lit by a normal that is not the geometric one were
  unreachable without reimplementing the whole stage by hand.
- **What it does not give.** A displaced normal is a per-fragment lie about geometry, so the
  silhouette does not move. A leaf that turns or a coat of cloth wants the *outline* to move too,
  which is a vertex stage hook and a different decision. `tidewater`'s reeds sway in shading
  only and the file says so.
- The emission stage gained a `helpers` slot for the same reason the lighting stage has one: a
  stage whose body cannot reach a shared function cannot use one, and a surface that emits the
  reflection of the same sky the frame's background is drawn from has no other way to say the
  two are one sky.

`kit::instance` takes `impl Into<String>` rather than a `&str`, because a label names one
instance in the **whole** scene rather than one per pass, and a scene that places one mesh in
three passes has to qualify them. Every builder that takes a constant label still takes a `&str`.

### `tidewater`

A lake at first light: water, a far shore in silhouette, mist lying on the surface, and a sun
behind the ridge. It is the first gallery scene composed as a photograph rather than as a
diagram, and three of its decisions are the framework being right rather than being worked
around.

- **The water is drawn by the emission stage, not the lighting stage.** Its colour is almost
  entirely a reflection, and a lighting pass runs once per light and adds — so the lake came
  out twice as bright as the sky above it. The emission stage exists for exactly this, which
  makes the cost of getting it wrong a discoverability problem rather than a design one. See
  [proposals/photographic-scenes.md](proposals/photographic-scenes.md) item 3.
- **The water is in the `depthing` and `emission` buckets and not the `shadowing` one.** A
  shadow map holds what is put in it, and a flat plane a thousand units across at two degrees
  off the light is the worst case for acne there is. A flat lake casts no shadow a viewer could
  see, and what the lookup then returns is the far shore's shadow lying across it.
- **One sky, written once**, handed to the lighting stage, the emission stage and the post
  stage. `dunes` wrote the same air twice and the two copies disagreed, which was the bright
  seam the *Known-incomplete* note below already recorded; it is now closed.
- **The camera is placed rather than fitted, and its height is the whole of it.** At four units
  above the water every visible part of a lake is at a grazing angle, the Fresnel term is near
  one everywhere, and the frame is one flat mirror. At nine the near water is seen steeply and
  goes dark, and the picture runs from a dark foreground to a bright horizon.
- **Both cameras name a lens.** Thirty-five and a hundred and fourteen millimetres on a full
  frame negative, rather than the angles they work out to. See below.

### A camera names a lens, and a frame can be rolled

Three optional fields on a camera, alongside the field of view it already had.

- **`focal_length` and `sensor_width`**, in millimetres. A focal length is quoted against a
  film size and is meaningless without one, so both are written and the angle is arrived at:
  `2·atan(sensor / 2·focal)` horizontally, divided by the frame's aspect to reach the vertical
  angle `field_of_view` is. **The angle a focal length buys is horizontal and the field of view
  is vertical**, which is the one thing worth stating twice: doing the conversion the other way
  round quietly widens every frame that is not square, and every frame a scene asks for is not
  square. A document carrying both takes the angle, because the angle is what the projection is
  built from and a scene that changed its picture when an unread field was added to it would be
  a document whose meaning depends on its order.
- **`roll`**, in radians, turning the frame about the view's own axis. The eye and the target
  are untouched, so a rolled camera still looks at exactly what it looked at. This is the
  whole difference between a roll and `up`: `up` is a statement about the world, and a camera
  whose up is not world-up has a tilted frame *or* a horizon that is not the world's, and a
  photograph wants the first. A level horizon is a decision, and this is the field it is
  written in.
- **`shift` is not here, and the reason is the interesting part.** A shifted principal point
  has to reach the post stage as well, because `ray_through` rebuilds a ray from `half_extent`
  and the shift is not in it — a shifted camera with the current post stage draws its sky in
  the wrong place. That is a field on the post uniform, which is the same emitter change
  exposure wants. Both are deferred together.

### One air, described once, handed to every stage

`gallery::air::Atmosphere` is a scene's description of the atmosphere, and `wgsl()` writes it
out as module-level `WGSL` that any stage can be handed: `air_colour()`, `air_sky(ray)`,
`air_closure(eye, point)`, and a sun's bearing, glow and disc if the scene named one.

- **What must not disagree is the colour, not the law.** Every outdoor scene has its own aerial
  perspective — a height falloff lying on water, a flat distance wash over a plain — and
  forcing one law on all of them would be a worse mistake than the seam it fixes. So the module
  owns the colour the air settles at and offers a law as a convenience, and a scene that wants
  another law writes its own body against the same colour.
- **`dunes` is the exhibit.** Its lighting body mixed distant surfaces towards
  `vec3(0.58, 0.62, 0.72)` under the comment *"the colour a distant surface tends towards is
  the colour of the sky behind it"*, and its grade drew the sky's horizon as
  `vec3(0.70, 0.66, 0.60)`. The comment was right and the two values were not, and a comment
  is not a constraint. `dunes` now reads both from one constant.
- **The compiler cannot supply this**, and that is the half that is still open. A compiler can
  hand a stage a uniform and nothing else — it cannot inject `WGSL` into a scene's own source —
  and the uniform that would carry it is the per-light uniform `feature/lighting` is replacing.
  The framework-level version of item 2 is therefore still owed; see the proposal.
- **`tidewater`, `terraces` and `monoliths` each carry the same duplicate pair `dunes` had.**
  `tidewater` is the one that got it right by hand, which is what this module is that code
  promoted.

### A rough surface is not a mirror, and the shading now says so

Two changes in the gallery's lighting template, both in the specular.

- **The Fresnel has a ceiling of `max(1 - roughness, f0)`** rather than saturating at one. The
  rise towards one at a grazing angle is a property of a *smooth* interface; a rough surface
  is a field of microfacets tilted in every direction and reflects about as much edge on as
  head on. Schlick's term hands it the whole difference whatever the roughness, and the result
  is that a thin object seen edge on — a reed, a wire, a branch, a railing, a wet edge, a
  horizon-lit rim — renders as a white stick whatever tint it carries, because at a grazing
  angle there is nothing left to tint it with.
- **The specular peak is divided by the lobe's own width**, normalised against the smoothest
  lobe the template builds, so a mirror is exactly as bright as it always was and only the
  roughness moves. Widening a lobe is half of what a microfacet distribution does; the other
  half is that the peak falls as the lobe widens, because the same energy is spread thinner.
  Left unnormalised, `pow(n·h, exponent)` gives a searchlight — a surface as rough as a wall
  still shows a highlight at a mirror's brightness.
- **Measured, and the measurement is not flattering.** Across the gallery this changes a mean
  of 0.0009 to 0.026 per channel out of 255, the largest single-channel difference anywhere
  being 20. That is because the gallery no longer holds the case that motivated it: the reed
  bed was removed from `tidewater` once it was diagnosed as the bug. The change is correct and
  cheap and the class of scene it fixes is large, but the original claim that it "improves
  every frame in the gallery at once" was wrong, and a reviewer who built a scene to see the
  fix would have concluded it did not work.

### A reflective surface is drawn by the emission stage, and `kit` now says so

`kit::instances` takes `Vec<(Stage, Vec<Value>)>` rather than `Vec<(&str, Vec<Value>)>`. The
lighting stage runs once per light and its passes add, so anything a lighting body computes is
computed and added *per light* — which is how a lake drawn there comes out twice as bright as
the sky above it. The emission stage runs exactly once, which is the whole of why it exists.

Taking the stage as a type does not make an author choose correctly. It puts four choices side
by side in one signature where the fourth is a different *word*, rather than four
interchangeable strings that a typo turns into a different pass, and it puts the rule on the
argument list where the decision is actually made.

The schema field that would make the compiler route an unlit material by itself is **not**
here, on purpose: a field nothing reads is data waiting for a stage, and the part that would
act on it is `groups`/`read_instances` in the emitter, which is the lighting path.

### Known-incomplete

Stated rather than left to be discovered:

  - **`tidewater` is composed, not finished.** The sun's reflection breaks into hard horizontal
    steps where the waves line up rather than sparkling, the near bank reads as a sandbar
    rather than as the corner it was placed to be, and the sun's disc shows through the treeline
    in the wide frame because the shore's crest sits lower than intended.
  - **`dunes` is tuned, not finished.** The bright seam where the field's far edge met the sky
    is **closed** — `dunes` now reads its air from `air::Atmosphere` and both the haze and the
    sky's horizon are the same constant, so the cold ridge along the horizon is gone. What is
    left is faint shadow acne at the far edge. The penumbra used to be wider than it should be;
    at a 2048² shadow map over a 380-unit volume a texel is a fifth of a unit, which is what a
    step two units wide needs.
  - **`lanterns` is composed, not finished.** The field is a grid with a scatter over it
    rather than a layout, and the two lantern lights are placed by hand where the water
    wanted a reflection. A post-stage tone curve was tried and removed: compressing the
    lanterns also lifts the water, and a Reinhard curve and a gamma turn a night into a grey
    fog. What keeps a lantern from clipping is the emission stage writing a dim enough
    colour.
  - **`terraces` is composed, not finished.** The valley floor is a patchwork of crop colours
    keyed by cell hash, and the cells are square and the same size everywhere, so the floor
    reads as a grid laid over the land rather than as fields cut into it. There is a ring of
    contour steps at the centre of the valley that the terrace mask draws and the scene does
    not explain.
  - **`monoliths` is a sketch.** The composition is a row of small stones on a plain, and the
    camera is framed so the ground — which reaches much further than the monoliths — decides
    the fit, so the crop is a blunt instrument. The wet-ground reflection is a Fresnel
    stand-in rather than a reflection pass, which is why the ground is drawn as geometry
    instead of a clear colour.
  - **A scene samples an image, and the compiler is what binds it.** `images` is a list of
    labelled images — pixels, a size and a format — which a material names inside its
    `custom_data`. The name is not a number, so the compiler sorts the custom data by kind: the
    scalars and vectors are packed into the instance row as they always were, and the image
    becomes a texture payload and a sampler bound at a point named after the field. The author
    writes one name; the emitted documents hold a row and a texture.
  - **The kind comes from the shader's `custom_schema`, not from the value.** A field carrying
    the image marker is a field the compiler lifts; everything else is a field it packs. That is
    why the marker is a schema attribute rather than a `SchemaKind`: `definition` is a general
    data representation, and a texture is a graphics concept.
  - **The binding point is the field's own name**, derived by the compiler and written by
    nobody, so a shader declares the point its own schema already names and no author counts to
    a binding. The pass schema the shader is checked against is generated from the same field, so
    the two cannot disagree.
  - **Images are part of the group key.** A pass binds one texture at one point, so two
    instances naming different images in the same field are split into separate passes rather
    than drawn by one pass that reads the wrong texture for half of them.
  - **The one gallery scene that samples an image is `tiles`**, and it generates its own pixels:
    nothing in this framework reads the filesystem, so a texture is something a scene compiler
    produces. A pattern a loop can write is a texture, the same way a dune field's points are.
  - **The scene-side gap is closed as far as the mesh stages.** A screen material's `custom_data`
    could carry an image by the same rule and the post stage's binder set would grow the same
    way; that is the same mechanism and it is not built.

### Invariants to keep

  - A gallery scene is an ordinary standard scene document: the same `Value`, the same schema,
    the same compiler. It is not a second kind of scene.
  - A shader carries its own source, so a scene stays self-contained and renders wherever it
    is sent.
  - The uniform struct of a shader is written in the order its interface schema sorts its
    fields into, and the device checks it against the parsed shader and will say so. It caught
    two real mistakes: a stage declaring a uniform its pass does not supply, and a struct
    written in the wrong order.
  - **A shader's resources and its vertices are checked the same way, and by the same
    parse.** The framework owns three numberings — the uniform's field offsets, the binding a
    resource is at, and the location a vertex is read at — and it now derives the last two
    and compares all three against the parsed shader. A shader that names a binding or a
    location the pass did not lay out is a render error naming the pass, the shader's own
    variable and both numbers, rather than a backend message about a view dimension that
    names neither document. This closed a gap the gallery had been working around by
    transcribing its own attribute list by hand: the transcription was unchecked, and a
    transcription that is one field out draws nothing at all, or draws something wrongly.
  - **The comparison is one-directional.** The shader is checked against the pass, never the
    reverse: a layout may declare a binding no shader reads, which is legal and common, and
    reporting it would make every shader that ignores a binding an error.
  - A surface body is written into the body of the lighting stage's fragment function, so it
    cannot redeclare a name the template or another surface already uses. A scene's helper
    functions go in the `helpers` argument, because `WGSL` has no nested functions.
  - **Every triangle of a mesh is wound the same way round, and no generator is exempt.** The
    two triangulations of a quad both fill it exactly, so the choice of diagonal looks free —
    and it is, *provided* both triangles of a quad are wound alike. Winding one the other way
    round fills the same area on paper and loses it on a gpu, and nothing reports it: the
    depth buffer has a hole where that triangle was, every other pixel of the frame is
    correct, and what a picture shows is a herringbone of missing geometry that looks exactly
    like a shading artefact and is not one. There is no right way round, only a consistent
    one, and it is a property of a mesh rather than of any point in it.
  - **A heightfield's normal is taken per point, from that point's own gradient.** One normal
    per cell is a rounding error away from the right answer on a smooth hillside, and nothing
    like the right answer on a slope one cell wide: the two triangles sharing a quad disagree
    about which way the surface faces along the diagonal, and a terrace riser renders as a
    stripe of alternating triangles down its whole length. It cost four more calls to `height`
    per cell, which is the price of a surface that is perpendicular to the surface the point
    is actually on.

## References

The spec's `definition` crate now carries references: a value slot a schema marks may hold a
`Reference` — a path inside the same document, a path to a registered definition, or a file —
instead of a value. What this amends for a scene:

  - a mesh may be named from another definition rather than written into the scene, so several
    scenes share one mesh instead of each holding a copy of its points. `meshes` is a list whose
    element schema is marked, so a scene may write a mesh or name one, and a name that does not
    resolve fails the compile rather than the render.
  - a compiler reads a scene that holds everything it names: `CompileContext::scene_value`
    resolves the document through the registry before the compiler sees it, so nothing in a
    compiler has to know which parts were named rather than written.
  - the same store serves the device, which is what keeps a compiler and a renderer from
    resolving references in two ways.

What was deliberately not built: a compiler emitting shared definitions of its own, so that two
compiled outputs reference one copy of a mesh's bytes. The bytes are a function of the mesh, so
the copy is derived data and regenerating it per scene is correct; sharing it pays only when one
source has a large fan-out, and it would grow the compiler contract with extra definitions, their
versions, and a rule for retiring a shared part whose last user went away. Sharing a mesh in the
authored document is what the fan-out needs first, and that is what is built.

## Reference slots: the decisions behind them

What a reference slot means, and what the loaded form is. These are the contracts the code is
written to; the code states the same invariants at the points they matter.

  - a schema declares which slots may hold a reference, and such a slot holds a **union**: a
    reference or a value of the declared kind. The declared kind describes both ends, so an inlined
    value must satisfy it and a reference must resolve to something that does.
  - a reference is recognised by its tag, so a malformed reference is reported as one rather than as
    a kind mismatch, and a value that happens to look like one in an unmarked slot is data.
  - traversal is transparent: two slots naming one path name one node. Re-pointing a slot is a
    `Replace` of the slot, which is why a delta whose path *reaches* a reference belongs on the
    loaded form rather than the serialized one.
  - loading resolves eagerly, so a loaded document is a graph of nodes rather than of spellings, and
    reading a value through a path costs what reading any node costs.
  - a loaded version is a **generation**: an arena of nodes addressed by index, behind one reference
    count, freed whole when its last holder drops. A cycle is therefore free and there is no
    collector, and an old generation lives while anything still refers to it, which is what makes a
    hot reload safe for a holder that is mid-read.
  - definitions refer to each other in a **graph, not a cycle**: a reference keeps its target alive,
    so two definitions naming each other would hold two generations alive with nothing else
    reaching either. That is the one cycle refcounting cannot free, so a store refuses a definition
    already being loaded rather than loading it again.
  - a delta mutates the live generation in place, so the sharing of a node is a diagnostic and a
    packer hint rather than a correctness input.
  - `Append` carries the position it appends at, and a `Remove` of indices is a contiguous range, so
    an operation map holding one primitive per path can always invert exactly. A scattered removal
    is deliberately not representable, because no `Append` could invert it.
  - a loaded generation carries a packed list as a node of its own, holding the run whole.
    **Invariant**: a run holds no slots, because an element of it is a stride rather than a node.
    A reference *to* a packed list therefore resolves to the run and a reader decodes the element.
    A path that goes *through* a packed index is refused at the `Loaded` level — there is no node
    for `points[42].position` — while `Value`-level `Path::resolve` and `Path::update` both handle
    it by decoding.
  - **invariant**: `find_reference` and `crossed_reference` stop at a packed list rather than
    decoding it. Packing only accepts structs of numbers, so nothing beneath a run can spell a
    reference, and descending into a run to look for a declaration that cannot be there is the
    whole cost the run was chosen to avoid.

## What a packed list bought, and what it did not

A dune field went from a **305 ms compile and a 10 ms render** to a **10 ms compile and an 8 ms
render**, with every frame byte-identical to before. A frame had been 97% compile; it is now split
evenly. The compile cost scales with *bytes* rather than with tree nodes — `monoliths`, at 182k
points against `dunes`' 55k, compiles in 32 ms against 10 ms.

What made it possible is that the uniform structure of a list is **guaranteed by the schema**
rather than discovered by a packer: a list declares one element schema, and a struct is closed. So
packing is a decision about representation, taken where the data is written, rather than a
verification the compiler repeats on every compile.

The same change is why validation was kept. See *ValidationError* above.

## Clustered lighting and the shadow page pool: the seam, and a blocker

Stage 4a of the scalable lighting effort. This records the two decisions that were not obvious
and one that could not be made here.

### The lifted module is general because it takes a `ViewProjection`, not a camera

`crates/scene/src/lighting/` sits beside `diff` and `reference` rather than inside `standard`.
The rule this follows is that a property of lighting in general belongs outside a compiler, and
the test of it is whether the module can name anything of the standard scene's.

It can now name none. The arithmetic took `&standard::Projection` and read its `eye`, `right`,
`up`, `forward`, `near`, `far` and its two depth/distance mappings. Those nine readings are
behind a `ViewProjection` trait now, and `Projection` implements it **by forwarding to its own
fields and methods**.

**Invariant**: a camera that implements `ViewProjection` by recomputing a reading rather than
forwarding it can disagree with the matrix its passes are drawn through, which puts a light in
clusters no fragment is in. Forwarding is the only implementation that cannot drift.

**What stayed in `standard`, and why**: the camera itself (`Projection`, `Camera`) and the light
document an author writes (`scene_schema::light_schema`). Both are the standard scene's own
schema and WGSL surface. What moved out: the cluster grid and its four documented defaults, the
per cluster cap, the contribution ranking, the packed record schema, the three buffer schemas,
and the page pool. None of them mention a material, a mesh, a pass or a shader.

**The two ways a general module quietly stops being general**, both of which this lift had to
undo: reaching into another module for a constant (the clustering field names moved with the
grid), and reaching into another module for a two line helper (`dot` and `subtract` are now
local to `assignment.rs` rather than taken from `standard::math`, which is a matrix module).
The second one duplicates two functions. It is worth it, because a general module that borrows
a compiler's private arithmetic is not a general module.

### One ranking, two budgets, and why they disagree

The shadow page pool and the per cluster cap both ask "how much does this light matter", and
both are answered by `contribution`, which is brightness over one plus distance squared. There
is one notion on purpose: two rankings that could disagree would be two answers to one question.

They *are* used differently, and the difference is not an accident:

- a cluster asks **how bright is this light there**. The distance is to a box, so a light just
  outside it still counts for a little, and a light inside it counts fully at any radius.
- a page asks **is this light worth rendering once**. The distance is over how much of the
  image the light reaches at all.

So a light can be kept by a cluster and still be given no page. That is the whole reason to rank
twice rather than derive the pool from the cluster lists: the cluster lists have already spent
their budget on the lights nearest each box, and the page budget is a different question about
the same lights.

**Invariant**: a light that reaches no cluster is given no page and is not reported as dropped.
A page for it would render nothing, and counting it would point an author at a pool that was
never the problem. `Clusters::reached` is what tells the two apart; `pool_notes` is given the
whole assignment rather than a list of pages for exactly this reason.

**Invariant**: a light with no page carries minus one, not zero. Zero is the first page. A
sentinel a shader can distinguish is worth four bytes a light, and a shadow lookup into the
wrong light's page is a wrong image nobody sees until the render finishes.

### A blocker: the device cannot create a texture array

Stage 4b needs one shadow page pool a shader can index, and the pool was designed as a texture
array. **The device cannot make one**, in four independent places, plus a missing schema field:

- `graphics/binders.rs` `binder_payload_schema`'s `Texture` variant has no `layers` field, and
  `texture_resource()` has no layer count either. A document cannot say a texture has depth.
- `device/wgpu_renderer/bindings.rs` hardcodes `depth_or_array_layers: 1` and
  `TextureDimension::D2`, and creates a default view, which on one layer is a `D2` view of
  layer zero.
- The same file hardcodes `view_dimension: TextureViewDimension::D2` in the bind group layout,
  so a texture that *were* an array would still be declared as a flat one.
- `device/wgpu_renderer/raster.rs` draws into `slot.view` with no way to select an array layer,
  so a shadowing pass could not render into page *n* even if one existed.

**This is not a gap that could be closed from the compiler side**, which is the thing worth
recording: the design principle in this project is that the compiler decides and the device
executes, and that principle has a boundary. A resource the device cannot create is not
expressible as schema data, so a stage that is forbidden from touching the device is forbidden
from using one.

**Two ways forward, neither made here.** Extending the device is the settled design and needs
four files across two crates, two of which are the device. A 2D atlas — pages tiled into one
`texture_depth_2d`, the index resolved with a multiply and a modulo — needs no device change and
fits what the device can already do, at the cost of making the page size and the count
dependent (a 2048 square texture holds 16 pages of 512, not 256) and needing the nine tap
kernel clamped so it cannot read a neighbour's texels across a page boundary.

### naga validates this project's WGSL, and it fits

Correcting an earlier belief: the WGSL here **is** verifiable without an adapter.
`naga::front::wgsl::parse_str` and the validator are pure cpu work, `naga` 30.0.1 is already in
the lock file through wgpu, and the device crate itself depends on this being true
(`wgpu_renderer/mod.rs` parses every shader with it). So `naga` is a dev dependency of `scene`
and `tests/wgsl_validation.rs` parses and validates every shader the six gallery scenes emit.

**Invariant**: the check reads sources out of the **compiled instance**, never off a template. A
template still holding `__SUN__` is a template with a live token in it, and a token that
replaces to nothing is exactly the mistake a template hides.

**Invariant**: it validates against `Capabilities::empty()`, not the defaults. The defaults are
a modern adapter's feature set; `WgpuRenderer` asks for `Features::empty()`. Validating against
the defaults would accept a shader this renderer cannot run, which makes the check worse than
useless. This is a statement about **features only**: the device's *limits* are the adapter's own
rather than the downlevel floor, so a limit is no longer a constant this test could state, and
the two limits this project does bound are bounded where they are owned — a document is held to
the limits of the machine its compile is made against, which arrive through
`CompileContext::limits`, and the clustered pass's three storage buffers are counted in the test
file itself.

**What the check is not**: it is not pipeline creation. `naga` cannot compare a bind group layout
against a shader, because that is `check_bindings` in the device and needs a renderer instance.
"Validated" here means well formed and within capabilities. **Nothing was executed**; this
machine has no adapter, and the two adapter backed render tests in `crates/device/tests/` were
not run.

## Image based lighting: one ambient term, and what feeds it

### The environment **replaces** the stand-in, and the stand-in is the stated fallback

The lighting stage carried a two colour hemisphere: darker below, dimmer blue above, brighter
towards `+y`. It was explicitly a stand-in — the code said so — and an `Environment` light is
the thing it stood in for. The decision is therefore **replacement, not addition**, and it is
written in the shader rather than left emergent:

> There is **one** ambient term. What feeds it is the scene's `Environment` light when the
> scene names one, and the two colour hemisphere when it does not. Never both.

A scene naming an environment *and* keeping the hemisphere underneath would count an author's
environment twice and add a sky of the renderer's own that no author could turn off, which is
exactly what the hemisphere was. The alternative — coexist — was rejected for the same reason.

**A scene naming no environment light still renders, and this is the rule rather than an
accident.** It binds a **one texel black** map at `environment_map` and writes a uniform field
of `vec4(colour * intensity, present)`. The shader branches on `present` and takes the
stand-in. Three consequences, all of them load bearing:

- The **binding is unconditional.** A binder that came and went with the document would move
  every binding number below it, because the device numbers a pass's binders in name order. The
  shader would then be a different shader per scene, and `lighting_bindings` generates its
  numbers from a flag rather than reading the pass.
- The **flag is a fourth number in the colour vector, not a field beside it.** Two numbers a
  shader could disagree about are one too many. And a colour of zero is a perfectly good
  environment to want, so presence cannot be inferred from brightness: the absent map is
  deliberately **black**, so a shader deciding on the sampled colour could not tell "no
  environment" from "a black environment".
- The absent map's instance entry is **not** named after an image. It is
  `environment_absent_map`, because the entry is named after the image it holds and this one
  has no image; a name in the `image_` namespace standing for a thing no author declared is a
  name a reader of the instance would go looking for.

**The material's own tint and metalness still reach the term.** The multiplication stays in the
fragment body, not in the function that picks the source, and it is the same
`sky * mix(1, tint, metallic) * diffuse_color` the stand-in had. A term that had stopped
multiplying by them would be a material that had stopped responding to its own colour, which is a
regression nobody asked for.

### How a direction becomes a texel: a two dimensional octahedral map

**The obvious answer is a cube map, and the device cannot build one.** `ImageShape::Cube` exists
in `shader_interface.rs` — but only as a value the interface check can *recognise* when a shader
declares one. `TextureShape::view_dimension` in `crates/device/src/wgpu_renderer/bindings.rs`
returns `D2` or `D2Array` and **there is no third answer**, `view_descriptor` produces a default
or a `D2Array`, and no document can name a cube payload at all. A cube would need a payload
variant, a view descriptor, a layout entry, a six face upload, and then a prefiltered specular
cube. `ImageShape::Cube` is a shape in a comparison, not a texture this renderer can make.

The landed version is a **two dimensional octahedral map**, which needs nothing new: an uploaded
image is already one flat texture with a box filtered level chain behind it
(`levels_for` and `fill_texture`), so:

- **The fold.** `p = n.xy / (|n.x| + |n.y| + |n.z|)`, and for `n.z < 0`
  `p = (1 - |p.yx|) * sign_not_zero(p)`. The sign is taken as `>= 0` rather than through
  `sign()` because `sign(0.0)` is zero in WGSL and a zero component must land on the square's
  **edge**, not its middle.
- **The read is `textureLoad`, not a sampler**, because the fold is a seam and the two sides of
  one are as far apart in direction as the map is wide. A filtered read across it blends a
  direction with the direction nearly opposite it. Because there is no sampler, the payload
  declares **no `sampled` key** and the binding takes **one** number rather than two — which the
  interface check caught on the first run, naming both sides, because the image payload builder
  was writing the key for every image.
- **The texel is clamped, not truncated.** A normal of exactly `(0, -1, 0)` gives a square
  coordinate of exactly `1.0`, which multiplied by the size is the first texel **past the end**.
  The clamp is the difference between the last texel and an out of bounds read. *Honest limit:
  this backend's out of range `textureLoad` returns the last texel anyway, so removing the clamp
  leaves the pixel test green. The clamp is argued, not proved; what the test does catch is a
  fold with the vertical axis the wrong way round, which swaps the two surfaces completely.*
- **The size is read at the level being read**, not the size of level zero. A coarse level of a
  256 texel map is four texels across.

**The map's convention, which an author needs and the schema cannot state:** the **centre** of
the square is the world's `+z` direction, the **top** row is `+y`, and the four edges are the
seams the fold puts there. There is no rotation field, so a scene whose sun is not towards `+z`
cannot put a glow at it. The sunset map sidesteps this by being a function of a direction's
**height** only — the horizon band runs the whole way round the square and so needs nothing
placed.

### How roughness becomes a level, and why it is a clamped read

```wgsl
let levels = i32(textureNumLevels(environment_map));
let level = clamp(i32(roughness * f32(levels - 1)), 0, levels - 1);
```

- The chain length is **read from the texture**, not written. It is the device's to build and it
  is a function of the image's size, so a number in the shader would be right for one map and
  past the end of another.
- The clamp is what decides the question of whether a document can ask for something impossible.
  Written this way it is a **clamped read**: a roughness of `1.0` lands on the **last** level
  rather than one past it, so the top of the roughness range is the top of the chain and no
  material is refused for wanting a blurrer environment than the map has. Had the level been
  derived and then used unguarded, the same document would be a validation failure on some
  adapters. Roughness is a **material**, not a document request, so the honest answer is that it
  cannot ask for something impossible.
- The box filter averages in **linear light** (`filtered_level` linearises the three colour
  channels of an sRGB format and leaves alpha alone), so a coarse level is a bluer environment
  rather than a grey one.

`naga` accepted `textureNumLevels` and a dynamic level in `textureDimensions` against
`Capabilities::empty()` without complaint; it is the interface check, not `naga`, that caught the
`sampled` key and the `studio` scene's stale uniform.

### One environment per scene, and why a second is refused

A lighting pass carries one `environment_map` binding and one `environment` uniform field, so a
scene naming two has one of them read and the other ignored — a light an author wrote that
nobody sees. `SecondEnvironmentLight` refuses it and names both. This is a **first version's
limit rather than a design**: a document wanting two needs a second binding and a second field,
which is additive and reversible in one place.

### The environment light is still in the light record buffer

`ClusterLight` carries the environment light like any other, so it occupies a 96 byte record it
never needs: it is exempt from clustering and gets no page, so no fragment ever loads it. The
invariant in `lighting/mod.rs` — "whether an exempt light is applied is the shading pass's
business, and a record the emitter dropped would be a light nothing could find" — is still
right for a `Directional` light, whose own path will read its record. Dropping the environment's
record would shift every index the cluster lists point into, which is not worth 96 bytes.

### Where the handoff's stage 5 plan was wrong, and it is worth saying how

The plan read: *"one environment per material, sampled as a two dimensional octahedral map...
One roughness per material means one texture with no mip chain and no schema change at all."*

Both halves of the last sentence turned out to be wrong, and in opposite directions:

- **"no mip chain" was pessimistic.** The device already derives a full box filtered level chain
  from every uploaded image (`levels_for`, `fill_texture`), so roughness became a **level index
  with nothing to build and nothing to ship**. This is the whole reason the octahedral 2D map
  beat a cube here: a cube would have needed a prefiltered specular chain *built for it*, because
  a cube's mip levels are not a blur of a direction. The plan had costed the map's own chain as
  work when it was already done, and had not noticed that the cube was the expensive one.
- **"no schema change at all" was optimistic.** Three landed: `light_pass_schema` gained the
  `environment_map` binder and the `environment` uniform field, and `texture_with_data` gained a
  `sampled` parameter so the map could declare **no** sampler and take one binding number rather
  than two. The last is the kind of change that looks like nothing and is a binding number: the
  device caught it on the first run and named both sides.

The octahedral choice itself was right, and for the reason the plan gave — it needs no new
dimension or layer field — plus the one it had not thought of, which is that a 2D image's mip
chain is already a direction blur and a cube's is not.

## Silent errors, and where configuration lives

Five things, no shader work, and the standing of each is worth saying because two of them are
about the difference between a number that is a fact and a number that is a decision.

### A scene naming no light gets one record it will never read

`lighting::buffers::light_records` reserves one record per light, so an empty `lights` list
finished into a **four byte** buffer where the shader's `array<LightRecord>` wants a whole
ninety six byte one, and wgpu refused the draw with a validation error naming a buffer size. This
was found while writing the environment test, which wanted a scene whose only light is the
environment, and was worked around there with a point light of **zero intensity** — a workaround
in a test file with its reason in a comment beside it.

**The placeholder is a record no cluster points at**: a zero colour, the origin, a zero radius,
`NO_PAGE`, and the identity projection. `NO_PAGE` and the identity are the answers an environment
light's **own** record already gives, for the same reason — it is in no cluster list, so no
fragment ever looks a page up through it. Zero colour is the one place this answer is *stronger*
than the environment's, because an environment record carries the environment's real brightness:
a record that somehow came to be read would then add a brightness nothing in the document asked
for. The thing that actually makes it unreadable is structural rather than numerical — with an
empty list every cluster's count is zero, so the loop that would load the record never runs.

**Invariant**: the record count is `max(lights, 1)`. The binding is declared unconditionally by
every lighting pass, which is the same rule the environment map follows and for the same reason
(`shaders.rs`: a binding that came and went would move every binding number after it), so the
buffer has to be a whole record wide even when every cluster's list is empty.

**The fix went in `light_records` and not in `pack`.** The proposal had said `pack`; `pack` owns
the assignment and the budgets, and `light_records` owns the shape of the record buffer. A minimum
in `pack` would be a rule about the assignment, saying nothing about the buffer.

### The page pool is a frame property, and a count is bounded by the machine

`shadow_page_width`, `shadow_page_height` and `shadow_pages` moved from the `clustering` block to
the `frame` block, beside the width and height of the image and the depth buffer that share them.
A page's size and count are properties of the frame being rendered — a pool is a set of render
targets this view draws its pages into — and an author reading their own document should be able
to tell which numbers are about this view and which are about how lights are sorted into it.

**The handoff said this changes the seven gallery scenes. It does not.** No gallery scene writes
a `clustering` block at all, so the migration was two test files and one schema. Worth recording
because "seven scenes" is the kind of number a reader would have believed.

`MAX_TEXTURE_ARRAY_LAYERS` was 256, and it stayed there for a while with documentation saying it
was a floor this compiler enforces rather than a ceiling the hardware imposes. That was already
the wrong shape, and the sentence defending it is the argument against it: a document refused at
a floor is refused *everywhere*, including on the machines that could have held it. **The
constant is gone**, along with `CompileError::PoolTooDeep`.

What replaced it is `scene::DeviceLimits` and `CompileContext::limits`, and the direction of
travel is what the old paragraph argued for: the ceiling comes **from the device into the
compile**, because the device is the only thing in the system that knows it. `Renderer` answers
`fn limits(&self) -> Option<DeviceLimits>`; `WgpuRenderer` answers from `self.device.limits()`
rather than from an adapter, because a resource is validated against the numbers the device was
created with; `NoopRenderer` answers `None`, which is a real answer and not a default one. A
compile with no machine behind it is made against `DeviceLimits::downlevel()` — 2048 and 256,
**the two numbers the constant used to hold**, which is what they always were: the smallest
answer that is true everywhere.

**All three counts are bounded rather than refused, and the two page dimensions are bounded too.**
The dimensions had no guard at all, not the constant: `pool_count` only refused a count below
one, so a scene naming `shadow_page_width: 32768` compiled, reported 256 GiB from
`LightPool::total_bytes`, and failed at `TextureDescriptor` creation — the least legible failure
this project can produce, and one that takes the whole frame with it. `frame.width` and
`frame.height` had the identical hole and are now bounded the same way.

The line between a bound and a refusal is: **a count above what the machine has is bounded and
named, and a count below one is refused.** The first is a machine being too small, which is a
fact about this run; the second is a wrong image on every machine, which is a fact about the
document. That also settles the question the proposal left open about a pool named by budget —
a clamped pool is a note, always, because the note names the number asked for, the number given
and the ceiling it hit, and an author who read it is not the failure the overflow note exists
to prevent.

The consequence sentence is supplied by the caller rather than the shared helper, because one
ceiling bounds three different things: a narrower frame is a crop of the picture asked for, a
smaller page is a coarser shadow for **every** light rather than a missing one, and a shallower
pool is a light lit by the approximation. `Frame::read` still falls back to its default for a
dimension that is not a positive integer — it did before and it is not what this changed — while
`pool_count` still refuses that as `MalformedScene`. Unifying those two is a real open question,
not a missing line.

## Directional light: a stated input, and a page for a light in no cluster

### The exemption is about a cluster's cap, and a page is not a cluster slot

This is the one place checkpoint 7 can be wrong **in a way that renders fine**, and the
reason is worth stating before anything else about it.

A directional light stands nowhere and reaches every depth equally, so it is in every cluster
by definition and `takes_cluster_slot` exempts it. `assign` then skips it entirely when it
walks the candidates, and `reached` — read off that walk in `pack` — is `false` for it. The
original page rule read `!light.clustered || !reached[index]`, so a sun was given **no page**.

That does not fail. A record's page of `NO_PAGE` means *lit unshadowed*, **by design**: the
pool is the scene's decision about how many shadows it wants, a light that lost the ranking
is a light the scene chose to approximate, and handing it a neighbour's page would be a wrong
shadow rather than a missing one. So the failure mode is a scene that renders correctly and
wrongly at once — a sun that lights everything and casts nothing, with no error, no note, and
nothing in the frame to say so.

The fix is `ClusterLight::wants_page`, which asks the question in one place because the two
halves answer from two different facts:

- a light that takes cluster slots is worth a page when the walk found it reaching one;
- a light that takes no cluster slots covers the whole view by definition, so the walk never
  sees it and `reached` is **false for a light that is on screen in every pixel** — its page
  is decided by the light itself, which is why `casts_shadow` is a field of its own rather
  than something read out of `shadow_projection`.

**A light of a kind that casts no shadow still gets no page.** The environment light is
exempt *and* casts nothing, and the two halves have to be asked apart or the environment
would be handed a page rendering nothing. That is what the new `casts_shadow` flag is for, and
it is asked apart from `shadow_projection` because a light the pool had no room for and a
light that casts nothing both carry a matrix nothing looks through.

`crates/scene/tests/directional_lights.rs` reads the page out of the **packed record** rather
than asking the assignment, because the record is what a fragment looks the page up in — the
number the test pins is then the number the shader sees. `crates/scene/tests/shadow_pages.rs`
gained the paired test at the level below it.

### Its data is a uniform field, because a record has no path to it

A light record is read through a cluster list, and a directional light is in none, so a
fragment cannot reach a record of it. It is a **stated input of the lighting pass** on exactly
the environment's terms: one `Directional` struct in `Uniforms`, carrying its colour with the
flag in the fourth component, its direction with its **page** in the fourth component, and its
orthographic matrix.

**A nested struct rather than three fields of its own.** One light is one thing, and the
record run has one entry per light for exactly this reason; a uniform reading as
`directional_colour` beside `directional_projection` would be one light spelled three times.
The fields sort as `colour`, `direction`, `shadow_projection`, between `cluster_grid` and
`environment`.

**A matrix in a uniform is fine; a thousand records in one is not.** That is the whole reason
the records are a storage buffer, and the two facts are consistent: one light's projection is
sixty four bytes and a scene's light count is unbounded.

**Written for every lighting pass, present or not.** The layout is the same for every scene, so
a field that came and went with the document would put every field after it somewhere else.
The environment map's rule, applied again.

### The page is fitted to the scene, and the handoff's reason for it was wrong

The handoff said the orthographic page "is what makes a sun's shadow the same sharpness across
the whole scene rather than sharp near the camera and soft far away", and contrasted it with
"a box looked at from where it stands".

**The contrast does not hold in this renderer.** `projection_of` builds
`orthographic × look_at`, so a **point** light's page is already a parallel projection and its
shadows are already parallel. The only kind whose page is a perspective frustum is a **spot**
light, whose cone is what gives it a field of view. And the sharpness claim is about the
**camera's** projection, not the light's, so `directional_projection` is not what makes it
true.

What the function actually decides, and what is left of the design:

- **the half width is `shadow_extent` and nothing else** — the same field a point light's is,
  meaning the same thing, so a scene mixing a lantern and a sun says which is which with one
  author's number;
- **the page is centred on the scene rather than on the light** — a point light's page is a box
  of that half width around the light reaching `distance + 2 × half_width` deep, so a scene
  wider than `shadow_extent` falls *outside* it, where a fragment reads as unshadowed;
- **the depth range is the scene's own extent along the light**, from the box's support
  interval: a sum over the three scene axes of `|direction| × half_width`, because a point of
  the box is a choice on every axis at once.

**The eye distance does not change the page.** Both near and far are passed as distances from
the eye, so moving the eye back moves them with it; and `look_at`'s rotation is the normalized
`centre - eye`, which is the light's direction at any distance. It is a number anyway because
a light that stands somewhere is a thing a reader can reason about, and it has to be behind
the whole of the bounds.

The cost of a near plane at the nearest point of the box is the same sliver
`SPOT_NEAR_FRACTION` accepts for a cone — a fragment nearer the light than the plane is
clipped out of the page and reads as unshadowed — except that here **nothing in the scene is in
that sliver**, because the plane is derived from the box.

### The record is still emitted, and the shader's `w` early-out is inert for a sun

The record a directional light is still given is dead weight, and it is kept on purpose: one
light one record is what makes a cluster index a plain index into an array, which
`environment-light-limits.md` item 4 already argues for the environment light.

The shader's `shadowed` returns unshadowed for a fragment whose `w` is not positive, because
dividing by a negative `w` would fold it back into the page from behind. That branch is the
**perspective** case alone: a page built as `orthographic × look_at` has a bottom row of
exactly `(0, 0, 0, 1)`, so every point keeps a `w` of one. `directional_lights.rs` pins that
with the bottom-right entry rather than `m[11]`, because for the perspective product `m[11]` is
`-back.z`, which is zero for any light shining straight up or down and would pass for a spot
light aimed at the floor.

### One directional light per scene, and the record is still ninety six bytes

A second is refused by name, on the environment's rule: one pass carries one field of each, and
a light silently dropped to make room is a light an author wrote and nobody sees. The refusal
lives in `read_lights` beside `SecondEnvironmentLight` because both are the same fact about two
stated inputs.

A directional light costs **nothing** in the clustered buffers: the record is still ninety six
bytes and the cone run still thirty two, because its data is a uniform field rather than
record fields. The test that says so is the same one the spot cone run needed.

## A camera that is inside something is named, and drawn

Three agents ran one task against this framework and each of them put a camera inside geometry:
one left it in a pond and got a water jar filling the frame, one grew a ridge on the camera side
and left two cameras inside a hundred metre hill, and one spent twelve rounds on a single camera
without ever getting it right. Every one of those is a fact the compile holds at the moment it
emits the frame — the eye is a field, the target is a field, and the box every instance covers
came out of the walk the camera is framed from. The scene compiler now says so in three
`CompileNote`s, and says **nothing** about the picture.

| the condition | the note points at | it names |
| --- | --- | --- |
| the eye is inside an instance's bounds | `cameras[i].eye` | the instances drawing that box, and how far in the eye is |
| one instance holds the frame and stands between the eye and the target | **that instance** | the instance, and the share of the frame it takes |
| the target is nearer than the near plane | `cameras[i].target` | the distance aimed at, and the plane |

- **All three are notes. None of them refuses to compile.** This is the page pool's policy
  generalised: *a count above what the machine has is bounded and named; a count below one is
  refused.* A machine being too small is a fact about this run; a wrong image on every machine is
  a fact about the document. All three of these are facts about this run and this camera, so all
  three are named and all three are drawn — and what a note distinguishes here is *this picture
  is degenerate* from *this picture is fine and the number is just unusual*. A scene may
  legitimately want a camera inside a volume, inside foliage, inside a building.
- **The path names the field the note is about, and the message names everything else.** A note
  that names an instance and points at the camera sends the reader past the thing the message
  just told them about, so the frame note points at the instance and the other two at the camera
  field that is wrong.
- **A camera the scene did not write says nothing at all.** `chosen` is `None` when the scene
  names no camera, or names fewer than the compile was asked for, and the frame is then the
  convention's: pulled back until the whole box fits, which puts the eye three half widths off the
  centre of the box and outside every instance's box by construction. A note about that camera
  would point at a field the document does not have and ask for a change nobody can make.
- **The share of the frame is a bound and the note says so.** The eight corners of an instance's
  box are projected and the rectangle they reach is compared with the frame, so a box turned away
  from the axes is counted by the axis aligned box around it. It reads high, which is the
  acceptable direction. A box the eye is inside, or one with a corner at or behind the eye, is
  **not measured at all**: a perspective projection divides by the distance in front of the eye,
  so the near face of such a box flips over the whole frame and any number computed from it is a
  statement about the divide.
- **Holding the frame is only a fault in front of the subject.** A still life's backdrop wall
  covers every pixel and is what a backdrop is for; the water jar filled dsf's frame because it
  stood between the eye and the target. So the note needs both halves — at least ninety percent
  of the frame, and nearer to the eye than the target is — and `stilllife` and `spotlight` are
  silent under it while the covered-frame case fires.
- **One note per box rather than one per instance.** A mesh drawn in three stages is three
  instances of one box: `terraces` draws its terrain as `terrain-depth`, `terrain-shade` and
  `terrain-cast`, which are one valley three times, and the note names all three and says so once.
- **It costs one walk that was already happening.** The per instance boxes come out of the same
  pass of `bounds_of_instances` that frames the camera, so nothing re-reads the scene document —
  which is the most expensive thing a compile can do ([`compile-cost.md`](proposals/compile-cost.md)).
  The three measurements are `Bounds::depth_of`, `Bounds::distance_from` and two methods on
  `Projection`, all of them in the `scene` crate.
- **Nothing about the picture moves.** A scene that raises one of these renders byte-identical to
  the same scene without it, checked by SHA-256 over the PNGs, and the harness's own manifest
  reports `documents identical` / `pixels identical` with the binary hash moved. Seven of the ten
  gallery scenes say nothing at all; the two that do are `terraces` and `tidewater`, whose
  heightfield is a box around a valley that the eye is inside while standing in open air — a true
  statement about a box and not a fault, which is the price of measuring bounds rather than
  surfaces, and the reason the numbers are in the message.
## A scene's size is measured, and what a pipeline constant costs at it

Three agents ran one task against this framework and two of them inherited a light falloff they
could neither see nor set — `1 / (1 + 0.0035r + 0.00002r²)`, whose own comment places it in "a
scene a couple of hundred units across" — and applied it to scenes five and fourteen times that
size, so their suns were attenuated by factors of twenty and eighty. A third built a roof from a
rise in metres against a ridge in world coordinates, computed "a 6.6-meter-tall roof in a
14-meter-wide house", and found it with a world-position image rather than by reading the code.
The compiler knows every scene's extent at the moment it emits the frame. It now says what that
extent costs, in one `CompileNote`, and says **nothing** about the picture.

- **The note points at `instances`**, because the transforms there are the only thing in the
  document that sets how far apart anything is. It names the scene's span, the fold the
  template's falloff gives at the **half diagonal of the scene's box** — the only distance the
  document determines on its own, since where a scene's lights stand is a fact about its lights —
  the same fold in stops, and the reference it is measured against.
- **It is a note and never a refusal.** A scene is legitimately any size: a still life wants a
  falloff across its table and a lake wants the same constant to be a factor of eighty. Neither
  is a fault, and refusing either would be a compiler ruling on a scene's subject.
- **The threshold was measured, and it is two** — one stop, the smallest difference in light a
  photograph is named for. The ten gallery scenes have three clusters: `studio` 1.01, `sunlight`
  1.05, `spotlight` 1.05, `tiles` 1.07; then `dunes` 2.91, `terraces` 3.51, `stilllife` 4.79,
  `lanterns` 6.09, `monoliths` 7.31; then `tidewater` 86.35. Six of ten are reported. The bar
  sits in the **first** gap, because the four small scenes are ones the constant does nothing to
  at all; the second gap was left alone, since a bar above it would report `tidewater` and
  nothing else, which is a bar fitted to one observation.
- **It is measured against the template's own constant.** The numbers are a copy, held to the
  gallery's WGSL by a test, because a note cannot be honest about a constant it does not hold
  and a copy is only honest while it is a copy. The lighting stage is being rewritten elsewhere
  and the constant lives in the arithmetic that rewrite replaces, so this is where the copy goes
  when the falloff becomes a field a scene sets.
- **It is a box and not a surface**, the same limitation the camera notes carry. `tidewater`'s
  box is nearly three thousand units across and its far shore is a few hundred away, so the box
  says 86.4 and the picture may hold a fold of two. The number is quoted rather than the verdict.
- **Nothing about the picture moves.** Verified by SHA-256 over the PNGs with the note's own code
  path disabled and enabled, on `dunes` and both `tidewater` cameras.

## A photograph is exposed once, in linear light, before the curve

Two runs lost hours to exposure, both to a transform they could neither observe nor query: one
cut it 44% and the image barely moved because a sun glow term in the grade was adding to the
sky, and found this by sampling its own PNG from PowerShell; the other darkened a clay albedo
60% and the pumpkins stayed orange, because ACES pushes bright red towards yellow-white at the
high end — 92 minutes. `exposure` is a scene field, and what it means is stated rather than
inferred.

- **It is a linear multiplier on the radiance the post stage reads, applied before that stage's
  own curve, and it is not a curve parameter.** A document that can name one number must not be
  able to mean two things; a scene that wants to move the curve names a different field.
- **It is written into the post uniform only when the scene names it**, and the post interface
  comes in two forms for that reason: `post_pass_schema()` and `post_pass_schema_exposed()`
  differ by exactly that one field. A field that came and went with the document would move every
  field after it — the layout is derived from the names — so a scene that names no exposure emits
  the documents it emitted before the field existed.
- **Naming it is a three-place opt-in**: the field on the scene, `post_pass_schema_exposed()` as
  the post shader's `pass_schema`, and `exposure: f32` in that shader's own uniform, declared in
  name order between `depth_range` and `projection`.
- **A scene that names it without declaring it is refused, and the refusal says why.** This was
  measured rather than designed: a pass's uniform is uploaded with the layout its own shader
  declares, so an undeclared field is a field the **upload drops** — the first version of this
  compiled clean and drew an unexposed frame, which is the exact class of failure the field
  exists to remove.
- **It cannot brighten what is already at one, and the field says so.** The shaded image is an
  `rgba8unorm` target, so the lighting stage clipped it before any grade saw it: an exposure above
  one brightens only what was below one. Lowering the light moves a clipped pixel; nothing an
  exposure can do will.
- **A scene that names it and runs no post process is noted**, because a field nothing reads is a
  state this project calls out as one to avoid — and the frame of a scene with no grade *is* the
  shaded image.
- **Verified on the adapter**, through a pass-through grade so the frame is the linear value:
  no exposure named and an exposure of `1.0` give a **byte-identical** frame, and `0.5`, `0.25`
  and `0.01` scale the mean channel to 0.497, 0.250 and 0.007 of it, within the rounding of an
  eight bit target. A scene's own curve is not proportional in display space, and that is the
  point of the decision rather than a defect in it.

## A shader says which file and line each of its blocks was written at

A gallery shader is a template with a scene's own WGSL spliced into it, so the string the device
parses is a module that exists on no disk. Three of the failures in the dsf run were that: a
block of **function definitions** written where a function body wanted statements, reported as
``lighting/lighting/ground` does not parse as wgsl: expected assignment or increment/decrement,
found "c_sun_share"``, which names the shader and nothing about the cause. The source is the only
part of a shader that survives the trip from a scene document to a device — the scene emitter
rebuilds the instance entry from a shader's fields with `source` and `schema` and nothing else —
so the map travels **inside** the source, as one WGSL comment line above each block that declared
where it was written.

- **The declaration is the scene's, and it is a `wgsl!` invocation.** `file!()` and `line!()` read
  as the macro expands, so the origin cannot drift from the text it describes; nothing counts
  lines and nothing is written down twice. A block written as a plain `&str` declares nothing,
  and a plain `&str` is still what every existing scene passes, so the whole mechanism is opt-in
  per block.
- **The block's first line is the line after the one the macro was invoked on**, which is where
  a literal opening on the next line puts the block's first line of text. The marker carries
  that line, and the reader adds only the offset within the block, so nothing in
  `graphics::provenance` reads a declaration.
- **There is one shape, and it is the one `cargo fmt` writes.** The literal must open on the
  line below the invocation. `line!()` reports where the invocation starts and cannot see where
  the literal opens, and two invocations holding the same literal and starting on the same line
  put the block's first line of text on different lines — so no arithmetic can be right for
  both. A literal on the invocation's own line is reported one line high, and `cargo fmt`
  rewrites it into the supported shape; the arithmetic that assumed it (`invoked_at` itself) was
  a line high for every block in the shipped scene, which named the middle of a neighbouring
  statement and, once, a WGSL comment. A test reads every `wgsl!` declaration in the gallery's
  sources and holds it to the shape.
- **The marker names the splice point as well as the file**: the same text is a `surface` in the
  lighting stage and a `grade` in the post one, and a line in a scene file of some thousands of
  lines is not a location without that.
- **A line no marker covers is reported exactly as it was.** `graphics::provenance::explain`
  returns nothing for one, which is the whole of the message when nothing declared anything. The
  scan runs only after a parse has already failed, so a shader that compiles never pays for it.
- **Nothing about a frame moves.** The marker is a comment. Verified by SHA-256 over the PNGs
  with the marker written and not written, on `dunes` and both `tidewater` cameras: three
  identical pairs.
- **It changes the text of an error, and there is no note.** The camera and scale notes describe a
  legitimate scene — the render succeeds and the frame is a smaller one than was asked for. This
  is a scene that cannot render as assembled, so the error already exists and what changes is
  where it points.
- **What it does not fix: stage scoping.** A helper block injected into every shader that uses a
  post-only symbol is a missing symbol in an unrelated stage, and nothing here prevents it: the
  gallery's builders return `Vec<Value>` rather than a `Result` so there is nowhere to refuse from,
  and `shader_entry` drops the stage tag, so the device cannot check it either. Carrying the
  splice points' stages in the marker and the shader's own stage through the emitter is the
  smallest change that would, and the first half of it is in `crates/scene`.
## A scene's own schema, and the geometry that needed it

Recorded at `7f5ff03`, from the framework half of three parallel attempts at one brief. No
scene lands with it, so this is a capability with no user yet — which is stated here rather
than left to be found.

- **A scene with more than a handful of surfaces has to put a field of its own in the instance
  row**, because the alternative is one shader and one pass per material, and a pass is the
  unit of render cost. The row is `instance_schema` merged with `custom_schema`, and a location
  is a position in that merge — so a field the scene adds **moves every field that sorts after
  it**. Nothing checks a vertex layout: the failure is a draw that reads a normal where a
  matrix is, or a renderer that refuses a one-component attribute where the shader asked for
  four. `attributes::gallery_vertex_inputs_with` is the fix, and it is a function rather than a
  discipline because the list is derived, not written.
- **The three `_with_schema` builders are one shape, not three features.**
  `lighting_shader_with_schema`, `emission_shader_with_schema` and
  `placement_shaders_with_schema` each differ from its sibling in exactly two places: the row
  they are verified against, and the locations that row lays out. The placement pair needs it
  for a reason worth stating — a shadowing or depthing fragment stage never reads the row, it
  writes depth, but its `custom_schema` is what the row is still verified against. They were
  written against the pre-`Block` shader API and **re-expressed** rather than merged.
- **A surface mask belongs in the normal's third component.** A shading normal always satisfies
  `nx² + ny² + nz² = 1`, and a terrain rolled out of a heightfield never has `nz = 1` except at
  a face pointing exactly along z, which ground does not. The component is therefore free for a
  value that is not a normal, and the caller reads it back with `mask_at` rather than
  reconstructing it. The reason to do it there rather than in a second attribute is that a mask
  recomputed in the shader from the world position is the same arithmetic written twice, and the
  two copies are free to disagree at exactly a courtyard's paving edge.
- **`Gallery::census` and `Gallery::bounds` answer for any name, not for a scene's own.** Both
  arrived written against six scenes that were not merged, and as landed they would have been
  dead code with six dead names in them. They read the **document**, never the frame, so they
  are available for a scene that does not compile — which is the half of the time a scene
  author needs them. `bounds` is of translation only: an origin box, not a geometry box.
  Folding in rotation and scale would need each placement's matrix against each mesh's own
  extent, and an origin box is what tells an author their arithmetic was wrong.
- **`INSTANCES` and `TRANSLATION` are now names rather than strings.** `INSTANCES` was a `pub
  const` in the private `scene_schema` module that the `pub use` list had never caught up with,
  and `TRANSLATION` did not exist at all: `transform_schema` spelled it inline while its sibling
  `TRANSFORM` had a const beside it. **Twelve more consts are in the same state** — `FAR`,
  `FOCAL_LENGTH`, `LIGHTS`, `MESHES`, `NAME`, `NEAR`, `POINT_SCHEMA_FIELD`, `POST_PROCESSES`,
  `PROJECTION`, `ROLL`, `SCREEN_MATERIALS`, `SENSOR_WIDTH`, `TIME` — and are left alone here,
  because that is one manual list falling behind rather than thirteen separate oversights, and
  it wants one pass rather than a change smuggled in beside a merge.
- **Not fixed, and still in the tree: fog on a sky pixel.** A post stage recovers a world
  position from the depth buffer, and for a sky pixel nothing was drawn, so the recovery runs
  out to its clamp — `terraces.rs:1015` reads `min(t, 2400.0)` — and the aerial term integrates
  2400 units of air. The honest fix is to divide height fog from Rayleigh air so each is
  integrable along an open ray in closed form. Recorded in
  [proposals/sky-fog-distance.md](../proposals/sky-fog-distance.md).

## The objects a scene is put together from, and the law about joining them

Recorded at `0a7dfe4`, from the object half of three parallel attempts at one brief. The
framework half is under *A scene's own schema, and the geometry that needed it*; no scene lands
with either, and both are capabilities with no user yet.

- **A skyline is a line of crests and saddles, not a wander**, and the sawtooth is smoothed
  *before* it is folded. A bare fold is a triangle wave, and a triangle wave along a skyline
  reads as saw teeth rather than as rock. Two folded periods, differing by a little under a
  factor of two, give crests and saddles that are not all the same size.
- **A ridge is a 1D profile along its axis multiplied by a 2D crease across it, and the two
  carry different seeds on purpose.** Sampling the along-axis noise across the ridge as well
  makes it a field of swells whose high points are wherever two coordinates happened to agree,
  and a range built from that is a field rather than a line of mountains. Two agents built this
  same landscape independently and both reached seed 131 along the axis and 139 across it,
  neither having seen the other's code; a shape two people reach separately with the same
  constants is a shape the eye already has a name for. **A mass wants a fractal profile and a
  skyline wants the closed form**, and neither substitutes for the other.
- **A heightfield's silhouette has to die down to nothing before the mesh stops**, or the far
  ground is a cliff of cut earth standing against the sky, and the only thing that can hide a
  cliff is what is drawn beyond it. Every scene that ends in weather needs this, and one
  without it has a stage set standing on a table.
- **High ground in a quarter is a valley; high ground all the way round is a bowl**, and a bowl
  puts its own wall between a camera and its subject on every side, which is a hole in the
  world rather than a valley.
- **How far out a mass stands is the whole of what makes it a range rather than a wall.** A mass
  rising from fifty units fills half the frame above the roofline at forty degrees of elevation,
  and forty degrees is a cliff three hundred metres high with a house at its foot; from a
  hundred and fifty it subtends twenty, which is a mountain in a valley.
- **A cut platform blends over a *distance*, not under a mask on the rectangle.** A mask stops
  at the edge, the bank begins as a discontinuity, and a heightfield takes its normal from its
  own height function — so a discontinuity has no normal to give, and the bank shades as a hard
  line where the paving meets the ground. Note the consequence: a blend of this kind can only
  ever ease *toward* a level, so a generator that multiplies the land by a ramp rather than
  easing it to a level cannot lift a pad to a height above the ground it is cut into.
- **A bare `floor` tears a terraced surface, and a tear shades as noise.** The heightfield's
  normals come from the height function, so the riser needs a width *in the height function* —
  which is the only place a riser can have a width at all. And nobody terraces a cliff: a
  contour stripe on a steep slope reads as snow, so the terracing is gated by slope.
- **A wall's top face is the whole reason it reads as a wall.** A run with no top is a folded
  sheet, and from any camera above the coping you look straight through it into the inside of
  the run, which in an enclosed yard is three of the six views. The two end faces are there so
  a run that stops mid-wall shows a face rather than a hole — which is what makes a gate.
- **A gate is a gap in a run rather than a boolean cut**, so there is no interior face to get
  the winding of wrong; the two runs' end faces *are* the reveal, and the gate is drawn by the
  same rule that draws the wall. The ring is one mesh because the sides are one wall as far as
  the picture is concerned: a seam at a chamfer is a seam in the only unbroken horizontal line
  in the frame.
- **Two boxes must never stop on the same plane.** A lintel run a centimetre past its posts
  buries its own end faces inside them; flush, the two runs leave coincident faces and a gate
  pier shows it as a bright vertical line up the corner of the opening, which is the one thing
  in the whole form that cannot be explained as a shadow. A coping must be **wider** than the
  wall it caps, because a cap flush with its wall is a lid, and a lid throws no shadow line of
  its own along the top of the band it caps.
- **A feature smaller than about twice the mesh cell must not be geometry.** A tile is a quarter
  of a metre and a roof's own cell is a twentieth of the run, so a ripple modelled into the
  surface is sampled twice a cycle exactly at the distances a roof is looked at from, and
  aliases into a moiré. It belongs in the surface body, drawn from the world position, where it
  can be faded out as the feature falls below a pixel.
- **A turned vessel's profile is closed**, so the inside is a real surface and the rim is a lip
  rather than an edge. An open profile gives an inside made of the outside's back faces and a
  seam at the rim. `BASKET` as first written ended at a radius rather than on the axis while
  its own documentation claimed it was closed.
- **A height function has no way to say which way is up, and a roof profile naturally reads as
  "distance from the ridge".** One of the three grew its fall *away* from the ridge and placed
  the mesh origin at the ridge height, so the eaves stood a rise above it: a trough, dressed as
  a roof. The trap is the framework's rather than that scene's, and it is the reason
  `roof_ridge` measures the fall *down* from the ridge and says so in its own documentation.
- **`foldyard` calls all of it, and building it found four things the modules got wrong.** The
  point of a generator with no caller is that nothing executes it, and the first scene to use
  `land` and `built` turned up defects that no amount of type-checking would have: a roof that
  grew into a trough, a profile whose documentation promised a closed vessel it did not give, a
  module doc promising one-mesh wall composition that privacy made impossible, and an
  `edge_normal` whose stated winding contradicted its own crossing product. Two of the three
  scenes that produced these modules also registered a scene called `bamboo` and would have
  collided had both landed. This is the argument for landing a scene early and in a separate
  branch with the modules read-only: read-only turns a silent patch into a report, and a scene
  built on a module is the only thing that will ever find out that the module is wrong.
- **A smooth gradient written out at eight bits is drawn in contour rings.** The hillside
  banding was never in the land. The land shades slowly and always along a level, so where a
  whole slope falls inside one or two quantisation steps the steps *are* the image, and they
  come out as marbling following isolines across ground that is perfectly smooth. A reviewer
  reads that as a bug in the terrain and goes looking for a region gate to remove, and finds
  nothing, because there is nothing to find: the error is introduced at the write. The fix is
  a triangular dither of one step seeded on the pixel, added as the last arithmetic the frame
  does, and it is invisible everywhere the image was not already wrong.
- **A modelled far range is erased by the air long before it is far, and the fix is not to
  model it nearer.** `f645805` gave `foldyard` a properly creased far range with a saddle
  between two crests, and the frame could not show it: it landed within **zero to five
  luminance levels of the sky**, and over one stretch *brighter* than the sky. The two causes
  compound. The range's material was a near-neutral grey deliberately matched to the haze,
  which is the correct choice and is why it has no value of its own to lose; and
  `air_closure` at three hundred and forty units is **0.615**, so sixty-one percent of it is
  replaced by the air's own colour, which by construction is the sky's colour. Three rules
  come out of it, and each is a trap rather than a bug:
  - **A far range must be **darker** than the sky behind it, not lighter.** Aerial perspective
    lifts a distant range *towards* the sky from below. A range that arrives at or past the
    sky's own value has no silhouette, and no amount of crest detail will give it one.
  - **A material chosen to match the haze and air chosen to close over it are individually
    correct and jointly fatal.** The scene is not wrong anywhere; the horizon is a patch of
    sky a shade off.
  - **The way out is to stop asking the air to carry the horizon**, not to move the range
    closer. Painted into the sky in the post stage, a ridge is *after* the air and so is inked
    rather than lit, and a mark on a surface is not hazed by that surface's own depth. That is
    `ridges::RIDGES`, which `terraces` had written privately and k3 had written before that.
- **A backdrop's height in the frame is a fact about the camera, and one scene's number is
  wrong in the next scene's frame.** `ridges` draws its layers at fixed elevations tuned for
  a camera looking level. `foldyard`'s cameras all look **down** on the yard, which puts its
  whole horizon high in the picture, so at the shared scale the painted ranges crested at
  `ray.y ≈ 0.15` against a real far range at `0.091` — the painted ones stood **taller than the
  real one behind them**, and read as a cloud bank. Hence the one parameter, and a rule
  behind it: **a shared generator's constants belong to the idea, and anything a second
  caller can reasonably want differently has to be a parameter or it will be a defect in the
  second scene wearing the first scene's clothes.**
- **A claim about a picture is measured on the picture the viewer gets.** "The silhouette now
  carries a saddle" was true of a two-times upscale made for inspection and false of the
  delivered frame, and the gap was not close: the range measured 0–5 levels against the sky
  and the crop made it look like a range. The same work, measured on the full frame at the
  size it is delivered, gave 32 levels — and it also showed the right of the same frame still
  at 7–8, which a flattering crop had hidden completely. **Judging a change by a crop is
  judging a different image.**

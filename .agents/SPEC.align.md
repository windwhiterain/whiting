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
        - each carries `source`, the instance entry the payload is in.
      - `Uniform`: uniform data, carried by `source`.
      - `Shader`: verify schema against `uniform` and `binders`.
    - **decision**: the shader's `schema` lives on the instance entry, not in the
      graphics document. The interface travels with the shader that declares it, the
      pass keeps to naming its shader, and the instance schema is open so the new field
      needs no schema edit.
    - an instance entry of kind `Shader` carries `path`, `entry_point` and `schema`,
      where `schema` is a schema document declaring the shape the shader reads.
    - **invariant**: the declared kind and the kind of the entry it names must agree, and
      a payload must be at least as large as the declaration reads. A `Buffer` declared
      over a texture, or a read past the end of a buffer, would otherwise bind
      successfully and produce nonsense.
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

### `device` crate

a server that can hot reload `Graphics`, `GraphicsInstance`, incrementally apply
`Graphics` and `GraphicsInstance`'s `Delta`, do headless rendering using http RPC.

- state is in-memory in a `Registry`, with explicit snapshot endpoints that write it
  to disk on demand. There is no filesystem watcher.
- `Renderer`: a trait the hot-reload path depends on, so the gpu backend is an
  injection point.
  - `fn render(&self, graphics: &Value, instance: &Value) -> Result<Frame>`
  - `fn available(&self) -> bool`
- `WgpuRenderer`: executes `Rasterize` and `Compute` passes headlessly and reads the
  framebuffer back. Behind the optional `wgpu` feature.
  - a rasterize pass binds one shader for both stages, because a pass has one `shader`
    field and a shader that exposes both entry points is the only thing that field can
    mean. The two entry points are `vertex_main` and `fragment_main`: WGSL has one
    namespace for functions, so a shader cannot name both `main`.
  - every pass runs in the order the document lists them, into textures shared by the
    instance entry they come from, and what comes back is the colour target of the last
    pass that drew into one. A pass whose targets are all depth formats has no image, and a
    document with no colour pass reports that rather than returning a placeholder.
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
      - `radius` is optional and means one box radius of what the scene draws, which is
        the measure the camera and the shadow volume already use. It is a field of the
        light rather than of the scene because a scene that mixes a lantern with a sun
        has to say which is which, and it is separate from `shadow_extent` because the
        two answer different questions: how far a light's *light* falls off, and how much
        of the scene its shadow map covers. A radius of zero or less is refused rather
        than read as a light that reaches nothing, which is a scene lit by nothing.
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

`crates/scene/src/gallery/` holds the WGSL library, the geometry generators and the scenes
`dunes`, `lanterns`, `monoliths`, `terraces` and `studio`. They are rendered by the harness in
`crates/device/examples/render.rs`:

```text
cargo run --release -p whiting-device --features wgpu --example render -- <scene> <out> [times] [mode]

  render dunes out/dunes                    one frame, at the time the scene carries
  render dunes out/dunes 0 5 0.25           twenty-one frames, a quarter second apart
  render dunes out/dunes 0 5 0.25 --sheet   the same range as one contact sheet
  render dunes out/dunes 0 5 0.25 --video   the same range as an mp4, encoded with ffmpeg
```

`--release` matters, though less than it did: the compile is no longer the bottleneck, but a
debug build is still several times it. The harness reports clone, validate, compile and render
apart, which is the measurement [proposals/compile-cost.md](proposals/compile-cost.md) rests on.
`ffmpeg` must be on the path for `--video`; the frames are written either way.

### Known-incomplete

Stated rather than left to be discovered:

  - **`dunes` is tuned, not finished.** The horizon has a bright seam where the field's far
    edge meets the sky, and there is faint shadow acne at the far edge. The penumbra used to
    be wider than it should be; at a 2048² shadow map over a 380-unit volume a texel is a
    fifth of a unit, which is what a step two units wide needs.
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

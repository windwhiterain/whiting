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
- **invariant**: a binding point that is one of a pass's targets is an attachment, not a
  binding. A backend treats an attachment as an exclusive use of the texture, so a pass
  that both wrote and sampled one would be rejected. The declaration is how a pass says
  what it renders into, so the same point serves as the attachment.
- **invariant**: a texture whose format is a depth format is declared to a shader with the
  depth sample type, not the float one, and a pass whose targets are all depth formats is
  built with no fragment stage.
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
  - the scene document: a `name`, a list of `lights`, a list of `shaders`, a list of `meshes`
    and a list of `instances`.
    - `Light`: position, colour and the extent of the volume it shadows.
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
    - `Instance`: a mesh placed in the scene and drawn by one stage.
      - `mesh`: the label of a mesh in the same scene, which is what lets one run of
        points be drawn by several stages without being listed again. A label no mesh
        carries is a compile error rather than a pass that draws nothing.
      - `transform: Transform`: `translation`, `rotation` in radians, `scale`. The three
        compose into the model matrix the instance row carries, in that order: scale, then
        a turn about x, then y, then z, then the translation.
      - `material: Material`.
    - `Material`:
      - `shader`: the label of a shader this scene declares, which is also where the stage
        its instance is drawn in comes from.
      - `custom_data`: an open dict, verified against the shader's `custom_schema`.
    - `Shader`: what a material is drawn with, and the interface it expects.
      - the three mesh stages and the screen stage are the variants of one enum. The spec
        nests a `MeshShader` around the three stages; a variant payload in this language is
        a struct of named fields, so the stage is the tag and the nesting is vocabulary.
      - `MeshShader`: a shader over mesh points, in one of the three stages.
        - enum of `ShadowingShader`, `DepthShader` and `LightingShader`.
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
      - **decision**: a material's shader names the stage its instance is drawn in, and
        each of the three passes draws the instances whose material belongs to it. A mesh
        that has to appear in several stages is several instances of one mesh, so its
        points are authored once. This replaced listing a mesh once per stage, which is
        what made `Instance` worth having.
      - **decision**: a pass draws one instance. A pass binds one shader and a shader
        declares one interface, so a pass that drew several instances would have to bind
        several shaders and the pass model has no room for it. More passes than a merged
        pipeline needs, in exchange for a binding model where what a shader declares is
        exactly what the pass supplies.
      - **decision**: an instance row is the model matrix and nothing else. Per instance
        data beyond a placement is not needed, because the transform places an instance and
        the material's `custom_data` carries what else a shader reads.
      - the emitted instance carries one vertex buffer per mesh, shared by every instance
        that draws it, and one instance buffer per instance, holding that instance's row
        alone.
  - `PassSchema`: the schema of a pass the standard pipeline emits.
    - `ProjectionSchema` is the schema of a projection: a view projection matrix and the
      point it was made from. It is data on the pass, not arithmetic in the renderer,
      because a reload can change the scene a projection covers and the pass has to say so,
      and because a shader reads it from the uniform it declares.
    - `ShadowingSchema`, `DepthingSchema` and `LightingSchema`, one per stage, each
      carrying a `projection` of that schema.
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

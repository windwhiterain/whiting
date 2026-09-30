# Agentic Creative Content Creation Framework

rust multi-crates workspace, each crate has publish name prefixed with "whiting" while in-workspace name use the un-prefixed one.

> This is a proposed alignment of [SPEC.md](SPEC.md) for human review and merge.
> It records the decisions taken during spec alignment and fills in the terms the
> original spec left undefined. Nothing here has been implemented yet.

## Principles

- The domain has **no Rust types**. `Graphics`, `Pass`, `Binder`, and `Scene` are
  schema documents and dynamic values, never `struct` or `enum` definitions.
- A **schema can change at runtime without recompiling rust**. Schemas are data.
- Agents read and write documents; every other layer is a pure function of them.

## Dependencies

- `definition`, `graphics`, `scene` depend on `facet` for `(de)serialization only`.
  Schema validation does not use `facet` reflection.
- `device` additionally depends on `axum` with `facet-axum`, `tokio`, and `wgpu`.
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
- `Attributes`: per-node constraints and metadata.
  - `deny_unknown_fields`: reject a field absent from the `Schema`.
  - `optional`: the field may be absent.
  - `default`: value substituted when the field is absent.
  - `extensions`: open map for domain-specific constraints.
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
    - `Append`: values to append to the end of a `List`.
      - tail only, by design: an insert at an interior index would shift every
        later index and silently invalidate the `Path`s of other `Delta`s.
    - `AppendEntries`: key-value pairs to append to a `Dict`.
    - `RemoveIndices`: indices to remove from a `List`.
    - `RemoveKeys`: keys to remove from a `Struct` or `Dict`.
  - every `Delta` carries the `version` it was computed against. A `PrimitiveDelta`
    whose `version` does not equal the current `version` is rejected as a conflict,
    so two agents can never silently clobber each other.
  - the applier rejects an out-of-bounds index, an out-of-order or duplicated
    `RemoveIndices` entry, and a `Path` that does not resolve.
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
    - `RemoveIndices` is applied in ascending index order, and its inverse appends the
      removed values in that same ascending order to restore the original ordering.
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
    - `Rasterize`
    - `Compute`
  - `Binder`: an internal indirection, not authored data. A `Pass` references a
    binder by name and the renderer resolves that name against the
    `GraphicsInstance` when building the pass.
    - enum of:
      - `Resource`: GPU resources.
        - enum of:
          - `Buffer`
          - `Texture`
      - `Uniform`: uniform data.
      - `Shader`: shader path.
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

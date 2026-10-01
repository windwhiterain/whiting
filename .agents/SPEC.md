# Agentic Creative Content Creation Framework

rust multi-crates workspace, each crate has publish name prefixed with "whiting" while in-workspace name use the un-prefixed one.

## Terms

in pseudo rust code.

### `definition` crate

- `Definition`: a general data representation.
  - nested struct-enum-list-dict structure like ".ron".
  - `Path`: the chain of key's and indices that accessing certain value in `Definition`.
  - `registry definition: FilePath`
  - a value slot is inlined value by default.
    - can be marked as `Reference`: enum of
      - `InternalReference`: a absolute or relative `Path` inside the `Definition`.
      - `ExternalReference`: a `Path` to the `registry definition`.
      - `FilePath`
    - once loaded into memory, every `Reference` must be resolved to rust reference.
  - `schema: Schema`
- `Schema`: a general schema representation to verify a `Definition` and `Delta` referencing it.
- `Delta`: a general data representation that represent how to change its `referencing_definition`.
  - `referencing_definition: Definition`: the `Definition` this delta will aplly to.
  - the structure is sparse subset of its `referencing_definition` that each value is a `Delta` to its `referencing_value`.
  - `Delta` is not a `Definition`, for simplicity.
- `PrimitiveDelta`: enum of:
  - `Replace`: schema must be same with the `referencing_value`.
    - can replace structural value.
  - `Append`: a set of values or key-value pairs to append.
  - `Remove`: a set of indices or keys to remove.
- `History`: a `Delta` replay system that generate a inverse `Delta` for each `Delta` apply to help building undo-redo system.

### `graphics` crate

provide a schema of `Definition` that represents instruction set of lowlevel grahics computation.

- `Graphics`: a `Definition` in `graphics`'s schema.
  - `instance_schema: Schema`:
  - its binders defines a corresponding schema for `GraphicsInstance`
  - `Pass`: 
    - enum of:
      - `Rasterize`
      - `Compute`
    - `uniform: Uniform`
    - `shader: Shader`
    - `binders`
  - `Binder`: retrieve data from `GraphicsInstance`, used by `Pass`:
    - enum of:
      - `Resource`: GPU resources.
        - enum of:
          - `Buffer`
          - `Texture`
          - `TextureArray`
      - `Uniform`: uniform data.
      - `Shader`: verify schema against `uniform` and `binders`.
- `GraphicsInstance`: a `Definition` provide data to its `referencing_graphics` in corresponding schema.
  - `referencing_graphics: Graphics`

### `device` crate

a server that can hot reload `Graphics`, `GraphicsInstance`, increntally apply `Graphics` and `GraphicsInstance`'s `Delta`, do headless rendering using http RPC.

### `scene` crate

provide utility to build compilers to compile `Definition` with certain schema into `Graphics` and corresponding `GraphicsInstance`, and its `Delta` version for incremental compile.

- `Compiler`: trait.
  - `scene: Schema`: the schema to compile.
  - `fn compile`
- `StandardScene`, `StandardCompiler`
  - `PassSchema`
    - `ProjectionSchema`
    - `ShadowSchema`
      - `projection: Schema`
    - `DepthSchema`
      - `projection: Schema`
    - `LightSchema`
      - `projection: Schema`
  - `Light`: enum of:
    - `Directional`
    - `Point`
    - `Spot`
    - `Environment`
  - `lights: Vec<Light>`
  - `cameras: Vec<Camera>` 
  - `textures: Vec<Texture>`
  - `meshes: Vec<Mesh>` 
    - `point_schema: Schema`: schema of point data (e.g. position, uv ...).
    - `points`, `Point`
    - `Topology`: enum of:
      - `Triangles`
      - `Segments`
  - `instances: HashMap<Pass, Vec<Instance>>`
    - `transform: Transform`
    - `mesh: Mesh`
    - `material: MeshMaterial`
  - `shaders: Vec<Shader>`
    - verify aginst `CustomSchema`.
    - enum of:
      - `MeshShader`: 
        - verify schema aginst `PassSchema` + `InstanceSchema` + `PointSchema`.
        - enum of:
          - `ShadowingShader`
          - `DepthShader`
          - `LightShader`
          - `EmissionShader`
      - `ScreenShader`
  - `mesh_materials: Vec<MeshMaterial>` 
    - `shader: MeshShader`
    - `custom_data`
  - `screen_materials: Vec<ScreenMaterial>` 
    - `shader: ScreenShader`
    - `custom_data`
  - `post_processes: Vec<ScreenMaterial>`

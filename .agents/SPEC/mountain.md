# Mountain

crate `mountain` (publish name `whiting-mountain`).

Author a mountain massif as a `Skeleton` of ridge and valley polylines, then expand it into a `FacetMesh` whose planar triangles are the rock facets. The triangle is the authoring unit: the faceted block faces of real rock are expressed directly as mesh faces, not approximated by noise.

## Principles

- dual-line control: ridges (high) and valleys (low) are both first-class editable polylines.
- derived defaults, no guarantee: generated content (`starter`, `derive_valleys`) is only an initial state for editing, never a correctness claim.
- edit through rust code: the rust api is the authoring interface; agents author by writing rust against it. A DSL is out of scope.
- the mesh mirrors the edit: `build()` reflects the current `Skeleton` exactly. No simulation, no beautification.

## Terms

in pseudo rust code.

### `Massif`

the document.

- `skeleton: Skeleton`
- `tessellation: Tessellation`

### `Skeleton`

- `ridges: Vec<Ridge>`: a forest of ridge trees.
  - `Ridge`
    - `nodes: Vec<RidgeNode>`: a polyline, summit-first.
    - `step_scale: f32`: sampling density multiplier for this polyline.
    - `attach: Option<Attach>`: `None` for a root ridge.
      - `Attach`: `{ parent: RidgeId, t: f32 }`: the child ridge's first node sits on the parent polyline at arc-length parameter `t`; its position is derived, never stored.
  - `RidgeNode`
    - `position: Vec2`: plan position.
    - `elevation: f32`
    - `half_width: f32`: flank width hint, consumed by sampling and valley derivation.
    - `sharpness: f32`: 0 = rounded ridge, 1 = knife-edge arete.
  - node attributes interpolate linearly along edges.
- `valleys: Vec<Valley>`: free polylines (no tree structure), the low control lines between ridges.
  - `Valley`: `nodes: Vec<ValleyNode>`, `step_scale: f32`
  - `ValleyNode`: `position: Vec2`, `elevation: f32`
- `footprint: Vec<FootprintNode>`: closed boundary polyline with elevations, delimits the massif domain and forms the mesh rim.
  - `FootprintNode`: `position: Vec2`, `elevation: f32`

### Invariants

checked by `validate() -> Vec<Violation>`, never enforced mid-edit: an edit may pass through invalid states and the agent repairs them explicitly.

- elevation is non-increasing along every root-to-leaf ridge path.
- a child ridge's first node elevation equals the parent edge elevation interpolated at `attach.t`.
- no polyline self-intersects; valleys never cross ridges; everything stays inside the footprint.

### Derivation

- `starter(seed: u64, params: StarterParams) -> Massif`: a procedural initial ridge tree (main ridge with recursive spurs) plus derived valleys and a footprint. Initial state only.
- `derive_valleys(massif: &mut Massif)`: regenerate `valleys` as the approximate dual of the ridge network (medial lines between neighboring ridges, clamped to the footprint). Replaces existing valleys. Initial state only.

### `Tessellation`

- `step: f32`: base sampling step along polylines; effective step per polyline is `step * step_scale`.
- `diagonal: DiagonalRule`: default diagonal choice for interior quads.
  - enum of: `StrataDip`, `Shortest`, `Alternate`
- `overrides: Vec<DiagonalOverride>`: explicit diagonal choices from `flip_diagonal`; an override survives rebuilds while its quad still exists.

### `FacetMesh`

the output of `build()`.

- `points: Vec<Vec3>`
- `faces: Vec<Face>`
  - `Face`: `{ indices: [u32; 3], attributes: FaceAttributes }`
  - `FaceAttributes`: `{ strata: StrataId, dip: Vec2, dip_angle: f32, ruggedness: f32 }`
- guarantee: every face is exactly planar. Elevation is only ever sampled on skeleton lines; triangle interiors are linear interpolations, never displaced.

### `build()`

1. sample ridges, valleys and footprint into constraint segments; elevation and attributes interpolate per sample.
2. constrained Delaunay triangulation of the plan positions, constraint edges forced.
3. resolve each interior quad diagonal by `Tessellation::diagonal`, then apply `overrides`.
4. face attributes inherit from the enclosing skeleton region (which ridge flank, which valley side).
5. deterministic: the same `Massif` always builds the same `FacetMesh`.

### Ops

plain rust functions on `Massif`; small, explicit, composable.

- skeleton: `add_ridge`, `attach_ridge(parent, t)`, `move_node`, `insert_node`, `remove_node`, `remove_ridge`, `set_node`, `set_step_scale`
- valleys and footprint: the same polyline ops
- tessellation: `flip_diagonal(quad)`, `set_diagonal_rule`
- derivation: `starter`, `derive_valleys`
- `validate() -> Vec<Violation>`, `build() -> FacetMesh`

### Out of scope

- erosion, simulation, shading, texturing: downstream pipelines consume `FacetMesh`.
- DSL, serialization, undo/redo, integration with `definition` / `scene` / `graphics`.
- LOD and runtime streaming.

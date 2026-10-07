# Mountain (alignment)

> Agent-side design elaboration of [SPEC/mountain.md](../SPEC/mountain.md).
> Everything here is my proposal, not user requirement. The spec fixes *what*
> (dual-line skeleton, expansion to a triangle facet mesh, rust api, raw output);
> this document explores *how*.

## Data model

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
      - `Attach { parent: RidgeId, t: f32 }`: the child ridge's first node sits on the
        parent polyline at arc-length parameter `t`. Its position and elevation are
        derived from the parent edge, never stored — this makes the "child joins the
        parent flank" relation structural instead of coincidental, and moving the
        junction is editing `t`.
  - `RidgeNode`
    - `position: Vec2`: plan position.
    - `elevation: f32`
    - `half_width: f32`: flank width hint, consumed by sampling and valley derivation.
    - `sharpness: f32`: 0 = rounded ridge, 1 = knife-edge arete.
  - node attributes interpolate linearly along edges.
- `valleys: Vec<Valley>`: free polylines (no tree structure), the low control lines
  between ridges.
  - `Valley`: `nodes: Vec<ValleyNode>`, `step_scale: f32`
  - `ValleyNode`: `position: Vec2`, `elevation: f32`
- `footprint: Vec<FootprintNode>`: closed boundary polyline with elevations. Delimits
  the massif domain and forms the mesh rim, so the massif can be stamped into a larger
  terrain without a skirt heuristic.
  - `FootprintNode`: `position: Vec2`, `elevation: f32`

Design notes:

- Valleys are deliberately *not* a tree. A valley network is the hydrological dual of
  the ridge tree and often has its own branching logic; forcing one tree structure on
  both couples two things the author wants to control separately. Free polylines keep
  the model honest; structure can be imposed later if a use case needs it.
- The spec says "tree or network". The tree is the primary case; the network case
  (loops: cirques, calderas, spur loops) can be layered on later as *tie edges* between
  two existing ridge nodes, leaving the tree intact for traversal and elevation
  ordering. See open questions.

### Invariants

checked by `validate() -> Vec<Violation>`, never enforced mid-edit: an edit may pass
through invalid states and the agent repairs them explicitly. Enforcing per-step would
make legitimate multi-step edits (e.g. lowering a summit below its child before raising
the child) impossible to express.

- elevation is non-increasing along every root-to-leaf ridge path.
- a child ridge's first node elevation equals the parent edge elevation at `attach.t`.
- no polyline self-intersects; valleys never cross ridges; everything stays inside the
  footprint.

## The junction problem

The hardest part of expanding a polyline tree into a facet mesh is where a child ridge
joins its parent. Worth stating precisely, because it shapes the data model.

What happens geometrically: the child ridge's first node sits on the *flank* of the
parent ridge (on the parent polyline's plan position, at parent-edge elevation). Around
that junction point, facet fans of three ridges meet: the parent's two flanks (one of
which the child lands on) and the child's two flanks. The triangles around the junction
must:

1. share the junction vertex exactly — no T-junction cracks, no duplicated vertex with
   a slightly different elevation;
2. keep every incident face planar — automatically true for triangles, but only if the
   junction vertex is a real vertex of the triangulation rather than a point projected
   onto a face afterwards;
3. transition the parent's flank facets into the child's flank facets without a visible
   seam — the junction is where a new sharp edge (the child ridge) is *born* out of an
   otherwise smooth facet fan.

Approach: the junction is materialized as a vertex of the parent polyline before
triangulation. Attaching at parameter `t` splits the parent edge at that point, so the
constrained triangulation sees the junction as an ordinary shared vertex and everything
downstream (constraint edges, face attribution) works uniformly. This is why
`Attach.t` is stored instead of a position: re-materializing the junction after a
parent edit is just re-evaluating the parameter.

Consequences and open sub-problems:

- **Sampling alignment.** The parent edge is sampled at `step`, but the junction vertex
  must land exactly at `t`. The sampling of a parent edge is therefore anchored at its
  junctions: junction positions are fixed samples, and the intervals between them are
  subdivided independently. Deterministic, and local edits re-sample locally.
- **Facet handoff.** On the parent's flank, the facet fan runs down to the valley; past
  the junction, the child's fan takes over part of that slope. The triangular faces
  near the junction inevitably span "parent-flank" and "child-flank" territory — face
  attribution there is by nearest constraint edge, and the author can flip diagonals
  (below) to control where the visual break lands.
- **Junction sharpness.** A real spur junction is a small V-notch in the parent flank,
  not a knife-cut. This is expressible as a junction attribute (notch depth/width)
  applied to the junction vertex's neighborhood; deferred until the basic model proves
  itself.
- **Network case.** A tie edge (cirque loop) creates a junction at *both* ends plus a
  closed ring of constraint edges. No new mechanism is needed for triangulation, but
  elevation ordering is no longer a tree invariant — validate must check consistency
  along cycles instead. Deferred; tree-first.

## Expansion: `build()`

1. **Sample.** Ridges, valleys and footprint become constraint segments. Junctions
   anchor parent-edge sampling (above). Elevation and node attributes interpolate
   linearly per sample. Effective step per polyline is `tessellation.step *
   polyline.step_scale`.
2. **Triangulate.** Constrained Delaunay triangulation of the plan positions with all
   sampled segments forced as constraint edges. CDT rather than plain Delaunay because
   ridge and valley lines *are* the sharp edges of the mountain — if a ridge segment
   were not a mesh edge, its two flanks would blend across it and the whole premise
   (triangles as block faces meeting at ridges) collapses.
3. **Resolve diagonals.** Each interior quad bounded by constraint edges has two
   possible diagonals; the choice decides how the two triangles read as block faces
   (which way the facet pair tilts, hence how it catches light). Default choice by
   `Tessellation::diagonal`:
   - enum of: `StrataDip` (diagonal follows the strata dip direction — geologically
     motivated default), `Shortest` (minimizes edge length — safest), `Alternate`
     (staggered — breaks up repetitive facet rhythm).
   - `flip_diagonal(quad)` records a `DiagonalOverride`; an override survives rebuilds
     while its quad still exists. Diagonal control is a first-class authoring
     operation, not an implementation detail.
4. **Attribute faces.** Each face inherits `FaceAttributes` from its enclosing skeleton
   region (which ridge flank / which valley side), plus provenance: which constraint
   edges bound it.
   - `FaceAttributes { strata: StrataId, dip: Vec2, dip_angle: f32, ruggedness: f32 }`
5. **Stay deterministic.** The same `Massif` always builds the same `FacetMesh` —
   sampling, triangulation (tie-breaking), and diagonal resolution are all seeded or
   fully ordered. Agents iterate by diffing code and mesh; nondeterminism would make
   every diff noisy.

### `FacetMesh`

- `points: Vec<Vec3>`, `faces: Vec<Face>`, `Face { indices: [u32; 3], attributes: FaceAttributes }`
- guarantee: every face is exactly planar. Elevation is only ever sampled on skeleton
  lines; triangle interiors are linear interpolations, never displaced. This is the
  contract that keeps the block-face look intact for downstream pipelines.

## Derivation (the "no guarantee" defaults)

- `starter(seed, params) -> Massif`: procedural initial ridge tree — a main ridge with
  recursive spurs at plausible angles, elevations decreasing outward — plus derived
  valleys and a footprint. Initial state only.
- `derive_valleys(&mut Massif)`: regenerate valleys as the approximate dual of the
  ridge network: medial lines between neighboring ridge polylines (a Voronoi diagram of
  the sampled ridge segments, pruned and clamped to the footprint), with elevations
  from a default valley profile. Replaces existing valleys, so it is a "start over"
  operation, not an incremental one. Initial state only.

Both are deliberately cheap and honest: their value is giving the author (human or
agent) something concrete to edit, which is exactly R3.

## Ops

plain rust functions on `Massif`; small, explicit, composable.

- skeleton: `add_ridge`, `attach_ridge(parent, t)`, `move_node`, `insert_node`,
  `remove_node`, `remove_ridge`, `set_node`, `set_step_scale`
- valleys and footprint: the same polyline ops
- tessellation: `flip_diagonal(quad)`, `set_diagonal_rule`
- derivation: `starter`, `derive_valleys`
- `validate() -> Vec<Violation>`, `build() -> FacetMesh`

The api surface doubles as the agent's action vocabulary: an agent authors by writing
rust that calls these, reads `validate()`'s violations, and inspects `build()`'s mesh.
This is why ops stay granular — coarse "smart" ops would hide decisions the agent is
supposed to make.

## Open questions

- **Multi-massif composition.** Two neighboring massifs whose footprints touch: union
  of meshes, or a shared footprint boundary with matching samples? Probably the latter,
  deferred until a second massif exists.
- **Id stability.** `build()` re-triangulates, so face indices shift after any edit.
   Provenance (which constraint edges bound a face) is the stable handle, not the
   index. Whether downstream needs stronger stability is unknown.
- **CDT robustness.** Collinear samples and near-degenerate quads are inevitable in
   hand-edited skeletons. Tie-breaking rules must be part of `build()`'s determinism
   contract, not left to the triangulation library's defaults.
- **Adaptive sampling.** Curved ridges want curvature-adaptive steps; junctions want
   anchored samples; both fight the uniform `step`. Start uniform + `step_scale`,
   revisit with real mountains.
- **Relation to `scene`.** A `FacetMesh` could later compile into a `scene` `Mesh`
  (points + `Topology::Triangles`) for preview rendering. Out of scope per R5/R6, but
  the attribute layout should not preclude it.

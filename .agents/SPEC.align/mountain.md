# Mountain (alignment)

> Agent-side design elaboration of [SPEC/mountain.md](../SPEC/mountain.md).
> Everything here is my proposal, not user requirement. The spec fixes *what*
> (dual-line skeleton, correspondence, direct expansion to a triangle facet mesh, rust api,
> raw output); this document explores *how*.
>
> **Revision note.** The first version of this document proposed a constrained Delaunay
> triangulation of the sampled lines. That was rejected: if a valley node and a ridge node
> carry their mapping at edit time, expansion is a direct walk and there is nothing to
> triangulate. R4.1 is that requirement written down. What it buys is not only speed: a
> CDT makes the mesh's watertightness a *property of a search* — two faces meet along a
> ridge only because the search happened to recover the same edge — where a walk makes it
> structural. The junction problem below is kept, because the reason it is no longer a
> problem is the whole argument for the correspondence.

## Data model

### `Massif`

the document.

- `skeleton: Skeleton`
- `tessellation: Tessellation`

### `Skeleton`

- `ridges: Vec<Ridge>`: a network of ridge lines.
  - `Ridge`
    - `nodes: Vec<RidgeNode>`: a polyline, summit-first.
    - `step_scale: f32`: sampling density multiplier for this polyline.
    - `start: End`, `end: End`: where the two ends stand. See below.
  - `RidgeNode`
    - `position: Vec2`: plan position, **overridden at an end that names a station**.
    - `elevation: f32`, likewise overridden at such an end.
    - `half_width: f32`: flank width hint, consumed by sampling and valley derivation.
    - `sharpness: f32`: 0 = rounded ridge, 1 = knife-edge arete.
  - node attributes interpolate linearly along edges.
- `valleys: Vec<Valley>`: free polylines (no tree structure), the low control lines
  between ridges.
  - `Valley`: `nodes: Vec<ValleyNode>`, `step_scale: f32`, `start: End`, `end: End`
  - `ValleyNode`: `position: Vec2`, `elevation: f32`, `left: Station`, `right: Station`.
    **The two stations are the correspondence**; see below.
- `footprint: Vec<FootprintNode>`: closed boundary polyline with elevations. Delimits
  the massif domain and forms the mesh rim, so the massif can be stamped into a larger
  terrain without a skirt heuristic.
  - `FootprintNode`: `position: Vec2`, `elevation: f32`

### `End`

Where one end of a line stands.

- `End::Free`: the line's own first or last node stands there.
- `End::On(Station)`: the end stands on another line, at that station.

Design notes:

- **`End` replaces `Attach`.** The old `Attach { parent: RidgeId, t: f32 }` said one thing — a
  child ridge's start sits on a parent's flank — and the model needs that thing *for both ends of
  both kinds of line*, so it is one enum rather than two mechanisms. A spur on a parent's flank is
  `On(Station::ridge(parent, t))`; a valve mouth on the rim is `On(Station::footprint(p))`; a
  **tie edge** is a line whose two ends are both `On`, which is the network case.
- **An end that names a station is the same vertex as that station.** This is not a nicety of the
  representation, it is the reason a junction is a junction: eight ridge ends all naming one
  ridge's summit resolve to *one sample of that ridge*, so the ring of triangles round the apex
  shares a vertex and the surface closes. Eight ends that merely stood at the summit's coordinates
  would leave sixteen vertices and a **zero-area slit** — a hole that the plan-area comparison
  cannot see, because a hole with no area contributes no area. The old model had exactly that bug
  and its eight roots were eight connected components, which is neither a tree nor a network.
- Valleys are deliberately *not* a tree. A valley network is the hydrological dual of the ridge
  network and often has its own branching logic; forcing one structure on both couples two things
  the author wants to control separately. Free polylines keep the model honest.
- The **network** case (loops: cirques, calderas, spur loops) is therefore expressible with no new
  mechanism, and `tie_ridge` is the op that makes one. What it still needs is the valleys that
  flank the tie: a tie drawn across a wedge lands inside a region another valley already tiles, so
  an author adding one hand-binds its two flanks. The generator does not derive them.

## The correspondence

This replaces the CDT. It is the one idea the rest of the design hangs off.

### `Station`

A point named on a polyline, by arc-length parameter.

- `Station { line: LineRef, t: f32 }`
  - `LineRef`: `Ridge(RidgeId)`, `Valley(ValleyId)` or `Footprint`.
  - `t`: arc-length parameter, `0.0` at the line's first node, `1.0` at its last.
- A station's plan position and elevation are **derived** from the named line, never
  stored. This is the same rule `Station` already follows, and it is why moving a ridge
  moves everything pinned to it: there is no second copy to move.
- **Every line is listed, including valleys.** A valley-valley pair is what closes the
  region beyond a ridge tip, so a station that could only name a ridge would leave that
  region unbuildable and would need a special case to say so.

### `ValleyNode.left` / `.right`

The two lines the valley node sits between. They are the **ribs**: the lateral mesh edges
running from a valley vertex up the flank to the ridge.

Why the ribs live on the valley node and not on the ridge node: a ridge node is the crest,
and the surface leaves it in every direction down the two flanks. A *valley* node is the
point at which two flanks meet, so it is the only place where "which two lines bound me"
is a fact about the node rather than a search. Written on the ridge instead, every ridge
node would need to know how many valleys face it, and that number changes as the author
adds spurs — the coupling the tree was supposed to avoid.

Consequences of a node-level correspondence:

- A child ridge's first station *is* its `Station`, so a belt whose two stations name
  parent-then-child has a top path that turns through the junction. Its first row
  collapses to a triangle exactly where the spur is born. No junction machinery.
- A summit closes the same way: the first valley node of every valley leaving the summit
  names `t = 0.0` on the ridge, so all their top rows collapse to the summit point.
- A ridge tip closes the same way, on the two valleys that flank it.

### Invariants

checked by `validate() -> Vec<Violation>`, never enforced mid-edit: an edit may pass
through invalid states and the agent repairs them explicitly. Enforcing per-step would
make legitimate multi-step edits (e.g. lowering a summit below its child before raising
the child) impossible to express.

- elevation is non-increasing along every root-to-leaf ridge path, and along every valley.
- a child ridge's first node elevation equals the parent edge elevation at `Station`.
- **the station parameters on each side of each line tile that line's `[0, 1]`**: no gap
  (which would be a hole in the mesh) and no overlap (which would be a doubled surface).
  The two flanks of a ridge are tiled independently, because the two valleys facing one
  ridge are free to subdivide it differently.
- **consecutive stations on one side either name the same line with increasing `t`, or
  name two lines whose named points coincide.** The second case is a corner: a junction,
  a ridge tip meeting the footprint, a valley mouth. Anything else is a correspondence
  that cannot be walked.
- no polyline self-intersects; a rib crosses no line and no other rib; everything stays
  inside the footprint.

The last one is the expensive check and the one that catches an authored correspondence
that is geometrically nonsense even though it is topologically walkable.

## The junction problem

Kept from the previous revision, because it is the best statement of what the
correspondence is *for*.

What happens geometrically at a junction: the child ridge's first node sits on the *flank*
of the parent ridge. Around it, facet fans of three ridges meet: the parent's two flanks
and the child's two flanks. The triangles there must:

1. share the junction vertex exactly — no T-junction cracks, no duplicated vertex with a
   slightly different elevation;
2. keep every incident face planar — automatically true for triangles, but only if the
   junction vertex is a real vertex of the mesh rather than a point projected onto a face
   afterwards;
3. transition the parent's flank facets into the child's flank facets without a visible
   seam.

Under a CDT, (1) and (2) are the hard part and are why the junction had to be
*re-materialized* as a vertex of the parent polyline before triangulating. Under the
correspondence they are free: the junction vertex is **the sample of the parent line that the
child's end names**, one vertex with one position, and both the parent's belt and the child's
belt are built from it. Nothing has to be welded afterwards, because nothing was ever two
things.

(3) remains a shading question and is answered by the same thing that answers it for any
crest: the two belts on either side of the child ridge are separate planar facets, and the
crease between them is the child ridge.

Sub-problems that survive, in reduced form:

- **Sampling alignment.** A ridge's samples must include every station parameter that
  names it *and* every end that stands on it, so an end resolves to an actual vertex. The
  intervals between anchors are subdivided independently. Deterministic, and a local edit
  re-samples locally.
- **Facet handoff.** Near a junction a belt spans "parent-flank" and "child-flank"
  territory. Attribution there is by the belt's own two lines; the author controls where
  the visual break lands by where the stations sit.
- **Junction sharpness.** A real spur junction is a small V-notch in the parent flank, not
  a knife-cut. Expressible as a junction attribute (notch depth/width) applied to the
  junction vertex's neighborhood; deferred until the basic model proves itself.

## Expansion: `build()`

1. **Sample.** Every line becomes a vertex run: its own nodes, plus an anchor at every
   parameter that some station or line end names on it, plus a subdivision of each interval
   at `tessellation.step * line.step_scale`. Anchors are why a station resolves to a
   vertex and not to a point on an edge. Elevation and node attributes interpolate
   linearly along each edge.
2. **Belt.** For each valley, for each side, for each consecutive pair of valley nodes
   `(i, i + 1)`:
   - the **top path** is the sampled run between `node[i].side` and `node[i + 1].side`.
     When the two stations name different lines, the run is followed through their shared
     point, so a junction is a corner in the path and not a discontinuity;
   - the **bottom run** is the valley's own samples between the two nodes;
   - the belt is the two paths **zipped** (below).
3. **Zip.** Two polylines with independent vertex counts become a triangle strip by always
   stepping the one that is behind in arc-length progress, emitting one triangle per step.
   This is the whole triangulator. It is O(vertices), it is deterministic, and it needs no
   two lines to agree on a vertex count — which is what lets the two valleys facing one
   ridge subdivide it differently without a T-junction, because the strip's boundary *is*
   the input polyline.
4. **Resolve diagonals.** A belt is a strip, and the diagonal of each of its cells is
   decided by which side the strip steps on. `Tessellation::diagonal` picks the rule:
   - `Progress` (default): always step the side that is behind. The straightest strip.
   - `Alternate`: on a near-tie, step the side that was not stepped last, which staggers
     the strip and breaks up a repetitive facet rhythm.
   - `flip_diagonal(belt, cell)` records a `DiagonalOverride`, and an override survives
     rebuilds while its cell still exists. Diagonal control is a first-class authoring
     operation, not an implementation detail.
5. **Attribute faces.** Each face inherits `FaceAttributes` from the belt it came from,
   plus provenance: which two lines bound the belt.
   - `FaceAttributes { top: LineRef, valley: ValleyId, dip: Vec2, dip_angle: f32,
     ruggedness: f32 }`
   - `dip` and `dip_angle` are read off the face's own normal — the fall line of the
     block face. `ruggedness` is the ridge's `sharpness` interpolated at the belt.
   - The previous revision's `strata: StrataId` is **dropped, not deferred into a stub**:
     nothing in the `Skeleton` authors a stratum, so the field had no source and a
     default value would have been a claim about rock the document never made. A strata
     layer is a future authored field with its own requirement.
6. **Stay deterministic.** The same `Massif` always builds the same `FacetMesh`. Sampling,
   belt order, zip stepping and flip resolution are fully ordered; no hashing of floats,
   no iteration over unordered maps. Agents iterate by diffing code and mesh;
   nondeterminism would make every diff noisy.

### `FacetMesh`

- `points: Vec<Vec3>`, `faces: Vec<Face>`, `Face { indices: [u32; 3], uv: [[f32; 2]; 3], attributes: FaceAttributes }`
- guarantee: every face is exactly planar. Elevation is only ever sampled on skeleton
  lines; triangle interiors are linear interpolations, never displaced. This is the
  contract that keeps the block-face look intact for downstream pipelines.
- guarantee: the mesh is watertight, and its plan area equals the footprint's. A hole or
  an overlap is a number that disagrees, which is a test, not a judgement. This is the
  check that makes "the author got the correspondence right" measurable.
- `uv`: **where the face is in its belt's own parameter space**, one pair per corner, in
  `indices` order. `u` is the fraction of the belt's upper boundary walked **by distance**,
  `v` the fraction from that boundary down to the lower one — crest zero, valley one.

  It is written beside the indices rather than beside the attributes because it is a fact
  about the same three points, and a caller zipping two lists by hand would have a second
  thing to get wrong.

  - **decision**: `u` is **distance, not sample index** (2026-10-03). A belt's top path turns a
    corner where a spur leaves its parent's flank, and there the two disagree — the path walks a
    different line, spaced by that line's own `step_scale`. An index fraction would make `u` a
    fact about the tessellation, so re-tessellating the same crest would answer a different `u`
    for the same piece of rock. "62% along the crest" is the sentence an author means.
  - **invariant**: the uv is **permuted with the winding**. `push_face` fixes a face's winding by
    its own plan area, and reversing the indices without reversing the uv would leave every
    reversed face reporting a point on the far side of its belt — an answer that looks entirely
    plausible and is the wrong half of the surface.
  - **invariant**: the space is defined **once, here**, and a scene writes what
    [`FacetMesh::locate`]'s inverse reads rather than choosing its own. A definition written
    down twice is wrong silently, and wrong by a rotation of the triangle.
- `fn locate(&self, face: &Face, uv: [f32; 2]) -> BeltPoint`: the inverse, **O(1)** per pixel.
  It reads the three corners a rasteriser interpolated between and returns the belt and the
  position on it, barycentric by subtriangle area so it needs no assumption about the winding.
  A degenerate uv triangle — which is what a summit that closes produces, a whole row of corners
  sharing one coordinate — falls back to the corner rather than dividing by nothing.

  - **why it is in this crate**: the mesh is **already the index**. A face carries its belt and
    the two lines that bound it, so the only hop that did not exist was *pixel → that face*.
    Putting the belt id in the image instead would add a fifth number that has to agree with
    this table forever, which is the one thing the arrangement is designed to prevent. See
    [proposals/agent-content-identity.md](../proposals/agent-content-identity.md).

## Derivation (the "no guarantee" defaults)

- `starter(seed, params) -> Massif`: procedural initial ridge tree — a main ridge with
  recursive spurs at plausible angles, elevations decreasing outward — plus valleys and a
  footprint, **all three generated together**, because a valley and the stations that pin
  it are one decision. Initial state only.
- `derive_valleys(&mut Massif)`: regenerate valleys from the current ridge tree using the
  same wedge walk, replacing the existing valleys. Valleys are derived per **wedge**: at
  each ridge junction and at each summit, the wedge between two adjacent ridge flanks
  opens down-slope, and one valley runs out of it to the footprint. Its stations land on
  the two ridges at the wedge's mouth and on the footprint at its rim. Replaces existing
  valleys, so it is a "start over" operation, not an incremental one. Initial state only.

Both are deliberately cheap and honest: their value is giving the author (human or agent)
something concrete to edit, which is exactly R3.

## Ops

plain rust functions on `Massif`; small, explicit, composable.

- skeleton: `add_ridge`, `attach_ridge(parent, t), tie_ridge(from, to)`, `move_node`, `insert_node`,
  `remove_node`, `remove_ridge`, `set_node`, `set_step_scale`
- correspondence: `bind(valley, node, side, station)`, `unbind`, `slide_station`
- valleys and footprint: the same polyline ops
- tessellation: `flip_diagonal(belt, cell)`, `set_diagonal_rule`
- derivation: `starter`, `derive_valleys`
- `validate() -> Vec<Violation>`, `build() -> Result<FacetMesh, BuildError>`

The api surface doubles as the agent's action vocabulary: an agent authors by writing
rust that calls these, reads `validate()`'s violations, and inspects `build()`'s mesh.
This is why ops stay granular — coarse "smart" ops would hide decisions the agent is
supposed to make. `bind` is the op the correspondence exists for, and it is a first-class
edit like `move_node` rather than something derived on the author's behalf.

`build()` returns a `Result` rather than panicking on an unwalkable correspondence: the
author is an agent iterating on code, and a violation list is more use than a backtrace.

## Open questions

- **Multi-massif composition.** Two neighboring massifs whose footprints touch: union
  of meshes, or a shared footprint boundary with matching samples? Probably the latter,
  deferred until a second massif exists.
- **Id stability.** `build()` re-expands, so face indices shift after any edit.
  Provenance (which belt, which two lines) is the stable handle, not the index. Whether
  downstream needs stronger stability is unknown.
- **Tiling check cost.** "Stations tile each line side" wants a sort per line side. Fine
  at authoring scale, and it is the check that turns a hole into a violation, so it is
  worth its cost.
- **Adaptive sampling.** Curved ridges want curvature-adaptive steps; junctions want
  anchored samples; both fight the uniform `step`. Start uniform + `step_scale`, revisit
  with real mountains.
- **Relation to `scene`.** A `FacetMesh` compiles into a `scene` `Mesh` (points +
  `Topology::Triangles`) for preview rendering, downstream of this crate: the gallery's
  `landscape` scene is the first caller, and it converts each face into three points carrying the
  face's own normal, because a shared or averaged normal would shade a block face as part of a
  smooth surface. That conversion is the evidence that the attribute layout does not preclude it.
  What the layout gave up: the standard point schema has a normal, a position and a two component
  texture coordinate, so the face's `ruggedness` and its steepness travel in the texture slot —
  this mesh samples no texture, and those are the two numbers the crate knows that a shader could
  not recover from the geometry.
- **A fan is not a massif.** What `starter` and `derive_valleys` generate is radially symmetric:
  every ridge has the same plan length, because a wedge's two stations have to reach their tips at
  the same point on the bisector, and a ring of equal ridges reads as a cone however its crests
  are profiled. A main ridge with spurs of different lengths wants an authored skeleton — which is
  what the crate is for and what `landscape` does by editing the generated fan — and, for
  `starter`, a wedge walk that measures the ridge tree's planar embedding rather than assuming a
  fan.

## Amendments (2026-10-07, rewriting `landscape`)

Three things the scene work found that the design above does not say, recorded here because each
one changed what could be built rather than only how.

### A spur hung off a parent's flank cannot be built, and the reason is structural

`attach_ridge(parent, t > 0, …)` and `Ridge::child(parent, t, …)` both take a parameter for a
position along the parent, so the model reads as though a ridge may leave a parent's flank
anywhere along it. **It cannot.** Two independent reasons:

1. **Elevation.** Non-increasing along every root-to-leaf path means a connected skeleton has
   exactly one summit, and a parent and a spur at `t > 0` do not share it.
2. **The wedge doubles on itself**, which is the one that is not obvious. The fan of triangles
   around a junction closes only when the wedge between the parent and the spur has its **first
   row collapsed onto the summit** — a degenerate row is what closes it. But that row leaves a
   boundary edge running from the summit to the spur's junction, and that edge lies *along the
   parent ridge*, whose own flank the same wedge is simultaneously covering on its other side.
   The surface folds onto itself along the parent's upper segment.

So the reachable topology is a **fan of tips**: every ridge from the one summit, wedges between
angular neighbours, and asymmetry carried by the crests being genuinely different from each other
rather than by branching. The hierarchy is still authored — every ridge after the first is a
`Ridge::child` of the first, at `t = 0.0` — and it is what the hierarchy is *for* in this model,
but it is a naming of parenthood and not a statement about geometry.

**Whether a spur is worth a mechanism.** The argument above is a statement about this expansion,
not a proof that a spur is inexpressible. A `t > 0` spur becomes buildable if the wedge's first
row is allowed to collapse onto the junction rather than the summit — which means the parent and
the spur share a summit by construction (a child whose first station is its parent's summit, and a
parent that runs a *second* crest line along the spur's flank). That is a different model, not a
bug fix, and it would need its own requirement.

### `sample_line` read a line's own nodes back interpolated

Found while authoring unequal reaches, and the symptom was a `Disjoint` whose cause was invisible:
a crest tip that is also a rim vertex is the same authored point on two lines, and both lines'
samplers were returning it about a ten-thousandth of a unit apart. `weld_key` quantizes at
`1/1024`, so whether the two agreed on a cell was arithmetic rather than intent — and the massif
validated on one set of crest numbers and not on another.

The parameter is a rounded `arc / total` and `edge_at` recovers the amount from it, so at a node
the amount lands a rounding error short of exactly zero or exactly one, and the lerp returns a
point beside the node rather than the node. **Fixed**: a break within `PARAMETER_EPSILON` of a
node's end of an edge is read from that node. The general form of the rule is that a node is a
vertex of the mesh and the mesh may only have one of it.

### `WELD` is documented as `1.0e-2` and `weld_key` implements `1/1024`

Not changed, and it should be a decision rather than a patch. `WELD` says how far apart two plan
points may be and still be one corner; `weld_key` hashes on `round(position * 1024)`, which welds
only within about `9.8e-4` — **ten times tighter than the constant that names the tolerance**, and
the mismatch is invisible because both sides of a corner are usually exact. Now that node samples
are exact the two agree in practice, but the constant and the key should say the same number, or
the key should say why it is deliberately tighter.

### What the gallery scene found about facet scale

`STEP` has to be measured against the massif's **relief**, not its width. A belt's cells are
bounded across by a rib, and a rib is the whole drop from crest to valley floor — a couple of
hundred units on a massif of a thousand. Sampling the crests at a tenth of that gives triangles
ten times longer than they are wide however the silhouette is drawn, and a massif of those reads
as folded paper. Sixty units puts a facet at about a quarter of a rib.

Two smaller things the scene found the same way: **a crest profile has to be written out rather
than shaped by one exponent** (a single smooth curve from summit to foot is a ruled surface, and
ten nearly planar surfaces meeting at one point are a tent however good the silhouette), and **a
valley floor has to run with its crests and only open out near its mouth** (a floor placed at the
same parameter as the two crest stations beside it, on a wedge whose reaches differ, puts its
ribs mostly *along* the flank instead of across it).


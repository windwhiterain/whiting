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

### A belt changes line only at a line's end — and that is all the "no spurs" rule ever was

**This section replaces "A spur hung off a parent's flank cannot be built, and the reason is
structural", which was wrong and is retracted.** It was written after a hand-bound hierarchy failed
to expand, and it read that failure as a theorem about the model. The failure was in
`Sampling::path`, which chose where to turn a belt by measuring which end of the leaving line was
nearer the point it was aiming at. That is right for a fan — every crest runs away from the one
summit, so the nearer end is always the tip — and on a crest that is one segment of a longer chain it
turns the walk back on the summit and answers `Disjoint`. Asking instead which pair of ends **welds**,
which is a fact about the document, makes the hierarchy expand: a chain `S → J → T` with a spur born
at `J` builds to a closed manifold surface, with faces hanging off all four segments. See
[proposals/network-derivation.md](../proposals/network-derivation.md).

Both of the old reasons were bad reasoning, and it is worth keeping why:

1. *"Non-increasing along every root-to-leaf path means a connected skeleton has exactly one summit,
   and a parent and a spur at `t > 0` do not share it."* The spur's first node **is** the parent's
   station, so the path through it is non-increasing and there is still exactly one summit.
2. *"The wedge doubles on itself."* The image — a surface folded along the parent's upper segment —
   was real, but it was what a wrongly-turned walk *builds*, not what the correspondence implies. The
   wedge between a parent above the junction and its spur does not close onto the summit: its first
   row collapses onto the **junction**, because both of its stations name that one point, which is the
   same degenerate row a summit already uses to close.

**What is actually true, and is the rule an author needs.** A belt changes line only at a line's
**end**. `corner` inspects the leaving line's two ends and nothing else, so a child named at its
parent's *interior* welds geometrically but is walkable in one direction only — child to parent finds
the junction, parent to child does not. Author a junction by splitting the parent there:

```text
ridge A:  S ─────────────▶ J            A.end names nothing; J is A's last node
ridge B:                   J ─────────▶ T   B.start = On(Station::ridge(A, 1.0))
ridge C (the spur):        J ──────▶ U      C.start = On(Station::ridge(A, 1.0))
```

`attach_ridge(parent, 0.5, …)` is accepted by the ops and expands to `Disjoint`. That is the whole,
narrow content of the old claim, and it is a consequence of the corner rule rather than of wedge
geometry.

**And what the checks will not tell you.** `euler_characteristic`, `boundary_loops` and `is_manifold`
certify *closure*. Two flanks doubled onto one crest are two discs glued along an edge, which is a
disc — `euler 1, 1 boundary loop, manifold true`, with a plan area ten times the rim's. Only the area
comparison and the coverage check see a doubled cover, so a report that gives the first three and
omits the second two is not a check on a mountain.

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

### A station names a parameter, so a table of floors is checked against an interpolation

Rewriting the same scene so every node's elevation is an authored number exposed a gap between what
a table says and what `validate` sees. A valley's node carries `Station { t }`, not a node index:
`t = 1/11` on a crest written with seventeen nodes falls *between* two of them, so "this floor is
below the rock on both sides of it" is a claim about a piecewise-linear read of the crest's own
numbers. Nothing in the crate can check it — `validate` checks each line's monotonicity and its
correspondence, not the two lines against each other — and it cannot be checked by eye either, since
the column being compared is not a column.

So the scene now keeps a second read, `Crest::height_at(fraction)`, that exists **only** to make
that comparison expressible, and asserts the two tables against each other before the skeleton is
built. The general form: once a caller is allowed to author two lines' elevations independently,
some relationship that used to hold by construction now holds only if somebody checks it, and the
check needs a read of the form the mesh itself never takes.

### A floor derived from the crests it runs between is one shape repeated

The valley floors used to be `min(two crests at t) * (0.85 - 0.80 * smoothstep(t))`, which reads
like a relationship and is not one. It is a **fixed ratio against whichever crest happens to be
lower**, and two consequences follow that are worth stating separately.

- **Two valleys sharing a lower crest came out identical to the decimal**, because the product
  collapsed to the same number. Three of the ten rows were duplicates, so the massif had fewer
  valleys than it appeared to.
- **A fixed ratio against a falling crest is a plane.** Where the floor holds its ratio, the floor
  and the crest are two parallel lines in section, so the belt between them is one flat face — the
  folded-paper failure again, produced this time by *constancy* rather than by smoothness. Every
  row of an authored table has to break its ratio at a different rate.

Neither is visible in a plan diagram or a validate report. Both are obvious the moment the numbers
are written out side by side, which is the argument for the table over the formula and the whole of
what the table is for.

### The block face, and why it is not a quad

The expanded form gained [`BlockFace`](../../../crates/mountain/src/facet.rs): the surface a belt
spans between two of its ribs, corners `(crest a, crest b, valley a, valley b)` with `a` paired
with `a`, and a triangle as the degenerate case where one boundary run is a single point. R5's
"each triangle is one planar block face" stops being true; the triangles stay exactly planar and
stop being the unit of surface. See
[proposals/block-faces-and-soft-edges.md](../../proposals/block-faces-and-soft-edges.md); the spec
itself is human-editable only and is **not** changed here.

Three things the design above does not say, found while adding it:

- **A belt is a general triangle strip, so a quad has to be asked for rather than recovered.**
  Measured on the `landscape` massif: 16 of 240 belts are two triangles and the rest are three to
  thirty, because R4.1 lets the two boundaries subdivide independently and they do — crest step
  scales run 0.63–1.15 against the valleys' flat 0.80. Any design that wants a cell structure has
  to give up R4.1 first. It did not: the block's ends are the **ribs**, which the author already
  wrote as `ValleyNode.left` / `.right`, so "where does a block begin" needs no new surface at all
  and writing fewer nodes on a line is also how an author coarsens the blocks.
- **The primitive a mesh is drawn in and the primitive it is read as are different questions.** The
  crate's pipeline is unindexed and writes three corners per triangle, so soft edges need **no
  vertex splitting at all** — a corner is already its own vertex, and the whole change is writing
  the averaged normal instead of the face normal three times. A mesh representation that had been
  indexed would have needed a split pass and a second vertex array; this one needed neither.
- **The corner stencil fails here for the same reason it fails in `patch`.** Angle-weighted
  normals smooth a block's interior and leave its join to its neighbour a crease — measurably
  softer than an unrelated hard edge, not continuous. `patch::continuity` documents the same
  bounded defect for control points (4.8e-2 rad against 1.8e-1 rad) and the same reason. So the
  two crates say *soft edge* for one intent and mean different mechanisms by it, and a soft edge
  in neither is C1.

## Amendments (2026-10-07, two ranges and a valley between them)

Rewriting `landscape` as a corridor rather than a subject. Full argument in
[proposals/valley-corridor.md](../proposals/valley-corridor.md). Two of these are about the
massif; the rest are about the scene and the kit and are recorded here because they were found
through this crate.

### Relief is a function of the distance to the eye, not of the massif's own width

`SUMMIT_ELEVATION` was a number chosen so that **one massif photographed from outside** looked
right: low against its own width, so that the silhouette is a mountain and not a spire. Standing
the same massif twice and putting the eye **between** the two copies changes what the number is
for. The picture is now governed by the angle the summit makes from the eye — the summit over the
distance to it — and a summit chosen to look right as an object puts the walls far too low in the
frame. It went from 720 to 1044, and every crest node with it.

The cost is measurable and worth stating next to the change. A flank's cells are bounded across by
a rib and along by the crest step, so raising the relief without changing the plan makes the same
step produce a taller triangle for the same width: median aspect 5.2 → 6.7, worst 125 → 375, and
545 of 1005 facets over 6. **A heightfield's coarseness budget and a skeleton's step budget are
the same budget**, and relief is spent out of it.

### A range that is one blob in the plan is a thing in the frame, not a wall of it

Measured, and the measurement is printed by `alpine_probe`:

```
framing from the eye, half frame 35.5 degrees off the axis:
  west: summit at 37.6 degrees, nearest point at 35.8
  east: summit at 28.8 degrees, nearest point at 26.4
```

A range's **nearest point and its summit are both on the ray from the eye through the range's own
centre**, so the two agree to within a degree or two, and the whole range occupies **one narrow
slice of azimuth** — about two degrees here, against a half-frame of thirty-five. To be a wall a
range has to reach from the axis out past the edge of the frame, which is the whole width of the
frame; to have its summit inside the frame it has to stay well within it. One blob in the plan
cannot do both at any relief and at any focal length, and the two numbers moving together is why:
they are the same ray.

This closes the "a fan is not a massif" open question above from the other side. Authoring
genuinely different crests removes the *cone* from a fan of equal ridges, and it does not make a
range any less a **single blob**. A wall needs the plan itself elongated, which for this skeleton
means the reaches — and the plan area is a function of the reaches, so elongating a range grows
`radius`, which is the multiplier half the gallery's scene constants are written in. It is a
change to the shape of the object and not to the shape of one of its faces.

One thing this turned up while measuring it: `stood_at` centres a range on its **bounding box**,
and this massif's bounding box is not centred on its summit, so the two copies are translations
rather than mirrors and the valley is lopsided — the east wall reads nearer and larger than the
west one. That is a better picture than a mirror would be and it is left alone, but a scene whose
argument rests on a symmetrical corridor cannot assume it.

### A painted horizon's depth is the caller's, and a creased noise needs an envelope

Two things in `ridges`, both of which the same scene found at once. **How many ranges close a
horizon is a fact about the land in front of the eye** — a valley that recedes through four ranges
and a plateau that stops at the first are the same backdrop at two depths — so the layer count is
a parameter and not a constant; a constant forces a scene to lie about either how far its horizon
is or how far the air has closed. And **a creased noise with nothing under it is a row of tents**:
peaks need a broad envelope at a third of the crease's own frequency to stand *on*, which is what
makes two peaks a massif and six peaks a chain. The existing note already said a smooth bump the
width of the frame is a volcano; the same argument one octave down says an even row of creases is
a row of pyramids.

### Three facts about the frame, found while trying to stand in one

Not about the crate, and recorded because they are the kind of thing that costs a day each.

- **A backlit surface is black and a second lamp cannot fix it.** The pipeline carries one
  directional light; the second is a point light, and in a scene sixty times its own size what
  reaches a surface is a fact about how far the lamp stands, not how bright it is. A face's own
  share of the sky has to be decided where the face's normal and the sun's bearing both are —
  in the surface body, from a constant the scene already has.
- **A world-space pattern on a floor the eye is standing on has to fold back into its own mean.**
  A value-noise cell is a fixed length of ground, so under the eye it is a cell across a third of
  the frame and at the far end it is a pixel. Neither reads, and a per-pixel pattern that would
  fix it slides across the picture when the camera moves.
- **A plain's undulation has to be coarser than the plain's grid.** A ripple the grid cannot carry
  comes back as the grid: a hundred-unit ripple sampled every four hundred units is four hundred
  unrelated cells, and near the eye — where one cell is a sixth of the frame — it is a floor tiled
  in rectangles. The wavelength has to stay several cells at every octave.



### A flank is one surface, and the ribs between its belts are not edges of the rock

`CornerNormals::of` averaged each corner over the **block** its face belongs to, and a block is one
belt. A flank is written as a run of belts — one per segment along its crest — so grouping by the
block creases the normal at every rib *as well as* at every crest, and a massif of eleven strips
to a flank came out as a fan of straw laid down the hill. `facet.rs` justified the block as "the
ribs and the crest lines are the hard edges of the rock", and that is true of the crest lines and
not of the ribs: a rib is an edge of the correspondence and the height field is continuous across
it.

Which edges of a massif are creases is a statement about the **surface**, and not about the mesh, so
the group is now the caller's: `CornerNormals::of_grouped(mesh, key)` takes it, `of` keeps the block
as the conservative default, and the gallery passes `across_the_flank` — a valley and a side — or
`across_the_belt` for a scene whose rock is meant to read as cut. Measured over the landscape's own
mesh, both groupings read at the same shared vertex of every interior edge:

```
creases across a flank: 1508 of 2977 interior edges, worst 158.1 degrees, total 59763 degrees
creases across a belt:  1684 of 2977 interior edges, worst 158.1 degrees, total 62057 degrees
```

Which is the whole of the correction **and not more**: a flank removes the ribs and leaves the
crest lines, the valley floors and whatever the mesh itself folds, and on this massif those are
most of it.

### A per-face number written at three corners is a per-triangle number, and it shows

The massif's comb striping was not the normals at all. It was `in.texture.y` — the face's own
`dip_angle` — read by the surface body to decide how bare the rock is and how much snow it holds.
`dip_angle` is a property of a **face**, and the point list writes it at all three of a face's
corners, so it is constant across each triangle and steps at every triangle edge. Two thousand
triangles each carrying their own constant is a fan of straw at exactly the scale of the
tessellation, and it looks identical to a crease.

Three ways of being wrong here look the same in a render, and they need three different fixes:

- `ruggedness` (also per face) — ruled out by rendering with the term removed.
- a fifty-unit world-space noise on the albedo — ruled out the same way.
- the smoothing group — **ruled out by rendering the normal as a colour**: the flank field is a
  smooth gradient with a handful of hard seams at the crest lines, which is what it should be, and
  every stripe in the lit frame is absent from it.

So: **a quantity the rasteriser should interpolate has to arrive as a vertex quantity.** The body
now reads steepness off `normal.y`, which is the same angle by another route and is the one of the
two the pipeline interpolates. `dip_angle` stays on the point as a fact about the face for anything
that wants the face's own answer.

### The valley floor's drift is a fraction of the wedge, not of the radius

`VALLEY_SWING` was a fraction of the valley node's own **radius**, which is a different share of
the wedge in every wedge: seven percent of a radius is a third of the half width of a twelve-degree
wedge and a seventh of the half width of a thirty-degree one. On the paired long walls — the two
wedges this scene exists to look down — that share, added to each crest's own `bend`, carried the
floor across the crest it was supposed to run between. `validate` reported it exactly, and the
report is the argument for reading a scene constant against the thing it is a fraction of:

```
NotCovered { line: Ridge(RidgeId(2)), side: Left, from: 0.18181819, to: 0.36363637 }
RibCrossesLine { belt: BeltRef { valley: ValleyId(1), side: Right, segment: 0 }, line: Ridge(RidgeId(3)) }
```

A number written as "a fraction of the radius" reads as small and is small only where the crests
happen to be far apart. Normalised against the wedge's own half width it is the same quantity in
every wedge, and the same scene then validates at zero violations.
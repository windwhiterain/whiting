# Agentic procedural building generation: the `building` crate

Status: **DRAFT, revision 2 — chief designer's design.** Both reconnaissance notes are woven in
(repo side: [investigate/building-crate-integration-surface.md](../investigate/building-crate-integration-surface.md);
external techniques: [investigate/procedural-architecture-techniques.md](../investigate/procedural-architecture-techniques.md),
all sources accessed 2026-10-16, the load-bearing RNG and boolean-free claims re-verified against
the primary README by the chief designer). An adversarial design review (2026-10-16) verified the
first draft's boldest claim — "a roof is the same object `mountain` already expands" — **false
against source**, and this revision rewrites D2, D5, D10 and P1 on that evidence; the correction is
recorded inside D2 rather than silently fixed. **P1 implementation started on 2026-10-16** on this
design, and what it found amends four points — see *What implementation found, and where it amends
this design* below. Date: 2026-10-16.

Decisions taken with the superior on 2026-10-16, before this draft:

- **Scope: one building, plus site composition.** A city grammar (streets, parcels, skyline rhythm)
  is a later phase, not this design.
- **Authoring interface: a rust construction API first**, exactly like `mountain`'s R5. Document/Delta
  authoring is a possible later phase (see D7 for what it would actually cost).
- **Deliverable: this proposal.** No implementation code until the design is approved.

## Motivation

`mountain` exists because the ridge distribution of a real massif is *planned* — hierarchy,
direction, rhythm — and noise cannot plan ([SPEC/mountain.md](../SPEC/mountain.md) R1 motivation).
The same argument holds one level stronger for buildings: a building is planned *all the way down*.
Massing has hierarchy (body, wing, tower, porch), facades have rhythm (levels and bays), roofs have
structure (ridges, hips, valleys, eaves), and every face belongs to a named element of that plan.

The demand-side evidence is measured, not aesthetic
([recon §5](../investigate/building-crate-integration-surface.md)):

1. **The gallery already contains an architectural form library in the wrong crate.**
   `crates/gallery/src/built.rs` holds `edge_normal`, `wall_run`, `WallGap`, `wall_ring`,
   `roof_ridge`, `sweep` — forms assembled face by face, with no hierarchy, no rhythm and no
   validation. A `building` crate is therefore partly a **promotion**, not a greenfield write.
2. **`PackedList` cannot be read back and rejoined** (`built.rs:116-121`). That single fact forced
   `foldyard.rs:822 push_ring` to re-implement `wall_ring`'s walk — one behaviour, two
   implementations, and the file says why (`foldyard.rs:811-816`). Hand-composed architecture
   duplicates code *structurally*, not stylistically.
3. **A wrong outward normal is invisible**: "a wall that is not drawn, and nothing in the frame says
   which of the two happened" (`built.rs:86-88`). There is **no `validate()` anywhere in the
   architectural path** — the gap a planning crate exists to close.
4. **Roofs are single-sided height functions** (`built.rs:339-346`, `foldyard.rs:906-921`): every
   normal points up, a camera below the eave sees through, and each soffit is a separate hand-written
   band. A building crate must answer roof solidity once, for every building.
5. **Two incompatible placement strategies coexist** — baked meshes (`foldyard`) and instanced unit
   meshes (`terraces.rs:533 place_house`) — and nothing in the framework prefers either. Scenes pick
   by accident and cannot mix.
6. Three independent agent runs building a bamboo courtyard each hand-rolled a gatehouse or a roof,
   and each hit camera-burial, scale or black-box-exposure failures — facts a planning layer knows
   before a frame is drawn ([agent-scene-iteration-loop.md](agent-scene-iteration-loop.md), its
   camera, scale and exposure items).

## The one idea, inherited

`mountain`'s one idea is that **the lines carry their correspondence**: a valley node names the
station it faces on each side, so the correspondence *is* the mesh's topology, and watertightness is
structural rather than the outcome of a search (`crates/mountain/src/lib.rs:13-25`).

The `building` crate inherits it verbatim, applied to three skeletons:

> **A building is authored as skeletons that carry their correspondence, and expansion is a direct
> walk of what the author wrote down.** Two faces meet along an edge because they were built from
> the same authored edge — never because two independent constructions happened to agree, and never
> because a boolean or a triangulation search decided it after the fact.

## The three skeletons

### S1: the mass skeleton (planned volume)

- A building is a tree/network of **masses**: body, wings, towers, porches, chimneys.
- Each mass is a footprint outline plus a vertical extent and a wall thickness. Floor lines are
  *not* S1 data — they belong to the grid skeleton (S2) on the mass's faces.
- Attachment is **authored adjacency**: an attached mass names the host face and where on it the
  joint lands. The precedent is `attach_ridge`, not a bare station reference — it **splits the host
  first**, then attaches to the named end, because "a belt changes line only at a line's end"
  (`mountain/src/ops.rs:39-52`); a mass attachment does the same to a host face. An intersection
  with no authored attachment is a `validate()` violation and nothing repairs it. An *authored*
  attachment may deliberately authorize an intersection at the joint — a roof sunk 0.10 into a wall
  top, as `terraces.rs:573` places one — and only the unauthored kind is reported.

### S2: the grid skeleton (planned rhythm)

- Each mass face carries **levels** (floor lines) and **bays** (vertical divisions): the lattice a
  facade reads as rhythm, and the reason a building is not a box.
- A bay names its mass face, the level below and above, and the division it spans. Parts attach
  **into bays**; a part that exceeds its bay is a violation, not a style.
- The **algebra** of those edits is CGA's, borrowed as a vocabulary and not as a runtime: split
  with rhythm (`{ pier | bay }*` between bookends), repeat, fit (first candidate whose minimum
  width fits), face/edge decomposition, offset — the op set the mature shape-grammar engines expose
  ([survey §1](../investigate/procedural-architecture-techniques.md), symbios-shape [S2]). What is
  **not** borrowed is CGA's implicit correspondence: a facade scope knows its parent face only by
  descent, and the survey names exactly this as the gap against R4.1 — so a bay *names* its mass
  face and its two levels by reference, the way a valley node names the stations it faces. D10
  settles where the algebra lives and what a re-split does to the edits under it.

### S3: the roof skeleton (planned structure)

A roof is **an upright miniature massif**: ridge and hip lines above, roof valleys where two planes
meet concavely, and an eave loop as its footprint. The isomorphism with `mountain`'s skeleton is
real — polylines with elevations, explicit station correspondence, one planar face per authored
span — and the design exploits it. It is **not** a licence to hand the roof to `Massif::build`:
that expander walks belts hung from valley lines only and can emit only an upward skin (D2 records
the measured reasons). The roof skeleton is building-side, of the same shape as `mountain`'s, with
one authoring rule that the massif case never needed:

- **The eave is authored once.** Ridge and hip lines name stations on the eave loop directly; no
  tracing artifact re-states an eave edge as a second line, so nothing can drift between two copies
  of the same edge.
- Gables, hips, mansards and sheds are different skeletons, not different expanders; overhang is
  authored by placing the eave loop outside the wall line; a roof is a **solid** (D5), so soffit
  and fascia are faces of the roof mass, never a hand-written band.

### Parts (the style vocabulary)

- Doors, windows, columns, brackets, copings, tiles: small authored templates placed into bays with
  an authored attachment (which bay, what inset, what scale).
- **The joint is authored, never snapped.** A part template declares named sockets, and a placement
  names the socket of the part it mates with; an unfinished joint is a `validate()` violation and
  is never repaired. Proximity snapping and inferred mates are the measured failure mode of parts
  systems ([survey §4](../investigate/procedural-architecture-techniques.md)): a parts soup is not
  watertight unless the joint is enforced, and an inferred mate drifts silently. This is R4.1 in
  its purest form — joinery is data, not a search.
- The parts vocabulary is where *style* lives; the skeletons are where *plan* lives. Neither
  encodes the other: replacing every window template changes no skeleton, and moving a wing changes
  no part.

## Decisions

Recorded with the alternatives they reject, in the form this repo's proposals already use.

### D1: rust construction API first (decided with the superior)

`ops` for edits, `derive` for starting points, `validate` for what is wrong, `build` for the mesh —
the `mountain` R5 workflow three measured agent runs converged on independently. **Rejected for
phase 1:** a `Definition`-document authoring surface (see D7).

### D2: reuse mountain's types and discipline; the roof's expansion is building's own

**Correction, recorded rather than silently fixed.** Revision 1 claimed a roof is "the same object
`mountain` already expands" and that watertightness and coverage checks "come free" through
`Massif::build`. The review read the source and the claim is false in both halves:

- belts hang **only from valley lines** (`mountain/src/expand.rs:236-253` enumerates
  `skeleton.valleys`; verified by the chief designer), so a convex roof — gable, hip, shed, every
  family S3 promises — has no valley lines and expands to **zero faces**, with `build()` returning
  `Ok` on the empty mesh and only `validate()` complaining (`MeshOpen`, `validate.rs:214-223`);
- the expander winds every face **up** and **drops vertical faces** (`expand.rs:407-441`,
  "the mesh is a graph over the plan"), so `Massif::build` can never emit a roof *solid*: no soffit,
  no fascia, no gable-end wall — exactly the single-sided-roof pain (motivation 4) this crate exists
  to answer;
- the working convention inside today's `Skeleton` (trace each eave edge as a valley line, with
  degenerate ghost valleys to satisfy coverage bookkeeping) **double-authors the eave loop** — the
  footprint corners and the valley-node positions are two copies that can drift silently into real
  skirt faces — and leans on the coincident-rib side fallback (`validate.rs:568-587`) for every
  footprint span.

**The decision, restated on that evidence.** `whiting-building` depends on `whiting-mountain` for
what is actually reusable — `FacetMesh` and its self-checks (`facet.rs:179-407`), `Sampling`,
`CornerNormals`, the `Station`/`End::On` adjacency vocabulary, and the measured disciplines of
`ops`/`derive`/`validate` — and owns the rest:

- the **roof skeleton is a building-side type of the same shape** as `mountain`'s (polylines with
  elevations and named-station correspondence), with the eave authored once (S3);
- the **roof-solid expander is building-side P1 scope**: it walks ridge/hip stations paired with
  eave stations and roof-valley correspondence directly, emits the upward skin, and thickens it
  into the authored solid (soffit, fascia, gable ends). No boolean is involved; watertightness is
  the authored correspondence, the `mountain` idea kept and the `mountain` walk not reused.

**Rejected alternatives.** *(a) The eave-tracing convention over `Massif::build`*: buys a skin, not
a solid, and pays for it with double-authored eaves, ghost valleys and a coincident-rib fallback.
*(b) Extending `mountain`'s belt enumeration so footprint edges become first-class belt sources*:
one clean move that would delete the convention's fragilities, but it changes an expansion contract
under a SPEC that is **human-editable only**, in a crate with in-flight neighbour work (the catmull
and hermite probe worktrees exist; their contents were not audited). It is recorded here as a
**future unification proposal to the SPEC owner**, not as a P1 dependency. *(c) Copying the
expander*: duplication of the one invariant-bearing machinery in the workspace.

**Consequence, stated plainly.** What transfers for free is the philosophy, the types and the
discipline; what P1 must build and name is the roof-solid walk, the mass-solid expansion, and the
checks in D5. P1's scope below says so.

### D3: output is planar triangles, `FacetMesh`-shaped; no `Topology::Quads` dependency

The scene layer is triangle-only — `Topology` has exactly `Triangles` and `Segments`
(`SPEC.md:104-106`, `scene_schema.rs:1041-1051`, `emit.rs:721-723`), a hard gate. The crate emits
planar-facet triangles into `mountain`'s `FacetMesh` shape, so a building reaches a frame through
the same gallery adapter path a massif already uses, with **no SPEC change required**.

- The NURBS multipatch direction (`feature/nurbs-multipatch`, in flight, not landed) *reproduces a
  planar face exactly* (degree-1 bilinear, planarity preserved to `1.047e-15`) and a hard edge costs
  nothing — architecture is almost entirely hard edges. But soft edges are **not continuous today**
  (a lone seam folds `8.6e-2` rad; T-junctions are unrepresentable), so nothing in this design may
  *depend* on it. If and when it lands, `FacetMesh → PatchMesh` conversion already exists
  (`gallery/patchmesh.rs:68-83`) and a building can be subdivided downstream — the crate's output
  stays the authoring truth. **Rejected:** designing against patch meshes now (building a phase-1
  guarantee on an unlanded, non-continuous representation).
- Watertightness here is structural (authored correspondence), so no boolean kernel is in the
  critical path. For the one case where parts must behave as a single solid — a shell with openings
  cut across baked and instanced geometry — the survey nominates `manifold` (MIT OR Apache-2.0) as
  an **opt-in consolidation**: it guarantees manifold output and retains per-input surface IDs, so
  D6's provenance survives the boolean
  ([survey §6](../investigate/procedural-architecture-techniques.md)). It is not a phase-1
  dependency: P1 openings are composed as masses (see Phasing), so no boolean arises.

### D4: the plan is baked; the vocabulary is instanced

The two coexisting gallery strategies stop being an accident and become a rule:

- **Baked mesh** for what the skeletons cut uniquely: walls with their bay openings, slabs, roof
  solids. This is one `FacetMesh` per building.
- **Instanced unit meshes** for what the parts vocabulary repeats: a hundred identical windows are
  one part mesh and a hundred transforms, through the scene's `instances` and its per-axis scale
  (`terraces.rs:563-578` already places a whole roof by transform alone).
- **Rejected:** all-baked (loses the scene's instancing, repeats identical geometry per bay) and
  all-instanced (a facade with unique openings is not a repetition of one unit).

### D5: closure and orientation are `validate()` invariants, at two named levels

`mountain` turns "is the mesh what the skeleton claimed" into one number: the signed `plan_area`
compares coverage *and* orientation (`mountain/src/lib.rs:108-129`). Two honest notes on that
inheritance, both measured by the review: the comparison itself is a **caller idiom documented as
a contract**, performed today only in an example (`census.rs:4-6`) — not a violation `validate()`
raises; and `mountain`'s own mesh contract is a **disc** (`MeshOpen` fires unless one boundary
loop, euler 1 — `validate.rs:214-223`), while a building mass is a closed solid. Building therefore
defines its checks rather than inheriting them:

- **Per mass:** a closed solid — zero boundary loops, manifold, euler characteristic 2 (the three
  predicates exist on `FacetMesh`, `facet.rs:294-356`) — and **positive signed volume**. Signed
  volume is **new building-side machinery**: `mountain` has none (a grep for `volume` finds zero
  matches), and it is the check that turns the inside-out wall of `built.rs:86-88` from an invisible
  building into a named violation.
- **Coverage as a violation, not an idiom:** the plan-area-versus-footprint comparison is promoted
  into `validate()` beside the station-tiling check (the `NotCovered` analogue for bays and roof
  spans), closing the enforcement gap the mountain idiom leaves open.
- **Per building:** attached closed solids glued face-to-face make 4-face edges, so a *building* is
  deliberately **not** manifold as a whole; "watertight" at building level means *every mass closed
  and outward, every joint authored*. Whole-building manifoldness is claimed only under the opt-in
  `manifold` consolidation (D3), never by default.
- **Roofs are solids by default.** An open shell is an explicit per-mass marking for authored cases
  (a canopy, a ruin); the default answers motivation 4 once — a camera below the eave meets a
  soffit face, not a hole. **Rejected:** fusing "solid or open shell" into one silent option, which
  is how the single-sided roof happened the first time.

### D6: every face carries provenance, numbered by the expansion itself

Each expanded face records the skeleton element it came from (mass face, bay, roof line, part
placement). The numbering comes **from the expansion's own enumeration order**, the discipline
`gallery/identity.rs:33-35` already follows — its ids are taken from `mountain::expand::belt_references`
so id and mesh "are the same numbering by construction rather than by two loops kept in step"
(`expand.rs:228-232`). Note for P2: `identity.rs` itself is welded to mountain's types
(`belt_references`, `BeltRef`, `Massif` — `identity.rs:43-45`) and **cannot be reused as code**;
what transfers is the discipline, and building grows a sibling mechanism over its own expansion's
reference list. This is the crate-side half of the `Identifier` channel that
[agent-scene-iteration-loop.md](agent-scene-iteration-loop.md) shelved pending a shader graph (its
disposition item 3): the crate owes a stable face→element map; the harness owes the channel when it
exists.

### D7: document/Delta authoring is deferred, and its real cost is recorded

The user decision is a rust API first. The recon adds the fact that makes deferral cheap rather than
lossy: `Delta`'s `Append`/`Remove` are **contiguous-only** by construction (`definition/src/delta.rs:22-25`),
and no geometry crate uses `Definition`/`Delta`/`History` at all today (`mountain` has zero
dependencies). A document-shaped building would need a registered `Schema` and would still not
express non-contiguous edits. **This is a phase of its own, not a small add-on** — recorded so a
future design does not rediscover it as a surprise.

### D8: no terrain dependency; the ground is an authored input

A building stands on a plane or on authored sill heights in phase 1–2. Seating on a
`terrain::HeightField` (skirts, stepped foundations on slopes) is **P3 site composition**, and stays
outside the crate's R6-equivalent boundary: `terrain` produces numbers, never geometry
(`terrain/src/lib.rs:26-30`), and this crate produces geometry, never erosion.

### D9: crate name `building`, publish `whiting-building`

A noun naming the thing made, like `mountain` names the massif. **Rejected:** `architecture` (names
the discipline, not the artifact), `edifice` (not a word an agent greps for).

### D10: the split algebra is ops on the skeleton, with replace semantics

CGA's editable state is its rule set, and its re-derivation is global — the whole shape re-derives
from its initial polygon ([survey §1](../investigate/procedural-architecture-techniques.md)).
`mountain`'s invariant is the opposite and this crate inherits it: **the document is the skeleton**
(`mountain/src/lib.rs:68-74` — the tessellation is not part of it in any sense that survives an
edit). Split/repeat/fit therefore live as `ops` verbs that *write bays and levels into the
skeleton*. A rule is a construction move, not a persistent graph.

The staleness objection is real and gets the `mountain` answer rather than a hand-wave: widening a
mass face leaves its authored bays stale, and forty hand edits are forty chances to drift. The
precedent is `derive_valleys` — it **replaces every valley rather than merging**, which makes it
"a 'start over' operation and not an incremental one … the answer to 'the valleys are wrong', never
to 'the valleys are nearly right'" (`derive.rs:129-141`), and outside its supported domain it
**refuses to guess** and reports what it could not place (`:138-145`). So:

- `split(face, rhythm)` has **replace semantics** for that face's bay family: it re-reads the face
  and stores nothing from the old family, and it reports rather than guesses when the face cannot
  support the rhythm;
- part placements key by **rhythm index** (which slot of the family), not by bay identity, so a
  re-split re-seats forty windows instead of orphaning them;
- `validate()` gains the grid invariants that make staleness *visible*: bays must tile their face
  exactly once (the `NotCovered` analogue), levels and bays must lie inside the face extent after a
  resize, and a placement naming a bay that no longer exists is an orphan violation.

**The honest middle path, recorded:** keeping a rhythm as *annotation data* on a face and re-firing
it manually does **not** violate the document-is-the-skeleton invariant — an annotation is skeleton
data like any other. What D10 rejects is the **auto-refiring rule tree**, whose re-derivation is
global and whose rule and hand-edits below it are two sources of truth. The steelman stands: CGA
re-derives forty bays and re-places their windows for free on a width edit. This design answers
with one replace-semantics op plus rhythm-indexed keys, which is cheap for the same edit — and
pays in explicitness everywhere else, which is the point.

### D11: derived randomness forks per node, and shows itself as a choice

Where `derive` needs randomness (which window variant, which hip a roof gets), two disciplines are
borrowed from the MIT rust CGA engine the survey verified ([S2]; re-checked against its primary
README by the chief designer 2026-10-16):

- **Fork the RNG per node along the ancestry path.** The same seed re-derives the same building;
  queue order is irrelevant; and editing one wing re-rolls that wing alone, never its neighbours —
  the exact property the iteration loop's "nearest path that differs" diagnostic wants, on the
  geometry side.
- **Randomness is an explicit weighted alternative** (`70% WinA | 30% WinB`), never parameter
  jitter, so a derived default *looks like* a default (R3) and an authored choice looks authored.
  Style coherence — every floor picks the same window variant — is a keyed pick resolved once per
  building: coherence is data, not luck.

### D12: the default roof derives from the eave footprint, then is edited

`derive` may generate a starter roof skeleton (ridge/hip/valley lines) from a mass's top footprint —
the R3 pattern `mountain` already ships (`Starter`, and its refusal to invent spur ridges without a
guarantee, `derive.rs:19-23`, `:35-101`) — and the same move the mature roof families make (a hip
ridge derived from the footprint; mixed-width wings sharing one ridge height). Generated lines
carry no guarantee and are ordinary editable lines the moment they exist; outside the supported
domain, derivation reports rather than guesses (`derive_valleys`' contract, D10). **Rejected:**
parametric roof primitives as the representation — a `Roof(Hip, 30)` whose lines cannot be edited
individually is a finished artifact with no edit handle, which is what R3 exists to prevent.

### D13: shading groups are a design obligation, not a rendering afterthought

`CornerNormals::of` creases at **every block boundary** (`facet.rs:476-483`, `:136-141`), so a roof
plane emitted as per-segment belts would shade creased at every eave node — a faceted roof nobody
authored. The expander therefore emits a **grouping key per authored face** (one roof plane, one
wall run) and the mesh's corner normals come from `CornerNormals::of_grouped` (`facet.rs:505-533`),
the mechanism that already exists for exactly this. All building edges are hard between groups;
soft edges within a group wait for the NURBS direction (D3) and are not promised.

## What implementation found, and where it amends this design (2026-10-16)

The shell phase (P1) started on this design and its first measurements amend four things. Recorded
here because the phase's handoff is deleted when the phase closes and this file is the durable
record.

1. **D2's reuse is thinner than revision 2 claimed, and for a reason worth keeping.** Revision 2 said
   `whiting-building` reuses `mountain`'s `FacetMesh`, `Sampling` and `CornerNormals`. It cannot:
   `mountain`'s per-face attributes are keyed by a **belt**, and the gallery reads exactly that field
   as the identity handle — `gallery/identity.rs:406` numbers pixels by it and
   `gallery/blockmesh.rs:137` groups faces by it. Reusing the type would mean writing belt ids onto
   walls, which is a document that says something untrue about itself. `Sampling` samples skeleton
   *lines* rather than wall edges, and `CornerNormals` is inherent to `FacetMesh` and cannot be
   called on a foreign type. **So the dependency on `mountain` is `math` alone** — one definition of
   which axis is up, which two crates authoring into one scene must share — and `BuildingMesh` with
   its own predicates is this crate's. That is not duplication of a shared abstraction; the shared
   abstraction does not exist yet, and **a shared mesh crate (with the plan-geometry helpers
   `mountain` keeps private) is the unification to propose** when a second caller needs it.
2. **A flat roof has no representation yet.** A plane here is a *strip of transverse rows*, and a
   flat slab has no transverse extent — its skin **is** the eave polygon. `RoofStyle` therefore has
   `Hip` and `Gable` only, and authoring a flat roof needs a second representation (a plane bounded
   by a loop) before it can be honest rather than a degenerate row set that expands to nothing.
3. **The roof derivation's domain is narrower than "a quadrilateral".** A plane's quad is planar only
   when its crest line is parallel to its eave edge, so a skew quadrilateral is **refused** (`None`)
   rather than authored into a document that contradicts the crate's own "one plane is one planar
   face" invariant. This is `derive_valleys`' refuse-rather-than-guess rule applied to a roof, and it
   was found by implementing rather than by designing.
4. **Watertightness between two roof planes is a disjunction, and the design said only one half of
   it.** Measured on a starter house: a hip's plane pairs share **four rows** (the same crest index
   and the same eave station, written twice from one value), while a gable's two planes share **no
   row at all** — they share their inner **crest edge**, both naming crest 0 and crest 1. S3's "two
   adjacent planes share the boundary row" is the hip's case; the rule is *a shared row **or** a
   shared crest edge*, and either way the shared thing is an authored identity rather than a
   coincidence of coordinates.

Two further measurements belong to the phase rather than to the design, and are recorded because a
later reader will otherwise re-derive them:

- **Coplanarity must be measured against a least-squares plane, not a mean of normals**: for a 0.25 m
  corner lift the mean reports `0.0623` and the fitted plane `0.1875`, so the mean would have made
  the tolerance a re-tuned guess. The phase uses `1.0e-3` m — and, measured later, the fit itself has
  a property that has to be stated rather than discovered: **it reports the farthest vertex's distance
  to the best-fit plane, which is not one vertex's displacement.** Lift one corner of a quad by
  0.25 m and the fit tilts to share the error and reads `0.0833`, exactly a third. A number that
  "should" have been 0.25 is not a defect; measuring *how far one vertex stands off the plane its
  neighbours lie in* needs three vertices and the triangle they name, which is a different instrument
  and a different meaning for the violation.
- **A check is only as good as the surface it is exercised on**, and the phase paid for this twice: the
  planarity fitter returned a coordinate axis for every sloped surface, which no axis-aligned fixture
  could expose (the hip's own trapezoid reads `0.8` under the axis and `2e-7` under a correct fit),
  and a square's hip emitted facets of **no area** while closure, Euler, manifoldness and orientation
  were all green. Both now have controls that fail on the old behaviour: a sloped, exactly coplanar
  quad, and a facet of zero area. **`SolidOpen` and `SolidNonManifold` are also not independent** —
  a hole and a duplicated face each fire both, because an odd-used edge cannot cancel in a boundary
  walk, so a control that fires `SolidOpen` alone needs an edge used twice in the *same* direction.

**A defect found in a neighbouring frozen file, recorded rather than edited.** `gallery::built`'s doc
comment states that a ring is wound "with the ground it encloses on the walker's **left**". In the
frame `mountain::math` states (`x` east, `y` up, `z` **south**), `up × along` — that file's own
formula for a wall's outer face — is the walker's **left**, so a *positively* wound loop puts the
solid on the walker's **right**. The formula is right in both files; the prose reads true only if `z`
is read as north. This crate now states the test (`plan_area > 0`) and the verified direction. The
`built.rs` prose is left alone (another crate, and its callers' actual winding was not audited) and
is a follow-up for whoever next touches that file.

### What P1 measured, and what it still owes

**One process lesson, recorded because it is about this design rather than about that week.** P1's
first acceptance run read "zero violations" from a tree where the validator was still a stub: three
parallel works had been written against a frozen interface, two were merged, the third was not, and
the number the phase reported came from the gap. The corrected run — expander and validator on one
tip — turned zero into nine, and **not one of the nine was where it first appeared to be**: one was a
naming fault (the floor's top and the slab's soffit shared one element, so a planarity group spanned
two planes one slab apart), and eight were the **validator's own fitter**, which returned a coordinate
axis for any sloped surface and so reported a roof plane's vertical spread as its departure. Two
genuine derivation defects were found in the same pass, and **neither was visible to any predicate**:
a hip whose ridge inset reached half the long run — every square — stood two crest points on one spot
and emitted facets of no area, and an inset past that point shed the long planes inside out.

Two consequences for this design's own discipline, both already its stated aims and neither yet its
practice. **A claim about a system is measured with every part of that system present** — and **a
check is only as good as the surface it is exercised on**: a fixture whose plane is axis-aligned
cannot fail a plane fitter, and no closure predicate can see a triangle of no area. The row model in
particular makes planarity a *duty of the derivation* rather than a property the model guarantees, so
a derived roof has to be checked for it by measurement, on a sloped fixture, every time.

`cargo run -p whiting-building --example census`, three starters (hip 7×11, gable 7×11, hip 6×6):

| starter | violations | solid | faces | loops | Euler | manifold | volume |
| --- | --- | --- | --- | --- | --- | --- | --- |
| hip 7×11 | **0** | mass | 28 | 0 | 2 | true | 64.2080 |
| | | roof | 20 | 0 | 2 | true | 24.0000 |
| gable 7×11 | **0** | mass | 28 | 0 | 2 | true | 64.2080 |
| | | roof | 20 | 0 | 2 | true | 24.0000 |
| hip 6×6 | **0** | mass | 28 | 0 | 2 | true | 36.2880 |
| | | roof | **16** | 0 | 2 | true | 12.2500 |

The volumes are the authored numbers rather than plausible ones: `64.2080 = 0.4·77 + 3.2·(77 −
6.4·10.4)` is slab plus walls, to the digit, and `24.0000 = 96 · 0.25` is a slab of constant vertical
thickness covering its eave area exactly. A square's hip has **one** crest point rather than two — a
ridge of no length is one point and four triangles — which is why its roof has 16 faces and not 20,
and that difference is itself a defect the phase found and fixed rather than a shape it chose.
`examples/expand_probe.rs` carries the positive controls — a reversed solid reads −51.720 against
51.720, a dropped face reads one boundary loop and Euler 1, an over-wide wall reports four
`OffsetEdgeFolded` problems and no mesh — and the validator's own probe carries one control per check
it claims, including the sloped-plane fixture and the facet of no area that no closure predicate can
see.

Two things P1 deliberately does not have, both named rather than faked — and **both closed since**: a
**flat roof** (item 2 above) is a plate, which the slab slice below added as the crate's third
primitive; and **bays, openings and parts** are P2's. The proof frame —
`cargo run -p whiting-device-rasterize --example render -- house out/house`, a derived hip-roofed
house in 10 passes — carries one known artifact, attributed by measurement rather than by eye:
`--debug position` on the same scene shows clean wall planes and a crisp eave line, so the geometry
is what the census says; the toothing in the beauty frame is the **soffit's own shadow terminator**,
it gets finer as the shadow map's texel density rises (`shadow_extent` 30 → 14) and does not vanish,
so a bias term is involved too. That belongs to the shadow subsystem, where three works are already
in flight (`virtual-shadow-maps`, `shadow-texel-push`, `shadow-level-walk`), and the scene says so in
its own doc comment.

## What P2 found: the facade, in five mechanisms and one frame (2026-10-16)

P2 built the other half of the design — **grid, openings, parts, sockets, identity** — and the second
demonstration scene. Its record is here rather than in a handoff, because a handoff is deleted when its
phase closes.

### The five mechanisms, and the one invariant each turned on

| mechanism | the invariant that made it small |
| --- | --- |
| **S2 grid** (`grid.rs`) | A family **is** its stops: levels and divisions must span the face exactly, so tiling is structural rather than a check to infer, and a bay is a **slot** `(row, column)` so a re-split carries what hangs in it (D10). A rhythm is a **function with a domain** — `Rhythm::stops(extent) -> Option<Vec<f32>>` — so `Even { count: 0 }`, stops that do not span, a module wider than the face, and a **module so small it asks for millions of stops** are all refusals rather than guesses. `split_face` **replaces** the family; measured, 6 bays became 10 rather than 16. |
| **openings** (`openings.rs`) | A hole is **strictly inside its wall face**, which keeps the wall a rectangle with holes and lets the cut be proved; and it is authored **relative to its bay** (`Inset` in metres from the bay's four edges), so "the opening exceeds its bay" is a checkable violation. Measured on a 7×11 starter: three holes totalling `7.3 m²` of wall removed exactly `2.348 m³` of material — the number no closure predicate can fake. |
| **parts** (`parts.rs`) | **A template is a small skeleton**, so `Template::skeleton()` hands it back to the same `build` that builds a building; and the placement frame is **stated**, `x` along the edge, `y` up, `z` into the wall with `x × y = z`. Measured: a hollow door leaf stands as `SolidId::Part(0)` with `loops 0`, Euler 2, manifold, volume `0.026083` against the analytic `0.026084`. |
| **sockets** (`Socket`/`Anchor`/`Joint`) | **A joint is one subtraction, not a transform**: every part is authored in the placement's frame, so mating a socket to an anchor only moves the origin, and no rotation is expressible — and therefore none can be got wrong. Measured, and this is the slice's whole point: with the doorway's head moved from 2.35 to 2.55, a canopy **hung on the head** stayed exactly `+0.0000` above it while the same canopy **pinned by numbers** went from `+0.2200` to `+0.0200`. A pin cannot notice. |
| **identity** (`identity.rs`) | **One walk produces both the numbers and the table**, ids are **a contiguous run from `first_id`**, and **zero is reserved** for "not an authored element" — the sibling's three invariants. Measured: 108 faces naming 42 elements, 0 faces without an id, 0 rows without a face, two builds giving an identical table, and **0 of 42 ids changed** by adding a part on another face. |

**One deliberate divergence, recorded with its reason.** The mountain sibling's table is a function of
the skeleton alone; a building's cannot be, because a roof's **fascia is numbered by its position in the
skin's own boundary edge list** and that count is a fact the **expansion proves** rather than a number
the document holds. Re-deriving it in `identity` would be a second derivation of the skin's boundary —
the failure this crate has paid for twice — so the module asks the mesh, and says so in its own doc.
What survives is the property the invariant is *for*: ids stay put across edits that do not change a
surface's own boundary.

### The second demonstration scene

`crates/gallery/src/shed.rs` ("shed") is the phase's end-to-end proof: a 5×8 gable-roofed shed whose
gable end carries a **rhythm of three bays**, a **doorway and two windows cut through it**, and **two
parts** — a leaf and a canopy, the latter **hung on the doorway's head** — in 16 passes. It asserts its
own document is clean before drawing, so a fault is a panic rather than a frame, and that assertion
earned its place immediately by finding two real faults: a canopy **0.05 m outside its bay**, and a
"thin" body whose floor coincided with its top, which is not thin but **degenerate** — eight facets of
no area, because `Mass` is the crate's room model and a plate is a room with a very shallow floor.

**What happened to `foldyard`, which P2's plan named.** The plan said *"`foldyard`'s shed and enclosure
rebuilt as the second demonstration"*. That is what `shed` and `yard` **are** — a shed with a bay-mounted
door and window, and a walled yard with a gate, both authored in the crate instead of in `built::`. The
`foldyard` *scene* itself is not rebuilt and will not be: it is a 1655-line `built::` composition whose
subject is a **landscape** — a valley, cut terraces, a massif, a far range, vessels and a rack of poles —
and only its architectural part is the crate's business. So the debt is closed as **superseded rather than
outstanding**: the demonstration exists, and `foldyard` remains the `built::` showcase until that module is
retired. Rebuilding the landscape half would be a different proposal.

### Four lessons this phase paid for, stated so the next one does not

1. **A check's shape outlives the shape it assumed.** `SolidNotASphere` required Euler characteristic
   `2` and was right while a mass was a sphere; an opening is a **handle through the material**, so each
   one takes the characteristic down by two and a wall with three holes is a genus-3 surface reading
   `−4`. The check now reads `2 − 2 × the openings that resolve`. The same lesson one layer up: a
   plate is degenerate in a room model even though it is a perfectly ordinary solid.
2. **Two modules measured separately are not a system**, and this phase proved it three times: the
   expander cut holes while the validator measured a stub; the closure fix emitted a zero-area ear
   facet that the degenerate-face check then reported *on the phase's own verb*; and every jamb had to
   be split at every other hole's cut elevations before the wall closed. Each was invisible to either
   module alone.
3. **A lookup that filters needs a companion that names what it dropped.** `apertures_of` and
   `unresolved_openings`, `placed` and `unresolved_placements`: the filter is right for a walk that
   asks "what can I cut" and wrong for one that must not build silently.
4. **A grade's air needs an exponential with no asymptote.** `1 − exp(−L·k)·c` has its limit at
   `1 − c`, so it fogs a twelve-metre shed by a fifth whatever the distance: measured, that is the
   difference between a monochrome wash and a lit wall, and it had been copied into two scenes before
   anyone looked.

### What P2 owes, and what it does not

**D4's instancing is honoured in the document and not in the mesh.** One `Template` is authored once and
every `Placement` names it **by index** — the vocabulary side of D4 — but the expansion **rebuilds the
template's skeleton per placement**, so forty identical windows hold forty copies of the geometry. That
is correct and cheap at this scale and it is *not* what a scene wants; the fix is the gallery's (one
mesh per template, one instance transform per placement, which the kit's `instance` already supports),
and it is recorded here so the next session does not read the mesh as instanced because the document is.

Also owed: the **render half of identity** — an `id` channel the mesh shader writes is the device
layer's business, exactly as it is for the mountains.

## What P3 found: the site, in four slices and one picture (2026-10-16)

P3 built the third layer — **ground, walls, and the site as a unit** — and its record is here rather
than in a handoff, because a handoff is deleted when its phase closes.

### Slice 1: ground seating — the crate stays terrain-independent

The decision list says this crate takes **no `terrain` dependency**, and seating does not need one: a
height field is a function from plan to elevation, so `site::seat(skeleton, ground: impl Fn(Vec2) ->
f32, offset)` takes the **function**, and a caller with a `HeightField` passes `|plan|
field.sample(plan)`. That keeps the dependency decision true and, just as important, makes the slice
**measurable against an analytic ground** — which is why its numbers are exact rather than plausible.

Three invariants, each paid for:

- **One level per mass, at the highest ground under it.** The plate's top is a polygon at its
  nodes' elevations, so a quad with four different elevations is a **ruled surface and not a plane**, and
  the planarity contract would report it — correctly. A building therefore sits on a **terrace**: the
  highest sampled ground under its outline plus the offset. Measured against the analytic plane `z = 0.3
  x`: level `0.750000` = `0.3 × 2.5`, the plane's own maximum over the outline.
- **A seat is rigid.** The delta goes to every node, to `top`, and to the eaves and crest of every roof
  that names the mass as host, so the walls keep their height and the roof keeps its rise. Measured:
  volume `36.832001` before and after, bounds moved by exactly `0.750000`, the roof's rise unchanged at
  `1.850000`. Moving only the outline would have shortened every wall and flattened every pitch, and
  nothing in the closure predicates would have said so.
- **A half-known ground is not a ground.** Any non-finite sample refuses the mass outright; the first
  implementation skipped the bad sample and used the rest, which is exactly the guess this crate refuses
  elsewhere. Measured after the fix: a ground unknown over one half seats **0 of 1** masses.

### Slice 2: walls and courtyards — and the suggestion this phase rejected

The previous handoff guessed that the most-reusing answer was "a wall is a `Mass` with a thin outline and
its gate is an opening". **Both halves are wrong**, and the reason is the same one that bit the shed's
canopy: `Mass` is the crate's **room** model — a wall ring and the plate it stands on — so a thin mass
either leaves a cavity (a gate would look into the wall's inside rather than through it) or degenerates
its floor onto its top (eight facets of no area, measured). A freestanding wall has no inside.

So a wall is its own kind and deliberately the smallest useful one: **one straight run**, a box whose
bottom follows the ground and whose top is level. And a gate is **not a hole through a wall** — it is a
**gap between two runs**, which needs no cutting at all.

**A courtyard is several wall solids and not one cleverer element**, and that is the two-level
watertightness rule doing the job it was written for: offsetting a chain of runs into a ribbon would need
a miter on an open path, and the shared faces where two runs meet would coincide or interpenetrate
*inside one solid* — which a closed surface cannot do without interior faces. Several solids may overlap
at a corner freely. Measured by `examples/wall_probe.rs`:

- a 10 × 6 yard is **5 walls, 29.6 m of wall, 17.76 m³ written down**; every run is **12 faces, loops 0,
  Euler 2, manifold**, and the measured total is `17.760012` against the written `17.760000`;
- the gate is the gap, measured at `2.4000 m` between the two south runs;
- each corner is shared by two runs that interpenetrate, and **0 walls are unclosed**;
- seated on the plane `z = 0.3 x`, a wall's ends take their own ground (`−1.5000`, `+1.5000`) with its
  top still level at `2.0000`, its volume **unchanged** at `6.000000` — exact for a linear slope, because
  the bottom turns about its mean — and its box running from `y −1.5000` to `2.0000`, still closed.

**Walls follow the ground; buildings sit on terraces.** That is the difference between a fence and a
building, and `seat` now does both and says how many of each.

### Slice 3: the site is the skeleton

The open question was whether a *site* needs a new document type. It does not: the skeleton has held
several masses and walls since P1. What was missing was a way to **look at it as one place**, and two
things only a site can say:

- `Mass.name` with `Mass::named(..)` — and the invariant that makes it safe: **a name is for a reader and
  never a handle.** Nothing in the crate resolves by name, two buildings may share one, and the doc on the
  field says so, because an author who renames a building must not silently repoint anything at it.
- `site::report(&skeleton) -> SiteReport { buildings, walls, wall_length, ground, overlaps }` with
  `SiteReport::{building, footprint, relief}`. The overlap is between **plan bounding boxes** and the
  doc states the bound plainly: two L-shaped buildings can have overlapping boxes and no shared wall at
  all, which is why the report **lists** overlaps and nothing **judges** them.

Measured by `examples/site_probe.rs` on three buildings and a five-wall courtyard: **3 buildings, 5 walls,
38.0000 m of wall, footprint 92.0000 m²**, a box overlap of **4.5 m²** (`1.5 × 3` by hand), the ground
running `0.7500 .. 4.3500` after seating, the buildings' tops moving with it, and the seated place
building (88 points, 144 faces) with **0 violations**. The overlap was legal because the store is
attached to the shed as a wing; the probe's first version left it unattached and `UnattachedOverlap` said
so — **the validator caught the author, not the report**.

### Slice 4: the site scene — one law for the ground

`crates/gallery/src/yard.rs` ("yard"): a shed and a store up a `0.16 x` slope inside a five-run courtyard
with a gate, `13` passes. Three decisions the frame forced:

- **One law for the ground.** `ground(x, z)` is called once by the terrain mesh and once by `seat`, so
  what the frame draws is exactly what the document was seated on. **Two laws would draw buildings
  hovering over a slope or buried in it and the picture would still look plausible** — the one failure a
  render cannot show and a shared function cannot have.
- **The scene refuses to draw a site it could not seat**, asserting `2` masses and `5` walls were seated.
- **A building on a slope needs a deep foundation, and the frame proved it.** The terrace is cut to the
  *highest* ground, so the downhill edge stands the relief above the ground; with the starter's `0.3 m`
  slab the whole downhill side floated, and the scene's buildings were given `1.20 m` — which is what a
  real building on a slope has, and the reason the slab exists.

The first render read as a beige plane (a `0.09` slope and four materials too close in value); the
second, at `0.16` with separated materials, reads as a site.

### Three lessons, recorded so the next phase does not pay for them

1. **The room model cannot express a solid.** That is now **three** findings of one shape: the shed's
   canopy plate, walls, and any future slab. `Mass` is a room; a plate is a room with a very shallow
   floor; a wall is not a room at all. The pattern is worth knowing: **when a shape has no inside, look
   for the model's assumption that it has one.**
2. **A geometric rule with a consequence is not a bug.** Seating at the highest ground *implies* a
   foundation on the downhill side; the terrace is right and the missing slab was the fault. The frame
   found it because a picture is where a consequence becomes visible.
3. **The bound is not the intersection, and the report must say which it is.** An overlap of bounding
   boxes is a reading, not a fact about the buildings, and the doc says so at the type.

## What the slab slice found: a flat roof is a plate (2026-10-16)

P1 named two things it deliberately did not have, and the last of them was the **flat roof**. This slice
answers what a flat roof *is*, and the answer is neither of the two shapes the crate already had.

**It is not a roof with `rise = 0`.** The roof model's skin is a strip between a **crest row** and an
**eave row**; zero rise makes those rows coincide in plan, the strip has no width, and every facet of it
has no area — the same failure shape the square hip already taught this crate (a ridge of no length
became one point and two zero-area facets, which the degenerate-face check then had to be taught to
distinguish from structure). A flat roof expressed as a zero-rise roof would be a vocabulary fighting its
own geometry.

**It is not a mass either.** A mass is a **room** — its wall ring and its plate, two solids — and a
plate with a coincident floor is degenerate (measured on the shed's canopy: eight facets of no area).

**So it is the crate's third primitive: `Slab`, a level prism over a plan polygon.** With it the
vocabulary is complete in a way worth stating: **`Mass` is a room, `Wall` is a box along a line, `Slab`
is a plate** — and flat roofs, canopies, plinths, terraces, steps and balcony decks are all the plate.

**A slab is level, and that is the domain.** `bottom` and `top` are elevations, not per-node values, so
every face of the prism is planar by construction: the two caps are the polygon (triangulated by the
**existing ear clipper**) and each side is a rectangle. A *sloped* plate is a roof, and the type refuses
to be one rather than growing a second geometry — per-node elevations would make the caps a ruled surface
for any real outline, which is the fault the shell's floor already has to avoid.

`Slab` also carries `host`, and that is what makes seating work: `seat` moves a **hosted** slab with its
building and leaves an unhoused one where its author put it, because a flat roof's elevations are the wall
top's and a seat that moved the mass alone would leave the roof behind at the old level.

Measured by `examples/slab_probe.rs` on a flat-roofed 7 × 11 starter:

| measurement | value |
| --- | --- |
| the document | **1 mass, 0 roofs, 1 slab** — a plate is not a roof |
| the plate's plan | `8.4000 × 12.4000 = 104.159988 m²` (`w + 2 × overhang` by hand) |
| its volume | `29.164793 m³` written, **`29.164804`** measured |
| its topology | **12 faces, loops 0, Euler 2, manifold** — the count a box has, because a rectangle's caps triangulate to two triangles each |
| its underside | `3.4500` = the wall top `3.6000` less the sink `0.1500` |
| **the building under it** | mass volume `78.245979` and `28` faces for **both** the flat and the hipped starter |
| seating | `1` mass and `1` slab move, the plate holding its `0.2800` thickness, `0` violations |
| an unhoused plate | `masses seated 0, slabs moved 0` — still at `5.0000 .. 5.4000` |
| the two faults | `SlabNotThick { thickness: 0.0 }`, `SlabNotAPolygon { nodes: 2 }`, refused by the build |

**Re-measured 2026-10-17: a flat starter holds two slabs, not one, and the plate is what the second is.**
`derive::starter`'s `RoofStyle::Flat` arm used to build its own document and return, which dropped the
ground plate derived above it — so for sixteen days the "1 mass, 0 roofs, 1 slab" line in that table was
true of a building with no floor, and every other number in it still held. The fix and its measurements are
in *What the flat arm kept* below.

**One process lesson, the same one the wall slice recorded from the other side.** The new variants broke
two probes' exhaustive matches over `SolidId` — and the fix was not to widen a wildcard but to give
`validate_probe` something real to say: it now compares a slab's **authored plan loop** with the mesh's
measured coverage, which is the one solid in that document whose loop can be compared directly. **A new
variant is a compiler-enforced audit of every `match` in the crate**, and the audit is only worth the
trouble if each site it stops at gains a reading rather than a `_`.

## What the instancing slice found: a template's mesh and the frame that stands it (2026-10-16)

D4 says *"plan baked, vocabulary instanced"*, and only its document half was honoured: one `Template` is
authored once and every `Placement` names it by index, but `expand_placement` **rebuilt the template's
skeleton per placement**, so a building with forty identical windows expanded forty copies of the same
twenty-eight faces. The proposal recorded the fix as the gallery's and named what the gallery would need:
one mesh per template and one instance transform per placement.

**The gallery could not get there from what the crate handed out.** `BuildingMesh` is baked — every part's
points in world space with provenance — which is right for validation and useless for instancing: a scene
received `40 × 28` faces and no way to know they were one shape forty times.

So the crate now hands out **the two pieces**, and the baked mesh stays the thing `validate` measures:

- **`Template::mesh()`** — the template expanded **in its own frame**, once, as an ordinary `BuildingMesh`.
- **`PartFrame { origin, along, up, inward }`** with `apply` — where a placement stands its template, in the
  crate's own vocabulary (an origin and three unit directions, one dot product per direction) rather than a
  matrix, and `expand::part_frame` resolves it through the **same** `placed`/`offset_loop`/`WallFace` path
  the expansion used, so a mounted part's frame is its joint's and not a second reading of its pin. A wall
  the miter refuses returns its **problems** rather than a frame.

**The invariant that makes it one derivation.** `expand_placement` was **rewritten** on those two pieces and
nothing else — not given a parallel path — so the instanced geometry and the baked geometry cannot disagree,
and the probe proves it by comparing them as **point sets** rather than by eye.

Measured by `examples/instance_probe.rs` on a shed with six placements of one shutter and a canopy hung on a
bay's head:

| measurement | value |
| --- | --- |
| faces to upload | **168** baked against **28** in the template's own mesh — six times fewer, for the same geometry |
| instanced points vs baked points (as sets, `1e-4`) | **0** placements differ |
| instanced box vs baked box (per axis, `1e-5`) | **0** placements differ |
| a mounted part's frame | origin at `y 1.3000` = the bay's head `1.4000` less the socket's `0.10`; the baked canopy runs `1.3000 .. 1.4000` |
| an unresolved placement | **no frame** (`Ok(None)`), so a scene cannot instance what the document cannot resolve |
| the existing acceptance | `Part(0)` still `0.026083`, `Part(1)` still `0.045825`, the socketed canopy still exactly `0.0000` above a moving head, and the `shed` frame's SHA256 **byte-identical** to before the rewrite |

That last row is the point of doing it as a rewrite: a rendering that changed would have been a rewrite that
had quietly changed something else.

**What is still not done, and where it belongs.** A *scene* still draws the baked mesh; switching `shed` or
`yard` to template meshes plus transforms is a gallery change with its own questions (per-instance materials,
per-instance shadow pages), and `build` still expands each template per placement — a cache keyed by
template index is a performance change a caller can now make correctly, because a template's mesh is a
function of the template alone.

## What the floor slice found: two storeys, and a closed-form fault in the cutting (2026-10-16)

Every building the crate could author was a **single storey**. The grid's `rows` could split a wall into
storeys, but nothing put a **floor** between them, and the vocabulary was already there — `Slab` is a level
plate and `host` moves it with its building. What was missing was the **authoring**: an intermediate floor
is the mass's inner loop at an elevation, and the inner loop is `expand`'s private `offset_loop`. An author
who computed it themselves would be writing down a second opinion about where the inside of the building is.

So `derive::floor_slab(mass, id, elevation, thickness)` does it, with four invariants: the plate **is** the
mass's inner loop (the same inset the walls are built on), it is **hosted** so seating moves it, the
elevation must be **strictly inside** the mass's height and the thickness positive (every other case is
`None`, not a plate that is invisible or sticking out), and a wall the miter refuses answers `None`.

Measured by `examples/floor_probe.rs` on a 7 × 11 starter with a 0.35 wall and a floor at `2.0`:

| measurement | value |
| --- | --- |
| the plate's plan | `6.3000 × 10.3000 = 64.889999 m²` by hand, and `64.889999` from its own loop |
| its volume | `12.978000` written, `12.978002` measured |
| the inset on each of four sides | `0.3500` — the wall's own |
| the document, flat and seated | **0 violations** |
| the refusals | at the ground, below it, at the wall top, above it, of no thickness — all `None` |

The **workshop** scene (`crates/gallery/src/workshop.rs`, 13 passes) is the first two-storey building the
crate draws: three windows above, a window, an open doorway and a window below, and a mezzanine plate at
`2.2`. Its doc says plainly what the frame shows (the two storeys) and what it does **not** (the plate, of
which about seven centimetres are visible as the doorway's own ceiling) — a frame that overclaims is the one
kind of documentation this crate cannot check.

### The defect the scene found, and its closed form

The workshop's first configuration put the plate at `2.8` with the doorway's head at `2.05`, and the scene's
own assert fired: `SolidOpen { loops: 4 }`, `SolidNonManifold`, `euler -14`. **The fault was in the openings'
cutting and predated the slice.** Reduced by `examples/multi_row_probe.rs` to a table, and then to a form:

> **A wall's bands and both discs are split at every station, so every face that meets them along a
> horizontal or vertical edge must be split at the same stations — and every face that meets them along a
> vertical edge must also be split at the same cuts.** The cutting had learned the second half in P2 (a jamb
> split at every cut) and not the first: a hole's **head and sill** were single quads, which is right until
> another hole's jamb station falls inside the hole's own span.

The two failing rows and their fix:

| configuration | before | after |
| --- | --- | --- |
| a door below and a window above, **jambs not aligned** | `loops 4, euler -6` | **`loops 0, euler -2, manifold true`** |
| six holes, **a different inset per row** | `loops 12, euler -22` | **`loops 0, euler -10, manifold true`** |
| every configuration that already closed | closed | **closed, with the same face counts** |

The ownership dump that named it in one step is worth keeping as an instrument: for each boundary edge, print
the elements of the faces that use it. It showed one edge owned by a door's **sill reveal** and a shorter pair
owned by the **floor top** — two faces disagreeing about whether a horizontal edge is broken, which under
**authored-identity welding** is an opening that no coordinate comparison would ever find.

## What the identity slice found: a building whose every pixel names its element (2026-10-16)

The crate's oldest promise is that *the lines carry their correspondence*: every face of a mesh is a named
authored element, and `Identity` (P2) answers *"what is this element called"* from a document and its mesh.
What it could not do is answer it **from a picture** — and a picture is all an agent looking at a frame has.
*"The third window's reveal is too deep"* is a sentence about a picture, and the join from a pixel to the
authored element is what makes such a sentence checkable. The mountain side of the gallery had that join;
a building had none of it.

The slice came in **two halves**, and the second is where the design stopped being a theory.

### The CPU half: the numbers a pixel will carry

`crates/gallery/src/building_identity.rs`:

- **`building_extras()`** — one output, `authored_id` at location 1, read from the points and carried
  **flat**. An element is a property of a *face*, not a place on it, so interpolation is the wrong operation
  and flat is the right one: the three corners of a face must agree, or a pixel is pulled between two
  elements.
- **`outputs_field()`** — the scene's own declaration, `f32`, because `r32float` is exact where an
  `rgba16float` is exact only to 2048 and an id of 4093 would come back as 4092 with nothing saying so.
- **`ids(built, identity)`** and **`packed(built, identity)`** — one id per point, three per face, and the
  packed buffer that carries it beside the corner normals, the positions and the plan `texture`.
- **`not_authored_extras()`** — the reserved zero as a **constant**. A constant is not a field, so a surface
  that is not an element of the building (the ground) carries **no id field at all** and is drawn by its own
  set of stages; that is exactly the `plain_extras` design one crate over, and it is what keeps a ground
  pixel answering *"not an element"* rather than naming one.

Measured by `examples/building_identity_probe.rs` on a document holding a two-storey shell, a plate, a
courtyard wall and a part in a bay: **100 faces, 50 elements numbered `1..=50`, 0 faces whose corners
disagree, 50 distinct ids against 50 rows and equal, and zero never written**.

### The frame half: the picture, and the loop that closes on it

`crates/gallery/src/authored.rs` is the scene registered as **`identity`**: a two-storey shell with a plate,
a doorway, five windows, a shutter part and a courtyard wall, drawn as **one mesh** with one material body,
the ground drawn with the constant zero, and the scene declaring the output. `stage_set(prefix, body,
helpers, point, extras)` builds a mesh's four stages with the lighting one **declaring** the outputs, and
`locate(id)` rebuilds the scene's own document and answers with the row's path.

**The pipeline needed nothing.** `render_outputs` already reads a scene's `outputs` and hands back one image
per declaration, and the render example already writes each as `<out>-<channel>.png` plus a sidecar of the
values — because a png cannot hold floats. So the whole frame half was the scene's declaration and the
points' packing:

```
cargo run -p whiting-device-rasterize --example render -- identity out/identity
    → out/identity.png, out/identity-authored_id.png, out/identity-authored_id.txt
```

Measured from that frame's own sidecar:

| fact | value |
| --- | --- |
| the image | `1024x1024, r32float`, red over **`[0.000, 68.000]`** |
| building pixels | ids `3, 4, 16, 20, …` |
| sky and ground pixels | **`0.000`** |
| the join | `id 3 → mass 0, outer wall over edge 2`; `id 16 → mass 0, the head reveal of opening 0 in edge 2` |
| an id the table does not hold | `0` and `9999` both answer **`None`** |

### Three things the wiring taught

1. **A material names its shader by a label built from a prefix**, so a scene with two meshes needs two
   label sets and the two cannot read each other's point layouts. The failure is not an error in the
   shader — it is a mesh whose depth stage reads another mesh's numbers.
2. **A constant is not a field**, and the compiler says so in the only terms that matter: *"the points are
   packed as normal, position, texture but the mesh's point schema lays them out as authored_id, …"*.
3. **The scene's own assert is the cheapest test in the crate.** Every building scene checks its document
   before drawing, and this one found the openings' closure defect (the floor slice) — the same assert now
   guards the frame whose whole purpose is to be looked at.

### An accident worth keeping

A failed scripted edit — a `Substring` that threw, followed by a `Set-Content` on the null result — wrote an
**empty** `landscape/mod.rs`. The build named the file immediately and the previous commit restored it in
one command. The rule it argues for is not "be careful with scripts" but **commit before you rewrite a file
you did not read digests of**: the recovery was one `git checkout --` because the file was already on disk
in the history.

## What the instancing slice found: a scene finally stands one mesh many times (2026-10-27)

D4's mesh half was honoured in the crate — `Template::mesh()` plus `PartFrame`, with `build` rewritten on both
so the baked and instanced paths cannot diverge — and **the gallery never used it**. Every scene still drew
whatever `build` baked, so a facade with five shutters uploaded five copies of the same twenty-eight faces.
The proposal had recorded the fix as the gallery's and named exactly what it needed: *one mesh per template and
one instance transform per placement*. This slice is the scene side of that sentence.

`crates/gallery/src/instancing.rs` is the join:

- **`standing(frame) -> (translation, rotation, scale)`** — the transform a scene instance carries, read from
  the frame's **own axes**: `yaw = atan2(inward.x, inward.z)`, translation = the frame's origin, unit scale.
  The invariant that makes it possible is a fact about walls rather than about scenes: a wall-mounted frame's
  `along` and `inward` lie in the plan and its `up` is world up, so the rotation is **one angle** and an
  instance transform can carry it exactly rather than approximately.
- **`template_points(template)`** — one expansion in the template's own frame, which is the space the
  instance transform expects.

The **workshop** changed in three ways, all consequences: five shutters in its five windows from **one**
template; the shutters drawn from `template_points` and stood five times by `standing`; the five windows made
one height, because one template behind five placements is the point and five heights would be five templates.

**And the scene had been drawing no parts at all.** Its meshes filtered `Mass`, `Roof` and `Slab`; a part is
`Element::Part`, so the canopy it used to carry was authored and invisible — a fault no check could catch,
because a scene that does not draw something is a perfectly valid document. The shutters are the first parts
this scene draws, and the lesson is worth keeping: **a scene's mesh filters are a claim about what the frame
contains, and nothing compares that claim to the document.**

Measured by `examples/building_instancing_probe.rs` on six placements of one shutter:

| measurement | value |
| --- | --- |
| the upload | **168** baked part faces against **28** in the template — six copies, one upload; `84` points once instead of `84 × 6` |
| frame vs the scene's instance transform (as point sets, `1e-4`) | **0** placements disagree |
| frame vs the baked geometry | **0** placements disagree |
| the rotation | one yaw per part (`-3.1416 rad` on the front wall, where `along` is `(-1, 0)`) |

The scene renders at 16 passes (three more than before: the shutter mesh adds its stages), its own document
assert is clean, and the gallery suite passes.

## What the wall-placement slice found: a gate is not a hole, it is a part hung on a wall (2026-10-27)

`yard` has had a courtyard with a **gate gap** since the site slice and **no gate in it**, and the reason was
not an omission: every part the crate could place stood in a **bay of a wall of a mass**, and a leaf is not
something a bay holds. It hangs from the post at the end of one run and closes the opening beyond it. That is
a different kind of place, and the vocabulary to say it was the last target a building's parts needed.

`building::parts::Target` is that vocabulary:

```rust
pub enum Target {
    Bay { face: FaceRef, row: usize, column: usize },
    Wall { wall: WallId },
}
```

**One position field, two domains.** `Placement` still has a single `at: Pin`, measured in the target's own
frame — across a bay (from its start) and up from its sill, or along a run (from its start) and up from the
ground **under the pin**. `Placement::new` kept its signature and `Placement::on_wall` is new, which is why no
scene, probe or verb changed shape when the target was added.

Three consequences, each of them forced:

- **A wall run is its own frame.** `part_frame` reads the run's direction as `along`, its normal as `inward`,
  and interpolates the ground **along the run** for the base a pin's `up` is measured from. That last clause
  is a defect the probe found: the first implementation read the base at the run's `from` node, and a gate on
  a `0.25 x` slope hung **1.5 m under its own doorway**. The run's bottom follows the ground, so a part's base
  has to follow it the same way.
- **Only the pin is measured.** A bay placement is checked against its bay's rectangle; a wall placement is
  checked against the run's line and **not** against the part's box, because a gate leaf is hung at the end of
  a run and *must* reach past it. `PlacementOffItsWall` reports a pin off the run; the leaf's reach is the
  author's business.
- **A joint on a wall is refused, twice.** Every anchor names a feature of a bay — a sill, a head, an
  opening's head — and a wall run has none of them, so `place_part` answers `None` and a document that holds
  one anyway reports `PlacementMountOnAWall` rather than resolving the anchor against the run's ends by
  guesswork.

Measured by `examples/wall_part_probe.rs` on an 8 m run, a 3.2 m gate gap and a 2.4 m leaf hung at the run's
end:

| measurement | value |
| --- | --- |
| the frame | origin **`(2.0000, 0.0000, -5.0000)`** — the run's end on its centre line; `along (1, 0)`, `inward (0, 1)` |
| the leaf | spans **`x 2.0000 .. 4.4000`** — it closes **`2.4000 m`** of the gap; **watertight**, 28 faces |
| the document | **0 violations** |
| a pin `0.5 m` beyond the run | `PlacementOffItsWall { past: 0.5 }` |
| a wall the document does not hold | `place_part` answers **`None`** |
| a joint on a wall | `place_part` answers `None`; a document holding one reports `PlacementMountOnAWall` |
| seating on a `0.25 x` slope | the leaf's foot moves to **`y 0.5000`** — the ground under the pin |

And the **yard** now draws its gate: the leaf is one template placed on the run that stops at the opening,
drawn through the instancing path exactly as the workshop's shutters are (one mesh, one transform), so the gap
scene has had since the site slice is a **gate** rather than an absence. The scene renders at 16 passes and the
gallery suite passes.

The crate's eighteen probes are green and the **`shed` frame is byte-identical** (`0AF51AEC…`), which is the
evidence that adding a target disturbed no part that stands in a bay.

## What the cache slice found: the last debt was a measurement waiting to be structural (2026-10-27)

The proposal has carried one crate-level debt since the instancing slice, and it named its own shape: *"`build`
still expands each template per placement — a cache keyed by template index is a performance change a caller can
now make correctly, because a template's mesh is a function of the template alone."* Twelve placements of one
shutter expanded the shutter twelve times.

`expand::TemplateCache` is that cache, and the design point is that **`build` uses it**: `build(skeleton)` is
`build_with(skeleton, &TemplateCache::of(skeleton))`, so there is one path and no way for a caller to get a
different geometry by choosing the other one. The cache holds **failures too**, so every placement of a broken
template reports the same problems rather than each re-discovering them.

**The measurement is structural rather than timed.** `TemplateCache::expanded()` answers how many templates
expanded cleanly, so the probe does not have to argue about milliseconds:

| measurement | value |
| --- | --- |
| expansions for **12** placements of one template | **1** |
| placements whose baked points differ from the cached template's, mapped by their frames | **0** |
| the mesh | `336` part faces = `12 × 28` |
| two templates, 13 placements | **2** expansions — the count follows templates, not placements |
| a broken template among three | **2 of 3** expanded, and the document reports `OutlineTooShort { nodes: 2 }` |

That is the shape a performance change should have in this crate: the *cost* became a number the document can
be asked about, and the geometry stayed byte-identical, which the nineteen probes and the byte-identical `shed`
frame both say.

## What the style slice found: a style is a relation, and a share needs both its ends (2026-10-27)

P2's record has owed **"one coherent style set"** since the facade phase, and D11 has been an open question
since the design was written — *does a style's pick belong to the skeleton, which is authored and diffable, or
to `derive`, which has no guarantee?* They are the same work, because a style is what D11 is about.

`derive::Style` states the **relations** a building's numbers obey rather than the numbers: the wall's thickness
as a share of the smaller plan span, the slab as a share of the wall, the eave as a share of the span, its seat
as a share of the eave, the rise as a share of the **half** span, the roof as a share of the rise — plus the
facade the style implies (one storey's height, one bay's width, a window's proportion of its bay, and
`glazing()` in `Opening`'s own shape). `Style::starter(width, depth, storeys)` derives a `Starter`;
`COTTAGE` and `HALL` are the two the crate ships.

**D11 is decided here: a style belongs to `derive`, never to the skeleton.** The document-is-the-skeleton
invariant says a skeleton holds what an author *authored* — a thickness, a rhythm, a rise. A style is a rule for
computing those, and keeping it in the skeleton would make a document's geometry move when a constant changed,
which is the "derivations have no guarantee" rule run backwards. A document built from a style holds the
**numbers**; the rule stays in `derive`, where re-running it is a choice rather than a side effect.

Measured by `examples/style_probe.rs`:

| measurement | value |
| --- | --- |
| the same footprint, two styles | cottage `wall 0.3158, slab 0.3632, eave 0.4800, rise 1.6500, roof 0.2640` against hall `0.2308 / 0.2423 / 0.5400 / 2.5500 / 0.2550` |
| one style, three sizes | wall `0.2200` at 2.4 m (**the floor**), `0.3158` at 6 m (**the share**), `0.4500` at 12 m (**the ceiling**) |
| every coherence claim, every size | **true** |
| every derived building | **0 violations, both solids closed**, at one and two storeys |
| the facades | cottage 2/3/4/5 bays of 2.0–2.4 m with 0.62–0.74 m jambs; hall 1/2/3/4 bays of 3.0 m with 0.72 m jambs |

**The probe found the style defective on its first run**, which is why a style needs a probe at all: a `1/19`
share with **no ceiling** gave a 12 × 18 cottage a **0.63 m wall** — a fortress with a window in it. The relation
now carries a maximum, and the probe reads it back in its own words: *the smallest cottage is limited by the
floor, the middle one by the share, the largest by the ceiling.* A relation between a share and a span is only
a relation if both ends are bounded.

## What the offset slice found: an instanced id is a point's plus a row's (2026-10-27)

The identify slice recorded the tension — **the id channel and instancing do not compose**, because an
`authored_id` is a per-vertex value and one mesh drawn five times carries one set of ids — and named two designs:
a per-instance offset, or two channels. **The offset was chosen, and it turned out to need three things, each of
them measured before it was built.**

### 1. The arithmetic, measured first

An offset is only a mapping if the baked table lays each placement's surfaces down as a **contiguous run in the
template table's own order**:

```text
baked_id(placement, surface) = template_local_id(surface) + base(placement) - 1
```

`examples/instance_identity_probe.rs` measured it: six placements of one shutter gave runs `25..=38`,
`39..=52`, …, `95..=108` — **all contiguous, all 14 rows** — and the equation held over **84 surfaces**, with the
offsets `24, 38, 52, 66, 80, 94`. Continuity is not luck: `Identity::of` numbers its rows from a
`BTreeSet<Element>`, and `Element::Part { placement, surface }` orders by `placement` first.

### 2. Where the offset lives, which the first attempt got wrong

I assumed a **per-instance** value. A scene's instance is `{label, mesh, transform, material}` — **it carries no
custom values at all**, and the shader's custom inputs belong to the *material*. So the offset lives on a
**material**, and five placements need **five material sets** that all name one shader. That is not a workaround;
it is what the format says: a row is where a number that varies per draw belongs.

### 3. The sum, which is inert at zero

`shaders::Authored::Sum { name, components, flat, custom }` reads a **point** field and adds a **material** field,
in the vertex stage, into the varying the fragment already reads. `building_extras()` uses it with
`kit::AUTHORED_OFFSET`, whose default is `0.0` — and the identity scene, which bakes every surface it draws,
renders **byte-identically** (`D36C5EF5…`) with the new code, which is the evidence that the plumbing changes
nothing until a scene asks it to.

### The join, measured in one run

The render harness's `--locate` now **repeats**, because the interesting question is a comparison rather than a
point — five shutters, five pixels, five names, and one flag per run made that five runs. On the workshop:

| pixel | what it shows | the name |
| --- | --- | --- |
| `(620, 340)` | a shutter's corner | **`part 4, template mass 0, outer wall over edge 0`** |
| `(620, 590)` | the same mesh, another placement | **`part 1, template mass 0, …`** |
| `(860, 590)` | the same mesh, a third | **`part 0, template mass 0, …`** |
| `(750, 340)` | a wall's reveal | `mass 0, the head reveal of opening 4 in edge 2` |
| `(500, 900)` | **`0`** — the ground | *"no domain reading"*, which is the reserved zero meaning what it says |

**Three different placements of one instanced mesh, named by three pixels, in one frame** — and the ground still
answers the reserved zero. The tension the identify slice recorded is closed without giving up either D4's
instancing or the identity channel.

## What the scenes slice found: four of five buildings answer, and the fifth is named (2026-10-27)

The offset slice closed the tension between instancing and the identity channel. This one spent the answer: the
channel is now declared by **every building scene but one**, and the harness's `--locate` is the single door to
it.

**What each scene needed was the same three things** — `outputs_field()` on the scene, `stage_set` for two point
layouts (authored meshes carry `authored_id`, the ground carries none and writes the reserved zero from a
constant), and `packed_where` for the meshes — plus, where a part is instanced, **one material set per placement**
whose `offset_by(..)` is the base the baked table gives it. Measured:

| scene | what a pixel answers |
| --- | --- |
| `identity` | `mass 0, outer wall over edge 3` |
| `shed` | `roof 0, the skin of plane 1`; `mass 0, outer wall over edge 3` |
| `workshop` | `part 4` / `part 1` / `part 0`, three placements of one instanced mesh |
| `yard` | `mass 0, outer wall over edge 3`; `wall 1, its right side` |

and the ground answers **zero** in all four, which the harness reports as *"no domain reading"* — the reserved
zero saying what it says.

**Two traps the slice paid for twice each**, both worth stating because they are properties of the code rather
than slips:

1. **A material is named twice** — once where it is declared and once by the instance row that draws with it — so
   renaming a material's set (`earth-*` → `ground-*`) without renaming the row is `no mesh material in this scene
   is labelled …`, and the compiler finds it only when the scene is compiled to a document.
2. **A squash merge takes the branch's copy of every file it touches**, so a fix made on the trunk between the
   branch's last commit and the merge is silently undone. The shed's unused import came back exactly that way.
   The rule: after a squash merge, re-run the check that caught the original fault, or make the fix *on the
   branch*.

**`house` is the fifth scene and it is not done.** It is the same shape as `shed` and needs the same four edits;
the attempt ran out of a round, was reverted rather than half-landed, and is recorded here so the next slice
starts from a known tree.

## What the house slice found: every building scene answers (2026-10-27)

`house` was the last building scene without the identity channel — the same four edits as `shed`, which is why it
was its own slice rather than a footnote. It now declares the output, packs its walls and its hip with ids, gives
its ground the reserved zero, and **`locate_in` learned the scene**:

```text
render house out/hsi --locate 512,430
  locate house (512, 430):
  roof 0, the skin of plane 0
```

**That last arm is the lesson, and it is a second registration.** A correctly packed frame whose scene is missing
from `locate_in`'s match answers *"no domain reading"* at every pixel — which looks exactly like a channel that
was never written. **The channel and the naming are two separate registrations and both are needed**; a scene's
picture and its names can each be right while the join between them is absent.

With it, **every building scene the crate ships** answers the question:

| scene | a pixel's answer |
| --- | --- |
| `identity` | `mass 0, outer wall over edge 3` |
| `house` | `roof 0, the skin of plane 0` |
| `shed` | `roof 0, the skin of plane 1` |
| `workshop` | `part 4` / `part 1` / `part 0` — three placements of one instanced mesh |
| `yard` | `wall 1, its right side` |

and in every one of them the ground answers **zero**, which the harness reports as *"no domain reading"* — the
reserved value saying *"not an element of this building"* rather than anything ambiguous.

## What the where slice found: the tool now runs in both directions (2026-10-27)

Every building scene answered **"what is this pixel?"**; nothing answered **"where is this element?"** — which is
the other half of a debugging tool, and the half that catches the failure a pixel query cannot see: *the element
you meant to draw is not on screen at all*, or is smaller than you think.

`building_identity::id_in(scene, name)` resolves a name back to its `authored_id` from the same table
`locate_in` reads (one table, two directions), and the render harness gained **`--where <id|name>`**, repeatable
like `--locate`, which scans the id image and reports a pixel count and a bounding box. Measured:

```text
render house out/hsi --where "roof 0, the skin of plane 0" --where 9999 --where "nothing at all"
  where house roof 0, the skin of plane 0: id 15 covers 47261 pixel(s) in x 408..=915, y 286..=457
  where house 9999: id 9999 is not visible in this frame
  where house nothing at all: no element of this scene is called that
```

**Three different answers, kept different**: an element with pixels, an id with none, and a name that does not
exist. The last two are the pair that matters — a tool that answered `0 pixels` for a typo would be telling an
agent its geometry was off screen when in fact it had misremembered the name, and that is precisely the failure
mode this crate has spent many slices making impossible to confuse.

The two directions are now one table: the id `locate` reads off a pixel is the id `where_is` finds pixels for.

## What the pinning slice found: the channel's readings are now assertions (2026-10-27)

The readings the last several reports quote — *id 3 is this wall*, *id 49 is part 0's outer face* — were numbers in
prose, checked by a human reading them. That is the wrong kind of evidence for a mechanism whose two registrations
rot independently and **silently**: a scene whose meshes stop carrying ids draws a frame that answers zero
everywhere, and a scene missing from `locate_in` answers *"no domain reading"* at every pixel it does name. Neither
is a compile error.

`examples/scene_identity_probe.rs` walks **every building scene's whole table** and asserts that the ids are
contiguous from the first, that every name is non-empty, that every name round-trips to its id, that the authored
schema carries `authored_id` while the reserved-zero schema does not, and that the four readings the reports quote
are `locate_in`'s own answers:

| scene | rows | contiguous | names non-empty | round-trips |
| --- | --- | --- | --- | --- |
| `identity` | 74 | ✓ | ✓ | ✓ |
| `house` | 26 | ✓ | ✓ | ✓ |
| `shed` | 64 | ✓ | ✓ | ✓ |
| `workshop` | 124 | ✓ | ✓ | ✓ |
| `yard` | 92 | ✓ | ✓ | ✓ |

**380 rows, all two-way.** The tool is no longer a thing that can be used; it is a thing whose answers are
checked every time the probe runs.

## What the reverse query found on its first real question: the gate the frame could not name (2026-10-27)

The `--where` half of the debugging tool was added for the case a pixel query cannot see: *the element you meant to
draw is not on screen*. Its first real question found the opposite — an element that **is** on screen and whose
pixels name the wrong thing:

```text
where yard part 0, template mass 0, outer wall over edge 0: id 49 is not visible in this frame
where yard wall 1, its right side:                id 69 covers 14973 pixel(s) in x 669..=934, y 406..=512
```

The yard's gate has been drawn since the wall-placement slice. Its **three materials** carried the placement's
offset correctly; its **instance row** still named the unoffset `board-{stage}`, so every gate pixel reported the
*template's local ids* — numbers that alias onto the mass's walls. **The picture was right, the table was right,
and the join between them was wrong in a way that made the frame answer confidently about the wrong element.**

That is the failure mode this project keeps meeting in new clothes: *a material is named twice*. First the shed's
ground, then the workshop's shutters, now the gate — three occurrences, each caught by a different reading (a
compile error, a pixel query, a coverage query). The rule that emerges is worth stating plainly:

> **Every join that is written in two places is a join that will be wrong in one of them.** The instance row and
> the material declaration are one join; the mesh's ids and `locate_in`'s table are another; a scene's output
> declaration and its material's stage set are a third. Each has been wrong at least once, and each was found by
> asking the frame a question rather than by looking at it.

Fixed and measured:

```text
where yard part 0, template mass 0, outer wall over edge 0: id 49 covers 13713 pixel(s) in x 657..=788, y 540..=697
where yard part 0, template mass 0, outer wall over edge 1: id 50 covers 238 pixel(s) in x 652..=657, y 582..=632
```

## What the second hall found: a style composes, a scene does not (2026-10-27)

The hall scene was one footprint and one camera, which made the style's promise half-tested: a style that only
ever draws one building is a table of numbers with a picture attached. So the scene became
`hall_at(width, depth, storeys)` — and, more to the point, **the frame follows the building**: the camera's
distance and height and the sun's shadow page are derived from the ridge `HALL::starter` answers with, because a
scene whose camera is right for one size and wrong for two is a scene that does not compose.

Two scenes, one file:

| scene | footprint | identity rows | the wall's edge 2 |
| --- | --- | --- | --- |
| `hall` | 10 × 16 | 72 | id 3 covers **82,813** px |
| `hall_large` | 14 × 20 | **104** | id 3 covers **59,833** px |

**The style changed the building, not the scene.** `HALL::bays(10.0)` = 3 and `HALL::bays(14.0)` = **5**: the same
relations give a gable end with three windows and one with five, a rise that follows the span, and a wall that
stays a twenty-sixth of it. Both tables round-trip through the probe, so both scenes are named and both are
checked.

That is the difference between a style and a preset: **a preset is the numbers, a style is the rule that produces
them for whatever you ask** — and the picture of the second size is the evidence, because nothing in it was
chosen twice.

## The baseline a later slice measures against (2026-10-27, re-recorded by the volume refactor, again on 2026-10-29 when the plates were drawn, again on 2026-10-30 when the starter's plate was seated on the inner loop, again on 2026-10-31 when the ground was cut away from under every plate, and again on 2026-10-17 when `RoofStyle::Flat` kept the plate every other style already carried — **all nine unmoved**, see *What the flat arm kept* below)

Nine building scenes render, and each one's picture has a hash. They are recorded here because the question a
refactor has to answer is *"did the picture change?"*, and a hash answers it in a way that looking at two images
does not. The hash is the first sixteen hex of the `sha256` of the scene's own `.png`, and it is stable across runs
— measured, by rendering `shed` twice and reading the same number twice:

| scene | frame hash (first 16) | notes |
| --- | --- | --- |
| `house` | `B13154AD7D29F938` | the hip, the plate, three walls; the ground is cut away under the plate, and the plate stays **buried: 0 plate pixels** because the house has no opening |
| `hall` | `C4339E6310EA1C37` | `HALL` at 10 × 16, three bays; the camera is untouched and the plate's top went **665 → 3,166** now that the ground is not tied with it |
| `hall_large` | `43D3780A15E3D08E` | `HALL` at 14 × 20, five bays, its own lower and closer camera; the plate reads **2,483**, and `--locate` inside it now answers the slab where it answered a wall |
| `shed` | `882B8F9A7C671B0D` | a doorway, two windows, a door leaf and a canopy; the 0.30 m plate **2,246 → 2,255** |
| `terrace` | `EDF3F4B3CF2A9686` | three units that touch, nine windows; **only `slab 2`'s plate is in frame, 2,525 — unchanged** |
| `wing` | `14E3E27F9F9E1A83` | host plus attached wing, windows on the weather faces; `slab 0`'s top went **0 → 380** and `slab 1`'s is 3 — unchanged |
| `workshop` | `7DD5D40894181EA7` | two storeys, a plate, five instanced shutters; the ground plate's top **1,844 → 3,082** and the mezzanine's is still 0 |
| `yard` | `FDC446B24E98CE04` | the courtyard and its gate; the two 1.20 m foundations still stand clear of the slope, **15,458 → 15,460** pixels of their own sides |
| `identity` | `974FB5B24E1E7A35` | the identity picture, and every pixel still names itself; the plate's top went **0 → 1,261** through the one opening |

**Every one of the nine moved, and for one reason: a mass is two solids.** The wall ring's inner base stands on the
ground line where the shell's floor disc stood a plate thickness lower, the plate is a solid of its own and brings
its own sides and top, and a doorway notches that base instead of being floored by one — so the six scenes that cut
no opening moved for the first two reasons alone and `shed`, `workshop` and `identity` for all three. A baseline that
held these nine still would have been the shell's, and a baseline predicting the new ones before the branch merged
would have been a guess; these are the numbers the merged tip renders.

**Re-recorded 2026-10-17, and the table is unchanged.** A later slice gave the flat-roofed arm the ground
plate it had been dropping, which adds a slab to every flat starter — and **no scene is a flat starter**,
so all nine hashes below still read what they read: `house B13154AD7D29F938` … `identity 974FB5B24E1E7A35`,
with the nine `authored_id` buffers and the 806 identity rows beside them unmoved. The evidence for that
claim, and the canary that was fired to make the claim mean something, are in *What the flat arm kept*
further down.

### The plates are drawn: seven of the nine moved, and two of the nine never needed to (2026-10-29)

Seven scenes gained a `floors` mesh — `house`, `hall`, `hall_large`, `shed`, `terrace`, `wing`, `yard` — written as
`workshop.rs` writes its own: a `packed_where` whose predicate names **exactly** `Element::Slab(_, _)`, its own
material, and instances under all three stages. `workshop` and `identity` did not move, and **neither of them is a
regression**:

- **`identity` already drew its plate, and the brief that asked for it here was wrong.** `authored.rs` packs through
  `packed()`, which is `packed_where(.., |_| true)` — a catch-all that keeps every element, `Element::Slab` among
  them. Measured on the frame **before any edit**: `slab 0, its top` is id 66 and covers **40 pixels**, and the hash
  was already `D0C45EF53EE51909`. Giving that scene a second, narrower plate mesh would draw the same faces twice
  over and move nothing; **the correct count for this change is seven, not eight.** The row counts below are the
  proof that no document changed: 806 rows, the same total as the table above.
- **`hall_large` needed no second edit either.** `hall_large()` calls `hall_at()`, and the `floors` mesh is inside
  `hall_at`, so one edit serves both scenes — the six source files cover seven scenes.

**How each frame was checked, and what it found.** `render --where "<element name>"` counts the pixels carrying an
element's id and reports its box, which is the picture's own answer rather than an inference from the source.
**These are the numbers as this slice left them on 2026-10-29; the baseline table above carries the current reading**,
and the section below this one records what changed them.

| scene | plate pixels | what is actually in the frame |
| --- | --- | --- |
| `house` | **5** | a 0.45 m plate under a wall with no opening: 1 pixel of top, 4 of edge |
| `hall` | **128** | the plate's top, seen through the glazing |
| `hall_large` | **0** | **nothing** — see below |
| `shed` | **1,483** | the 0.30 m plate's top through both windows |
| `terrace` | **2,528** | **`slab 2` only**; `slab 0` and `slab 1` are behind the row |
| `wing` | **4** | the far arm's plate, 3 pixels of top and 1 of edge |
| `yard` | **20,313** | both 1.20 m foundations, standing free of the slope |
| `workshop` | 3,318 | unchanged |
| `identity` | 41 | unchanged; `slab 0`'s 40 pixels plus one edge |

**Three of these are numbers about a limitation, not about a fault, and they are the honest result.**

**`house` and `wing` are 4–5 pixels, because a foundation plate is coplanar with the wall it carries.** `derive::starter`
spans its plate over the **whole** footprint, from one plate thickness below the outline's elevation up to it, so the
plate's four edges lie in exactly the planes of the wall ring's outer faces over that thickness. Two outward-facing
surfaces in one plane is a depth tie, and a tie is won by whichever instance is drawn last: the plate takes a handful
of texels and loses the rest. The scenes that show a plate properly are the ones that show its **top** through an
opening — `shed` at 1,483 and `hall` at 128 — and the one whose foundation genuinely stands clear of the ground,
which is `yard`. `derive::floor_slab`, which `workshop` and `identity` use, is not exposed to this: it is seated on the
mass's **inner loop**, so a first floor is inset by the wall's own thickness on all four sides and never shares a
plane with a wall. That is why `workshop`'s `floors` mesh has always been safe and a starter's foundation plate is not.

**`hall_large` moved its hash with zero plate pixels in the frame, and the reason is measurable rather than
mysterious.** Its document carries four meshes and the plate is one of them (`render --census` reports `meshes: 4`),
but all six of its plate surfaces cover **0 pixels**: the camera stands at `ridge * 0.75` from 26 units out, the plate
is a buried foundation flush with the walls, and there is nothing of it to see. The picture still changed, by
**54,040 pixels of a 1024 × 1024 frame — 5.2%** — while its `-authored_id` image is **byte-identical, 0 pixels**.
No pixel changed which element it shows; every one of them changed *colour*. The plate is a new shadow caster, its top
is coplanar with the ground at `y = 0` and its edges with the wall rings, so the shadow pass's depth ties resolve
differently once the instance list is one row longer, and the ground's own shading follows. Measured by checking out
main's `hall.rs` on this branch, rebuilding and rendering beside the branch's: main reproduces its own recorded
`11579A16F341F49F` exactly and this branch reproduces `8D144A5CAFEB71E7` exactly, so the pair compared is
main-against-branch and not two runs of one tree.

**`terrace` shows one plate of three.** `slab 0` and `slab 1` are the two units furthest from the camera, behind a
row of party walls; `slab 2`, the near unit, covers 2,528 pixels. All three are authored and all three are in the
mesh — this is the frame, not the document.

**The shed's foundation is 0.30 m, and the 1.20 m plinth belongs to the yard.** `shed.rs`'s `STARTER` carries
`slab: 0.30`; the deep foundation that stands a relief clear of a slope is `yard.rs`'s `SHED` and `STORE`, both at
`slab: 1.20`, and it is the yard's that the frame shows at 20,313 pixels. The shed's own plate is in the frame and is
not floating or buried in any way the picture can show: 1,479 of its pixels are its **top** seen through the two
windows, and the 4 more are one coplanar edge winning a tie.

And the probes behind them: `scene_identity_probe` reads **nine** tables two-way — 77 + 109 + 77 + 31 + 66 + 123 +
123 + 99 + 101 = **806 rows** in the gallery's own scene order, every id contiguous from the first and every name
round-tripping back to it — and the building crate's **twenty-nine** probes exit zero. `census`, `expand_probe`,
`validate_probe`, `authoring_probe`, `roof_planarity_probe`, `grid_probe`, `opening_probe`, `part_probe`,
`identity_probe`, `socket_probe`, `seat_probe`, `wall_probe`, `site_probe`, `slab_probe`, `instance_probe`,
`floor_probe`, `multi_row_probe`, `wall_part_probe`, `template_cache_probe`, `style_probe`, `instance_identity_probe`,
`terrace_probe`, `courtyard_probe`, `courtyard_widths`, `wing_probe`, `shared_face_probe`, `ring_probe`, and the
volume refactor's own two, `patio_probe` and `ring_fill_probe`.

### Two decisions of the project's owner, 2026-10-30: seat the starter's plate, and frame `hall_large` (branch `feature/building-plate-vis`)

**Both verdicts are the owner's and both are recorded here as taken, not as proposed.** The first closes the
depth-tie paragraph above; the second answers the `hall_large` paragraph above it.

**Decision 1 — `derive::starter`'s plate sits on the mass's inner loop**, the way `derive::floor_slab` seats a first
floor, keeping its top at the ground and its bottom one plate thickness below it. *The owner accepts the
consequence: the wall ring's base annulus will stand outside the plate, and the pictures will change.*

`floor_slab` itself could not be reused, because its guard is `elevation > lowest` and a ground plate's top **is**
`lowest`. **What the two share is the seating, not the guard**, so the walk is now one private function,
`derive::inner_loop(mass, id)`, that both `floor_slab` and `starter` call — the guard stays where it was, about where
a floor *is*, and the plate is written from the loop it produces. `validate` reads the same function, for the reason
in the next paragraph.

**A third change the decision forced, which was not asked for and is recorded rather than buried.** `Violation::
PlateCoverage` held a ground plate to the mass's **whole outline**, which is exactly what the decision removes: on a
5 × 8 shed with a 0.25 wall the new plate measures `33.75` against an outline of `40.0`, `short: 6.25`, and `shed`,
`workshop` and `identity` all `assert!(violations.is_empty())` — the tree went red on the first render. A mass's
floor **is** its inner loop, because the wall ring owns the annulus outside it and stands on the ground line across
it, so the check now measures a ground plate against `inner_loop` and holds a mass whose miter refuses to its
outline. `validate_probe`'s control still fires, unchanged in kind: `PlateCoverage { plate: 50.759995, outline:
66.56, short: 15.800003 }` for a plate pulled half a metre further in on a 7 × 11 starter.

**Decision 2 — `hall_large` takes its own camera**, lower and closer than the shared three-quarter view, so the
building's plate is in its frame. *The owner accepts the composition change.* `hall_at` keeps its signature and its
picture: the shared view is now the named function `three_quarter`, `hall_large` passes `close_and_low`
(`ridge * 0.55` instead of `ridge * 0.75`, a plan distance of 23 units instead of 29).

#### What each frame says, measured by `render --where "<element name>"`

| scene | plate pixels before | after | frame hash moved | why |
| --- | --- | --- | --- | --- |
| `house` | **5** | **0** | yes | the plate is buried and the house has no opening to look through; the 5 were four coplanar edge texels and one tied top texel, and the tie is gone |
| `hall` | **128** | **665** | **no** | the id image changed and the picture did not: the patch of floor the glazing sees was already inside the wall line, so the frame is byte-identical (`0D196DA8035448CB`) |
| `hall_large` | **0** | **1,656** | yes | its own camera; **see the reading below** |
| `shed` | **1,483** | **2,246** | yes | the plate's top is now inside the wall ring rather than under it, so the floor through the doorway and the two windows is the plate's own surface |
| `terrace` | **2,528** | **2,528** | **no** | `terrace.rs` writes its own plates over each unit's whole footprint and never called `derive::starter` |
| `wing` | **4** | **4** | **no** | `wing.rs` does the same for each arm; **the brief that asked for `wing` to move was wrong about this scene** |
| `workshop` | **3,318** | **1,987** | yes | the ground plate loses its coplanar edge texels; the mezzanine's own 143 are unchanged |
| `yard` | **20,313** | **15,458** | yes | both 1.20 m foundations stand clear of the slope and are still visible — inset by their own wall, so 4,855 fewer pixels of side |
| `identity` | **41** | **0** | yes | the same burial as `house`, through a catch-all mesh that draws every element |

**Three of the nine did not move: `terrace`, `wing` and `hall`.** The first two never took their plate from
`derive::starter` — `wing.rs:90` and `terrace.rs:90` each build a `Slab` over their own footprint by hand — so no
change to the starter could reach them, and the brief's expectation that `wing` would stop reading 4–5 pixels could
not have been met without editing a scene the brief also forbade editing. The third moved in the id image and not in
the picture, which the next paragraph is about.

#### What `--where` counts, and the number that is not a floor

**`hall_large`'s 1,656 pixels are not floor a reader can see, and the record says so rather than banking them.**
The plate's top and `hall_large`'s ground are both at `y = 0`, so every pixel of the plate is a depth tie the shading
pass settles by rasterisation rather than by depth. Three cameras were measured at the same scene: `ridge * 0.75`
(the shared view) reads **0**, `ridge * 0.70` reads **30**, `ridge * 0.55` reads **1,656** — and the 1,656 are
scattered through a box that also holds 112,808 pixels of wall. `--locate` at the centre of `hall`'s own 665-pixel
plate box answers `mass 0, outer wall over edge 2`, so **those numbers are id-buffer coverage, not visibility**, and
the brief's premise — that the plate was a buried foundation whose sides a lower camera could find — was never true
for a building on level ground. What the new camera does achieve is a building that fills its frame instead of
floating in it, which is why it was chosen and why the count is recorded beside the reason rather than instead of
it.

**The scenes that show a plate honestly are the ones where it is genuinely in the open**: `shed` at 2,246 and
`workshop` at 1,987 through openings, and `yard` at 15,458 standing clear of a slope.

And the probes behind them: `scene_identity_probe` still reads **nine** tables two-way and **806 rows** is unchanged
(77 + 109 + 77 + 31 + 66 + 123 + 123 + 99 + 101), and the building crate's **twenty-nine** probes exit zero.

### The ground is not drawn under a building (2026-10-31, branch `feature/building-ground-holes`)

**The decision was the owner's and is implemented as taken.** A plate's top and the ground are both at `y = 0`, so
every visible pixel of a plate was a **depth tie** and the raster settled it by draw order — which is what the paragraph
above measured and could not interpret. `gallery::ground::mesh` now builds each scene's ground exactly as
`geometry::heightfield` built it **with the building's own floor cut out of it**, and nothing else about the scene
changed: no geometry, no camera, no material, no widened element predicate, and the ground still carries no
`authored_id` and still writes the reserved zero from a constant.

**The hole is the ground plate's own plan loop, read from the document the scene already builds**, and not the mass's
whole footprint. Both are inside the mass, and the difference is what the frame shows: `derive::starter` seats its plate
on the mass's **inner loop**, so cutting the outline would only take away ground that the wall ring's base annulus
already stands over and that no camera above the ground can see, while `terrace` and `wing` author their plates over the
whole footprint and there the hole reaches the wall's outer face exactly. One rule read off the document is the right
hole in both cases and leaves no moat in either, which a hole quantised to the ground grid could not do — a ground cell
is metres across and a building is metres across too. A slab is a ground plate when its own two elevations span its host
mass's lowest node; that is what excludes `workshop`'s and `identity`'s mezzanine, which is a `Slab` too.

#### The measurement that can fail: `--locate` on a floor pixel

A count was proved misleading last slice, so the reading here is a **pixel asked what it is**, and each pixel below has
plate on all four sides two pixels off, so it is inside the floor rather than on its edge:

| scene | pixel | `--locate` answers | `--where "slab 0, its top"` before | after |
| --- | --- | --- | --- | --- |
| `hall` | 236, 668 | `slab 0, its top` | 665 | **3,166** |
| `hall_large` | 133, 704 | `slab 0, its top` | 1,656 | **2,483** |
| `shed` | 632, 606 | `slab 0, its top` | 2,246 | **2,255** |
| `terrace` | 815, 620 | `slab 2, its top` | 2,525 | **2,525** |
| `wing` | 401, 558 | `slab 0, its top` | 0 | **380** |
| `workshop` | 744, 724 | `slab 0, its top` | 1,844 | **3,082** |
| `identity` | 593, 659 | `slab 0, its top` | 0 | **1,261** |

**Two of the nine have no floor pixel in the frame at all, and the record says so rather than choosing another pixel.**
`house` has no opening and its plate reads **0** on every surface, before and after, so the pixel a floor would be at
`(250, 700)` answers `mass 0, outer wall over edge 2` — the wall, which is right, and the frame simply has no floor in
it. `yard`'s plate tops read **0** too, because both foundations are seated on a terrace above the slope and their tops
are the room's floor seen through nothing; what the yard's frame shows of a plate is its sides standing clear of the
ground, and those name themselves: `(354, 626)`, `(352, 626)`, `(356, 626)`, `(354, 624)` and `(354, 628)` all answer
`slab 0, the edge from node 3`, and `(800, 410)` answers `slab 1, the edge from node 3`.

**The before column was measured, not copied.** Main's gallery sources were checked out into the worktree, rebuilt and
rendered beside the branch's; main reproduced all nine recorded hashes exactly (`D6EDBADCC7BD6C3D`, `0D196DA8035448CB`,
`26EBA2656E1A00CF`, `32E70B7865492056`, `2326C57243C9B30A`, `965DEDC907148149`, `B85B0988EC240016`, `9E6AF5F04A386CF8`,
`F1D17F028AB5D4BC`), so the pair compared is main-against-branch and not two runs of one tree. One number differs from
the table above it: `workshop`'s ground plate measured **1,844** here against the recorded **1,987**, because that earlier
reading was taken under a different element name and this one is taken under `"slab 0, its top"` on both sides.

#### What this slice got wrong about its own brief

**The brief's premise is right for eight of the nine and wrong for `yard`, and the ninth is `yard`.** "A building's plate
is at `y = 0` and the ground is also at `y = 0`" is true for the eight scenes whose ground is `heightfield(_, _, |_, _|
0.0)` and false for `yard`, whose ground is the slope `0.16 * x` and whose foundations are seated on terraces above it.
`yard` has no tie to break: its plate's top was 0 pixels before this slice and is 0 after, and its sides moved by two
pixels out of 15,458. The rule is still applied to it, because the rule is read from the document rather than from a
per-scene list, and on `yard` it removes ground that was inside the foundation solid and could not be seen anyway.

**The brief also said "pick a pixel well inside the building's floor area in the frame" as though every scene had one,
and two do not.** `house` and `yard` are reported above as what they are rather than answered with a pixel that happens
to be nearby.

**And the first cut of the ground was wrong in a way the counts alone would not have caught.** The first implementation
wrote each cut piece's corners as one triangle, so a piece with four corners shifted every triangle after it by one —
the shed's ground came out **86,446 points, which is not a multiple of three**, and the frame showed wedges of missing
ground across the whole foreground with the sky through them. The `--where` counts still looked like floors. What found
it was the `position` debug channel against an untouched scene as a control: `monoliths`, whose ground this slice does
not touch, draws a continuous one. Each piece is now fanned from its own first corner, and the point count is a
multiple of three again.

The probes behind this one are unchanged: `scene_identity_probe` reads nine tables two-way at **806 rows**
(77 + 109 + 77 + 31 + 66 + 123 + 123 + 99 + 101), and the building crate's **twenty-nine** probes exit zero.

### The store's plate, and why `yard` is the one hash this did not move (2026-10-28)

`yard` is the only scene that **composes** two derived buildings into one document rather than deriving one:
`site()` takes `derive::starter(&STORE)`, shifts the mass and the roof up the slope, and moves `store.masses[0]`
and `store.roofs[0]` into the shed's skeleton — and never carried `store.slabs[0]` across. A mass is a wall ring
**and** a plate, so the store was a building with no floor in the volume model, and the table said so before
anything was changed: `yard` held two masses and ended at **`slab 0`**, where `wing` holds two arms and ends at
`slab 1` and `terrace` holds three units and ends at `slab 2`.

The plate crosses now, hosted by `MassId(1)` — the id the store's mass holds in this skeleton and not in its own —
and its **footprint shifts with the mass**, which the mass's own loop cannot reach, because a `Slab` carries plan
positions and no elevations. Left at the origin it would have landed under the shed and `Violation::PlateCoverage`
would have named the store. Seated, the store's plate spans `x 12..=16` with its top at `2.56` and its host mass's
lowest node at `2.56`: **2 masses, 2 slabs, 0 violations.**

**The frame did not move, and that is the second half of this measurement.** The plate adds six faces to the
document — `yard` goes from 95 rows to **101**, ending at `slab 1` — but `yard.rs` packs only `Element::Mass`,
`Element::Roof` and `Element::Wall` into its meshes, and `Element::Slab` is a fourth variant that none of the three
predicates can match. **The yard has never drawn a plate, the shed's included**; `workshop` is the one scene that
packs `Element::Slab` into a `floors` mesh, and the other six draw none either. The six new ids land at the end of
the table (96..=101), so no existing id moved — which is the mechanical reason not one pixel of `7F1D25D497EF50B0`
changed. A plate the frame cannot draw is still the floor the volume model measures, and that is what was missing;
drawing it would be a second change, and it would reveal the shed's own plate at the same time.

**The `identity` frame is no longer the number a shader rewrite has to leave alone.** It was byte-identical through
every slice that touched the shader generator, and the volume refactor is the first change to move it — which is the
point rather than a loss: a channel's plumbing and a document's geometry are separate concerns, and a slice that
changes the geometry was always going to move this picture. What the hash is for now is what the other eight are for,
the next slice's number to measure against.

## What the terrace asked, and the answer the vocabulary already gave (2026-10-27)

A terrace is houses side by side sharing one wall, and the vocabulary has no "shared wall". So the slice started
with the question rather than the picture:

> Is a party wall **one wall claimed by two masses**, or **two leaves whose outer faces meet**?

The second reading costs nothing if the crate accepts two solids that touch. `examples/terrace_probe.rs` measured
it — two 6 × 8 masses flush at `x = 0`, a 0.15 m wall each:

| measurement | value |
| --- | --- |
| violations | **0** |
| the left house | manifold **true**, boundary loops **0**, volume **32.3520** |
| the right house | manifold **true**, boundary loops **0**, volume **32.3520** |

**Touching is allowed**: a zero-volume meeting is not an overlap, and `validate`'s overlap judgement was written
against interiors rather than against faces. So a terrace is **two closed solids whose leaves meet** — 0.15 + 0.15
of wall that reads as one 0.3 m party wall, which is how a great many real party walls are built. No new
vocabulary, no new violation, no decision owed.

That is worth recording as a *method* as much as a result: the slice's first artifact was a probe about the
**domain**, not a scene. The alternative — build the terrace and discover the refusal halfway through the picture —
is how a system ends up with a violation nobody can date.

## What the terrace found: a row is three buildings that touch (2026-10-27)

With the vocabulary question answered — touching is allowed — the scene followed: **three units of `COTTAGE`'s
proportions, flush at their party walls**, each its own closed solid, each with its own gable roof running along
the row.

```text
locate terrace (500, 600): mass 2, outer wall over edge 2
where terrace mass 0, … edge 2: id 3 covers 15847 pixel(s)
where terrace mass 1, … edge 2: id 29 covers 35505 pixel(s)
```

**Each unit answers as itself** — the row is not one building with two internal walls, it is three buildings that
touch, and the frame's table says so in three id ranges.

Two decisions the scene had to make, both recorded in its own docs:

- **The party leaf is 0.15 m, not the style's `1/19`.** A party wall is thin because nothing weathers it; the
  style's relation describes a wall that faces the sky. That is the one number in the document the style does not
  decide, and the terrace's own type does.
- **Each unit's roof is a gable along the row**, so the ridges are parallel and the party walls read as the
  valleys between them. A hip per unit would have cut the row into three separate cottages — the form is the
  terrace's, not the style's.

The `terrace_probe` that preceded it is the reason this took one round instead of three: the domain question was
answered by measurement, and the scene inherited the answer.

## `validate()`: reports, never repairs

The R4.1 contract, in `mountain`'s measured shape (`validate.rs:160-202`, `expand.rs:36-92`):

- **all** violations are returned, in a deterministic order, so two runs diff;
- checks are **staged** — downstream checks are skipped when the document cannot support them;
- ops never refuse an edit **except** the ones that would leave a dangling index, which refuse by
  returning `None` (`ops.rs:10-16`) — an invalid intermediate state is otherwise ordinary, and
  `validate` is the instrument that reads it;
- **build failure is a list**, not a first error;
- geometry self-checks live on the mesh itself;
- the reporting vehicle during implementation is an `examples/` probe, not a test file
  (`mountain/examples/census.rs` pattern; tests need the superior's permission per workspace rules).

Non-exhaustive violation list: unattached mass intersection; incomplete bay, roof or socket
correspondence; non-planar face beyond tolerance; mass not closed, or closed inward (negative
signed volume, D5); coverage — the plan-area comparison and bay/span tiling, as violations (D5,
D10); level or bay outside its face extent; orphaned part placement; part exceeding its bay; open
shell not marked open; zero-height storey; zero-width bay; collinear eave; tracing-artifact drift
(the double-authored eave the S3 rule exists to make impossible — kept as a check on imports and
derivations, not on authoring).

## The agentic loop

The loop already exists and is measured: compile → render → manifest → camera notes → contact
sheet, in `crates/device-rasterize/examples/render.rs` (the path the iteration-loop proposal gets
wrong at its line 6; scale notes have since been **removed** by `physical-light-units` and must not
be promised). What the crate contributes:

- `validate()` is the **cheap inner loop** — a wrong building is named before a frame is spent on
  it, and every geometry failure the three courtyard runs hit was a validate-shaped fact;
- the manifest's "the edit did not reach the documents this camera reads" verdict works unchanged,
  because a building compiles into the same document path a massif does;
- provenance (D6) makes the frame diagnosable per element;
- `derive` gives an agent a whole starting building in one call — starter masses, default grid,
  default hip roof — which it then edits as ordinary skeleton elements, the `Starter` pattern
  (`mountain/src/derive.rs:35-101`).

The role split the 2023–2026 literature converges on is adopted verbatim: the LLM **plans and
critiques** — it authors skeleton nodes, picks parameters, edits rule arguments — and the
deterministic expander **emits geometry**. The model never emits a number where a reference is
expected, and a hallucinated reference surfaces as a `validate()` diagnostic instead of broken
geometry ([survey §5](../investigate/procedural-architecture-techniques.md): LayoutGPT's visual
planner; CityCraft's "the LLM emits constraints, not numbers"; BuildingBlock's rule-based
hierarchical designs built by PCG; ProxyBuild's negative result on inferred correspondence).
`validate()` is therefore also the cheapest known guard against an LLM corrupting a good state.

## Surveyed and rejected as the representation

Recorded so a future design does not re-litigate
([survey §2, §5, §6](../investigate/procedural-architecture-techniques.md)):

- **Wave Function Collapse** — can return *nothing* (satisfiability is NP-hard), and a local edit
  can invalidate the whole solve. No editable hierarchy.
- **Layout optimization** (Make-It-Home-style) — a finished artifact with no edit handle; R3
  forbids relying on it.
- **Learned proxies / part-semantics inference** — *infer* the correspondence R4.1 requires to be
  authored; the strongest published attempt still reports over-smoothing, component collisions and
  structural corruption (ProxyBuild, 2026-09).
- **Graph-grammar rewriting** — a genuinely editable hierarchy (the survey's own credit), but rule
  applicability is a search, which R4.1 excludes, and solidity is unsolved.

Stochastic and search methods remain admissible in exactly one role: producing **default skeleton
nodes**, guarantee-free and immediately editable (R3), never the mesh.

## Relationship to `gallery/built.rs`: promotion, not demolition

`built.rs` stays; nothing is broken. The `building` crate becomes the layer *above* it in spirit —
the planning layer `built.rs`'s own header says it is not ("Nothing here names a place, a period or
a plant"). Migration is opportunistic: the P1 proof scene rebuilds one hand-built building through
the crate, and `foldyard.rs:822 push_ring`'s duplication is the natural first P2 consumer. Whether
`built.rs` forms eventually move into the crate is a later decision with a working precedent either
way. The invariants in its doc comments (winding rules, the coping-band logic) are measured
knowledge: P1 mines them into `validate` checks rather than letting them stay comments.

## Phasing

- **P1 — shell.** Mass skeleton; roof skeleton and the **building-side roof-solid expander** (D2 —
  named scope, not reuse); closed-solid and orientation checks **including the new signed-volume
  and coverage violations** (D5); `ops` with attach (split-host-first) and replace-semantics split
  (D10); `derive` defaults with forked RNG (D11, D12); shading groups (D13); the gallery proof
  scene. **Openings in P1 are composed as masses** — piers and a lintel, the way `WallGap` already
  thinks — and bay-mounted openings arrive in P2. The proof scene is the **terraces house**
  (`terraces.rs:361-367` unit wall and roof, placed by transform at `:533-579`): the only
  opening-free candidate, and its deliberately sunk roof joint (`:573`) exercises the
  authorized-intersection rule of S1. Exit: one rendered building in which every mass is closed,
  outward, and covered — and which refuses to be wrong silently.
- **P2 — facade.** Levels and bays with the grid invariants; the parts vocabulary with authored
  sockets; instance emission (D4); the identity **sibling** mechanism (D6); one coherent style set;
  `foldyard`'s shed and enclosure rebuilt as the second demonstration (their door and gate are
  bay-mounted openings).
- **P3 — site.** Several buildings, enclosure walls and courtyards, ground seating against
  `terrain` heightfields (D8), the site as the unit an agent composes.

Out of scope, all phases: city grammar; interiors and floor plans beyond slab extents; structural
simulation; weathering and damage; DSL and serialization; editor frontends; undo/redo service.

## What SPEC would say if this is adopted

SPEC is human-editable only, so this proposal states the delta rather than writing it: a
`building`-crate section in the shape of `SPEC/mountain.md` (motivation, R1..Rn, out of scope), plus
one line under the workspace description. **No `Topology::Quads` is required** (D3). The
mountain-side extension that D2 defers (footprint edges as belt sources) is likewise a human SPEC
conversation, recorded there so it is not lost. If adopted, a `SPEC/building.md` sibling to
`SPEC/mountain.md` is the natural home, and this file becomes the record of why.

## What the terrace's second reading uncovered: a defect, then a detector, then a proof (2026-10-27)

The terrace itself is the section above. What follows is what the *other* reading of a party wall found.

**Overlapping the two houses by the leaf** (the right one starting at `x = -0.15`, so both own the same 0.15 m
strip) reported **0 violations** — a strip whose pixels have two owners, accepted silently. `plans_share_area` was
`strictly_contains || strictly_contains || edges_cross`, and that pair falls between the predicates: no strict
containment, and collinear shared edges plus a corner touching an edge are not a proper crossing. **A test for
"shares area" made of containment and crossing misses overlap whose boundary is collinear** — which is exactly the
shape two rectangles sharing a wall have.

Three slices closed it, each one keeping the record honest about what it had:

1. **A detector**: a fourth predicate asking whether any **edge's midpoint** of one polygon stands strictly inside the
   other. It caught the shared leaf, left the flush pair alone (a shared edge's midpoint is *on* the boundary, which
   `TOUCH_TOLERANCE` excludes), and its own docs said it was a detector rather than a proof — a sliver crossing a
   corner would still slip.
2. **The proof**: `crate::clip` computes the **shared area** — ear-clip both outlines into triangles, clip each pair of
   triangles against each other, sum the areas — and `plans_share_area` asks whether the sum exceeds
   `TOUCH_TOLERANCE²`. A flush pair answers **exactly zero**. The four predicates and their helpers are **deleted**
   rather than kept beside the computation, because two answers to one question is the shape of every drift this
   project has had to fix.
3. **The regression**: the two leaves still report 0 violations, the shared leaf still reports `UnattachedOverlap`,
   and all twenty-two building probes and the gallery suite pass unchanged — the evidence that a computed area found
   no false positive in any document the crate already draws.

The method is worth more than the fix: **the slice's first artifact was a probe about the domain, not a scene**, and
when the domain answered wrongly it was the probe — not the picture — that said so.

## What the courtyard asked: a hole is unexpressible, and a U is refused (2026-10-27)

A courtyard is a room whose plan is a ring, and a `Mass` is authored as **one simple polygon** — a ring is not
simple, so **a closed courtyard is not expressible by construction**. That is a vocabulary answer, not a
measurement.

What *is* expressible is a courtyard **open on one side**: a U-shaped outline with two reflex corners. And that is
where the measurement found something:

```text
# a U-shaped mass: 10 x 10 outside, a 4 x 6 courtyard open to the north
  the outline is 8 nodes, two of them reflex
  the document reports 1 violation(s)
    DidNotExpand { problems: [OffsetSelfIntersecting { mass: MassId(0), left: 2, right: 6 }] }
```

**A concave mass is refused**, and the two corners that refuse it are the courtyard's mouth. The inner wall loop is
derived by offsetting the outline inward, and at a 4 m notch with a 0.3 m wall the offset self-intersects — which
should not happen arithmetically, so either the offset's reflex-corner handling is wrong (the miter at a reflex
corner must move **into** the notch, not across it) or the refusal is too eager.

Both readings are recorded here and neither is decided in this slice:

- if the offset is **wrong**, the crate cannot draw any concave building — porches, wings, L-shapes, courtyards — and
  that is a substantial capability limit nobody wrote down;
- if the offset is **right**, then the domain genuinely stops at convex plans, and the refusal is the crate doing
  what it promises, with the limit now stated.

### The sweep that settled it: the refusal is not monotone, so it is a defect

Rather than print the offset loop, sweep the **wall thickness** — the one variable that separates a systematic
error from a geometric limit. A limit refuses thin walls and accepts thick ones; a bug can do anything.

| wall (4 m notch) | result |
| --- | --- |
| 0.05 m | **refused** — `OffsetSelfIntersecting { left: 2, right: 6 }` |
| 0.10 m | **builds** |
| 0.15 m | **builds** |
| 0.20 m | refused |
| 0.30 m | refused |

and sweeping the **notch** at a 0.3 m wall refuses 4, 6 and 8 m alike.

**No geometry refuses a 0.05 m wall and accepts a 0.10 m one.** The pattern is not monotone in the variable a limit
would be monotone in, so the refusal is a **defect in the offset**, not a boundary of the domain: `offset_loop`'s
miter produces a loop that `segments_cross` reads as self-intersecting for some wall values and not others. The
first attempt's own summary line — *"a thinner wall builds, so it is a limit"* — was wrong, and the probe now tests
**monotonicity** rather than "did anything thin build", because that is the property a limit actually has.

### The offset loop is right; the detector is wrong

Printing the loop settled it. For a wall of 0.05 m the offset is

```text
0: (-4.9500, -4.9500)   1: ( 4.9500, -4.9500)   2: ( 4.9500,  4.9500)   3: ( 2.0500, 4.9500)
4: ( 2.0500, -1.0500)   5: (-2.0500, -1.0500)   6: (-2.0500,  4.9500)   7: (-4.9500, 4.9500)
```

which is a **correct inward offset**: the outer rectangle shrunk by 0.05, the courtyard widened by 0.05, every
corner mitred cleanly, and **no self-intersection anywhere**. The 0.10 m loop is the same shape. So the geometry is
right at both walls and the **detector** is what refuses one of them:

`segments_cross` is asked about non-adjacent edges, and edges **2** and **6** — the two halves of the north face —
lie on the **same line** `z = 4.95` with disjoint spans. A crossing test that is not strict about collinear input
can report a "crossing" for exactly that pair, and the wall thickness then only decides by accident, because the two
edges' coordinates move with it. **A false positive in the self-intersection test**, not a defect in the offset —
and the non-monotone sweep is what that looks like from outside, which is why the sweep was worth more than reading
the formula.

**The next slice decided neither, and narrowed the place to look.** `offset_loop`'s miter is already general: for
each node it takes `turn = cross(directions[before], directions[node])`, and computes
`reach = cross(far − near, directions[node]) / turn` with `mitred = near + direction[before] * reach` — a negative
`reach` at a reflex corner, which is the correct retreat. There is no separable "convex case" to have got wrong.

But the arithmetic and the report disagree: for the U below, the two offset edges the problem names (edges 2 and 6,
the two halves of the north face) both land near `z = 4.7` over **disjoint** x ranges (2..5 and −5..−2), so
`segments_cross` should answer false. Either the miter overshoots by more than `MITER_LIMIT` (which would report
`MiterTooLong` first) or `inner` is not the loop the geometry suggests — and **the only way to tell is to print
`inner`**, which lives inside `crate::expand` and needs a temporary debug print or a debug-only accessor.

That is the next slice's first move: print the eight offset points for this outline and compare them with the eight
a hand offset gives ((1.7,4.7) and (−1.7,4.7) for the mouth, and so on).

**The next slice decides which**, by reading the offset at a reflex corner against a hand-computed answer: a U of
10 × 10 with a 4 m notch and a 0.3 m wall should offset to a loop that clears the notch by 0.3 on both sides.

## What the strict crossing fixed: concave outlines are expressible (2026-10-27)

`segments_cross` asked `first * second <= 0.0 && third * fourth <= 0.0`, which answers **true for two collinear
segments** — `0 <= 0` on both products. The offset check asks it about non-adjacent edges, so the two halves of one
face read as a crossing, and a concave outline's offset loop — which the debug print showed to be **exactly right**
— was refused for some wall thicknesses and accepted for others. That non-monotone pattern was the whole clue.

`segments_cross_properly` asks the strict question instead: each segment must have the other's endpoints
**strictly** on opposite sides, so a touch is not a crossing and a shared line is not one either. The non-strict
test is deleted, because it had exactly one caller and keeping it would be keeping the bug for the next caller.

Measured after the fix:

| case | result |
| --- | --- |
| the U-shaped mass (0.3 m wall) | **0 violations, manifold, 0 boundary loops, volume 79.1680** |
| the wall swept 0.05 → 0.30 | **every thickness builds** |
| the notch swept 4 → 8 m at 0.3 m | **every notch builds** |
| all twenty-two building probes and the gallery suite | **pass unchanged** |

**A concave plan is expressible now** — porches, wings, L-shapes and courtyards open on one side. The chain that got
here was four slices long and none of them changed the offset: a probe that measured the refusal, a sweep that
proved it was not a boundary, a debug print that showed the loop was right, and finally the one-line strictness the
detector needed.

## What the wing found: an L is two masses with an attachment (2026-10-27)

The L-shaped building raised one design question — *one concave outline, or two rectangles with the wing attached?* —
and the crate answered it before the picture did: `attach_mass` writes an `Attachment { host, station }` into the
wing, and `check_overlap`'s own docs say an attachment **ends the overlap question** because "a wing overlaps its
host by construction". So the L is **a host and an attached wing, one gable each**, which is also how an L is roofed
in practice: two ridges meeting at a valley rather than one saddle over both arms.

`wing_probe` measured it before the scene was written: **0 violations**, and host, wing and both roofs each
**manifold with 0 boundary loops** (volumes 26.8296 and 21.3692). One trap on the way: an attachment station of
`0.25` on a 6 × 6 host lands exactly on a corner, and `split_outline` **always inserts**, so the outline gained a
zero-length edge and reported `OutlineEdgeDegenerate`. A station off the corners (0.2) builds. The rule is worth
knowing because it only ever shows up as a degenerate edge.

The `wing` scene draws it and the channel names it: `where wing mass 1, outer wall over edge 2` answers **111,205
pixels** — the wing's own face, as itself.

## What the wing's windows found: the faces that meet the weather (2026-10-27)

The L's two arms share one leaf, so **not every face may take a hole**: the host's east face is where the wing
attaches and the wing's west face is the shared leaf, and a window cut there would open into the other arm. The
scene therefore cuts bays on the faces that meet the weather only — host `0, 2, 3` and wing `0, 1, 2` — one window a
bay at `HALL.glazing`'s inset.

Measured: the wing's identity table grew from **41 to 89 rows** (a window brings its sill, its head and its jambs
with it), the host's south face covers **40,282 pixels** as `mass 0, outer wall over edge 2`, and the nine-scene
probe still reads every table two-way.

That is an invariant the crate can state now that L-shapes exist: **a face shared between two solids is not a
facade**, and the vocabulary has no way to say so — `attach_mass` records the attachment, but nothing stops a
`split_face` on the shared face. It is the scene's job today, and it is written in the scene's own docs. If a later
slice wants the crate to refuse it, the attachment's station is what a check would read.

## Where the system stands (2026-10-27, written at a session's close)

**Nine building scenes, nine identity tables, no recorded debt.** The scenes are `house`, `hall`, `hall_large`,
`shed`, `terrace`, `wing`, `workshop`, `yard` and `identity`; `scene_identity_probe` reads every table two-way
(**705 rows**: 26 + 72 + 104 + 64 + 108 + 41 + 124 + 92 + 74), the gallery suite renders all nine, and the building
crate's twenty-four probes exit zero. Every reading in this document is a probe's output, not a picture's
impression — that is the standard this project has held itself to since the first census.

**What this session added, in the order it was found:**

| capability | what proved it |
| --- | --- |
| template caching (`TemplateCache`, `build_with`) | 12 placements expand **1** template; geometry byte-identical |
| `Style` — relations rather than numbers | the same footprint in two styles; three sizes of one style |
| the identity channel on every scene, in both directions | `--locate x,y` names a pixel; `--where <id\|name>` counts it |
| instanced identities (`Authored::Sum`, one material a placement) | three placements of one mesh, three different names |
| walls as part placement (`Target::Wall`) | a gate leaf hung on a run, closing 2.4 m of a gap |
| terraces (two masses that touch) | 0 violations, two watertight solids |
| overlap by **computed area** (`crate::clip`) | a shared leaf refused, a flush pair still accepted |
| concave plans (a **proper** crossing test) | U / L / porch all manifold with 0 boundary loops |
| an L as **host + attached wing** | 0 violations, four solids closed, windows on the weather faces |

**The boundaries that remain, stated so nobody rediscovers them as bugs:**

- **A closed courtyard is not expressible.** A `Mass` is one simple polygon; a ring is not simple. A courtyard open
  on one side is what the vocabulary has (see the `U` in `courtyard_probe`).

  **And it is refused, not silently mis-drawn** (`ring_probe`, 2026-10-27). A 10 × 10 ring with a 4 × 4 hole, authored
  the way a hole is smuggled into a single loop — round the outside, in through a neck, round the inside the other
  way, back out through the same neck — reports

  ```text
  DidNotExpand { problems: [OffsetSelfIntersecting { mass: MassId(0), left: 4, right: 7 }] }
  ```

  and does not build. So the boundary is **stated rather than discovered**, which is the crate's own standard for a
  domain it does not cover. Two things follow: **a hole in a plan is a feature to add, not a defect to repair**, and
  if it is added the refusal is where the new shape has to be admitted — `OffsetSelfIntersecting` is the mechanism
  that catches it, not the cause, and a vocabulary with holes would have to say *"this outline is a ring"* rather
  than *"this offset crosses itself"*.
- **A shared face is not a facade, and the crate does not say so.** `attach_mass` records the attachment, but
  nothing stops a `split_face` on the face two arms share; the scene avoids it by choosing which faces to split. A
  violation would need the attachment's station, and a new `Violation` is a compiler-enforced audit of every match.

  **Corrected by measurement (2026-10-27, `shared_face_probe`).** That sentence was too strong, twice over. First the
  cut was refused on an attached pair; then a three-arm experiment said **why**, and the answer is that the guard is
  the right one:

  | the same split and the same 2.1 m door, on… | accepted |
  | --- | --- |
  | a lone 6 × 6 room's east face | **yes** |
  | an attached pair's **wing outer** face | **yes** |
  | an attached pair's **shared** face | **no** |

  So the refusal is **attachment-aware**, not a grid or reveal limit: the same cut succeeds on two faces nothing
  shares and fails only on the one the wing overlaps. **The crate already keeps the invariant this record claimed it
  did not**, and the honest statement is the opposite of the original: a shared face is treated as not-a-facade by
  `add_opening`, and a scene that wants a hole there is refused rather than drawing one backed by the neighbour's
  wall.

  What is still missing is only the **name**: the refusal is silent — `ops::add_opening` answers `None` and
  `validate` reports nothing, so a caller sees a missing opening rather than a stated reason. A `Violation` variant
  would say *why*, and that is a compiler-enforced audit of every match; whether it is worth that cost is a judgement
  for the next slice rather than a fault to fix.

  **The judgement, made (2026-10-27): no `Violation`, because `validate` could never report this one.** A violation
  is a statement about a **document**, and the refused opening never becomes part of one — `add_opening` returns
  `None` and the face keeps its wall, so the document that exists is the one *without* the hole. A variant that
  `validate` cannot reach is a variant that would sit in the enum carrying no reading, which is exactly the
  placeholder shape this crate's discipline forbids.

  The refusal is already visible to the caller, at the call, as `Option::None`. What would improve it is naming the
  *cases* in `add_opening`'s own documentation — which it has — and the four scenes that cut openings all ignore the
  return value because a scene picks its faces deliberately. Naming the reason would mean an enum only the callers
  see, and the crate does not carry API for a caller that does not exist yet.
- **`site::report`'s overlaps are plan bounding boxes** and say so in their own docs: two L-shaped buildings can
  have overlapping boxes and no shared wall. The report **lists** and does not judge.
- **The shadow terminator toothing** is the shadow subsystem's, not this one's.

## The design for a plan with a hole, written before it is built (2026-10-27)

The last capability this crate lacks is a room whose plan is a ring. `ring_probe` measured that the failure is
**stated** (`DidNotExpand { OffsetSelfIntersecting }`) rather than silent, so this is a feature to add and not a
defect to repair — and the shape of the addition is worth writing down before any of it exists, because five parts of
the crate have to agree about it.

**The one idea: a void is authored, not smuggled.** A `Mass` gains `voids: Vec<Vec<PlanNode>>` — inner loops, each
wound opposite the outline, each strictly inside it and disjoint from its fellows. Nothing about the ring is inferred
from a self-crossing neck; the document says *here is a hole* in the same way it says *here is an outline*.

**What each part has to answer:**

| part | the question a void asks |
| --- | --- |
| `mass` | the field and its invariants: simple, inside, disjoint — reported or refused, never repaired |
| `expand` | the wall shell becomes *outline to void* for each void, and each void's own wall ring; the slab becomes a plate with a hole |
| `identity` | a void's faces need names: `void 0, its wall over edge 2` alongside `mass 0, outer wall over edge 0` |
| `openings` | a face on a void's wall is a **facade that looks inward** — a window onto the courtyard, which is the whole point of one |
| `validate` | three new faults (a void not inside, two voids overlapping, a void not simple) and one new surface arm |

**The acceptance criterion already exists**, and that is the nicest thing about this design. The crate measures
`2 − 2 × resolved holes` as a solid's Euler characteristic, so **a ring-shaped solid must come out at 0** — one hole
— where a plain box comes out at 2. A courtyard is therefore not a new kind of measurement: it is the number the
crate has been computing all along, finally with a shape that can exercise the non-zero case.

**What does not change:** the smuggled-neck ring stays refused, because it *is* a self-intersecting polygon; a
courtyard authored through a void is a different document, and the two must not be conflated. That is the same
distinction the strict-crossing fix drew between a shared line and a proper crossing.

### The ring fill: two attempts, both measured wrong, and what each ruled out (2026-10-27)

The prerequisite for any of the above is **a fill of the region between two loops** — the plate's top and bottom and
the wall's top ring all need it — and its acceptance is arithmetic: the triangles' areas must sum to the outer area
minus the inner area, to within `1e-4`. Two attempts were written, measured, and reverted:

| attempt | triangles | filled | wanted | what it ruled out |
| --- | --- | --- | --- | --- |
| bridge the loops into one polygon, ear-clip it | 6 | **87.0000** | 84.0000 | a bridged ring **is not a simple polygon**, so ear clipping emits overlapping triangles (+3 m²); excluding the duplicated vertices by position did not fix it |
| two-pointer walk over two convex loops, greedy smaller ear | 8 (the right count) | **140.0000** | 84.0000 | the greedy rule is a heuristic for monotone pieces, **not a triangulation**: the count is right and the area is 56 m² wrong |

**A correct triangle count is not evidence of a correct triangulation** — the second attempt would have passed any
test that counted triangles, and it is the area that said it was wrong. That is why the acceptance is a number.

What is left to try is known rather than speculative: **cut the annulus at the bridge into two simple polygons and
ear-clip each** (the bridge duplicated consistently, which is what the first attempt lacked), or a **monotone
decomposition** of the bridged ring. Either way the probe's acceptance does not change, and neither attempt's code
survives: both are reverted, so the tree carries the requirement and the number, not a half-right fill.

### The third rule (2026-10-27, and measured in the section below)

**Measured afterwards: 84/84, 92/92, 92/92 on three cases, and a non-convex loop refused with 0 triangles and 0
stray.** The rule written here is the one that shipped; the "not yet measured" this heading used to carry stopped being
true in the same session that wrote it.

The two-pointer walk was right about the *shape* of the answer and wrong about the *rule*. The classic form of it
does not choose the smaller ear; it chooses the ear whose triangle **holds no other vertex of either ring**:

```text
candidate A = (outer[i], outer[i + 1], inner[j])      // advances the outer loop
candidate B = (inner[j], inner[j + 1], outer[i])      // advances the inner loop
take A when only A is clean, B when only B is clean, and the smaller when both are
```

That is a different question from the one the failed attempt asked — *which ear is smaller* is a heuristic for
monotone pieces, while *which ear is empty* is the ear-definition itself — and two convex loops are precisely the
case where at least one of the two candidates is always clean. The helper it needs is a **strict** point-in-triangle
(the boundary-inclusive test in `clip.rs` would veto every ear whose leg runs along another loop's edge, which is the
same trap that broke the first attempt's duplicate exclusion).

**This is the design to implement next**, and the way to implement it is what the last two rounds taught: write the
fill, **print every triangle's three vertices and its area for the centred rectangle case**, check by eye that each
lies in the annulus before trusting any total, and only then compare the sum with `84.0000`. A correct triangle count
has already lied once.

### The ring fill works: three rules, and the symmetric case that needed a fourth (2026-10-27)

`crate::fill::ring_fill` fills the region between an outline and a loop inside it, and all four measured cases are
**exact**:

| case | triangles | filled | wanted | stray triangles |
| --- | --- | --- | --- | --- |
| centred 4 × 4 void in a 10 × 10 outline | 8 | **84.0000** | 84.0000 | **0** |
| off-centre void | 8 | **92.0000** | 92.0000 | **0** |
| triangular void | 7 | **92.0000** | 92.0000 | **0** |
| a notched (non-convex) void | **0** | — | 88.0000 | 0 — refused, as designed |

The probe prints every triangle and checks each one's **centroid** against the annulus before it trusts any total,
because the third attempt's predecessor had the right triangle *count* and 140 m² of overlapping ones. Three rules
were needed, and each was earned by a measurement:

1. **The ear rule is "holds no vertex", not "is smaller"** — the failed walk used the smaller-area heuristic and
   produced 140 m² against an 84 m² annulus.
2. **A candidate must also cross no ring edge** — the first pass of the vertex rule let a triangle whose corners all
   sat *on* the hole cover the hole's interior: its edges crossed the hole's boundary, and the fill leaked to 168 m².
3. **A candidate must not contain the hole's centre** — and this one is the pretty one: in the **symmetric** case a
   chord from one outer corner to the hole's corner passes *through* the hole's opposite corner, so it crosses
   nothing **properly** and contains no vertex strictly, yet it runs straight through the hole. The off-centre and
   triangular cases already measured exact; only the centred one failed, and only because symmetry made every
   crossing degenerate. **A centre-inside test is what a symmetry needs.**

`ring_fill` is the prerequisite for a plan with a hole: the plate's top and bottom and the wall's top ring all need
it. The next slice is `Mass::voids` plus the shell, and the acceptance there is already written: a ring-shaped solid
must measure an Euler characteristic of **0**.

### The slice that remains, and the simplification that shrinks it (2026-10-27)

`expand_mass` builds a mass's faces as **three per outline edge** — `OuterWall { edge }`, `WallTop { edge }`,
`InnerWall { edge }` — plus the two plate surfaces it then called `FloorTop` and `SlabSoffit`, which the volume decision below replaced with a `Slab` solid carrying the plate. A void needs the same three around
*its* loop, which reads as a second, smaller outline inside the first.

**The simplification: the plate does not need a hole.** A courtyard has a floor — it is paved, and it is the same
slab the rooms stand on — so those two stayed exactly as they were, and `ring_fill` was not needed for
them at all. What a void actually adds is:

- **the wall ring around it**, three faces per void edge: the courtyard-facing wall, the room-facing wall, and the
  strip of wall top between them (the mirror of `OuterWall` / `InnerWall` / `WallTop`);
- **its names**, so `--where` can say *void 0, the wall over edge 2* rather than *something in mass 0*.

That leaves `ring_fill` where it is — a tested tool waiting for the one surface that *does* need a hole, which is the
wall top of a mass whose own outline is a ring, and that is a later question than a courtyard.

So the next slice is smaller than the design first said: a `voids` field, three faces a void edge, the names, and
the acceptance **Euler characteristic 0**. The plate work and the fill of a ring-shaped mass are deferred, and both
are written down rather than discovered again.

## The volume decision: walls, plates and roofs as solids (2026-10-27, the project's own)

The courtyard slice stopped on a modelling question, and the answer came back as a direction rather than a patch:
**make walls, floors and roofs volumes, not faces of one shell.**

That decision does not merely unblock the courtyard — **it removes the reason the courtyard was hard.** The shell
model forced one closed surface to carry both the plate and the courtyard wall, and the wall standing *in the middle
of* the plate then had to produce a three-surface edge (measured at the outline elevation: the wall's inner base would
have to pair with both the paving and the courtyard-facing face). No winding fixes that; it is inherent.

With volumes the same courtyard is **trivial**:

- a mass is **two solids** — the wall ring and the plate — and **each closes itself**;
- the courtyard is the wall ring carrying a **void loop**, so it becomes a ring-shaped solid: closed, manifold, and its
  Euler characteristic is **0** where a plain box is 2 — the number the crate has been computing all along;
- the plate is a box under both the rooms and the courtyard, so nothing has to be split, and the three-surface edge
  never arises because no face is shared between the plate and the wall in the first place.

**What this costs, stated plainly:**

1. **`SolidId` gains parts.** One mass can no longer be one solid: it is a wall and a plate, so
   `SolidId::Mass(MassId)` becomes two kinds (or one kind with a part). The enum is matched across the crate, so the
   compiler will find every site — the same audit that made `MassSurface` and `Violation` safe to change.
2. **`expand_mass` is rewritten**, not patched: the shell's vertices, `WallFace`, `EdgeNode`, `resolve_holes` and the
   plate's disc are replaced by two closed volumes. `WallFace`'s hole machinery is still needed for windows, but it
   now cuts **a wall volume**, which is the ordinary "wall with a hole" problem rather than a shell's.
3. **`validate`'s solid checks stay meaningful and get simpler**: every volume answers manifold and Euler on its own,
   so a courtyard's ring is `euler 0` by construction rather than by luck.
4. **The roof is already a volume** and the `Wall` and `Slab` primitives already are too, so the change is confined
   to the mass — which is where the shell's complexity lived.

**The shell work is deleted, not merged.** `feature/building-patio3` carried a working courtyard emitter against the
shell; with the model changing underneath it, keeping it would be keeping a careful solution to the abandoned
problem. It is removed from the tree and the handoff points at the new plan instead.

**What the implementation measured against that list (2026-10-27, branch `feature/building-vol2`):**

1. **Item 1 did not apply: no `SolidId` variant was needed.** `SolidId::Mass(MassId)` is the wall ring and the
   plate is authored as a `Slab`, which the crate already had as a first-class volume — `derive::floor_slab` derives
   one from a mass, and two scenes were already drawing their plates that way. The compiler-enforced audit the list
   predicted shrank to zero sites, because only the mass was pretending not to be a volume.
2. **A ring is a torus, so the sphere check has to count handles**, and one of them is the ring's own: measured, a
   mass with no opening reads `0`, one window `-2`, two windows `-4`, and **a doorway reads `0` as well — it is a
   notch in the wall's section and not a tunnel through it**, because it leaves a lintel and there is no material
   under its sill for a tunnel to be cut from. The count is therefore over the openings that are *not* doorways.
3. **A doorway notches the base, and a sill standing on the wall's own floor is not a face of its own.** Both halves
   land together: the base strip is not drawn across the opening, and the sill reveal is not drawn either, because the
   two name the same four points and drawing both is the same face twice — measured as `SolidNonManifold` with an odd
   Euler characteristic, which is an orientation failure and nothing else.

## What the courtyard slice measured (2026-10-27, branch `feature/building-court`)

The courtyard is drawn, and the refusal that said it could not be is deleted in the same commit that draws it
(`Problem::VoidNotDrawn` is gone; nothing carries that name).

1. **The plate is not cut, so the three-surface edge never arises.** The courtyard is a **second ring**, built exactly
   as the mass's own ring is: the void loop, the same loop offset **outward** by `mass.wall`, the strip of wall top
   between them, and the annulus at the void loop's own elevations. The plate is a solid of its own beneath both, and
   the courtyard's floor *is* the plate's top. Measured on a 4 × 4 courtyard in a 10 × 10 mass with a 0.3 m wall:
   `manifold true`, `loops 0`, `euler 0`, volume **16.5120 against the annulus times the height 16.5120** (0.000008 off
   — `f32` noise, and the arithmetic is in the probe rather than compared against a second reading of the same code).
2. **`SolidId` gains a kind, and it is `Void { mass, void }`.** The alternative — folding the ring into
   `SolidId::Mass` — was measured green **for the wrong reason**: the mass's ring reads `0` and the courtyard's reads
   `0`, so one face set holding both sums to `0`, which is exactly what `SolidNotASphere` demands of a single ring.
   Every predicate still passes (each edge used twice, no boundary, two positive volumes), so the closure checks would
   have kept passing on a solid that is not a solid. With the ring its own solid the arithmetic is per-solid again.
3. **`MassSurface` loses a variant and gains one.** `VoidFloor` is deleted — it existed because the shell's plate could
   not be one disc across a courtyard, and in the volume model the plate is not cut at all, so a "paving" element
   would name a face no expander emits. `VoidBase` is added: a ring is open at both ends, and the courtyard's ring is
   no exception. `VoidWall`, `VoidInner` and `VoidTop` stay and mean what they said.
4. **`fill::ring_fill` is no longer on the courtyard's path, and the convexity restriction is kept anyway.** Nothing is
   filled: the ring is the mass's own construction, one quad per loop edge between two loops of the same length. The
   restriction that survives is therefore not the filler's — it is that **this crate has no measurement of the
   *outward* walk across a reflex corner**. A mass's outline is offset inward and a void is offset outward, and the
   corner that fails in one is not the corner that fails in the other; a ring fill has been shipped wrong by area twice
   here and the inward offset has closed a courtyard's mouth. So a non-convex void is **refused by name**
   (`Problem::VoidNotConvex` in the expander, `Violation::VoidNotConvex` in the document) rather than walked. What
   would lift it is a measurement, and `offset_loop` is the walk that would be measured.
5. **A fourth courtyard fault, because the other three cannot see this one.** `Violation::VoidReachesTheWall`: a void
   loop inside the outline is not a courtyard that *fits*, because its ring stands `wall` outside that loop, so a void
   authored within a wall's width of the outline has its wall standing in the mass's own wall — and two rings
   occupying the same material are **two sound solids**, so nothing else in the validator can see it. Measured on a
   void at ±4.9 in a ±5.0 outline: `over: 19.800018` m², and `VoidNotInside` correctly *silent*, because the loop is
   inside the outline and only its wall is not inside the rooms. Both loops in the check are `offset_loop`'s own walks,
   so the document half cannot disagree with the drawing.
6. **The nine scene hashes did not move**, and the baseline was re-measured at `cc7aaa5` first so a move could be
   attributed rather than argued about: all nine reproduce the table below exactly, before and after.

## What the courtyard's wall slice found: one walk, and one flag (2026-10-27, branch `feature/building-court-open`)

The courtyard was drawn but **its wall took no openings** — there was no door from the rooms into it and no window
through it, on either side. This slice makes an `Opening` cuttable into it, and the answer turned out to be one
question asked once rather than a second cutting engine.

**The design: a face names a loop, and everything downstream asks which one.** `FaceRef` gains
`void: Option<usize>`, so a rhythm, a bay, an opening and a placement can be authored on a courtyard's wall exactly
as they are on the building's — `FaceRef::court(mass, void, edge)` is the whole new vocabulary, and
`FaceRef::new` keeps the mass's own. That is the unification D10 wants (a bay is a slot, and a slot is a slot
whatever it is a slot of), and it is why the slice needed no second grid, no second bay and no second `Opening`.

**The expander's answer is one walk with one flag.** `courtyard.rs` had its own four-quads-per-edge ring; the mass
had `expand_mass`'s. Both are *two loops of the same length, one ground line, one top*, and they differ in exactly
one fact — **which way the offset went**. So `Ring { loop_of, walked, offset, wall, top, side }` carries it and
`ring_walk` is the mass's own walk with `OffsetSide::Outward`. Every winding then reduces to
`Ring::authored_is_outer()`: a mass's authored loop *is* its outward-facing wall, a courtyard's is the face that
looks into the courtyard, and **reversing a quad's corners is the entire difference**. That is the kind of fact
that is invisible to every other predicate — a courtyard wound inside-out is closed, manifold, of the right Euler
characteristic and *positive* volume if you also got the sign — so it is stated at the type.

**`MassSurface::Reveal` gains the loop, and it had to.** A reveal in a courtyard's wall is a face of `SolidId::Void`,
not of `SolidId::Mass`; without the loop it would have been gathered into the mass's face set, two disjoint rings in
one solid, and their Euler characteristics would have summed to `0` for exactly the wrong reason — the same trap
`SolidId::Void` was added to escape, re-entered through the one surface a hole can be on either ring.

**Every opening refusal now names the loop**, because "edge 1" is two different pieces of masonry. The four
`Problem` variants swap `mass: MassId` for `loop_of: AuthoredLoop`, which the offset walk was already carrying for
the same reason.

Measured by `examples/patio_probe.rs` on a 10 × 10 mass, a 0.3 m wall, a 4 × 4 courtyard, three bays across and two
storeys on the courtyard's own edge 0:

| document | courtyard ring | mass's ring |
| --- | --- | --- |
| no opening | manifold, `loops 0`, **euler 0** | euler 0 |
| one window in the wall | manifold, `loops 0`, **euler −2** = `2 − 2(1 + 1)` | euler 0 |
| one door, sill on the ring's base | manifold, `loops 0`, **euler 0**, base annulus 10 triangles where the un-cut ring draws 8 | euler 0 |
| door **and** window | manifold, `loops 0`, **euler −2** | euler 0 |

and the material: the window's ring reads **16.2920** against the un-cut ring's `(4.6² − 4²) · 3.2 = 16.5120` less
`0.7333 m² · 0.3 = 0.2200` — **0.000006 off**, the same `f32` noise the un-cut ring already carries. The door's reads
**16.3720** against `16.5120 − 0.1400` — **0.000004 off**. Four reveals per hole, named
`mass 0, the left reveal of opening 0 in void 0, edge 0`. **0 violations** in all four documents.

**Every refusal is reached by the probe and prints its name**, all four naming `loop_of: Void { mass: MassId(0),
void: 0 }`: a slot the rhythm no longer holds (`OpeningUnresolved`), a head at the ring's own top and a jamb at the
ring's own end (`OpeningOutsideWall`, `past: 0.001` — the grid's top row is the wall top, so a hole with no head inset
*is* the one this catches), two holes sharing wall (`OpeningsOverlap`), and a part in a courtyard's bay
(`PlacementOnACourtWall`).

### What this slice got wrong about its own brief, recorded because it is the useful part

1. **"a doorway that would eat the ring's whole thickness" is not a distinct case, and the reason is worth having.**
   A mass's doorway notches the ring's base annulus, and a courtyard's does the same by the same code: the probe
   counts **10 base triangles against 8** and the ring stays closed. What the brief may have meant — a hole standing
   *on the ring's own base* — is exactly the doorway, and it is measured above, not refused.
2. **"a station that overlaps a corner" is unreachable from a courtyard, and the geometry says why.** `resolve_one`
   refuses a jamb that lands off the **offset** loop's mitred ends, which is the miter's reach. A mass's offset is
   *shorter* than its outline, so a jamb can land in that reach; a courtyard's outward offset is **longer** at both
   ends, so no jamb can. The check is vacuous for courtyards and reachable only for masses — a real asymmetry between
   the two rings that the shared walk made visible and that no probe of a mass alone would ever have shown.
3. **"an opening whose sill lands on the plate"** is reachable, and it is `OpeningOutsideWall` like any other sill
   below the ground line — but only because `reaches_the_wall_edge`'s ground line is the **plate's underside**, one
   slab below where the ring stands. That is the shell-era reading the check was written against and it is what makes
   a door legal on either wall. Giving a courtyard its loop's own elevations instead — which looks more honest — makes
   a courtyard door illegal while the identical mass door is legal, for a difference that is in nothing but which loop
   the grid was laid out on. The probe is the control: the door reads **0 violations** only because both rings are read
   the same way.

### Still open

- ~~**A part in a courtyard's bay is refused, not built.**~~ **Closed by the part-in-a-courtyard's-bay
  slice below** (2026-10-27, branch `feature/building-court-part`): the frame is read off the void loop
  through the ring's own offset side, and `PlacementOnACourtWall` is gone from both halves of the crate.
- **An opening naming a void the document does not hold is named by `validate` (`OpeningBayMissing`) and dropped by
  `build`,** because there is no ring to cut into. That is the same as a mass's behaviour today and this slice did not
  change it, but it is the one opening fault with no `Problem` behind it.
- **`roof_planarity_probe` is not run-to-run deterministic** (its plane grouping is printed in hash order). Verified
  by running the same binary twice on this tip; it is pre-existing and unrelated.

## What the part-in-a-court slice found: the frame has to stay a rotation (2026-10-27, branch `feature/building-court-part`)

The item the last section left open is closed: **a part stands in a courtyard's bay**, and
`Problem::PlacementOnACourtWall` and `Violation::PlacementOnACourtWall` are deleted — the expander no
longer declines, so nothing carries those names.

**The unification is one derivation with three callers.** `expand::loops_of(skeleton, loop_of, wall,
problems)` answers *"the loop the author wrote, walked the way the expander walks it, and the loop a
wall's thickness away from it"*, and it decides the **offset side from which loop was named** rather
than from its caller — the mass's outline is offset inward, a void's loop outward, and that one flag is
already what every winding in `ring_walk` is derived from. `expand_mass`, `courtyard::ring` and
`expand::part_frame` all read that call, so the strip a part is framed on is the strip the ring was
built from, and "which way does a courtyard's wall go" has one answer in the crate rather than one per
reader of it. `courtyard::ring` lost its own winding copy and its own convexity check in the process;
the convexity restriction stayed with the **outward** walk it is about, inside `loops_of`.

Measured by `examples/patio_probe.rs` on a 10 × 10 mass, a 0.3 m wall, a 4 × 4 courtyard and a shutter
in bay (0, 0) of the courtyard's edge 0, pinned at (0.2, 0.1):

| reading | value |
| --- | --- |
| the bay | `[0.0000, 1.3333] × [0.0000, 1.6000]` |
| the frame | origin `(-1.8000, 0.1000, -2.0000)`, along `(1, 0, 0)`, up `(0, 1, 0)`, inward `(0, 0, 1)` |
| `(along × up) · inward` | **`+1.0`** |
| the document | **0 violations** |
| the part's solid | 32 faces, loops **0**, euler **0**, manifold, volume `0.025004` |
| where it stands | `x [-1.8000, -0.9000]`, `z [-2.0000, -1.9400]`, `y [0.1200, 1.4500]` |
| the courtyard's ring | manifold, loops 0, euler **0** — its reading |
| the mass's own ring | manifold, loops 0, euler **0** — its reading |

and `examples/part_probe.rs` prints the mass-wall frame beside its own solid so the pair that had to keep
reading is on the frame itself: origin `(3.5000, 0.0000, -5.0000)`, along `(0, 0, 1)`, up `(0, 1, 0)`,
inward `(-1, 0, 0)`, `(along × up) · inward = +1.0`, with `Part(0)` at `0.025002` and `Part(1)` at
`0.044745` — every number the probe has always printed, byte for byte.

### The measurement that decided it, and it is the most useful thing here

**"Seat the part in the ring's thickness" was built first, and it is not available.** The brief's
wording — *seated on the ring the way it is on a mass's wall* — asks for the part's origin on the
courtyard's face with its depth running away from the courtyard and into the ring. That needs `inward`
reversed, and `along × up` is the walker's **right** while a courtyard's loop puts its material on the
walker's **left** — so the frame becomes **improper**. `PartFrame::apply` on an improper basis
**mirrors** the template, and every one of its faces comes back wound the wrong way round. Measured on
the document above: the part builds, and the crate reports

```text
SolidInsideOut { solid: Part(0), volume: -0.02242782 }
```

which is `validate`'s own predicate saying what a mirror does to a closed solid, on a document with no
other fault in it. **A part that is inside out is a worse fault than a part that stands proud of the
face it was pinned to**, so the answer shipped is the frame that stays a rotation: the origin is on the
face the bay names and the depth runs *across* that face — into the masonry for a mass, into the
courtyard for a courtyard. A shutter or a bracket stands against the courtyard wall and a door leaf sits
in the courtyard's own reveal rather than 0.06 m inside it.

**Three ways out were checked before this was accepted, and all three are closed.**

1. **Reverse `along` instead**, which keeps the frame right-handed and puts the part in the ring — but
   the template's `x` then runs from the pin towards the bay's *start*, so the part occupies
   `[length − s − w, length − s]` instead of its bay. Worse, `Placement::reach` would still read the box
   as `[s, s + w]`, so `validate` would say the part fits while the mesh puts it three metres away. A
   pin that means one thing to the checker and another to the builder is the failure this crate has
   paid for more than once.
2. **Normalise a void to negative plan area** so its material falls on the walker's right. Then
   `OffsetSide::Inward` would carry the void's outward offset and `Ring::authored_is_outer` would return
   `true` for a courtyard — and be geometrically wrong, because a void's authored loop is the ring's
   *inner* face. It needs a second flag, which is the thing the last slice removed.
3. **Put the origin on the ring's outer face** — the offset loop, the one the rooms see — with the depth
   running into the ring. That frame is proper and is the best-behaved of all, and it is **not
   addressable**: a `FaceRef` names one loop, `FaceRef::court` names the void loop, and the offset edge
   is *longer* at both ends (measured by the courtyard-wall slice), so one arc length would not name the
   same span of wall there. A vocabulary gap, not an expander one — so it carries no `Problem` and no
   `Violation`, for the reason the shared-face item below records: `validate` could never report a
   document that cannot exist.

### What else this slice found

- **A fault a ring reports is listed once per reader of it.** A notched void with a part in one of its
  bays names `VoidNotConvex` twice — once from the ring walk and once from the frame. This is
  pre-existing and not courtyard-specific: a mass's part on a mass whose miter refuses already names
  that fault twice today, because `part_frame` derives the offset for itself. Left alone rather than
  deduplicated, because a list that collapses repeated problems would hide a fault reported by two
  independent reads of the same geometry.
- **`PlacementOutsideBay` became reachable on a courtyard's bay**, which it was not before: the
  court-wall check `continue`d before the containment reading was made. The probe reaches it with a
  1.8 m leaf in a 1.6 m bay, `past: 0.39999998`.
- **The nine scene hashes did not move** and `scene_identity_probe` still reads nine tables two-way
  (**800 rows**), with the building crate's twenty-nine probes at zero. `roof_planarity_probe` is still
  not run-to-run deterministic, re-verified here by running the same binary twice on this tip.

## What the rooms-facing side found: the frame was already a rule on all four faces (2026-10-27, branch `feature/building-court-rooms`)

The item the part-in-a-court slice could not close is closed. **A courtyard's wall has two addressable faces, a
part stands on either, and the one the rooms see is the seating that slice measured as unavailable.**

**The vocabulary is one word, and it is a word the crate already had.** `LoopSide { Authored, Offset }` was private to
`expand.rs`, where the ring walk winds by it; it is now `grid::LoopSide` and rides on `FaceRef` as `loop_side`. That is
the whole of the change in shape: a face names a loop, and the two loops of a ring are named by the word the walk
already used. `FaceRef::court_rooms(mass, void, edge)` and `BuildingSkeleton::room_faces_of` are the new spellings;
`FaceRef::new` and `FaceRef::court` build `LoopSide::Authored`, so **every face this crate could name before is the
value it was**, and the derived key order keeps them where they were — `mass`, `edge`, `void`, then the side last, so
`Authored` sorts before `Offset` within one (mass, edge, void) and nothing between two existing keys moves.

**The expander needed no new walk, which is the measurement that decided the design.** `WallFace::of` is the same two
lines with the loops swapped: `along` is the same direction for either face and the direction *towards the other loop*
is `side.apply(inward(along))` and then its reverse, which is the normal the offset walk already applied. What genuinely
differs is where the strip **starts and how long it is** — and that is why `BuildingSkeleton::face_length` is `O(1)` on
the authored side and `O(nodes)` on the offset side: the far edge is the mitre's edge, and asking the walk is the
cheaper of the two answers in everything but time. `face_extent` stays `O(1)` for both, because a ring's base annulus
runs between the offset corners **at the authored corners' own elevations** — the two loops of a ring share one ground
line, which `ring_walk` has always done.

**`inward` is the walker's left on every face of a ring, so `along × up` holds everywhere and what differs is only
which side the material lies on.** A mass's outline has it on the left, so a part runs *into* the masonry; a courtyard's
void loop has it on the right, so a part runs *out of* the ring and into the courtyard; a courtyard's **offset** loop
has it on the left again, so a part named there runs **into the ring**. That last one is the seating the part-in-a-court
slice could not have: it tried to get it by reversing `inward` on the courtyard's own face, which is what made the frame
improper, and it is reached instead by naming the other face — where no reversal is needed because the material is
already on the left. A mass's inner wall face is the fourth of the four and comes free with the same rule: its material
is on the walker's **right**, so a part named there stands proud of it into the mass's own rooms, which is the same
answer the courtyard's own face gives.

Measured by `examples/patio_probe.rs` on the same 10 × 10 mass, 0.3 m wall and 4 × 4 courtyard as the slice before it,
with the **same rhythm laid on both loops by the same op**, so the only difference between the two documents is which
loop each face names:

| reading | the courtyard's own face | the rooms-facing face |
| --- | --- | --- |
| the face | `{ mass: 0, edge: 0, void: 0, loop_side: Authored }` | `{ mass: 0, edge: 0, void: 0, loop_side: Offset }` |
| its length | `4.0000` m | **`4.6000` m** — 0.3 m longer at *each* end |
| bay (0, 0) | `[0.0000, 1.3333]` | `[0.0000, 1.5333]` |
| the frame's origin | `(-1.8000, 0.1000, -2.0000)` | `(-2.1000, 0.1000, -2.3000)` |
| `along`, `up` | `(1, 0, 0)`, `(0, 1, 0)` | `(1, 0, 0)`, `(0, 1, 0)` |
| `inward` | `(-0.0, 0.0, 1.0)` — into the courtyard | `(-0.0, 0.0, 1.0)` — into the ring |
| `(along × up) · inward` | **`+1.0`** | **`+1.0`** |
| the document | **0 violations** | **0 violations** |
| the part's solid | 32 faces, loops 0, euler 0, volume `0.025004` | 32 faces, loops 0, euler 0, volume `0.025004` |
| where it stands | `x [-1.8000, -0.9000]`, `z [-2.0000, -1.9400]` | `x [-2.1000, -1.2000]`, `z [-2.3000, -2.2400]` |

**How you can tell which loop it stands on from the numbers alone**, which is the part that has to be true rather than
argued: the two origins differ in `z` by exactly the wall's thickness and in `x` by the pin's arc plus the mitre's reach.
`z = -2.0000` **is** the void loop's own line — `rectangle(-2, -2, 2, 2)`'s edge 0 runs along `z = -2` — and `z =
-2.3000` is the same line one `mass.wall` outward, the courtyard's ring being offset **outward**. `inward` is `(0, 0, 1)`
in both, so the near-side part runs `z -2.0000 .. -1.9400` (into the courtyard) and the rooms-side part `z -2.3000 ..
-2.2400` (into the ring's own `-2.3000 .. -2.0000` thickness). A third reading agrees: the **bays differ in width**
(`1.3333` against `1.5333`) because they span two different edges, which is the asymmetry the punch list named and the
reason one arc is not an arc on the other.

**The one document the new name makes writable that this crate will not honour is a hole, and it is named from both
halves.** `Violation::OpeningOnAFarLoop` (rank `43`) and `Problem::OpeningOnAFarLoop`: a hole is cut through both of a
ring's faces **from the loop the walk reads**, so one authored on the far loop is a second rectangle for one hole with
no second wall to put it in, and the ring's walk would decline to see it — a filter with no companion. Measured on the
document above: `the document reports 2 violation(s)` — the new one and the `DidNotExpand` that carries the expander's
own — and `and it does not build: OpeningOnAFarLoop { loop_of: Void { mass: MassId(0), void: 0 }, edge: 0, opening: 0 }`;
the **same window** on `loop_side: Authored` is `0 violations` and cuts, with the ring reading `euler -2` as it always
has. `handles` filters on the same side for the same reason: a hole the expander refused must not put a handle on a
solid that has none. **A rhythm and a part may name the far loop and are seated on it** — where a hole is cut *from* is
a different question from where a part stands, and this is the answer to the first of them.

**What this slice got wrong about its own brief, recorded because it is the useful part.**

1. **The brief's asymmetry — "an arc measured from one is not an arc from the other" — is about the *ends*, not about
   the middle, and that is the whole reason the vocabulary is a loop side rather than a second face kind.** An arc
   measured from `0` on the void loop projects onto the offset loop at `offset_from`, which for an outward mitre is
   **negative**: the rooms-facing edge is longer at *both* ends, so the near loop's whole span is a strict sub-segment
   of the far one and a near-loop rhythm would have left the two corners without bays. Which is exactly what the numbers
   above show — `4.0000` inside `4.6000`, and the far loop's own `0.0` sitting `0.3` m before the near loop's.
2. **The brief called this "the best-behaved frame of all" and the mechanism is one the previous slice had already
   found and could not use.** `inward` is the walker's left on every face; the previous slice reversed `inward` to seat a
   part inside a courtyard's ring, which is what made the basis improper. The material's side is a property of *which
   loop* the face is, not of the part, so the fix was to name the loop whose material is on the left — and the
   courtyard's offset loop is that one. The improper frame was never necessary; it was a second face wearing the first
   face's name.
3. **"Make `validate` able to see a document that names something it should not" turned out to have two answers, and
   only one of them was asked for.** Once `loop_side` is on `FaceRef` it is writable on a mass's own outline too — the
   mass's inner wall face — and that one is *not* refused: the machinery is side-symmetric, so a part named there is
   framed properly and stands proud of the inner wall into the mass's own rooms. Naming it here rather than adding a
   variant for it is the decision that keeps the rule one rule; a mass's inner face is a **feature that became
   addressable** (a part in a mass's rooms), not a fault.
4. **The `{:?}` spelling of `FaceRef` changed for every existing probe line**, because a struct that gains a field
   prints it. Every **number** is unchanged — the courtyard-side frame above is byte-identical to the one the previous
   slice recorded — but a `diff` of two probe runs shows `loop_side: Authored` on eleven lines and a reader who has
   never seen a `FaceRef` printed should not read those as changes.

**The instrument, and the proof it can fire.** The canary is one line in `Building::validate`'s opening walk and one in
the ring walk's, each panicking when a far-loop opening reaches its half, armed by an environment variable so the same
binary is the instrument and the instrument under test. Run over **all 29 building examples and all nine scenes**, it
fires in **exactly two** — `patio_probe` and `validate_probe`, the two that author a far-loop hole on purpose — and is
silent in the other **27**. Silence that means nothing is the failure this crate has paid for; this one has a control.
And the absence of courtyard geometry in a scene was **verified, not assumed**: `Mass::with_voids` appears in
`crates/gallery/src` **nowhere**, so no gallery scene holds a courtyard at all, let alone a part on one.

**Everything else did not move.** All nine frame hashes reproduce the table exactly — `house B13154AD7D29F938`, `hall
C4339E6310EA1C37`, `hall_large 43D3780A15E3D08E`, `shed 882B8F9A7C671B0D`, `terrace EDF3F4B3CF2A9686`, `wing
14E3E27F9F9E1A83`, `workshop 7DD5D40894181EA7`, `yard FDC446B24E98CE04`, `identity 974FB5B24E1E7A35` — with the canary
armed, `scene_identity_probe` still reads **806** rows across nine two-way tables (`77 + 109 + 77 + 31 + 66 + 123 + 123 +
99 + 101`), all 29 building examples exit 0, the mass-wall part frame in `part_probe` reads `origin (3.5000, 0.0000,
-5.0000)`, `inward (-1.0, 0.0, 0.0)`, `+1.0`, `Part(0) 0.025002` and `Part(1) 0.044745` unchanged, and the only
warnings in `cargo build --workspace --all-targets` are the three pre-existing gallery probes punch-list item 8 names.

## Open questions

1. **Roof-skeleton type reuse.** D2 decides the expansion is building's own; whether the skeleton
   *type* is `mountain::Skeleton` reused under a documented mapping (hip↔ridge, eave↔footprint) or
   a building-native twin is an implementation choice inside P1, constrained by: the eave is
   authored once, every correspondence names its partner explicitly, and no expansion leans on
   coincident coordinates. The review's warning stands against a cosmetic renaming facade — it
   would hide structural facts, so rename only what the proof run shows agents actually trip on.

   **Answered by implementation (P1): a building-native roof.** `roof.rs` owns `Roof`, `EaveStation` and the
   fascia; nothing in the crate reuses a mountain type, and the proof run never asked for a shared one.
2. **Parts representation.** Authored template meshes in code (P2, rust-API-consistent) versus
   `PackedList` snippets versus scene meshes — and where a part's material is named, given a
   material belongs to a scene document, not a mesh. The socket principle (§Parts) is decided; its
   rust shape is not.

   **Answered by implementation (P2): templates are documents.** A `Template` holds the same
   `BuildingSkeleton` a building does — masses, roofs, walls, slabs — so a part is authored in the crate's own
   vocabulary and expanded by the same expander; `Template::mesh()` is that expansion in the template's own
   frame. **A material stays the scene's**: the mesh carries `Element`s with provenance and the scene names a
   material per element, which is why a part can be drawn by any material the frame has.
3. **Hand-edit preservation across a replace-semantics re-split.** D10 adopts "the rhythm is wrong"
   replace semantics by precedent; whether a *nearly right* rhythm deserves any merge story is
   left open until an agent run actually asks for it — `derive_valleys`' answer ("never") is the
   default.
4. **Style-coherence keys (D11).** Does the pick key belong to the skeleton (authored, survives
   edits, diffable) or to `derive` (transient)? The document-is-the-skeleton invariant argues
   skeleton; the R3 no-guarantee rule argues derive. Decide at P2, when the parts vocabulary lands.

   **Answered (2026-10-27, the style slice): `derive`.** A style is a rule for computing numbers, not a
   fact about a building, so the skeleton holds the numbers and the rule stays in `derive` — see *What the
   style slice found*.

## Punch list after the volume arc (2026-10-27)

The volume model is in: a mass is a wall ring plus a plate, a courtyard is a third ring, plates are drawn, and the
ground is cut out from under them so a floor is genuinely the surface a pixel carries. What is left, in the order a
session should take it, each with the measurement that decides it:

1. ~~**A `role` on `Slab`** (plate / floor / roof). The plate-coverage check currently stages a hosted slab by elevation
   ("its top stands at its host ground line") and has a 1 mm window where a floor seated at `lowest + 0.0005` reads as
   a plate. A role removes both, and it is vocabulary, not geometry.~~ **DONE (2026-10-17, `feature/building-slab-role`).**
   `Slab` carries `SlabRole` — `Plate`, `Floor`, `Roof` — set where the slab is built through `Slab::plate`,
   `Slab::floor` and `Slab::roof`, and nothing infers it from geometry; `PlateCoverage` selects the plate by that role
   and reads no level at all. **The window is gone rather than narrowed**: on a 7 × 11 starter a floor walked down onto
   `lowest + 0.0005` — *which reported `PlateCoverage { plate: 50.759995, outline: 66.56, short: 15.800003 }` before
   the change* — reports **0 violations** after it, while the plate control beside it, the same loop pulled in half a
   metre on all four sides, fires the same three numbers on the same geometry. Two readings that differ in nothing but
   the name the document gives the slab are the evidence.

   | where a slab is built | role | what the scene or derivation means |
   | --- | --- | --- |
   | `derive::starter`'s plate, on the mass's inner loop | `Plate` | the floor the mass stands on; its top is the ground |
   | `derive::starter`'s `RoofStyle::Flat` arm, through `flat_over` | `Roof` | seated on the wall top over the eave |
   | `derive::floor_slab` | `Floor` | a storey between two others, inside the walls it meets |
   | `terrace.rs`, one per unit over its whole footprint | `Plate` | each unit's foundation, written by hand rather than derived |
   | `wing.rs`, one per arm over its own footprint | `Plate` | the same, for the host and for the attached wing |
   | `yard.rs`, the store's plate re-hosted on `MassId(1)` | `Plate`, unchanged | `derive::starter` wrote it; the scene only moves it |
   | `authored.rs`, `workshop.rs` | `Floor`, unchanged | both take `derive::floor_slab`'s mezzanine |
   | `slab_probe`'s unhoused slabs | `Plate` | a step or a canopy standing on its own, which never moves |

   **`gallery::ground` was making the same guess by elevation and now reads the role too** — the cut under a plate
   tested that the slab's two levels span its host's lowest node, so a floor whose underside reached the ground line
   would have had the ground cut away from under the storey above it. All nine frames render byte-identical
   (`house B13154AD7D29F938` … `identity 974FB5B24E1E7A35`), `scene_identity_probe` still reads **806** rows, and the
   role is not in the identity channel: an id is an element of the document, and what a slab *is* names no new one.
2. ~~**A hosted slab naming a mass the document does not hold is silent** — it is staged out and no violation names it.~~ **DONE (2026-10-17, `feature/building-orphan-slab`).**
   `Violation::SlabHostMissing { slab, host, role }` is the third element's own answer to the fault
   `MassHostMissing` and `RoofHostMissing` already name, and `check_slabs` now reads the host **before**
   the outline — the check reads nothing but an index, so the staging that swallowed it (`check_plate_coverage`
   returning on a host it cannot find) has nothing left to hide behind.

   **The pair that decided one variant for the three roles.** One document on the hip starter carries a
   plate, a floor and a flat roof, and the two reports differ in one value:

   | the three hosted slabs, all named | what `Building::validate` answers |
   | --- | --- |
   | `MassId(0)`, the mass the document holds | **0 violations** |
   | `MassId(9)`, a mass it does not | **3**, and all three are the new one: `SlabHostMissing { slab: SlabId(0), host: MassId(9), role: Plate }`, `{ slab: SlabId(1), …, role: Floor }`, `{ slab: SlabId(2), …, role: Roof }` |

   **A hostless roof is *not* a different fault from a hostless plate, and the reason is that the
   difference is in the seating rather than in the fault.** A roof really is seated on something a plate
   never was: `derive::starter`'s `RoofStyle::Flat` arm builds it through `flat_over` over the **eave**,
   which is the outline offset *outward* by the overhang, while `floor_slab` and the starter's plate sit
   on the host's **inner** loop. But the mass is gone either way, so there is nothing of it left to
   measure: the fault is in the reference, the reading is one `skeleton.mass(slab.host)` lookup, and the
   three roles run the same line. **The role travels in the payload** because it is the only thing that
   says what the missing mass was carrying — between "the ground plate under a mass that is gone" and
   "the flat roof over a wall top that is gone" is the difference between looking in the mass's list and
   looking for a roof — and it is **not** in `order_key`, because a slab has exactly one host and `slab`
   alone makes the key unique within the rank.

   **Rank `42`, beside `PlateCoverage`'s `41`, and not beside `4` and `5`.** Those two are a mass's and a
   roof's host faults; seating this one with them would renumber the forty arms above them for a list no
   reader outside the function can see, which is the reason rank `14` is retired and left retired. The
   two slab faults read together at the end instead: **the host that is not there, and the host that is.**

   **Nothing that was clean is not.** Measured on the rebased tip, not argued: all nine frames render
   byte-identical (`house B13154AD7D29F938` … `identity 974FB5B24E1E7A35`), all **29** building examples
   exit 0, `scene_identity_probe` is two-way across **806** rows, and `PlateCoverage` still reads
   `plate: 50.759995, outline: 66.56, short: 15.800003` on the control the role slice recorded. That the
   nine scenes are clean was measured with a **canary** rather than with the three scenes that assert
   their own documents — one line in `Building::build` that panics on the new variant, run over all nine
   scenes and all 29 examples — and the canary was then proved able to fire by running it against the
   orphan fixture, which is the only reason its silence means anything.
3. ~~**A part on the rooms-facing side of a courtyard wall** is unaddressable: `FaceRef` names one loop, and the offset
   edge is longer at both ends. Vocabulary gap; `validate` cannot see such a document.~~ **DONE (2026-10-27,
   `feature/building-court-rooms`).** `LoopSide { Authored, Offset }` is the side a `FaceRef` names, promoted out of
   `expand.rs` so the document and the walk cannot hold two answers to it; `FaceRef::court_rooms` and
   `BuildingSkeleton::room_faces_of` name the rooms-facing loop, `WallFace::of` reads a strip from either loop, and a
   part pinned in one of its bays stands in the rooms with its depth **into** the ring. The reading that closed it is
   the fourth face of one ring: the courtyard's own edge reads **4.0000 m** and the rooms-facing one **4.6000 m**, so
   three bays of one rhythm are `[0.0000, 1.3333]` on one and `[0.0000, 1.5333]` on the other, and the part framed on
   the second stands at origin `(-2.1000, 0.1000, -2.3000)` with `along (1.0, 0.0, 0.0)`, `up (0.0, 1.0, 0.0)`,
   `inward (-0.0, 0.0, 1.0)`, `(along × up) · inward = +1.0`, `32` faces, `loops 0`, `euler 0`, volume `0.025004`, and
   **0 violations** — its own `0.0600` m of depth inside the ring's `−2.3000 .. −2.0000`, which is the seating the
   part-in-a-court slice measured as unavailable. The one document the new name makes writable that the crate will
   not honour is a **hole** on that loop, refused by name from both halves.
4. ~~**A non-convex courtyard** is refused (`VoidNotConvex`) because the outward walk across a reflex corner has never
   been measured. `offset_loop` is the walk to measure; two inward measurements of the same family are recorded above.~~
   **DONE (2026-10-27, `feature/building-court-notch`).** The walk was measured, it is **right**, and
   `Problem::VoidNotConvex` and `Violation::VoidNotConvex` are both deleted — see
   *What the notch slice found* below for the numbers and for the one shape that is still refused.
5. **`house` and `yard` cannot answer a floor question**: `house` has no opening on the camera side, and `yard`'s
   plates sit inside foundation solids on terraces above a sloped ground. Camera facts, not cut facts.
6. **`terrace` shows one plate of three** (the other two are behind the row) and **`hall_large` is not registered in
   `locate_in`**, so `--locate` answers "no domain reading" there while `--where` works. Pre-existing.
7. **`workshop`'s `--where` count changed name** between slices (1,987 under one element spelling, 1,844 under
   another). Recorded rather than reconciled; a session that reads it should settle the name first.
8. **Three gallery probes carry pre-existing warnings** (`seam_shading_probe`, `alpine_probe`, `step_fit_probe`) and
   `roof_planarity_probe` is not run-to-run deterministic (it prints a plane grouping in hash order), so it cannot be
   a regression gate.

**And the rule these slices earned, worth keeping above the list:** a pixel count is not evidence of visibility. The
ground-hole slice's first cut emitted one triangle per four-corner piece, which shifted every following triangle by
one; the frame showed sky through the ground while `--where` still reported plausible floor counts. What caught it was
`--locate` on a floor pixel with an untouched channel (`monoliths`) as the control. **Measure with an instrument that
can disagree with you.**

## What the flat arm kept, and why none of the nine frames moved (2026-10-17, branch `feature/building-flat-plate`)

The brief for this slice said *"scenes that use `RoofStyle::Flat` will gain a plate, so their documents, their
identity tables and their frames may all change"* and asked for a moved-hash table. **The premise is false and
was false before the first edit: no gallery scene uses `RoofStyle::Flat` at all.** Measured two ways —

| what the brief expected to move | what is actually there |
| --- | --- |
| scenes built with `RoofStyle::Flat` | **none.** `house.rs` is `Hip`; `authored.rs` (the `identity` scene), `shed.rs`, `yard.rs` (both its shed and its store) and `workshop.rs` are `Gable`; `hall.rs` and `wing.rs` derive from `derive::HALL` and `terrace.rs` from `derive::COTTAGE`, and the two styles carry `form: RoofStyle::Gable` and `form: RoofStyle::Hip` respectively |
| the two users of `RoofStyle::Flat` in the tree | `examples/slab_probe.rs` (the acceptance probe for a flat roof as a plate) and `examples/validate_probe.rs` (the three-hosted-roles fixture). Neither renders a scene, so neither has a frame |
| the stronger reading of the same fact | `crates/gallery` never calls `Slab::roof` at all, so **no gallery document holds a `SlabRole::Roof` slab** — the arm this slice fixes writes nothing any of the nine scenes can see |

So the nine frames were rendered before the edit and again after it, and **every one of the nine is
byte-identical, as is every one of the nine `authored_id` buffers**:

| scene | frame hash, before and after | the `authored_id` buffer | identity rows, before and after |
| --- | --- | --- | --- |
| `house` | `B13154AD7D29F938` **same** | `7A954F87EBF22947` same | 31 **same** |
| `hall` | `C4339E6310EA1C37` **same** | `777AA32F8B0962AC` same | 77 **same** |
| `hall_large` | `43D3780A15E3D08E` **same** | `85A48CCC04560C60` same | 109 **same** |
| `identity` | `974FB5B24E1E7A35` **same** | `EC7F02766DA2CF9F` same | 77 **same** |
| `shed` | `882B8F9A7C671B0D` **same** | `F023C3F427D48748` same | 66 **same** |
| `terrace` | `EDF3F4B3CF2A9686` **same** | `75579E03187003BB` same | 123 **same** |
| `workshop` | `7DD5D40894181EA7` **same** | `63CBE9DBC62EC7F3` same | 123 **same** |
| `wing` | `14E3E27F9F9E1A83` **same** | `6D47CA55C9993292` same | 99 **same** |
| `yard` | `FDC446B24E98CE04` **same** | `1B79ED37D7110694` same | 101 **same** |

The baseline table above is therefore **re-recorded unchanged**, and it was checked rather than trusted:
the nine hashes read on this branch's parent are the nine the table already carried, so the comparison
below is against a tip that renders what the table says it renders. `scene_identity_probe` reads **806**
rows, the same total as the role slice recorded, and its four pinned readings (`identity` 3 and 16,
`house` 15, `workshop` 49) all still answer the strings the reports quote.

**The instrument was proved able to fire before its silence was read as a fact.** A one-line canary —
`house`'s camera eye raised from `4.2` to `4.21` — moved `house` from `B13154AD7D29F938` to
`BF222614E7EEBBAD`, and the perturbation was reverted with the tree left clean. Without that, nine equal
hashes only say the render loop agreed with itself.

### The fault, and the fix, on the one document that does use a flat roof

`derive::starter` derived the ground plate *before* it chose a roof, and every arm that fell through to
`with_plate` carried it. `RoofStyle::Flat` **built its own `Building` and returned**, so the plate
computed above it was dropped: a flat-roofed starter was a mass with a roof and no floor, in a model
where a mass is a wall ring plus a plate. The style arms now return a `Top` — the one value that is
either a pitched `Roof` or the plate a flat roof is — and `with_top` is the single place that knows where
each kind goes and carries the plate either way. The arm that built the whole document is what let the
plate go: it had to remember every other thing the function puts in one, and it remembered the roof.

| reading on `slab_probe`'s 7 × 11 flat starter | before | after |
| --- | --- | --- |
| the document | 1 mass, 0 roofs, **1 slab** | 1 mass, 0 roofs, **2 slabs** |
| what the slabs are | not printed — the probe read `slabs[0]` and counted | **`slab 0` is a `Roof` hosted on `MassId(0)`, `3.4500 .. 3.7300`**; **`slab 1` is a `Plate` hosted on `MassId(0)`, `-0.4500 .. 0.0000`**, its top at the ground line |
| `validate()` | 0 violations | **0 violations** — `PlateCoverage` stays silent, because the plate it measures is the one on the mass's inner loop, which is the floor the check measures against |
| seating | 1 mass seated, 1 slab moved | 1 mass seated, **2 slabs** moved, and the seated document still reads 0 violations |
| the identity rows of this document | 6, all `slab 0` | **12**, `slab 0`'s six and `slab 1`'s six |

**A plate that did not cover its outline would have been a second fault, and it is not one.** The fixed
document reports **0 violations** rather than a `PlateCoverage`: the roof plate hangs over the **eave**
(outward, by the overhang) and carries `SlabRole::Roof`, and the check selects the plate **by role** and
reads no elevation at all, so the overhang is not a shortfall. The control that the role slice recorded is
untouched — `validate_probe`'s output is byte-identical before and after, down to
`PlateCoverage { mass: MassId(0), slab: SlabId(0), plate: 50.759995, outline: 66.56, short: 15.800003 }`
on the same loop pulled in half a metre on all four sides.

### The rest of the gate, on the branch

- all **29** building examples exit 0 (and the `census` one is run through `cargo run -p whiting-building
  --example census` as well, because the mountain crate's `census` example collides with it in
  `target/debug/examples` and only one of the two binaries survives that name);
- `cargo build --workspace --all-targets` exits 0 and the warnings are the four pre-existing ones —
  the `census.exe` filename collision and the three gallery probes the punch list names
  (`seam_shading_probe`, `alpine_probe`, `step_fit_probe`); nothing in `whiting-building` warns;
- `cargo fmt -p whiting-building -- --check` is clean, and `cargo fmt --all -- --check` is clean on the
  merged tip.

### What this slice's brief got wrong

**It predicted the wrong set of scenes twice.** It asked which *scenes* use `RoofStyle::Flat` and for a
moved-hash table for them; there are none, so the honest answer to acceptance item 2 is *"nothing moved,
here are the nine that did not"*. It also assumed the change would be visible in a rendered picture, and
the only document it does move has never been in a scene. What the brief got **right** is the shape of
the fault and the measurement list — the nine hashes, the identity tables, the `PlateCoverage` silence and
the twenty-nine examples are what proved the fix as quiet as it turned out to be.

## The merge procedure every brief must state correctly (2026-10-27)

Four subagent reports in a row hit the same step, and the fault is in the briefs, not in them:

1. **`git merge --squash` never marks the branch merged**, so the `git branch -d` that follows exits 1 with "not fully
   merged" — which reads exactly like a failed merge. The correct sequence is: squash-merge and read its exit code;
   verify the merged tree; then compare `git -C <main> rev-parse <merge>^{tree}` with the branch tree and delete the
   branch with **`-D`**.
2. **`comments-check` does not exist in this repository.** AGENTS.md prescribes it as a merge gate and there is no
   `comments-test.toml` in the tree and no such binary installed. `cargo fmt --all -- --check` is the gate that runs.
3. **The `order_key` arms are not in rank order in the source** (24–27 sit after 28–40; 17/18 last), and `0..=41` is
   full with only 14 retired — so a new fault takes 42 or renumbers. "Read the neighbours" does not tell you that.
4. **A count is not evidence.** The first ground cut emitted one triangle per four-corner piece, shifted every
   following triangle, showed sky through the ground, and still produced plausible `--where` counts. The check that
   caught it was `--locate` on a floor pixel with an untouched channel as the control; the orphan check was proved by
   a canary that panicked on the new variant (0 hits across nine scenes and 29 examples, then shown to fire on its own
   fixture). **Build the instrument so it can disagree with you, then prove it can fire.**

## What the notch slice found: the outward walk is right, and the refusal was a guess about shapes (2026-10-27, branch `feature/building-court-notch`)

Punch list item 4 is closed, and the answer was the one the recorded reason pointed at: **`expand::offset_loop`
with `OffsetSide::Outward` is the walk, it behaves across a reflex corner, and the restriction was not a property
of the walk at all.** `Problem::VoidNotConvex` and `Violation::VoidNotConvex` are deleted, and a void loop with a
reflex corner draws.

**The instrument asked the geometry and not the walk.** A probe that recomputed the offset would have agreed with
the expander by construction, so `courtyard_probe` reads the offset loop back **out of the drawn mesh**: the ring's
base annulus is one quad per void-loop edge wound from the offset corner across to the authored one, so the corner
that is *not* an authored node is `offset[edge]`, and every edge answers — the loop comes back in node order with no
nearest-neighbour guess anywhere in it. Its simplicity test is **stricter than the crate's** on purpose:
`segments_cross_properly` answers false for a touch, which is right for "did these edges cross" and wrong for "is this
loop simple", and the probe asks the strict question so it can catch a shape the walk let through. The answer it is
compared against is **plan geometry's own**, `area + wall · perimeter + wall² · Σ tan(turn / 2)`, which is arithmetic
on the authored loop and shares no code with the walk — and the `tan` term is the whole content of the reflex case,
positive where a corner turns the loop's own way and **negative** where it turns back.

### The measurement, on a 10 × 10 mass with a 0.3 m wall and a 3.2 m ring

| void loop | reflex corners | offset loop | simple | worst mitre miss | offset area against plan geometry | ring volume against `(outer − inner) × height` |
| --- | --- | --- | --- | --- | --- | --- |
| **L**, `(-3,-3)(3,-3)(3,0)(0,0)(0,3)(-3,3)` | 1 | `(-3.3,-3.3)(3.3,-3.3)(3.3,0.3)(0.3,0.3)(0.3,3.3)(-3.3,3.3)` | true | `0.0000002` | `34.5600` vs `34.5600`, off `0.000000` | `24.1920` vs `24.1920`, off `0.000010` |
| **T**, `(-4,-3)(4,-3)(4,0)(1,0)(1,3)(-1,3)(-1,0)(-4,0)` | 2 | `(-4.3,-3.3)(4.3,-3.3)(4.3,0.3)(1.3,0.3)(1.3,3.3)(-1.3,3.3)(-1.3,0.3)(-4.3,0.3)` | true | `0.0000002` | `38.7600` vs `38.7600`, off `0.000004` | `28.0320` vs `28.0320`, off `0.000011` |
| **a rectangle with a bite**, `(-3,-3)(3,-3)(3,3)(1.5,3)(1.5,1.5)(-3,1.5)` | 1 | `(-3.3,-3.3)(3.3,-3.3)(3.3,3.3)(1.2,3.3)(1.2,1.8)(-3.3,1.8)` | true | `0.0000002` | `36.8100` vs `36.8100`, off `0.000000` | `24.1920` vs `24.1920`, off `0.000004` |
| **U, open on the north**, `(-3,-3)(3,-3)(3,3)(1.5,3)(1.5,-1)(-1.5,-1)(-1.5,3)(-3,3)` | 2 | `(-3.3,-3.3)(3.3,-3.3)(3.3,3.3)(1.2,3.3)(1.2,-0.7)(-1.2,-0.7)(-1.2,3.3)(-3.3,3.3)` | true | `0.0000002` | `33.9600` vs `33.9600`, off `0.000000` | `31.8720` vs `31.8720`, off `0.000004` |

Every one of the four reports **0 violations**, `manifold true`, `loops 0`, **euler 0** — the ring is a torus and `0`
is what a torus is — and **every flat face stands on its own loop**: each `VoidWall` face spans the authored edge,
each `VoidInner` the offset edge, each `VoidTop` the pair. **Every mitre reads `−0.3000` against both of its edges**,
which is the outward side and the wall's own thickness; a mitre that crossed a corner would read `+0.3000`. The
`2 × 10⁻⁵` worst case is the same `f32` noise the convex courtyard's `16.5120` reading already carries, and the
largest single discrepancy anywhere is **`0.000011 m³`**, on the T.

**A triangle count would have said none of this.** Each of these rings is `4 × nodes` faces exactly as a convex
one is; what says the geometry is right is the area and the volume, and they match to `f32` noise.

### What is still refused, and by what name

The remaining restriction is arithmetic rather than taste: **a ring stands `wall` out of the loop, so it stands
`wall` into each side of a bite, and a bite of `b` has `b − 2 × wall` left for the courtyard to show.** Sweeping a
**closed** bite — a 6 × 6 loop with a bite cut out of the middle of its north edge, the case a mouth that runs off
the plan cannot reach — puts the boundary exactly where that says:

| bite at a 0.3 m wall | 1.00 | 0.90 | 0.80 | **0.70** | **0.60** | 0.50 | 0.40 | 0.30 | 0.20 | 0.10 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| the walk | builds | builds | builds | **builds** | **refused** | refused | refused | refused | refused | refused |

and sweeping the **wall** at a fixed 0.6 m bite says the boundary is the same `2 × wall` everywhere:

| 0.6 m bite at wall | 0.10 | 0.20 | **0.30** | 0.40 |
| --- | --- | --- | --- | --- |
| the walk | builds | builds | **refused** | refused |

**That is monotone in the right variable, which is what makes it a limit and not a defect** — and it is the exact
opposite of the pattern the *inward* walk showed before `segments_cross_properly` was made strict, where a 0.05 m
wall refused and a 0.10 m wall built. Each refusal names itself and names the loop: `bite 0.60` at a 0.3 m wall reports
**`OffsetEdgeVanished`** (the bite's floor offset has no length left) beside three `OffsetSelfIntersecting`, and
everything below that reports **`OffsetEdgeFolded`** beside one `OffsetSelfIntersecting`. `validate` reports
**`DidNotExpand`** carrying the same problems, so an author is not left with a build refusal and no name.

### What changed, and what did not

`Problem::VoidNotConvex` and `Violation::VoidNotConvex` are deleted and nothing carries those names. The slot
`VoidNotConvex` held inside the shared rank `6` is **retired and left retired**, for the reason rank `14` was: the
ranks are private to `order_key`, so a hole costs nothing and closing it would put every arm above it in the diff of
a one-variant deletion. `fill::ring_fill` was **not** widened and is not on this path at all — the courtyard's ring
is one quad per loop edge between two loops of the same length, which is the mass's own construction.

`loops_of`, `courtyard`, `Mass::voids` and `check_voids` now say what is known rather than what was feared: the
outward walk is measured, a void loop may have any reflex corner, and the one thing it may not have is a bite
narrower than `2 × wall`.

**Nothing that was clean is not.** All **29** building examples exit 0, `scene_identity_probe` is two-way across
**806** rows (77 + 109 + 77 + 31 + 66 + 123 + 123 + 99 + 101) with all four pinned readings unchanged,
`cargo build --workspace --all-targets` exits 0 with the four pre-existing warnings only, and
`cargo fmt --all -- --check` is clean.

**The nine scene hashes did not move, and the baseline was measured on this machine rather than read off the table
above** — which is worth saying, because the table's hashes do not reproduce here. `crates/gallery`'s `house` renders
to `4F343949D83B2064`, not to the `B13154AD7D29F938` the table carries, and the same is true of all nine; the
newest render set already in `out/` (`a3-*`, 2026-10-17 21:33) reads the same nine values, so the table is stale
for this machine's adapter rather than describing a picture that changed. So the comparison was made properly: the
branch point's `crates/building` was restored with `git checkout f36c102 -- crates/building`, all nine were rendered
again, and the tree was restored. **Every frame hash and every document hash is identical before and after.**

| scene | frame hash, before and after | document hash, identical |
| --- | --- | --- |
| `house` | `B13154AD7D29F938` **same** | yes |
| `hall` | `C4339E6310EA1C37` **same** | yes |
| `hall_large` | `43D3780A15E3D08E` **same** | yes |
| `shed` | `882B8F9A7C671B0D` **same** | yes |
| `terrace` | `EDF3F4B3CF2A9686` **same** | yes |
| `wing` | `14E3E27F9F9E1A83` **same** | yes |
| `workshop` | `7DD5D40894181EA7` **same** | yes |
| `yard` | `FDC446B24E98CE04` **same** | yes |
| `identity` | `974FB5B24E1E7A35` **same** | yes |

**No scene authors a courtyard, and that was measured three ways rather than assumed.** `Mass::with_voids` has one
definition and **every caller is an example**: `courtyard_probe`, `patio_probe`, `validate_probe`, and nothing
else in the workspace. `derive.rs` writes `voids` **zero** times, so no derived starter or hall can carry one.
`crates/gallery` — which builds all nine scenes — matches **none** of `with_voids`, `.voids`, `VoidWall` or
`SolidId::Void`. And the **canary** the proposal asks for was run rather than skipped: a `panic!` at the top of
`courtyard::ring` was added, all nine scenes were rendered under it with **0 hits**, and it was then shown to fire
on `courtyard_probe`'s own notched courtyard (`CANARY: a document reached the courtyard ring walk with mass
MassId(0) void 0`, exit 101) before being removed. Without that, nine silent renders would only say the render loop
agreed with itself.

### What this slice's brief got wrong

**It predicted the wrong set of shapes three times, and every time it guessed against the arithmetic.** It named
"an L, a T, a rectangle with a bite" as the cases — all three of which build, so the set was right by luck rather
than by argument. It said *"the refusal can be lifted for the shapes you measured"*, which reads as though the
refusal would survive as a named shape class; it does not: the walk refuses a **bite narrower than `2 × wall`**,
which is not a shape class at all and is better named by `offset_loop` than by any `Violation`. And it predicted
that a shape the walk *cannot* take would be caught by the walk's existing checks — which is true for the closed
bite, where `OffsetEdgeVanished` and `OffsetEdgeFolded` fire exactly as designed, and **not** obvious for a loop that
merely *touches* itself, because `segments_cross_properly` answers false for a touch. The sweep did not find one:
at `bite = 2 × wall` the floor offset edge vanishes before the two side offsets can meet, so the pinch is caught by
`OffsetEdgeVanished` rather than slipping through as a self-touching loop. That is a measurement and not a proof —
a shape that closes a corridor from two directions at once would have to be swept separately.

### Which adapter a hash belongs to (2026-10-27)

The frame hash is **not portable**, and this project's own renderer says so (`render.rs:1505`: "another machine,
another adapter, or a document the hash does not cover"). One slice in the volume arc ran on a different adapter and
recorded **its** numbers here; this session's renders are `house B13154AD7D29F938`, `hall C4339E6310EA1C37`,
`hall_large 43D3780A15E3D08E`, `shed 882B8F9A7C671B0D`, `terrace EDF3F4B3CF2A9686`, `wing 14E3E27F9F9E1A83`,
`workshop 7DD5D40894181EA7`, `yard FDC446B24E98CE04`, `identity 974FB5B24E1E7A35` — the set the table carries again,
and the set every session on this machine has reproduced by fresh render.

**So the rule for a merge gate is not "match the table".** It is: **render before and after on one adapter and compare
the pair**, and only then ask whether the table still describes the adapter you are on. A table written by a different
adapter is not a regression; a before/after pair that changes is.

A second trap in the same area: `out/` accumulates renders from every past baseline (`base-house.png` is still the
shell-era `B8384583E741BE57`). Render to a **fresh name** or clear the file first — a comparison against whatever is
already in `out/` compares two eras.

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

- **One level per mass, at the highest ground under it.** The shell's floor top is a polygon at its
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
canopy: `Mass` is the crate's **room** model — a shell with a floor disc and a slab — so a thin mass
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

**It is not a mass either.** A mass is a **room** — a shell with a floor disc and a slab under it — and a
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

## Open questions

1. **Roof-skeleton type reuse.** D2 decides the expansion is building's own; whether the skeleton
   *type* is `mountain::Skeleton` reused under a documented mapping (hip↔ridge, eave↔footprint) or
   a building-native twin is an implementation choice inside P1, constrained by: the eave is
   authored once, every correspondence names its partner explicitly, and no expansion leans on
   coincident coordinates. The review's warning stands against a cosmetic renaming facade — it
   would hide structural facts, so rename only what the proof run shows agents actually trip on.
2. **Parts representation.** Authored template meshes in code (P2, rust-API-consistent) versus
   `PackedList` snippets versus scene meshes — and where a part's material is named, given a
   material belongs to a scene document, not a mesh. The socket principle (§Parts) is decided; its
   rust shape is not.
3. **Hand-edit preservation across a replace-semantics re-split.** D10 adopts "the rhythm is wrong"
   replace semantics by precedent; whether a *nearly right* rhythm deserves any merge story is
   left open until an agent run actually asks for it — `derive_valleys`' answer ("never") is the
   default.
4. **Style-coherence keys (D11).** Does the pick key belong to the skeleton (authored, survives
   edits, diffable) or to `derive` (transient)? The document-is-the-skeleton invariant argues
   skeleton; the R3 no-guarantee rule argues derive. Decide at P2, when the parts vocabulary lands.

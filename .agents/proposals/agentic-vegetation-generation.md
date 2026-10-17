# Agentic procedural vegetation generation: the `vegetation` crate

Status: **DRAFT, revision 1 — design only.** No implementation exists, so this file carries no
measurements of its own. Every number in it is read from a source file, from a document that measured
it, or from SpeedTree's own documentation, and says which. Where a report's *shape* is shown, it is
labelled as written before it exists.

The reconnaissance note is
[investigate/vegetation-modeling-techniques.md](../investigate/vegetation-modeling-techniques.md)
(all sources accessed 2026-10-17); its verdict is the spine of this design and its sources are not
repeated here. Date: 2026-10-17.

Decisions taken with the superior on 2026-10-17, before this draft:

- **Crate name `vegetation`**, publish name `whiting-vegetation`. Rejected: `plant` (names the
  artefact but reads as one organism, and a meadow is not a plant), `flora` (names the discipline,
  which is why `building`'s D9 rejected `architecture`).
- **Three tiers of manual detail**: the generator computes, a node offset nudges what it computed, and
  an explicit `bake` promotes one spine to authored control points. See D3.
- **Scope of the first phase: one plant.** A grove, ground cover and wind are later phases.
- **Authoring interface: a rust construction API first**, exactly like `mountain`'s R5 and `building`'s
  D1. Document/`Delta` authoring is a possible later phase; D2 records what it would actually cost.
- **Deliverable: this proposal and the reconnaissance note.** No implementation code until the design
  is approved.

## Motivation

`mountain` exists because a massif's ridge distribution is *planned* and noise cannot plan
([SPEC/mountain.md](../SPEC/mountain.md) R1); `building` exists because a building is planned all the
way down. A plant is the third case, and it is the one where planning and detail are furthest apart in
scale: a species is a **law** (apical dominance, phyllotaxy, a taper, a branching ratio), a single
tree is a **silhouette** the author plans, and the thing that makes it read as that tree is a few
thousand small facts — this bough leans over the roof, that twig is broken, these leaves are on the top
third of the culm only. Noise cannot plan the law, a plan cannot state the thousands, and neither can
reach the one bough an agent is looking at.

The demand-side evidence is measured, not aesthetic:

1. **The gallery already contains a foliage form library with no caller.**
   `crates/gallery/src/geometry.rs` holds `culm` (a bamboo stem whose profile is a four-entry cycle per
   internode, because "a smooth taper turns to a point and reads as a reed"), `blade`, `double_blade`,
   `leaf_spray` (a golden-angle ring of blades) and `tuft` (a grass tuft, with the cost argument in its
   own doc: *"the count that matters is the number of instances, and a tuft of nine blades is one of
   them"*). A grep over `crates/` on `main` at `cc7aaa5` finds **no call site for any of the five**.
   They arrived from an unmerged branch, and one of their private helpers carries a recorded defect
   class — the `blade_stations` out-of-bounds panic in
   [agent-scene-iteration-loop.md](agent-scene-iteration-loop.md) item 7, whose own sentence is *"a bug
   any future foliage hits"*. A `vegetation` crate is therefore partly a **promotion**, not a
   greenfield write — the same relation `building` had to `built.rs`.
2. **Trees in the gallery are two primitives with a scatter.** `terraces.rs` targets 110 China firs
   (`while firs.len() < 110 && index < 6000`), each drawn as
   **two cones** (`cone(9, 1.75, 2.6)` with `cone(9, 1.15, 2.3)` standing on it), and its broadleaf crowns are
   `sphere(10, 1.0)` scaled `(1.9, 1.5, 1.9)` — *"a crown is a squashed sphere sat low"* (its own doc).
   [sphere-is-a-hemisphere-twice.md](sphere-is-a-hemisphere-twice.md) already measured the cost of that
   sphere (*"neither is a crown a reason to spend 2592 triangles on"*) and left the caller question
   open: fixed into a real ball, the crown now *"hangs half into"* the cones it sits on.
3. **Three independent agent runs building a bamboo courtyard each hand-rolled the same foliage**, and
   the one that got furthest recorded the facts a crate should have known: a grove is *"six hundred
   placements of two meshes"*; *"a grove drawn with leaves down its whole height is a hedge; bamboo
   carries its leaves on the top third of the culm"*; and *"a culm is a cylinder of almost nothing: the
   whole of what it looks like is the light on it, so the body is a rim, a lane of wax, and no albedo at
   all to speak of"* — a shading fact found by rendering, which no generator in the gallery states
   (`worktrees/dsf/crates/gallery/src/courtyard.rs`, never merged).
4. **The cost model those runs reasoned from is wrong against `main`'s emitter, and the error shaped a
   scene's architecture.** The same file states *"a pass draws one instance, so one shader per material
   is one pass per material, and a grove of six hundred placements is six hundred passes"*. `main`
   groups instances by `(stage, shader, mesh, images)` — the material is **not** in the key — and one
   pass draws every row of one instance buffer (`crates/scene/src/standard/emit.rs`, `Group`'s own doc:
   *"A pass binds one vertex buffer and one instance buffer, and the instance buffer steps once per
   row"*). A vegetation crate has to state the cost model once, correctly, because foliage is the case
   where getting it wrong is most expensive.
5. **A wrong foliage fact is invisible, and nothing in the path can say so.** There is no `validate()`
   anywhere in it. `double_blade`'s doc records the class exactly: *"Reversing only the normals left the
   second ribbon wound against them, which is a leaf lit as though it were edge on"* — a defect found by
   looking, in a generator nothing calls. Leaves inside branches, a crown that is a hedge, a branch
   floating off its parent's skin, a tube expanded inside out: all of these are facts a planning layer
   knows before a frame is drawn.
6. **The one manual tool SpeedTree offers at this granularity is fragile by its own documentation** —
   *"Vertex edits can be lost if you change the model's resolution; use this tool at the very end of
   your modeling workflow"* ([survey §1.2](../investigate/vegetation-modeling-techniques.md)). An agent
   changes resolution constantly, cannot see the loss, and would read it as a picture that stopped
   answering. Whatever this crate's finest detail is, it must not be an offset on a mesh vertex.

## The one idea

`mountain`'s is that **the lines carry their correspondence**; `building` inherits it verbatim. This
crate inherits it and adds the half a plant needs, because a plant's detail is not planned:

> **A plant is a spine tree whose every node names the station and the ring slot it hangs from, and
> whose manual detail is an offset on a computed value, keyed by a handle that survives the
> computation.**

Two halves, and each is load-bearing:

- **The rings carry their correspondence.** A branch is a run of rings around a spine; ring *i* is
  paired with ring *i+1* by construction, so the tube is watertight because it was walked and not
  because a search agreed with itself. A junction is not found by an intersection test: a child *names*
  the station on its parent's spine, the ring slot around it, and how far it sinks into the parent's
  skin. SpeedTree's welding — *"branches will be pushed out and fused to their parent"*, with a
  `Force ray casts` option and a `Keep failed` flag for the welds that did not take — is exactly the
  search R4.1 excludes, and the authored station is what replaces it.
- **The edits carry their handles.** A node's manual value is an offset on the value its generator
  computed, so a generator edit does not destroy a hand edit; and the offset is keyed by a handle that
  is a **derivation path**, never an index into an expanded list. This is SpeedTree's own model
  (*"Node edits are offsets to the value it was already given by the generator … 0.0 means 'just use the
  generator value'"*) with the hole in it closed: SpeedTree loses a node's edits silently when *"the
  node no longer exists due to changes to the model"*, and here that is a **reported violation**.

## The instrument problem: what hand-sculpting means without a hand

An agent has four instruments and no fifth. It reads a picture; it reads text (a census, a report, a
violation list, a sidecar of numbers); it writes text; and it asks a harness `--locate x,y`,
`--where <name>` and `--census`. It has no cursor, no gizmo, no brush and no tablet. [agent-content-identity.md](agent-content-identity.md)
already settled the two wrong shapes this could take — a selection tool degenerates into a `pick(x, y)`
query, and text drawn into the frame asks the agent to use its weakest instrument — so the translation
is: **every gesture becomes either a named write against a handle, or a predicate that names a set.**

| SpeedTree's gesture | this crate's sentence |
| --- | --- |
| select a node, nudge a property (node value offsets) | `edit(Offset { at, property, by })` — one line, and it survives the generator edit that follows |
| select a node, edit its offset curve | `edit(Curve { at, property, points })` — a curve *added to* the computed profile, which is SpeedTree's own semantics |
| drag a node's local X/Y/Z | `edit(Shift { at, frame, by })` with the frame stated (`Local`, `Parent`, `World`), because an unstated frame is a canopy that hangs upside down |
| hand-draw a branch, then move its control points | `edit(Spine { at, points })` — a **bake**: the node's spine becomes authored control points and its generator's shape laws stop applying to it. Children still grow off it |
| click-place a node where you clicked | `edit(Graft { at, generation, station, slot })`, where `station` and `slot` come from `--locate`'s own answer (D8) |
| delete a node | `edit(Prune { at, reason })` — a tombstone, not a removal from a list, so nothing renumbers (D4) |
| paint a value over a brushed region | `paint(selector, change)` — **the brush is a predicate** (D10) |
| soft-select vertices and move them | **refused as a representation.** The equivalent write is against `(station, ring slot)`, which moves with the tessellation instead of being destroyed by it (D9) |
| bend a region | a `Field` in the document, applied per station in the expansion walk (D12) |
| constrain a crown to a sphere | an `Envelope`, authored; and `report()` answers with the profile the mesh actually has (D13) |
| leaf collision | a **report** with handles, and `derive::thin` writing prune marks the document holds — never a silent removal (D14) |
| knot / fin / cap / shell decorators | a `Decorator`: a small authored template attached at a station with a stated frame, which is `building`'s parts and sockets under another name (D11) |

The two things an agent genuinely loses, stated rather than hidden: **a stroke is a way of discovering
a shape**, and dictating control points is more precise but slower to arrive at; and **a brush gives
continuous feedback**, where a paint op is blind until the next frame. The compensations are D13's
numbers and D8's answer-that-is-an-edit.

## The vocabulary

In pseudo-rust, in the shape `mountain` and `building` use. Names are provisional; the relations are
the design.

```rust
/// One plant: the authored laws, the hand's table, and how finely it is expanded.
pub struct Plant {
    pub document: Document,
    pub tessellation: Tessellation,
}

pub struct Document {
    /// The laws: one per level of the hierarchy (trunk, bough, branch, twig, leaf, ground cover).
    /// A generation says how many nodes it makes, where they sit on their parent, and what each
    /// computes — the branch generator's property surface, minus the searches.
    pub generations: Vec<Generation>,
    /// Leaf and decorator shapes, authored in their own frames, placed many times.
    pub templates: Vec<Template>,
    /// Named laws applied to spines during the walk: gravity, ancestor, light, obstacle.
    pub fields: Vec<Field>,
    /// The canopy: what the silhouette was asked to be. Optional, and reported against either way.
    pub envelope: Option<Envelope>,
    /// The hand. Every entry is keyed by a handle; nothing else in the document is per node.
    pub edits: Vec<Edit>,
    pub seed: u64,
}
```

**V1 — the spine tree.** A node is a spine plus a skin law. The spine is a **cubic Hermite control
polygon**: a position and a tangent per control point, so continuity is structural and there is no
reconciliation pass — the argument `patch::hermite` already makes one dimension up (*"the corner jets
are shared rather than reconciled"*). The skin law is a radius profile (a curve of the station) times an
absolute-plus-relative radius, a radial segment count, an optional squash (an elliptical cross-section)
and a roll. A node's attachment is `Attachment { parent: Handle, station: Station, slot: f32, sink:
f32 }` — the station and ring slot it hangs from, and how far it sinks into the parent's skin.

**V2 — the leaf.** A `Template::Blade` is a closed outline control polygon in `(along, across)`, a
station count, a midrib law (droop and curl along `along`), a fold (curl across), a twist, and a
two-sided marking. Expansion is a loft of the outline along the midrib — the same walk as a branch, one
dimension smaller, and the general form of what `geometry::blade` computes as a fixed taper formula.
Placement is by generation mode (interval, phyllotaxy, bifurcation, proportional, absolute, flood) with
a station band on the parent (`first`, `last`, `clip`), an orientation pipeline in three stages (local
orientation, sensitivity to an anchor, final adjustments), a deformation group (fold, curl, twist, noise,
per-axis scale) and a **normal law** (smooth, puffiness toward an anchor at one of five anchor levels,
world-up for grass). The anchor levels are SpeedTree's and they are the mechanism behind both "leaves
point away from the trunk" and "a crown shades as a volume rather than as a bag of leaves".

**V3 — the decorator.** A knot, a burl, a scar, a bracket fungus, a thorn, a root flare, a fruit, a tip
cap. A small authored template attached at a station with a stated frame — and the frame is stated, not
inferred, on `building`'s socket rule: *"a joint is one subtraction, not a transform"*, so mating a
socket to an anchor moves the origin and no rotation is expressible, and therefore none can be got
wrong. A tip cap is a decorator the expander places by default at the end of a visible branch, which is
how an open tube stops being a hole seen from below.

**V4 — the field and the envelope.** A `Field` is a named law with a per-generation allow, a strength
and a profile curve of influence along the spine: gravity (world down), ancestor (toward or away from
the parent's direction), light (toward a direction the *scene* supplies, because the crate does not know
a sun), obstacle (an `impl Fn(Vec3) -> f32` occupancy, so a caller with a `terrain::HeightField` passes
its sampler and a caller with a building passes its mesh's signed distance — the same move
`building::site::seat` makes by taking a function rather than a crate). An `Envelope` is
`radius_at(height) -> f32` plus a plan outline, and its law is per node, not a search: a branch's length
is scaled so its tip lands on the envelope.

**V5 — the edit table.** One shape, six kinds, one application order:

```rust
pub struct Edit { pub at: Handle, pub what: Change }

pub enum Change {
    Offset  { property: Property, by: f32 },          // a nudge on a computed scalar
    Curve   { property: Property, points: Vec<[f32; 2]> }, // an offset curve added to the profile
    Shift   { frame: Frame, by: Vec3 },               // the local X/Y/Z offset
    Spine   { points: Vec<ControlPoint> },            // a bake: authored from here on
    Prune   { reason: Prune },                        // a tombstone
    Graft   { generation: GenerationId, station: f32, slot: f32, seed: u32 }, // an authored addition
}
```

**The order is stated once and is an invariant, not an implementation detail**: (1) compute the node
set from `(generations, seed)`; (2) apply grafts; (3) apply prunes; (4) apply spine overrides; (5) apply
offsets, curves and shifts; (6) apply fields and the envelope; (7) expand. A probe reads the same
document under two orders and reports where they differ, because an edit whose meaning depends on an
unstated order is a defect that only appears in a second caller.

**V6 — the handle.** A chain of slots from the trunk down, `trunk/bough 7/branch 3/twig 12/leaf 4`.
Each slot is a generation plus an index **in that generation's own canonical enumeration order**, and an
authored node (a graft) is numbered in a separate space, so a graft never renumbers a generated sibling
— the property [agent-content-identity.md](agent-content-identity.md) decision 1 wanted from an id
space. A handle **round-trips**: `parse(display(handle)) == handle`, checked by a probe, because an
agent writes back what a `--locate` answer printed and a handle that does not parse is a sentence it
cannot say.

## Decisions

Recorded with the alternatives they reject, in the form this repo's proposals already use.

### D1: crate name `vegetation`, publish `whiting-vegetation` (decided with the superior)

A class of thing, like `mountain` names the massif and `terrain` names the numbers. **Rejected:**
`plant` (an artefact name that reads as one organism, and the crate's second phase is a meadow),
`flora` (a discipline, which is why `building`'s D9 rejected `architecture`), `tree` (excludes the
bamboo, the fern and the grass that are the measured demand).

### D2: rust construction API first; the document phase is deferred with its cost recorded

`ops` for edits, `derive` for starting points, `validate` for what is wrong, `report` for what arrived,
`build` for the mesh — `mountain`'s R5 workflow, which three measured agent runs converged on
independently and which `building`'s D1 adopted. The agent's edit table is then a **list of rust calls
in a file**, and appending one is a one-line edit, which is what an agent is good at.

`building`'s D7 recorded why the `Definition` route is a phase of its own rather than an add-on, and
both halves still hold, the first of them verified against source rather than inherited: `Delta`'s
`Append`/`Remove` are **contiguous-only** by construction — *"An arbitrary non-contiguous set of indices
is deliberately not representable, because no append primitive can invert it"* (`definition/src/delta.rs:23-25`)
— and no geometry crate uses `Definition`/`Delta`/`History` at all. What is
new here, and worth recording because it makes the deferral cheaper than it was for a building: **the
edit table is already a delta.** It is a sparse set of changes keyed by paths into a derived structure,
which is `Delta`'s own definition ("the structure is a sparse subset of its referencing definition that
each value is a `Delta` to its referencing value"). So a future document phase is a *registration* —
a `Schema` for `Document` and `Edit`, with the derived node set as the referencing definition — and not
a redesign. It is still deferred, because a schema nothing validates against is a cost with no caller.

### D3: three tiers of manual detail, and the rule that separates them (decided with the superior)

| tier | what it says | when an agent reaches for it |
| --- | --- | --- |
| the generator | a law, applied to every node of a generation | "twigs are too long" — a species fact |
| the node offset | an offset on one node's computed value, surviving generator edits | "this bough, and only this bough, leans over the roof" |
| the bake | authored control points replacing one node's computed spine | "the generator is not going to get there" — a bonsai's dropped branch, a broken limb, a culm's specific lean |

The separating rule is a sentence an agent can decide: **an offset says "the law is nearly right here";
a bake says "the law is wrong here".** The two are not interchangeable and the crate does not pretend
they are — a bake stops the generation's *shape* laws from applying to that spine (its children still
attach to it, and still compute from it), which is SpeedTree's own rule for a hand-drawn branch
(*"Generation Parameters do not apply to hand drawn branches"*), kept.

**One deliberate divergence from SpeedTree, recorded with its reason.** SpeedTree lets forces and
perturbance keep acting on a hand-drawn spine, and warns that the result *"may appear pulled off the
line while drawing"*. Here a baked spine is **not shaped by fields either**: a field that bent an
authored control polygon would move the points the agent just wrote, which is the one thing the bake
exists to prevent. An agent that wants a hand-drawn branch to droop authors the droop into its control
points — that is what authoring means.

**Rejected:** *(a) offsets alone.* An offset can only nudge; a control polygon cannot be reached by one,
and "the agent can hand-sculpt detail" is the brief. *(b) bake alone* — mountain's R3 taken literally,
every derived node written into the document as an ordinary editable element. It is the right answer for
a massif (a few hundred ridge nodes) and for a building (tens of masses, hundreds of bays), and it is
wrong here by scale: a plant has one trunk, tens of boughs, hundreds of branches, thousands of twigs and
tens of thousands of leaves, and a document holding every leaf cannot be read, diffed or written by an
agent. *(c) SpeedTree's own mesh-vertex editing*, for the reason its documentation gives (motivation 6).

### D4: the handle is a derivation path; a stale one is a violation

A handle is `(generation, slot)` at each level, and slots are numbered by the generation's own canonical
enumeration order under a seed. Three consequences, each chosen over an alternative:

- **The RNG forks per node along the ancestry path**, so the same seed re-derives the same plant, the
  queue order is irrelevant, and editing one bough re-rolls that bough alone and never its neighbours.
  This is `building`'s D11, borrowed from the MIT rust CGA engine the architecture survey verified, and
  it is what makes "the edit did not reach the frame" diagnosable on the geometry side too.
- **A prune is a tombstone, not a removal.** Deleting node 4 does not renumber node 5, because the
  enumeration order is a function of `(generations, seed)` alone and never of previous edits.
- **A generation-count change does renumber, and says so.** Asking a bough for nine children instead of
  eight shifts every slot above the change, so handles below it go stale. `validate()` reports
  `HandleOrphaned` naming the edit and the count of handles that no longer resolve. This is the hole
  SpeedTree leaves open (*"the only time you'll lose a node edit is … the node no longer exists due to
  changes to the model"*) and it is closed here the only way it can be: by reporting.

**Rejected:** *(a) persistent authored integers per node* — the naming system with its own lifetime and
its own conflicts that [agent-content-identity.md](agent-content-identity.md) rejected for the same
reason under *Deliberately not proposed*. *(b) an index into the expanded node list or the mesh* —
"no index is ever a handle" is that document's rule and `SPEC.align/mountain.md`'s before it. *(c)
naming leaves by a species-wide ordinal* — 20 000 of them, and an ordinal is a slot by another name.

### D5: a branch is a loft of rings with authored correspondence; triangles only

The expander walks a spine's stations, lays a ring of `radial` points around each in the spine's own
frame, and zips ring *i* to ring *i+1*. The frame is **new machinery and is named as such**: a massif's
frame is a height field's (`up`, plus a plan direction), and a branch is a space curve, so the ring
frame is a transported one — the tangent plus a normal carried along the spine without spinning, with
the authored `roll` and `twist` applied on top of it. The design does not promise a specific transport
law; it promises one definition, written once, that `surface_uv` and the expander both read (D8).

`Topology` has exactly `Triangles` and `Segments` (`SPEC.md`, `scene_schema.rs`, `emit.rs`), which is a
hard gate and the reason a branch skin is two triangles per quad and not a quad. **Rejected:** emitting
quads and depending on `Topology::Quads`, which does not exist and would be a SPEC change; and emitting
a `patch::PatchMesh`/`HermiteSurface` — tempting, because a leaf blade is naturally a Hermite surface
whose one boundary is the spine's Hermite curve, and `patch::HermiteSurface` is on `main` (exactly C1
inside a soft-edge component, measured `across 0.000e+00` on every soft seam by
[handoffs/hermite-surface.md](../handoffs/hermite-surface.md) — a handoff still present, whose own
caveat is *"Not verified: that any scene renders"*). It is recorded as a **later unification** with its
condition stated: a leaf would be one patch, so `HermiteSurface::build`'s refusals about neighbouring
faces disagreeing on an edge's kind do not arise, but the crate would then depend on a representation
whose own lib doc still says the two models are "kept side by side while `HermiteSurface` is being
measured against real input". P1 emits triangles and keeps the authoring truth in the crate, exactly as
`building`'s D3 did.

### D6: every branch is a closed solid; a plant is deliberately not manifold

A branch is closed by a **base disc** (inside its parent's skin) and a **tip disc**, so each branch
answers the three predicates `building`'s D5 made its per-mass checks — zero boundary loops, manifold,
Euler characteristic 2 — plus **positive signed volume**, which is the check that turns an inside-out
tube from an invisible plant into a named violation. A plant is a set of **interpenetrating** closed
solids and is *not* manifold as a whole; "watertight" at plant level means every branch closed and
outward and every attachment authored, which is `building`'s two-level rule verbatim.

**Rejected:** *(a) SpeedTree's welding* — a fusion found by pushing geometry out and ray-casting it back
onto the parent, with a flag to keep the welds that failed. It is a search, and R4.1 is the reason this
crate exists. *(b) an open tube with a buried base* — cheaper by one disc, and it gives up the strongest
check in the crate: an open tube has no volume to be negative. *(c) `manifold` consolidation* — the
opt-in boolean `building`'s D3 nominated for the one case where parts must behave as a single solid. A
plant has no such case at this phase; a vine that must read as one solid with a wall is P3's question.

The interpenetration is licensed by `building`'s own measurement: *"Touching is allowed: a
zero-volume meeting is not an overlap"*, and a courtyard is *"several wall solids and not one cleverer
element"*. A branch entering a trunk is the same fact with a non-zero overlap, and `validate()`'s
overlap judgement is written against **unauthored** overlaps only — an attachment authorizes its own
junction, the way `attach_mass` ends the overlap question for a wing.

### D7: within a plant, bake; between plants, instance

The cost model, read from `main` rather than from a scene's comment:

- one pass draws **every row of one instance buffer**, and instances group by `(stage, shader, mesh,
  images)` (`emit.rs`, `Group` and `groups`);
- the instance row is the model matrix, an optional normal matrix, and **the material's custom data**
  (`emit.rs`'s own sentence: *"once one pass draws several instances at once a per instance attribute is
  the only place either can go"*);
- so per-instance numbers are reachable, but only through a material, and a material is a document entry
  that is verified and packed per instance;
- points, by contrast, are `Value::Packed`, and the compile cost now scales with **bytes**: `monoliths`
  at 182k points compiles in 32 ms and renders in 10 ([compile-cost.md](compile-cost.md)).

Therefore: **a plant is baked into a few meshes, one per material, with everything that varies carried
in the points** — position, normal, `(u, v)`, `authored_id`, a wind weight, a tint. A leaf count in the
tens of thousands costs point bytes, which are cheap, and one pass per material, which is what a scene
wants. **Instancing returns at the plant level**: a grove is one plant's mesh stood N times, which is
`building`'s D4 (*"the plan is baked; the vocabulary is instanced"*) one scale up, and the stranded
bamboo scene's 600 placements of two meshes are that shape already.

**Rejected:** an instance per leaf. It would need a material per leaf for any per-leaf number at all,
including the identity offset, and a document of 20 000 verified material entries to draw what one packed
mesh draws in one pass. Its *shadow* cost is the second reason: pages are fitted to instance boxes, so a
leaf-level instancing scheme multiplies the boxes the page pool sees, where a baked plant is a handful.

**The boundary this creates, stated rather than discovered.** Per-instance identity needs a per-instance
material (that is how `building` solved it: `Authored::Sum` plus one material a placement, measured at
five shutters). A grove of hundreds of plants therefore cannot give every plant its own id range without
hundreds of materials. So identity is a **plant-level** channel: a single-plant scene numbers every
element, and a grove either numbers the plants it can afford materials for or declares no id channel and
says so. `report()` states the element count so an agent knows the size of the table before it renders.

### D8: the identity channel, and an answer that is an edit

The mechanism exists and is landed: a scene declares `outputs: { authored_id: { location: 1, type: f32
} }`, the compiler generates the binding and adds it to every lighting pass, `f32` is `r32float` and
exact to 2^24 where an `rgba16float` is exact only to 2048, an identity is carried `@interpolate(flat)`
because a triangle is one facet of one element, and zero is reserved for *"not an authored element"*
([agent-content-identity.md](agent-content-identity.md), `gallery/src/building_identity.rs`). This crate
adds a third registration, not a mechanism, with two decisions of its own:

1. **An element is a node's surface, and a place on it is `(u, v)`.** `Element::Skin { node }`,
   `Collar { node }`, `TipCap { node }`, `Leaf { leaf }`, `Decorator { decorator, face }` — so the
   table is ~nodes + leaves rather than ~faces, and *"where on the branch"* is answered by the
   continuous half of the channel. A doubled blade's two ribbons are **one** element, because an element
   is what an author names and an author names a leaf, not a ribbon; which side a pixel is on is a fact
   about `(u, v)`, not a second row. **`(u, v)` has one definition, owned by the crate**: for a branch
   skin, `u` is the station fraction along the spine and `v` the ring fraction around it; for a leaf,
   `u` is along the blade and `v` across it. `PlantMesh::surface_uv(face, corner)` is the one function
   that returns them and the scene writes what it returns — mountain's `belt_uv` rule, adopted because
   the alternative is a second table: *"A `u` that meant 'fraction along the belt's top path' in the
   crate and 'first barycentric coordinate' in the shader would make every answer off by a rotation of
   the triangle, silently, and only on faces where the two differ."*
2. **`locate` answers with the edit, not only with the path.** Mountain's answer ends at
   `massifs[0].skeleton.ridges[5].nodes[21]`; this crate's ends at the call that would change what the
   pixel shows, because the pixel's `(u, v)` says *where on the element* the complaint is and the
   complaint is usually about that place:

```text
render oak out/oak --locate 742,508
  id 4213  u 0.612  v 0.208
  twig 12 of bough 7 of the trunk — its skin, 61% along and 21% around
  the edit:  edit(Offset { at: trunk/bough 7/twig 12, property: Length, by: -0.05 })
        or:  edit(Shift  { at: trunk/bough 7/twig 12, frame: World, by: [0.0, 0.0, 0.0] })
        or:  edit(Spine  { at: trunk/bough 7/twig 12, points: <authored> })   # a bake
```

The shape above is **written before it exists** and is an aspiration, not an output. The property it is
for is the one that document states: *"Not 'can look it up' — cannot look it up wrong."*

### D9: the finest detail is written against `(station, slot)`, never against a vertex

This is the answer to motivation 6, and it is the design's claim to be better than the reference rather
than merely text-shaped. A manual displacement of a branch's surface is authored as a **station band and
a ring arc with a value** — `edit(Curve { at, property: SkinDisplacement, points })` or a
`paint(Selector::Station { band }, …)` — and the expander applies it while it lays the rings. Change the
radial segment count from 6 to 12 and the displacement is still where the agent put it, because it was
never a vertex. SpeedTree's own vertex edit is the rejected alternative, and its documentation's caveat
is the measurement.

The rule that bounds it, from [SPEC.align.md](../SPEC.align.md): *"A feature smaller than about twice the
mesh cell must not be geometry."* Leaf veins, bark grain and a culm's striation are therefore **not**
geometry — they are a normal displacement in the surface body, which the lighting stage has offered since
`photographic-scenes` item 1 landed (*"a scene's body may reassign `normal`"*), or a procedural image
sampled against the `(u, v)` this crate writes. That is also why the crate writes `(u, v)` at all: a bark
law needs a parameter space to be written against, and a scene that invented its own would be a second
definition.

### D10: the brush is a predicate

```rust
pub enum Selector {
    Generation(GenerationId),                 // every node of a level
    Descendants(Handle),                      // one subtree
    Slot { parent: Handle, from: u32, to: u32 }, // a range of siblings
    Station { band: (f32, f32) },             // a band along the parent's spine
    Where(Property, Range<f32>),              // a test on a computed value: "twigs shorter than 0.2"
    In { low: Vec3, high: Vec3 },             // a world box
    Ids(Vec<u32>),                            // the ids a caller read off a frame
}
```

`Selector::Ids` is the picture-shaped one, and it is the mechanism that closes the loop: the harness
already writes an id image plus a sidecar of values beside every declared output, so **a rectangle of a
rendered frame becomes a set of handles**, and `paint` applies one change to all of them. That is the
brush. The crate takes **ids, not a file**, because nothing in this framework reads the filesystem and
because the dependency direction is already settled — `gallery::locate` takes three numbers rather than a
frame for exactly this reason (*"a frame is the device's type"*). The caller that reads the sidecar is
the harness (see *What lands where*).

**Rejected:** any picking that needs a cursor; text labels drawn into the frame (a font atlas, a quad per
label, occlusion, over a frame the labels then cover — and it asks the agent to use its weakest
instrument); and a `Selector` that holds a closure over the mesh, which cannot be written into a
document later and cannot be printed for a report.

### D11: decorators are parts, and a joint is one subtraction

A knot, a fin, a cap, a root flare, a fruit, a thorn: a `Template` in its own frame, attached at a
station with a socket that names the anchor it mates with. This is `building`'s parts vocabulary, and the
property that made it measurable transfers unchanged — *"with the doorway's head moved from 2.35 to 2.55,
a canopy hung on the head stayed exactly +0.0000 above it while the same canopy pinned by numbers went
from +0.2200 to +0.0200. A pin cannot notice."* A bracket fungus that stays on its bough when the bough
bends is the same sentence.

SpeedTree's four decorators (cap, fin, knot, shell) and its target generator (*"used to precisely locate
children off any parent"*) are the same idea with a viewport; the cap is the one this crate needs by
default (D6's tip disc).

### D12: fields are laws applied in the walk, and one of them is the ground

Gravity, ancestor (grow toward or away from the parent's direction), light (toward a direction the scene
supplies), obstacle (an occupancy function), and SpeedTree's force containers reduced to their honest
form: an include/exclude test against an occupancy function, which is a per-node predicate and not a
search. A field is applied at step 6, per station, in the same walk that expands — so it is O(stations)
and there is no solver, no iteration to convergence and no result that depends on an order the document
does not state.

**The ground is an authored input, not a dependency.** `building`'s D8 rule is adopted verbatim: seating
takes `impl Fn(Vec2) -> f32`, so a caller with a `terrain::HeightField` passes its sampler and the crate
stays terrain-independent (`terrain` produces numbers, never geometry; this crate produces geometry,
never erosion). A root flare is a decorator seated by the same function, and "a culm's base is not on the
ground" is a violation rather than a picture of a floating plant.

### D13: the report is the crown profile against the envelope

`report()` answers what arrived, in numbers, and the headline number is a comparison:

> **the mesh's own radius-versus-height profile, measured against the envelope that was asked for, is
> this crate's one-number check that the geometry is what the plan said** — the sibling of
> `Massif::footprint_area` against `FacetMesh::plan_area`, and of `building`'s measured coverage against
> a footprint.

```text
vegetation report oak_starter            # the shape of the report, written before it exists
  generations   trunk 1 · bough 8 · branch 41 · twig 320 · leaf 12 480
  elements      13 589 numbered (370 skins, 369 collars, 370 tip caps, 12 480 leaves),
                ids 1..=13 589, zero never written
  triangles     402 118 · points 1 206 354 · packed 48.3 MB at a 40 byte stride
  bounds        x [-3.10, 3.40] · y [0.00, 9.20] · z [-3.00, 3.20]
  crown         height 9.200 · radius at 6.00 m: asked 2.10, measured 2.34 (+0.24)
  wood          volume 0.4182 m³ · leaf area 214.6 m² · leaf area index 3.9
  junctions     369 measured, 4 outside the ratio band [1.8, 2.4]
  wind weight   zero at all 12 849 pivots, non-zero at none
  violations    0
```

Every line is a fact the crate has before a frame is drawn, and each answers a complaint an agent
actually makes: *the crown is too narrow at six metres* (the crown line), *it reads as a hedge* (the leaf
distribution over the station band, which is a `report` line and not a guess), *the trunk is too thin for
its boughs* (the junction line), *the leaves are floating* (the violations), *the frame is too big* (the
counts). `wood volume` is `building`'s signed-volume argument at plant scale: a number no closure
predicate can fake.

### D14: collision is a report, and its removal is an authored prune

SpeedTree's leaf collision **removes** the loser (*"the one with the higher weight wins"*, optionally
taking the parent and the descendants with it). That is a search whose result silently rewrites the model,
so under R3/R4.1 it is split in two: `validate()` **reports** `LeafInsideABranch { leaf, node, depth }`,
bounded to the worst N with the bound stated, and `derive::thin(..)` **writes prune marks** into the edit
table — guarantee-free, immediately editable, and ordinary document data from then on. The agent sees a
count, asks for a thinning, and reads the count again. Nothing is deleted behind its back.

The cost is bounded by construction: a leaf is tested against the branches of **its own ancestry** (the
node it hangs from and the chain above it), not against every branch, so the check is O(leaves × depth)
rather than O(leaves × branches). A leaf inside a *neighbouring* bough is not reported, and that bound is
stated in the report rather than discovered by an agent that wondered why a collision it can see is not
listed.

### D15: a species is a `derive::Species` of relations, and never document data

`building`'s style slice decided this for a building and the argument is unchanged: *"a style is a rule
for computing numbers, not a fact about a building, so the skeleton holds the numbers and the rule stays
in `derive`"*. A `Species` states relations — height as a share of trunk radius, an internode as a share
of its parent's length, a leaf as a share of its twig, a crown ratio, a branching exponent, a phyllotaxy
— plus the facade of a plant: which generation modes it uses, its station bands, its envelope.
`Species::starter(..)` derives a whole plant in one call, the R3 `Starter` pattern, and every number it
wrote is then ordinary editable data.

Three ship with P1, chosen because each exercises something the others do not: **`PINE`** (proportional
steps, a narrow envelope, whorled boughs), **`OAK`** (bifurcation, a broad envelope, a bifurcating trunk
split), and **`BAMBOO`** (a culm whose joints are a radius offset at stations, leaves confined to a
station band in the top third, and a clump) — the last because it is the measured demand (motivation 1,
3) and because it is the one that says whether a station band and a ringed profile are expressible at all.

The relations carry their bounds, on the lesson `building`'s style probe paid for: *"a `1/19` share with
no ceiling gave a 12 × 18 cottage a 0.63 m wall — a fortress with a window in it. A relation between a
share and a span is only a relation if both ends are bounded."*

### D16: no LOD chain; one resolution dial, because the deliverable is a still frame

SpeedTree's LOD is a runtime mechanism — LOD curves over a sequence, a dynamic leaf LOD whose survivors
*"grow to fill the vacated space"*, "phantom" branches hidden rather than deleted. This framework renders
a frame, or a range of frames at a stated `time`, and has no runtime consumer for a chain. So: one
`Tessellation` dial (stations, radial segments, leaf stations, and a scalar), overridable per generation,
plus `report()`'s counts. The dial is **not** allowed to move an authored shape, which is D9's invariant
restated: the spine is a curve, so retessellation changes the triangle count and nothing else. This is
also the answer to SpeedTree's `Accuracy` warning (*"changing accuracy may drastically affect the shape of
your spines. Set this value to the maximum desired fidelity early"*) — a warning that exists because its
spine is sampled rather than a curve.

If a runtime ever arrives, an LOD chain attaches at the same place `building`'s D4 instancing would: a
per-generation resolution and a per-generation visibility threshold, both already in the document.

### D17: wind is the crate's weights and the scene's law

A still frame does not move, but the harness renders a `time` range and `time` reaches every pass
(`scene_schema.rs`: *"Seconds since the scene started, which every pass carries to its shader"*), so a
wind law is checkable with `--sheet`. The split:

- the crate writes a **per-point wind weight** and a group, with the invariant SpeedTree states and this
  design adopts as a mesh self-check: the weight is **zero at the attachment**, because a non-zero weight
  at the pivot detaches a leaf from its parent (*"The left-hand side of the profile curve must be zero to
  avoid disconnecting from the parent"*). It also writes the weight's natural parameterisation — along the
  spine for a branch, along the blade for a leaf, with the leaf's cross term weighting the edges more than
  the centreline — because those are facts about the geometry the crate has and the shader does not.
- the scene writes the **law**, in its own WGSL, because a shader carries its own source and a wind law is
  a shading decision.

**What is owed, named rather than assumed.** The gallery's shader template splices a body into the
*fragment* stage (`{surface}`, and the normal-displacement point before it); the `Authored` extras carry
values into a varying from the vertex stage but **cannot move a vertex**. [photographic-scenes.md](photographic-scenes.md)
item 1 already named this as the missing half: *"a leaf that turns … wants the silhouette to move too,
which is a vertex-stage hook and a different decision."* So wind needs either a scene-authored vertex
stage (available today: `kit::shader` takes arbitrary source) or a vertex splice in the template. P3
decides which, and until it does, `tidewater`'s reeds remain the precedent — *"sway in shading only and
the file says so"*.

### D18: two-sided leaves are a shading law if the builtin is available, and doubled geometry otherwise

The device creates its pipeline with `PrimitiveState { topology, ..Default::default() }`
(`device-rasterize/src/wgpu_renderer/raster.rs`), and that default is **no culling**: `PrimitiveState`
derives `Default` and its `cull_mode` is an `Option<Face>` (`wgpu-types 30.0.1`, `src/render.rs`, read
2026-10-17), so `None`. **Both sides of every triangle are already drawn** — the problem is not culling,
it is that a leaf seen from behind is lit by its front normal. Two answers, in order of cost:

1. a fragment-stage flip in the surface body (`@builtin(front_facing)`), which costs nothing in geometry
   and lands in the splice point that already exists;
2. `geometry::double_blade`'s doubled ribbon — the same taper run twice with opposed normals **and the
   winding reversed**, which its own doc records as the fix for the version that reversed only the
   normals.

**Which of the two is available is not measured here.** Whether a scene's fragment entry may declare
`@builtin(front_facing)` past the interface check is a **probe**, and this design states it as a question
rather than a capability, on the rule that a capability claim is a hypothesis until it is measured. The
check is known to be one-directional and to skip at least one builtin — `@builtin(vertex_index)`, *"which
`check_vertex_inputs` deliberately does not check"* — so the answer is probably yes. Probably is not
measured.

### D19: the branching ratio is a check with a stated exponent and no botanical guarantee

A junction's radii obey `r_parent^n = Σ r_children^n`, with `n = 2` the relation attributed to Leonardo
and argued from a constant wind-stress criterion; measurements on real trees give a range rather than a
number ([survey §2](../investigate/vegetation-modeling-techniques.md), whose [S20] and [S21] are the two
— **neither paper was read this session**, so the range is not established here). The crate therefore
computes the ratio at
every junction in O(1), reports the ones outside an **authored band** whose default is stated in the
report, and guarantees nothing botanically. It is the same kind of instrument as `plan_area`: a number
that turns an invisible error — a bough too thin for what it carries, which reads as a tree that has been
drawn rather than grown — into a named violation. The band is a `Species` relation, so a palm and an oak
are not judged by one number.

## `validate()`: reports, never repairs

`mountain`'s measured shape, adopted without amendment: **all** violations returned in a deterministic
order so two runs diff; checks **staged**, so a downstream check is skipped when the document cannot
support it; ops never refuse an edit except one that would leave a dangling handle, which refuses by
returning `None`; **build failure is a list**, not a first error; geometry self-checks live on the mesh;
and the reporting vehicle during implementation is an `examples/` probe, not a test file.

Non-exhaustive violation list, in the order the stages run:

| stage | violation |
| --- | --- |
| the document | `GenerationCycle`; `PropertyNotComputed` (an offset on a property this generation does not compute — a field nothing reads is data waiting for a stage); `HandleOrphaned`; `EditOnAPrunedNode`; `GraftStationOutOfRange`; `AttachmentStationOutOfRange`; `SpineControlPointDuplicated`; `BladeOutlineNotSimple`; `DecoratorSocketUnresolved` |
| the walk | `BaseNotBuried { node, clearance }`; `RingDegenerate { node, station }` (zero radius, coincident points, a facet of no area); `RingSelfIntersecting`; `SpineKinked { node, station, angle }`; `ZeroLengthBranch`; `LeafOffItsParent`; `LeafInsideABranch { leaf, node, depth }` (bounded to the worst N, with the bound stated, per D14); `BranchRatioOutside { node, parent, children, exponent }` (D19); `TipOutsideTheEnvelope { node, distance }` |
| the mesh | `WindingInconsistent { face }`; per branch: boundary loops not zero, not manifold, Euler not 2, **signed volume not positive**; `FaceWithoutAnElement`; `IdsNotContiguous`; `WindWeightNotZeroAtTheAttachment` |

`TipOutsideTheEnvelope` is the one whose severity is the author's: an envelope is a plan and a branch may
deliberately exceed it, so it is a **note** unless the envelope is marked strict, on the policy
[adapter-limits-and-the-page-pool.md](adapter-limits-and-the-page-pool.md) settled — a note distinguishes
*a smaller frame* from *a wrong one*. The crown profile's departure from the envelope is a `report()`
number and not a violation at all, because it is a measurement of what arrived rather than a claim the
document made (D13).

Three of these are new machinery and are named as such: **signed volume** (which `mountain` has none of,
and which `building` had to write), **the transported ring frame's degeneracies** (a spine that turns
back on itself flips a transported normal, which is a kink and must be reported before it is a mesh), and
**the junction ratio** (D19).

## The agentic loop

The loop already exists and is measured: compile → render → manifest → camera notes → contact sheet, in
`crates/device-rasterize/examples/render.rs`. What this crate contributes:

- **`validate()` is the cheap inner loop.** A wrong plant is named before a frame is spent on it. Every
  foliage defect the gallery has recorded — a leaf lit as though edge on, a crown hanging half into its
  cones, a grove that is a hedge — is a validate-shaped fact.
- **`report()` replaces squinting.** The crown profile against the envelope is the sentence *"the
  silhouette is not what I asked for"* as a number, and it is available without a gpu.
- **`locate` answers with the edit** (D8), so the hop from a picture to a document is one line of output
  rather than an inference.
- **`--region`** (below) turns a rectangle of a picture into the set of handles in it, which is the brush.
- **The manifest's verdict works unchanged** — *"your change did not reach the frame"* and *"nearest paths
  that differ"* — because a plant compiles into the same document path a massif and a building do.
- **A segmentation view is a scene, not a framework change.** The id channel is per-element and flat, so a
  scene whose surface body writes a colour derived from `authored_id` draws one colour per element with no
  new channel and no new plumbing. That is the single most useful picture for judging structure, and it
  costs a scene file.

The role split the architecture survey converged on is adopted verbatim, and FloraForge corroborates it
from the plant-science side: the model **plans and critiques** — it authors handles, offsets, envelopes
and species relations — and the deterministic expander **emits geometry**. The model never emits a number
where a handle is expected, and a hallucinated handle surfaces as `HandleOrphaned` instead of broken
geometry. `validate()` is therefore also the cheapest known guard against an LLM corrupting a good state.

## What lands where

### The crate (new)

`crates/vegetation`, publish `whiting-vegetation`, with `document`, `spine`, `expand`, `leaf`,
`decorator`, `field`, `edit`, `selector`, `derive`, `validate`, `report`, `identity`, `mesh` — small files
for sparse exploring, on the workspace's own rule. Its dependency is `mountain` for `math` alone, exactly
as `building`'s, and for the same reason: one definition of which axis is up, which two crates authoring
into one scene must share.

**And that is now a third crate.** `building`'s Cargo.toml records the arrangement and its debt: *"`mountain`
for the plan/space arithmetic only … the unification of the two is a future proposal rather than a copy
made here"*, and its D2 named *"a shared mesh crate (with the plan-geometry helpers `mountain` keeps
private)"* as the unification to propose when a second caller needs it. `building` was that second caller;
this is the third. The predicates
this crate would otherwise write a third time — boundary loops, manifoldness, Euler characteristic, signed
volume, consistent winding — are the same ones `building` wrote because `mountain`'s are inherent to
`FacetMesh`. **This design records the demand and does not perform the unification**, because doing it
inside a P1 would couple a new crate to a refactor of two existing ones; it is open question 1.

### The gallery

`vegetation_identity.rs` (the third sibling of `identity.rs` and `building_identity.rs`), `vegetation_mesh.rs`
(the conversion from `PlantMesh` to a `PackedList`, as `building_mesh.rs` is), and the scenes.

**And one debt it inherits.** `gallery::locate` asks two domains in sequence and takes whichever answers —
building first, then mountain. [agent-content-identity.md](agent-content-identity.md) already named the
limit (*"a second scene with its own naming, which is what turns `identity::locate`'s `scene ==
"landscape"` into a registry. One match arm is honest at one scene"*). A third domain is where the chain
stops being honest, so **the registry is owed by this work and is named here rather than added to it**:
one function per content crate, keyed by scene, with the harness asking the registry instead of a
hard-coded pair.

### The harness

One flag, `--region x0,y0,x1,y1`, repeatable like `--locate` and `--where`: scan the id channel over the
rectangle, collect the distinct ids, and print each one's name and pixel count. It reads the sidecar the
harness already writes and asks the same table `--locate` reads, so no library crate is involved and the
three answers stay three different answers — the discipline `--where` was built with (*"an element with
pixels, an id with none, and a name that does not exist"*). This is the mechanism `Selector::Ids` is fed
by, and without it a paint op can only be aimed by a box in world space.

### Nothing

No SPEC change, no graphics change, no device change. `Topology::Triangles` is enough (D5); the declared
output channel is landed (D8); per-instance data through a material is landed (D7); images as scene data
are landed, which is what a procedural bark or leaf mask would use when a phase wants one; `time` reaches
every pass (D17). The one gap this design touches is the vertex-stage hook, and it is a **gallery** change
or a scene's own source, not a device or graphics one — and it is already on record as missing in
`photographic-scenes` item 1 (D17).

## Deliberately not proposed

- **Cards and billboards, and any alpha-tested leaf.** The mechanism exists — an image is scene data named
  by a material, the compiler lifts it into a texture with a sampler, and `tiles` proves a scene may
  generate its own pixels — but there is no consumer at this phase, and a camera-facing card needs a
  vertex-stage law as well as a mask. Recorded as the LOD and ground-cover answer a later phase would
  reach for, and as the reason a far tree is not this crate's problem yet.
- **Seasons.** A season is a material and leaf-shape selector over a timeline; the picker is `building`'s
  D11 weighted-alternative mechanism (`70% WinA | 30% WinB`, resolved once per model so coherence is data
  rather than luck) and the timeline is the harness's `time` range, so it is expressible without new
  vocabulary. Not proposed, because nothing asks.
- **Growth animation.** SpeedTree's growth group is a timeline over the same geometry. A `time`-keyed
  station band would express it; it is not proposed, because the deliverable is a plant.
- **Photogrammetry.** Nothing in this framework reads the filesystem, and there are no scanned assets.
- **Fronds as a distinct generator.** A palm frond or a fern is a blade loft along a spine, which is V2
  with a midrib that is itself a node's spine. SpeedTree needs a separate generator because its leaf mesh
  is an asset; this crate's leaf is authored, so the case does not arise.
- **L-systems, space colonization and stochastic botanical rules as the representation.** Admissible in
  exactly one role: producing default spines and default placements, guarantee-free and immediately
  editable (R3), never the mesh.
- **A DSL, a serialization format, an editor frontend, an undo/redo service.** `mountain`'s out-of-scope
  list, unchanged. `History` in the `definition` crate is the undo mechanism if the document phase (D2)
  ever lands, and the edit table is already in its shape.
- **A whole-plant boolean.** No `manifold` consolidation, on `building`'s D3 reasoning: a plant has no case
  where parts must behave as one solid, and a vine on a wall is P3's question, not a dependency.

## Phasing

- **P1 — one plant.** The document and the handle; the Hermite spine and the transported ring frame; the
  ring-loft expander with base and tip discs; the leaf template and its loft, with `derive::blade_taper`
  as the default outline (the taper `geometry::blade` already computes as a formula, promoted to a
  starting point an agent then edits), the placement modes
  (interval, phyllotaxy, absolute, proportional) and the station band; the edit table's six changes and
  its stated application order; `derive::Species` for `PINE`, `OAK` and `BAMBOO`; `validate()` with the
  document, walk and mesh stages including signed volume, winding, burial, degenerate rings and orphaned
  handles; `report()` with the crown profile against the envelope and the junction ratio; identity
  (`Element`, contiguous ids, the reserved zero, one walk producing both the numbers and the table); the
  gallery proof scene. **Exit: one rendered plant in which every branch is closed and outward, every base
  is buried, every face names its element, the crown profile matches its envelope to a stated tolerance,
  and a wrong plant refuses to be wrong silently.** The proof scene is the bamboo courtyard's subject —
  one culm with its leaves in the top third — because that is the plant three agent runs each hand-rolled,
  and because it exercises the station band, the ringed profile and the two-sided leaf at once.
- **P2 — the hand.** The selectors and `paint`; `bake` and the authored-spine path; the decorators with
  sockets; the fields (gravity, ancestor, light, obstacle) and the envelope as a trimming law; the
  collision report and `derive::thin`; the leaf normal laws (puffiness toward an anchor, world-up for
  grass); the two-sided leaf decided by the probe D18 names; `--region`; the segmentation scene; the
  promotion of `geometry::{culm, blade, double_blade, leaf_spray, tuft}` into `derive`, which retires five
  uncalled generators from the gallery.
- **P3 — the site and the motion.** A grove (instancing at plant level, with D7's identity boundary
  stated in the scene's own doc); ground cover (a tuft mesh stood many times, world-up normals); seating on
  a `terrain::HeightField` by the `site::seat` precedent; wind weights plus the vertex-stage decision; the
  offline backend's view of a plant, where a two-sided leaf is a different question than it is in a
  rasteriser (`cycles-mesh-normal-orientation.md` is the note to read first); and the `locate` registry the
  gallery is owed.

Out of scope, all phases: LOD chains; cards and alpha maps; seasons; growth timelines; photogrammetry;
simulation of any kind (a plant's mechanics are a report, not a solver); a DSL and serialization; an
editor frontend; an undo/redo service.

## What SPEC would say if this is adopted

SPEC is human-editable only, so this proposal states the delta rather than writing it: a
`SPEC/vegetation.md` sibling in the shape of [SPEC/mountain.md](../SPEC/mountain.md) — motivation, R1..Rn,
out of scope — plus one line under the workspace description. The requirements it would carry are the ones
this design cannot violate without ceasing to be itself: a plant is authored as a spine tree whose
attachments are named rather than found (R4.1's sentence, one dimension up); manual detail is an offset on
a computed value or an authored spine, never an offset on a tessellation; derived defaults carry no
guarantee and are ordinary editable data the moment they exist; the output is planar-facet triangles; and
the crate is a pure rust crate with a rust api, no editor frontend and no runtime service of its own.
**No `Topology::Quads` is required** (D5) and no graphics or device change is required (D7, D8, D17).

## Open questions

1. **The shared mesh crate.** `mountain`, `building` and now `vegetation` each need the plan/space
   arithmetic and four mesh predicates, and `building`'s D2 already named the unification. Does P1 write
   them a third time and propose the crate afterwards, or does the crate come first? Writing them a third
   time is the choice this design makes, because a P1 that depends on a refactor of two crates in flight
   (`building` merged its volume model into `main` while this file was being written) is a P1 that cannot
   start; the cost is a third copy of predicates whose disagreement would be silent.
2. **The spine's basis.** A cubic Hermite control polygon is the recommendation (D5), because the jet is
   the data and continuity is structural — the argument `patch::hermite` makes one dimension up. The
   alternatives are a Catmull-Rom polygon (tangents derived, so inserting a control point moves its
   neighbours' tangents and an edit is not local in shape) and a NURBS curve (a knot vector and weights an
   agent would have to write, for a property — exact rational conics — a branch does not need).
   [investigate/nurbs-libraries.md](../investigate/nurbs-libraries.md) and
   [investigate/hierarchical-spline-bases.md](../investigate/hierarchical-spline-bases.md) already surveyed
   the surface side; neither surveyed curves, and `feature/catmull-probe` is about surfaces too.
3. **A leaf's identity when the count is large.** Every leaf is an element (D8), so 20 000 leaves are
   20 000 table rows beside every frame. SpeedTree's answer to the same pressure is a generator that
   *"forgoes individual node editing for orders-of-magnitude faster computation"*. The equivalent here is a
   generation marked **unnumbered**, whose leaves are answered by their parent node and a `(u, v)`; whether
   P1 needs it depends on a count nobody has measured yet, so it is a question and not a decision.
4. **The envelope's form.** A `radius_at(height)` profile revolved about the trunk is enough for a conifer
   and an oak and cheap to author; a plan outline plus a height profile covers an espalier and a hedge; a
   mesh or an occupancy function covers everything and cannot be written down. Which one P1 ships decides
   what an agent can say about a silhouette in one line.
5. **Where the wind hook lands** (D17): a vertex splice in the gallery's template, which every scene would
   carry, or a scene-authored vertex stage, which one scene would write. The template already grew one
   splice for the normal and the argument there was that a body which cannot reach a value cannot use it;
   the same argument applies to a position.
6. **Whether a plant may attach to a building.** A vine on a wall, a creeper in a courtyard, a tree whose
   roots meet a foundation: the attachment vocabulary exists on both sides (`Socket`/`Anchor`, `Station`),
   and the cross-crate question is who owns the joint. P3's site phase, recorded here so it is not
   rediscovered as a surprise.
7. **Whether `validate()`'s junction ratio belongs in P1.** It is O(1) per junction, it is the check that
   makes a tree read as grown rather than drawn, and its tolerance is not established from a primary source
   ([survey §2](../investigate/vegetation-modeling-techniques.md)). A band with no botanical guarantee is
   still a report an agent can use; a band that fires on every oak is worse than no check. Decide at P1,
   with a probe over the three species rather than an argument.

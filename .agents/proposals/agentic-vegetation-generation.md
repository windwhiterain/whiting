# Agentic procedural vegetation generation: the `vegetation` crate

Status: **DRAFT, revision 2 — design only, amended by the second round of verdicts.** No implementation
exists, so this file carries no measurements of its own. Every number in it is read from a source file,
from a document that measured it, or from SpeedTree's own documentation, and says which. Where a report's
*shape* is shown, it is labelled as written before it exists.

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
- **Scope of the first phase: one plant.** A grove and ground cover are later phases; wind is not in this
  design at all (see the second round of verdicts below).
- **Authoring interface: a rust construction API first**, exactly like `mountain`'s R5 and `building`'s
  D1. Document/`Delta` authoring is a possible later phase; D2 records what it would actually cost.
- **Deliverable: this proposal and the reconnaissance note.** No implementation code until the design
  is approved.

**Verdicts of the second round, 2026-10-17** (recorded verbatim, because each one overrides a decision
above and the reason must survive with it):

> 先做共享（尤其是曲线很值得做出来，后面必然导出都会依赖它）；曲线同意 Cubic Hermit。
> 设计的时候叶子就用 instancing 的方式设计，相当于设计叶子的分布场，然后某个算子在场里面 instancing。
> 包络线粗版本用曲线参数空间的 radius 曲线，可以精化到曲线参数空间上的垂直平面上的曲线的曲面。
> 先不管风；先不管 attach building；validate 看着办，需要的时候自然会产生这个需求。

What each one changes:

| verdict | what it decides here |
| --- | --- |
| the shared crate comes first, with the curve | a `shape` crate is P0 and this crate's P1 sits on it; its design is [shape-primitives.md](shape-primitives.md). **D5's basis question is closed: cubic Hermite.** |
| leaves are instanced through a distribution field | **D7 flips**: leaves are a field expanded into instances of a leaf template, not geometry baked into a plant mesh. D8's identity story is amended to the instance row it now depends on |
| the envelope is a radius curve in the curve's parameter space, refinable to a section curve in the normal plane | **D13 rewritten**: the envelope is a `shape::Sweep` about the spine, and refining it is a `Section` change rather than a new type |
| no wind | **D17 withdrawn.** The wind weights also leave the point schema, on the repository's own rule that *"a field nothing reads is data waiting for a stage"* |
| no attachment to a building | open question 6 deferred, and recorded as deferred rather than dropped |
| `validate` as it comes up | the violation list is **demoted from a designed contract to a candidate list**; what stays designed is the discipline (report, never repair) |

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
| constrain a crown to a sphere | an `Envelope` — a radius curve in the spine's parameter space (V4) — and `report()` answers with the reach the mesh actually has at each station (D13) |
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

**V1 — the spine tree.** A node is a spine plus a skin law, and both are `shape`'s (see
[shape-primitives.md](shape-primitives.md) D1–D4, which the superior's verdict made P0):

- **the spine** is a `shape::Curve` — a chain of Hermite jets, a position and a tangent per knot, so C1
  at every interior knot is structural and there is no reconciliation pass;
- **the skin** is a `shape::Sweep` whose section is `Section::Circle { radius }`, and the radius is
  **itself a curve of the station** — so a branch's taper, a culm's internodes and a root flare are one
  mechanism (a radius curve) rather than three special cases;
- **the attachment** is `Attachment { parent: Handle, station: f32, slot: f32, sink: f32 }`: the
  arc-length fraction on the parent's spine, the fraction around its section, and how far the base sinks
  into the parent's skin.

The station is an **arc-length fraction**, not a knot index, which is `shape` D2's decision and is what
makes "the top third of the culm" a number that means what a botanist means by it.

**V2 — the leaf, as a distribution field over instances.** The superior's verdict: *"设计的时候叶子就用
instancing 的方式设计，相当于设计叶子的分布场，然后某个算子在场里面 instancing"*. So a leaf is not
geometry the plant expands; it is two things:

```rust
/// What one leaf looks like, in its own frame. A `shape::Sweep` with an open section.
pub struct LeafTemplate {
    pub midrib: Curve,          // the blade's own spine: droop, curl and twist are this curve's shape
    pub section: Section,       // Flat { half_width, thickness } — a thin closed solid, so both faces light (D18)
    pub stations: usize,
}

/// Where leaves go: a field, and the operator that instances within it.
pub struct LeafField {
    pub target: Target,         // a generation, or a selector (D10)
    pub band: (f32, f32),       // `first`/`last`, as arc-length fractions of the target's spine
    pub density: Curve,         // leaves per metre of spine, as a function of the station
    pub phyllotaxy: Phyllotaxy, // the turn law: golden angle, decussate, distichous, whorled(n)
    pub size: Curve,            // leaf size as a function of the station
    pub orientation: Orientation, // the three stages: local, anchor sensitivity, final adjustments
    pub template: TemplateId,
}

impl LeafField {
    /// The operator: sample the field, and produce one placement per leaf.
    pub fn instances(&self, plant: &Plant) -> Vec<LeafInstance>;
}
```

`LeafInstance { node: Handle, station: f32, slot: f32, frame: Frame, scale: f32 }` is exactly what a
scene's instance needs — a translation, a rotation and a per-axis scale — so **a placement becomes one
scene instance of one leaf mesh with no per-placement document entry at all** (D7). Identity is the one
thing a placement cannot carry for free, and D8 states that boundary.

Editing a leaf is therefore editing the **field** — its band, its density curve, its turn law, its size
law, its orientation laws — and a hand edit to *one* leaf is a `bake` of that placement into an authored
exception (D3's third tier). This is a better fit for the brief than per-leaf geometry was: an agent
looking at a crown does not want to name four thousand leaves, it wants to say "fewer, and smaller, above
six metres", which is one curve edit.

The orientation and normal laws are SpeedTree's and they survive the change: an orientation pipeline in
three stages (local orientation, sensitivity to an anchor, final adjustments) and a **normal law**
(smooth, puffiness toward an anchor at one of five anchor levels, world-up for grass). The anchor levels
are the mechanism behind both *"leaves point away from the trunk"* and *"a crown shades as a volume
rather than as a bag of leaves"* — and with instancing they are per-placement data the crate computes,
not per-vertex geometry.

**V3 — the decorator.** A knot, a burl, a scar, a bracket fungus, a thorn, a root flare, a fruit, a tip
cap. A small authored template attached at a station with a stated frame — and the frame is stated, not
inferred, on `building`'s socket rule: *"a joint is one subtraction, not a transform"*, so mating a
socket to an anchor moves the origin and no rotation is expressible, and therefore none can be got
wrong. A tip cap is a decorator the expander places by default at the end of a visible branch, which is
how an open tube stops being a hole seen from below (D6).

**V4 — the field and the envelope.** A `Field` is a named law with a per-generation allow, a strength
and a profile curve of influence along the spine: gravity (world down), ancestor (toward or away from
the parent's direction), light (toward a direction the *scene* supplies, because the crate does not know
a sun), obstacle (an `impl Fn(Vec3) -> f32` occupancy, so a caller with a `terrain::HeightField` passes
its sampler and a caller with a building passes its mesh's signed distance — the same move
`building::site::seat` makes by taking a function rather than a crate).

An `Envelope` is, on the superior's verdict, **a radius curve in the spine's parameter space**:

```rust
pub struct Envelope {
    pub spine: Curve,     // usually the trunk's own curve, so a leaning trunk's crown leans with it
    pub section: Section, // Circle { radius } — the coarse form; Outline — the refined form
}

impl Envelope {
    /// How far outside the envelope a point stands, measured in the envelope's own parameter space:
    /// `t` is the closest station on `spine`, and the answer is the distance to `section` at that `t`.
    pub fn outside(&self, point: Vec3) -> f32;
}
```

The coarse form is a `radius(t)` curve about the spine; **the refined form replaces the section with a
curve in the plane perpendicular to the spine at each `t`** — a `Section::Outline`, which is `shape`'s
D4 and the reason refining an envelope is a data change rather than a new type. Its law is per node and
not a search: a branch's length is scaled so that its tip's `outside()` is zero, and a tip the author
deliberately let past it is a **note** rather than a violation (see the validate section).

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

### D5: the expansion is a `shape` sweep, and the output is triangles only

The expander walks the spine's `stations`, asks `shape` for each one's frame, evaluates the node's
`Section` in that plane, and zips consecutive rings — which is `shape::Sweep::sample`, not code this
crate owns. The superior's verdict put the curve and its frame in `shape` (P0), and the reason is that a
branch's skin, a leaf's blade and the envelope are **one primitive at three sections**; a second
implementation here would be the copy the shared crate exists to prevent.

The two things this crate still owns are the **buckets** (a wood loft and a leaf mesh are separate meshes
because they carry different materials and different point schemas) and the **`(u, v)` values it writes
into its points**, which come from `shape::Loft`'s one definition — `u` the station's arc-length fraction,
`v` the fraction around the section. `mountain`'s `belt_uv` discipline, unchanged: one function, one
definition, one consumer (D8).

`Topology` has exactly `Triangles` and `Segments` (`SPEC.md`, `scene_schema.rs`, `emit.rs`), which is a
hard gate and the reason a lofted quad is two triangles. **Rejected:** emitting quads and depending on
`Topology::Quads`, which does not exist and would be a SPEC change. **Recorded as a later unification, not
assumed:** a leaf *template* could be a `patch::HermiteSurface` once its one-patch case is worth a
dependency — `patch::HermiteSurface` is on `main` and is exactly C1 inside a soft-edge component
(`across 0.000e+00` on every soft seam, measured by
[handoffs/hermite-surface.md](../handoffs/hermite-surface.md), whose own caveat is *"Not verified: that
any scene renders"*), and a leaf is one patch so its refusals about disagreeing neighbours do not arise.
P1 does not take that dependency; the midrib is already a `shape::Curve`, so the swap is a
representation change and not a design change.

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
plant has no such case at this phase; a vine that must read as one solid with a wall is deferred with the
rest of the building attachment.

The interpenetration is licensed by `building`'s own measurement: *"Touching is allowed: a
zero-volume meeting is not an overlap"*, and a courtyard is *"several wall solids and not one cleverer
element"*. A branch entering a trunk is the same fact with a non-zero overlap, and `validate()`'s
overlap judgement is written against **unauthored** overlaps only — an attachment authorizes its own
junction, the way `attach_mass` ends the overlap question for a wing.

### D7: wood is baked; leaves are instanced from a field (revision 2, on the superior's verdict)

The cost model, read from `main` rather than from a scene's comment:

- one pass draws **every row of one instance buffer**, and instances group by `(stage, shader, mesh,
  images)` (`emit.rs`, `Group` and `groups`) — **the material is not in the key**;
- the instance row is the model matrix, an optional normal matrix, and **the material's custom data**
  (`emit.rs`'s own sentence: *"once one pass draws several instances at once a per instance attribute is
  the only place either can go"*);
- so a placement carries a transform for free, and any *number* about a placement only through a
  material, which is a document entry verified and packed per instance;
- points are `Value::Packed`, and the compile cost scales with **bytes**: `monoliths` at 182k points
  compiles in 32 ms and renders in 10 ([compile-cost.md](compile-cost.md)).

**The correction, recorded rather than silently fixed.** Revision 1 of this file said leaves are baked
into the plant's mesh, and gave two reasons. The superior's verdict is that leaves are instanced, and
re-reading the measurement says the verdict is right and one of my two reasons was wrong:

- **Right, and now answered by bucket choice rather than by baking:** my objection was that pages are
  fitted to instance boxes, so a leaf-level instancing scheme multiplies the boxes the page pool sees.
  That is true of the **shadowing** bucket, and it is why leaves are not put in it. `tidewater`'s water
  makes the same move for the same class of reason (*"The water is in the `depthing` and `emission`
  buckets and not the `shadowing` one"*), and the schema already names the idiom for exactly this case:
  `shadow_density: 0` means *"the instance is drawn into the coarsest level only, which is what a distant
  hillside or a foliage billboard wants"*. So the page pool sees the wood, which is tens of boxes, and the
  leaves cost rows.
- **Wrong:** I claimed an instance per leaf would need "a material per leaf for any per-leaf number at
  all". A leaf's transform is **not** a per-instance number in that sense — it is the row itself — so
  instancing costs one row per leaf and no material at all. Only *identity and colour variation* need a
  material, and that granularity is a choice (D8), not a property of instancing.

So the split is:

| what | baked or instanced | why |
| --- | --- | --- |
| trunk, boughs, branches, twigs, culms, roots | **baked** into one mesh per material, with `(u, v)` and `authored_id` in the points | the wood is where a face needs provenance and where the counts are moderate (hundreds of nodes); it is also what casts the shadow |
| leaves, and ground cover | **instanced**: one leaf mesh, one instance per `LeafInstance` (V2) | the counts are in the tens of thousands; a row costs 64 bytes and a packed leaf would cost ~24 triangles of point data each |
| a whole plant in a grove | **instanced** at plant level | `building`'s D4 one scale up: the plan is baked, the vocabulary is instanced |

Measured consequences, from the same table: a plant with 320 twigs and 12 480 leaves uploads its wood
once (a few hundred KB of packed points) and one instance buffer of 12 480 rows (~800 KB at a 64-byte
row), drawn in **one pass** because the leaf mesh, the shader and the images are one group. The baked
alternative would have been ~100k triangles of leaf points in a second mesh — affordable, but three times
the bytes for the same pixels, and it would have made "fewer leaves above six metres" a rebuild of the
whole mesh rather than a curve edit on a field.

**Rejected:** baking the leaves (revision 1's answer, for the reason above); one leaf mesh per leaf field
(an extra mesh per field, for no pixel); and a per-placement material for every leaf (D8's boundary,
where the cost is stated rather than paid by default).

### D8: the identity channel, and an answer that is an edit

The mechanism exists and is landed: a scene declares `outputs: { authored_id: { location: 1, type: f32
} }`, the compiler generates the binding and adds it to every lighting pass, `f32` is `r32float` and
exact to 2^24 where an `rgba16float` is exact only to 2048, an identity is carried `@interpolate(flat)`
because a triangle is one facet of one element, and zero is reserved for *"not an authored element"*
([agent-content-identity.md](agent-content-identity.md), `gallery/src/building_identity.rs`). This crate
adds a third registration, not a mechanism, with three decisions of its own:

1. **An element is a node's surface, and a place on it is `(u, v)`.** For the wood, `Element::Skin { node
   }`, `Collar { node }`, `TipCap { node }`, `Decorator { decorator, face }` — so the table is ~nodes
   rather than ~faces, and *"where on the branch"* is answered by the continuous half of the channel.
   A doubled blade's two ribbons are **one** leaf, because an element is what an author names and an
   author names a leaf, not a ribbon; which side a pixel is on is a fact about `(u, v)`, not a second row.
   **`(u, v)` has one definition, owned by `shape`**: for a branch skin, `u` is the station's arc-length
   fraction and `v` the fraction around the section; for a leaf template, `u` is along its midrib and `v`
   across its section. `shape::Loft` returns them with the positions, and the crate writes what it
   returns — mountain's `belt_uv` rule, adopted because the alternative is a second table: *"A `u` that
   meant 'fraction along the belt's top path' in the crate and 'first barycentric coordinate' in the
   shader would make every answer off by a rotation of the triangle, silently, and only on faces where
   the two differ."*
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
3. **A leaf's id comes from the instance row, so its granularity is a choice with a stated cost.** A leaf
   is drawn as an instance of one template mesh (D7), and the only per-instance *number* a row can carry
   is its material's `custom_data` — so naming a leaf costs one **material set** (one per stage), and
   `Authored::Sum { name: "authored_id", flat: true, custom: AUTHORED_OFFSET }` adds the placement's base
   to the template's local id, exactly as `building` names five instanced shutters. Three granularities,
   and the author picks:

   | granularity | materials | what a pixel answers |
   | --- | --- | --- |
   | per placement | one per leaf | `leaf 4213 of twig 12` — the leaf itself |
   | per field group (the default) | one per node's leaf field, so ~370 for the report's oak | `a leaf of twig 12`, with `(u, v)` locating the point on the template |
   | unnumbered | none | the node the leaf hangs from, and no leaf row at all |

   All three name **one shader and one mesh**, so they are one *pass* by D7's key and cost only document
   entries. The third row is what a 200 000-leaf meadow uses; `report()` states the element count, so an
   agent knows which row it is in before it renders.

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

### D13: the report is the crown profile against the envelope, in the envelope's own space

`report()` answers what arrived, in numbers, and the headline number is a comparison:

> **the mesh's own reach at each station of the envelope's spine, measured against the envelope's own
> `radius(t)`, is this crate's one-number check that the geometry is what the plan said** — the sibling of
> `Massif::footprint_area` against `FacetMesh::plan_area`, and of `building`'s measured coverage against a
> footprint.

The superior's verdict is why the comparison is **exact rather than approximate**: the envelope is a
radius curve in the *curve's* parameter space (V4), so the measured reach and the asked-for radius are
both functions of the same `t`. Revision 1 measured the profile against height, which is a different
parameterisation — a leaning trunk, or an envelope whose spine is not the world's up axis, would have made
the two disagree for reasons that are not the plant's fault.

```text
vegetation report oak_starter            # the shape of the report, written before it exists
  generations   trunk 1 · bough 8 · branch 41 · twig 320 · leaf fields 370
  instances     12 480 leaf placements in 1 pass
  elements      1 479 numbered (370 skins, 369 collars, 370 tip caps, 370 leaf groups),
                ids 1..=1 479, zero never written
  wood          60 480 triangles · 181 440 points · 7.3 MB at a 40 byte stride
  leaf mesh     16 points · 8 triangles · 12 480 instance rows
  bounds        x [-3.10, 3.40] · y [0.00, 9.20] · z [-3.00, 3.20]
  crown         at t 0.66 (y 6.00 m): asked 2.10, measured 2.34 (+0.24) · worst +0.31 at t 0.71
  wood volume   0.4182 m³ · leaf area 214.6 m² · leaf area index 3.9
  junctions     369 measured, 4 outside the ratio band [1.8, 2.4]
```

Every line is a fact the crate has before a frame is drawn, and each answers a complaint an agent
actually makes: *the crown is too narrow at six metres* (the crown line, and the worst station named with
it), *it reads as a hedge* (the leaf field's band and density, which is a `report` line and not a guess),
*the trunk is too thin for its boughs* (the junction line), *the frame is too big* (the counts).
`wood volume` is `building`'s signed-volume argument at plant scale: a number no closure predicate can
fake.

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

### D17: wind is withdrawn, and so is every number it would have needed

The superior's verdict: *"先不管风"*. So this design has no wind law, no wind weight in the point schema,
and no wind group — and the last two are a consequence worth stating, because the repository's own rule
makes them wrong to keep: *"a field nothing reads is data waiting for a stage"*. A point schema carrying
a `wind` weight for a shader that does not exist is exactly that field.

What is *not* withdrawn is the finding, because it is about the framework rather than about wind: the
gallery's shader template splices a body into the *fragment* stage, and the `Authored` extras carry values
into a varying from the vertex stage but **cannot move a vertex**. [photographic-scenes.md](photographic-scenes.md)
item 1 already has that on record — *"a leaf that turns … wants the silhouette to move too, which is a
vertex-stage hook and a different decision"* — so the gap is owed by the framework, not by this crate, and
nothing in this design waits on it.

### D18: a leaf template is a thin closed solid, so two-sidedness is closure rather than a shading trick

The device draws both sides of every triangle whether one asks or not: the pipeline is built with
`PrimitiveState { topology, ..Default::default() }` (`device-rasterize/src/wgpu_renderer/raster.rs`), and
`PrimitiveState` derives `Default` with `cull_mode: Option<Face>` (`wgpu-types 30.0.1`, `src/render.rs`,
read 2026-10-17), so `None`. **So culling was never the problem; the normal is** — a leaf seen from behind
is lit by the normal of its front.

The superior's instancing verdict makes the better answer available, and it is not a shading trick. A leaf
template is `shape::Sweep` with `Section::Flat { half_width, thickness }` and `closed: true`, i.e. **a
thin closed solid**: two faces and a rim. Then:

- both sides are lit correctly by their own outward normals, with no fragment-stage special case and no
  per-instance flag;
- the leaf has a **positive signed volume**, so `shape::signed_volume` — the same check D6 uses for a
  branch — applies to it, and a template expanded inside out is a named violation;
- the silhouette has a rim, which a zero-thickness ribbon does not.

The thickness is a species relation and it can be as small as the frame tolerates (a 0.2 mm blade at a
nine-metre crown, which is nothing in a picture and everything in a normal).

**Rejected, and both are now unnecessary rather than merely worse:** `geometry::double_blade`'s doubled
ribbon (the same taper run twice with opposed normals *and* the winding reversed — its own doc records the
version that reversed only the normals and produced *"a leaf lit as though it were edge on"*) costs the
same triangles as a closed blade with no rim and no volume; and a `@builtin(front_facing)` flip in the
surface body would be one more place for a shader to be wrong. Whether `front_facing` even passes the
interface check is still unmeasured, and this design no longer needs to know.

## `validate()`: reports, never repairs — and it accrues rather than being designed

The superior's verdict: *"validate 看着办，需要的时候自然会产生这个需求"*. So this section does not design a
contract. What it does is fix the **discipline**, which is `mountain`'s measured shape and does not depend
on which checks exist:

- **all** violations returned in a deterministic order, so two runs diff;
- checks **staged**, so a downstream check is skipped when the document cannot support it;
- ops never refuse an edit except one that would leave a dangling handle, which refuses by returning
  `None`;
- **build failure is a list**, not a first error;
- geometry self-checks live on the mesh, not on the document;
- and the reporting vehicle during implementation is an `examples/` probe, not a test file.

A check is added when a probe or a scene needs one, and the trigger is concrete each time. The list below
is therefore a **candidate list** — every entry is a check some measured failure in this repository or in
the survey would have caught — and none of them is owed by P1 as a contract:

| stage | candidate check | the failure it would catch |
| --- | --- | --- |
| the document | `HandleOrphaned`; `EditOnAPrunedNode`; `GraftStationOutOfRange`; `AttachmentStationOutOfRange`; `SpineControlPointDuplicated`; `BladeOutlineNotSimple`; `DecoratorSocketUnresolved`; `PropertyNotComputed` | SpeedTree's silent loss of a node's edits when the node stops existing (D4); a leaf template that cannot be lofted; an offset on a property nothing computes |
| the walk | `BaseNotBuried`; `RingDegenerate`; `RingSelfIntersecting`; `SpineKinked`; `ZeroLengthBranch`; `LeafInsideABranch` (bounded); `BranchRatioOutside`; `TipOutsideTheEnvelope` | a branch floating off its parent's skin; a facet of no area (the square hip's own defect in `building`); a transported frame that flips; a leaf in a branch (D14); a bough too thin for what it carries (D19) |
| the mesh | `WindingInconsistent`; per branch: boundary loops not zero, not manifold, Euler not 2, **signed volume not positive**; per leaf template the same volume check (D18); `FaceWithoutAnElement`; `IdsNotContiguous` | an inside-out tube or blade, which no closure predicate can see; a frame whose numbers disagree with its table |

`TipOutsideTheEnvelope` is the one whose severity is the author's: an envelope is a plan and a branch may
deliberately exceed it, so it is a **note** unless the envelope is marked strict, on the policy
[adapter-limits-and-the-page-pool.md](adapter-limits-and-the-page-pool.md) settled — a note distinguishes
*a smaller frame* from *a wrong one*. And the crown profile's departure from the envelope is a `report()`
number and not a violation at all, because it is a measurement of what arrived rather than a claim the
document made (D13).

**Two of the candidates are new machinery** and are named as such rather than discovered later: **signed
volume** (which `mountain` has none of, which `building` had to write, and which now belongs to `shape`),
and **the transported frame's degeneracies** (a spine that turns back on itself flips a transported normal,
which is a kink that must be reported before it is a mesh).

### D19 (candidate): the branching ratio, if a check is ever asked for

The one candidate with a botanical content, kept here because the reasoning is worth not rediscovering:
a junction's radii obey `r_parent^n = Σ r_children^n`, with `n = 2` the relation attributed to Leonardo and
argued from a constant wind-stress criterion; measurements on real trees give a range rather than a number
([survey §2](../investigate/vegetation-modeling-techniques.md), whose [S20] and [S21] are the two — **neither
paper was read this session**, so the range is not established here). It is O(1) per junction, and it turns
an invisible error — a bough too thin for what it carries, which reads as a tree drawn rather than grown —
into a number. It is a **candidate** and not a decision: under the superior's verdict a check arrives when a
probe asks for it, and the natural moment is the first probe that reports a junction and finds the number
useless without a band.

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

`crates/vegetation`, publish `whiting-vegetation`, with `handle`, `rng`, `node`, `plant`, `expand` and `leaf`
landed in P1, and `decorator`, `edit`, `selector`, `derive`, `report` and `mesh` owed by P2 — small files for
sparse exploring, on the workspace's own rule.

**The design's names moved when the code arrived, and the mapping is worth one line each.** Its `document` is
`plant` (the root and the walk) plus `node` (a spine, its taper, its section, and the three numbers that
attach it); its `spine` is `node`'s spine field. **`handle` and `rng` are two modules the design did not
name**: a node's address and a node's numbers each turned out to be a type other modules need on its own —
`node`, `plant`, `expand` and `leaf` all ask for one or the other, and neither is a thing a document *is*. The
design's `field` stays P2's name for the placement fields (gravity, ancestor, light, obstacle); this crate's
leaf distribution field is `leaf::LeafField`, and the type names are what keep two different things that share
one word apart.

**Its only dependency is `shape`**, and that is the superior's verdict made structural: the curve, the
frame, the sweep, the `(u, v)` definition and the mesh predicates are all `shape`'s
([shape-primitives.md](shape-primitives.md)), so this crate authors plants and expands nothing it did not
have to. What it does not depend on: `patch` (D5's later unification), `terrain` (the ground is a function,
D12), and `graphics`/`scene` (a crate that emits geometry does not know what a frame is).

### The gallery

`vegetation_identity.rs` (the third sibling of `identity.rs` and `building_identity.rs`), `vegetation_mesh.rs`
(the conversion from `PlantMesh` to a `PackedList`, as `building_mesh.rs` is),
`vegetation_instancing.rs` (a `LeafInstance`'s frame as a scene instance's transform), and the scenes.

**The one place leaf instancing is not like `building`'s part instancing**, stated because it is where a
defect would hide: `instancing.rs` derives **one yaw** from a wall-mounted frame, because such a frame's
`up` is world up. A leaf's frame is not — its normal points out of the stem in any direction — so a leaf
instance needs **all three** of the transform's Euler angles, which the schema defines as *"rotation in
radians about x, then y, then z"*. The conversion is therefore one function with a stated order, and the
probe is `building`'s `instance_probe` precedent: the instanced placement's points against the baked
placement's points, **as point sets at 1e-4**, zero differing. An Euler triple written in the wrong order
is a leaf that is in the right place facing the wrong way, which no picture names.

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
output channel is landed (D8); per-instance transforms and per-material custom data are landed (D7); leaf
instancing needs no new mechanism at all (D7); images as scene data are landed, which is what a procedural
bark or leaf mask would use when a phase wants one. The one gap this design *touches* — a vertex-stage hook
— belongs to the framework and is already on record as missing in `photographic-scenes` item 1; with wind
withdrawn (D17) nothing here waits on it.

## Deliberately not proposed

- **Wind.** Withdrawn on the superior's verdict (D17), together with the per-point weights it would have
  needed. The framework-side gap it exposed stays on record.
- **Attaching a plant to a building.** Deferred on the superior's verdict (*"先不管 attach building"*), and
  recorded as deferred rather than dropped: the vocabulary exists on both sides (`Socket`/`Anchor`,
  `Station`) and the open question is only who owns the joint.
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
- **Fronds as a distinct generator.** A palm frond or a fern is a leaf template whose midrib curve is long
  and whose section is lobed — V2 with a different `shape::Curve`, and speed-controlled by the same field.
  SpeedTree needs a separate generator because its leaf mesh is an asset; this crate's leaf is authored, so
  the case does not arise.
- **L-systems, space colonization and stochastic botanical rules as the representation.** Admissible in
  exactly one role: producing default spines and default placements, guarantee-free and immediately
  editable (R3), never the mesh.
- **A DSL, a serialization format, an editor frontend, an undo/redo service.** `mountain`'s out-of-scope
  list, unchanged. `History` in the `definition` crate is the undo mechanism if the document phase (D2)
  ever lands, and the edit table is already in its shape.
- **A whole-plant boolean.** No `manifold` consolidation, on `building`'s D3 reasoning: a plant has no case
  where parts must behave as one solid, and a plant-on-a-wall is deferred above.

## Phasing

- **P0 — `shape`.** The curve, the frame, the sweep, the mesh predicates, and `math` moving behind a
  re-export. Detailed in [shape-primitives.md](shape-primitives.md) P0–P2, and **it comes first on the
  superior's verdict**: every spine, every leaf midrib and every envelope in this crate is that curve, so
  building this crate first would mean writing the curve twice.
- **P1 — one plant.** The document and the handle; spines and skins as `shape::Sweep`s with base and tip
  discs; the `LeafField` and its instance operator, with `derive::blade_taper` as the default section (the
  taper `geometry::blade` already computes as a formula, promoted to a starting point an agent then edits);
  `derive::Species` for `PINE`, `OAK` and `BAMBOO`; the edit table's six changes and their stated order;
  `report()` — the crown against the envelope's own radius curve, the counts, the instance count; identity
  for the wood (points) and for leaf groups (materials); and the checks a probe asks for on the way.
  **Exit: one rendered plant whose leaves are instances of one template drawn in one pass** — branch closed
  and outward, base buried, wood faces naming their elements, the measured crown within a stated tolerance
  of the envelope's radius curve, and the element and instance counts printed before the frame is drawn.
  The proof scene is the bamboo courtyard's subject — one culm with its leaves in the top third — because
  that is the plant three agent runs each hand-rolled, and because one culm exercises the station band, a
  ringed profile, a leaf field, and a leaf template in one picture.
- **P2 — the hand.** The selectors and `paint`; `bake` and the authored-spine path; the decorators with
  sockets; the fields (gravity, ancestor, light, obstacle) and the envelope as a trimming law; the
  collision report and `derive::thin`; the leaf normal laws (puffiness toward an anchor, world-up for
  grass); `--region`; the segmentation scene; and the promotion of
  `geometry::{culm, blade, double_blade, leaf_spray, tuft}` into `derive`, which retires five uncalled
  generators from the gallery.
- **P3 — the site.** A grove (instancing at plant level, with D7's identity granularity stated in the
  scene's own doc); ground cover (one tuft template and a field, which is the same mechanism as leaves);
  seating on a `terrain::HeightField` by the `site::seat` precedent; the offline backend's view of a plant
  (`cycles-mesh-normal-orientation.md` is the note to read first); and the `locate` registry the gallery is
  owed.

Out of scope, all phases: wind; attaching a plant to a building; LOD chains; cards and alpha maps;
seasons; growth timelines; photogrammetry; simulation of any kind (a plant's mechanics are a report, not a
solver); a DSL and serialization; an editor frontend; an undo/redo service.

## What P1 implemented (2026-10-17)

**The tree and its wood are on the branch and measured; the leaf field is the part still in flight.** The
crate landed `handle`, `rng`, `node`, `plant` and `expand`, with `tree_probe` (5 rows) and `plant_probe`
(5 rows). Every number below is a probe's output.

| invariant | reading | threshold |
| --- | --- | --- |
| a handle reaches the node it names | 7 of 7 expected names matched, worst spine length error **5.960e-8** | `< 1e-6` |
| a stale handle says where it stopped | `trunk/bough 1/branch 2` → *"trunk/bough 1 has no child 2"*, and a path two generations past the end names its own last resolved handle | named |
| a node's numbers are its seed and its path | the same 8 draws in a 2-bough and a 3-bough plant; a different path differs (**0.832899** against **0.222013**); mean of 10 000 draws **0.498087** | identical / `±0.01` |
| a child hangs on its parent's skin | origin against the composed surface frame **0.000e0**; radius against the parent's taper law **3.725e-9**; `x` unit and square to the tangent **0.000e0** | `< 1e-6` |
| a placed spine keeps its shape | length change **8.941e-8**, start against the placement's origin **0.000e0**, initial normal turned **0.000e0** | `< 1e-5`, `< 1e-6` |
| a plant's wood is closed solids | **10 of 10** at χ = 2 with a positive volume, the smallest **7.367e-6** | all |
| the taper is the law, not a cone | worst radius against the law **5.960e-8** over 33 rings; the law's node rise of 0.006 reads **0.005429** in the solid, where a straight cone would read **0.023600** against the law's **0.028000** at `t = 0.32` | `< 1e-6` |
| a joint is on the skin, sunk by what it asked | **9** joints, worst **1.132e-7** from *the parent's taper less the sink* | `< 1e-5` |
| one resolution dial | 1400 triangles at `Detail::DEFAULT` (6 × 12), 5720 at twice that (**4.086** times), every solid still closed, the same nodes in the same order | ratio in `3.5..4.5` |
| a field is one group per node it carries | **6** groups over 6 twigs, **24** leaves, one template, each group's count its own density law rounded | one group per node |
| a leaf is one transform | 24 leaves draw the template's **108** triangles once and carry **24** transforms; baked instead they would be **2592** triangles and **1344** vertices | stated, not paid |
| the count and the stations are the law | 4 leaves at 20 per unit over a 0.6 band of a 0.3-long twig, at 0.375 / 0.525 / 0.675 / 0.825, each half a spacing in from the band's start; a density that rises toward the tip gives 7 with gaps shrinking **0.1152 → 0.0633**, and 12 above the band's middle against 9 below | inside the band, increasing |
| a leaf's anchor is on its parent's skin | origin against the composed attachment **0.000e0**, radius against the parent's taper **5.588e-9**; control: 0.05 of station moves the radius **2.000e-3** | `< 1e-6` |
| the slots advance by the phyllotaxy | 9 consecutive pairs against **0.38196602** to **8.941e-8**; three nodes' first slots read **0.995155 / 0.984525 / 0.749440**, so the spirals do not line up | `< 1e-6` |
| pitch turns forward toward the parent's tip | at pitch 0, forward·outward **1.000000** and forward·tangent **0.000000**; at π/2, forward·tangent **1.000000** and forward·outward **−4.371e-8** | `< 1e-6` |
| twist turns the face about forward | side against the *untwisted face* **4.425e-8**, face against the *negated* untwisted side **4.284e-8**, forward unchanged **0.000e0** | `< 1e-6` |
| every instance frame is orthonormal | worst `\|axis\| − 1` **5.960e-8** and worst pairwise dot **7.078e-8**, over a twist of 0 and 1.1 rad | `< 1e-6` |
| the size law and its jitter | with jitter 0, every scale is the law exactly; with jitter 0.2 the widest departure is **0.194** of it and 11 of 12 differ; the same field twice is identical | exact / `≤ 0.2` |
| a blade is a closed solid | χ = 2, volume **0.006387** over 9 × 12; slot 0 at the midrib's first point **0.000e0**; base ring's `\|x\|` **0.000e0**; tip spread **0.000e0** | closed |

**Seven decisions P1 made that this design did not say**, each of which a consumer meets:

1. **The parent is the tree, not a field of the attachment.** The design wrote
   `Attachment { parent, station, slot, sink }`; a node's parent is the path that reaches it, so the type is
   three numbers and a handle cannot disagree with the tree it is in.
2. **A child's frame is stated once**: `x` outward from the parent's surface, `y` along the parent's tangent,
   `z = x × y`. `sink` therefore moves a joint **inward along `x`**, and a zero taper at the station leaves `x`
   undefined — the fallback is the section's own `+x`, stated in `Plant::surface` rather than discovered.
3. **The plant carries the orientation contract down.** `Plant::spine` sets the world spine's initial normal to
   the node's own first frame normal, moved by the placement, so a twig's section is oriented by the surface it
   hangs from — `shape`'s open question 2 applied by the *plant* rather than by each sweep, and measured as
   `0.000e0` of turn.
4. **A joint is not welded.** Every node is its own closed solid, sunk into its parent; there is no merge pass
   between a child's base and its parent's skin, and the measured reading is that the child's origin is on the
   parent's surface to `1.1e-7` of the sink. That is `building`'s socket rule (*a joint is one subtraction, not
   a transform*) one crate over.
5. **A template's frame is stated by its midrib's initial normal.** A blade's width must land on the template's
   `y` and its thickness on `z`, and `shape` puts a section's `x` half axis on the frame's **normal** — so the
   midrib's initial normal is `+y`, and a `+x` midrib's default (`−z`) stands the blade on its edge. That is a
   trap a leaf author meets once, so `LeafTemplate` states it in one place and the probe measures it: the
   blade's own width reaches `|y| 0.494932` while its face reaches `|z| 0.009924`, so a transposed section
   would swap two numbers that both look plausible.
6. **A leaf's per-instance numbers are a rigid frame and one uniform scale, and the bill for that is measured.**
   No per-instance bend and no per-instance shape, because either would need one material set per leaf; shape
   variety across leaves is a second template. What it buys: 24 leaves draw the template's **108** triangles
   once and carry **24** transforms, where baking them would be **2592** triangles and **1344** vertices. What
   it costs: a leaf cannot bend differently from its neighbours, and the design says so rather than leaving a
   reader to find out.
7. **A leaf's scale is applied in the template's own frame, not to the anchored point.** The world point is
   `placement.place(point · scale)`; the other order, `placement.place(point) · scale`, differs by
   `(scale − 1) · origin` and therefore drags a leaf toward the **root** in proportion to how small it is drawn —
   measured at **1.362 m** on this plant's topmost leaf, which is the kind of defect a picture cannot show
   because the blade stays the right size while sitting in the wrong place. `LeafInstance::scale` states the
   order and `plant_probe` measures both sides: the base ring lands on its anchor at **0.000e0**, and the wrong
   order moves it by the metre above.

## What SPEC would say if this is adopted

SPEC is human-editable only, so this proposal states the delta rather than writing it: a
`SPEC/vegetation.md` sibling in the shape of [SPEC/mountain.md](../SPEC/mountain.md) — motivation, R1..Rn,
out of scope — plus one line under the workspace description. The requirements it would carry are the ones
this design cannot violate without ceasing to be itself: a plant is authored as a spine tree whose
attachments are named rather than found (R4.1's sentence, one dimension up); manual detail is an offset on
a computed value or an authored spine, never an offset on a tessellation; derived defaults carry no
guarantee and are ordinary editable data the moment they exist; the output is planar-facet triangles; and
the crate is a pure rust crate with a rust api, no editor frontend and no runtime service of its own.
**No `Topology::Quads` is required** (D5) and no graphics or device change is required (D7, D8).

## Open questions

Closed by the second round of verdicts, recorded so they are not reopened:

- **The shared crate, its name and its contents** — decided: `shape` first, curve included, as
  [shape-primitives.md](shape-primitives.md).
- **The spine's basis** — decided: cubic Hermite, because the jet is the data and continuity is structural
  (D5, and `shape` D1).
- **The envelope's form** — decided: a radius curve in the curve's parameter space, refinable to a curve in
  the perpendicular plane (V4, `shape` D4).
- **Where the wind hook lands** — withdrawn with wind (D17).
- **Whether a plant may attach to a building** — deferred (not proposed, above).

Decided on 2026-10-17 by the chief engineer, under the superior's directive to choose by **strongest
expressiveness and the most elegant architecture**. Each answer states what it costs and what it rules out,
so a later session reopens it by argument rather than by forgetting it.

1. **The leaf field's orientation contract** — **one composition in one stated order, and the anchor is a
   place on the parent's surface rather than a *level*.** A placement names `(parent, station, slot)` — the
   same `Attachment` a branch uses — and resolves to a point on the parent's **swept surface**, evaluated
   with the section arithmetic the twig's own loft used, so a leaf sits on the bark and not in the pith, and
   a bough attaches through the same code. The frame is composed in this order and no other:

   > anchor point → the reference ancestor's frame → the slot's radial direction → `pitch` (a `Scalar` of the
   > parent's `t`: radians from the radial direction toward the parent's tangent) → `twist` (radians about the
   > leaf's own midrib) → `aim` (a world axis and a `Scalar` weight) → scale.

   Three consequences, each of which is the reason for the choice. **SpeedTree's five anchor levels become one
   integer** — `reference: u32`, how many generations up the frame is read from, `0` meaning the parent — so
   "Global/Deep/Intermediate/Local/None" is one number and not five cases; the survey's vocabulary is
   expressible and nothing is left out. **The phyllotaxy is the slot, not a separate azimuth**: the golden
   angle decides *which slot the n-th leaf takes*, so there is one degree of freedom with one home and a
   `density` change and a rotation change cannot disagree. **`pitch` is a curve of the parent's `t`**, so
   *"leaves point away from the trunk"* is a curve edit and not a placement-level fix — which is the sentence
   the open question asked for.

   P1 implements the anchor, the phyllotaxy, `pitch` **and `twist`** — the first four terms, measured in
   `leaf_probe`. **`reference` is the one term that waits, and it waits on `closest`**: reading the frame from
   an ancestor means knowing where the leaf's own anchor lands on *that* ancestor, which is a closest-point
   query and is already owed by the envelope's trimming law (shape's open question 4). `aim` is one more term
   in the same composition and arrives with the first crown whose leaves need to turn to the light.
2. **The identity granularity P1 ships** — **per field group**, with per-placement available for a field the
   author *names*, and `report()` printing the group count either way. The number at which a field must go
   unnumbered stays unmeasured on purpose: a document-entry budget is a fact about a frame's page pool, and a
   number invented before the first crown existed would be a budget no frame was ever measured against.
3. **Whether leaves cast shadow** — **yes, and `shadow_density` is the dial**, not a bucket exclusion. D7's
   "outside the `shadowing` bucket" becomes the *default setting* for a distant grove rather than a
   prohibition, because a close crown's self-shadowing is most of what makes a canopy read as deep, and a
   design that cannot express it has thrown away the thing it was built for. Which bucket a plant's leaves go
   in is a scene decision, and the documented middle (`shadow_density: 0`, the coarsest page level only) keeps
   the billboard case expressible.
4. **The section's own orientation** — **closed by `shape`'s answer** (its D4 and open question 2): the
   section's `x` is the station frame's normal, and the spine's initial normal is the one place an author says
   which way the first section faces. Vegetation therefore applies **one rule to every spine it builds** —
   the same initial normal for the twig and for the leaf template — which is what makes the contract
   structural rather than per-call discipline, and `shape`'s `sweep_probe` checks the primitive half of it.
5. **Whether the junction ratio earns a place in `report()`** — **no, and not yet.** A report entry has to be
   checkable, and the band in `r_parent^n = Σ r_children^n` is not botanically established: a number with an
   invented threshold would be the first entry a reader learns to ignore. It stays candidate D19, and it lands
   only if a plant that is visibly too thin at a junction shows that the ratio says so where no other reading
   does.
6. **`closest`'s accuracy** — **deferred with the obligation `shape` recorded**: when the envelope's trimming
   law needs it, the answer owes its sample budget, a measured worst-case error, and a stated answer at a
   knot. Nothing in P1 or P2 calls it.

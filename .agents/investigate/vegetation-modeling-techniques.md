# Vegetation modeling: what fits an agent-authored spine tree with hand-written detail

**Question.** Which vegetation modeling techniques fit a pipeline in which an agent *authors* a
hierarchical spine tree, *writes manual detail against stable handles*, and the system *expands* the
result directly into a triangle mesh — and which do not? The reference the superior named is
SpeedTree, so its architecture is surveyed first and in the most detail.

**Date.** 2026-10-17. Every SpeedTree page listed in the sources was fetched on that date; the
bibliographic records in [S11]–[S21] were confirmed from a listing on that date and **the papers
themselves were not read**, which is recorded per entry rather than left to be assumed.

**Judged against** [SPEC/mountain.md](../SPEC/mountain.md): hierarchy, direction and rhythm are
authored because noise cannot plan (R1); derived defaults carry no guarantee and are only an initial
state for continued editing (R3); the correspondence between control elements is AUTHORED, not
inferred, so expansion is a direct walk with no search and the correspondence is what makes the mesh
watertight (R4.1). Judged also against an instrument set the SPEC does not mention, because it is the
whole of this design's constraint: **the author has no cursor.** It reads pictures and text, writes
text, and asks questions of a render harness (`--locate x,y`, `--where <name>`, `--census`,
`--debug <channel>`). Every technique below is therefore also judged on *what its manual step looks
like when the hand is a sentence*.

Sibling survey: [procedural-architecture-techniques.md](procedural-architecture-techniques.md),
whose verdict this one largely reuses and whose sources it does not repeat.

## Verdict

SpeedTree's **architecture** transfers almost intact and its **interaction** does not transfer at all.

- What transfers: the generator/node split in which a node's manual value is an **offset on a
  computed value** rather than a replacement of it; the parent-curve-plus-parent-level mechanism for
  "value as a function of where a node sits in the plant"; the placement modes (interval, phyllotaxy,
  bifurcation, proportional, absolute, flood); the anchor-based normal laws that make a crown read as
  a volume rather than as a bag of leaves; and the decorator generators (cap, fin, knot, shell) which
  are the `building` crate's parts vocabulary under another name.
- What does not: every freehand tool is a **gesture** — a tablet stroke, a brush with soft selection,
  a gizmo drag — and an agent has none. Worse, SpeedTree's own documentation states that its deepest
  manual tool is **fragile**: "Vertex edits can be lost if you change the model's resolution; use this
  tool at the very end of your modeling workflow" [S9]. A design for an agent must invert that: the
  finest manual detail has to be the thing that *survives* a retessellation, because an agent changes
  resolution constantly and cannot see the loss except as a picture that stopped responding.
- What is rejected outright, for the same reasons the architecture survey rejected its analogues:
  rewriting grammars (L-systems) because applicability is a search; voxel and field methods because a
  grid is not an authored hierarchy; learned reconstruction and sketch prediction because they
  *infer* the correspondence R4.1 requires to be authored.

The one published system that already has this design's shape is **FloraForge** [S19]: an LLM refines
a human-readable plant descriptor, a deterministic generator expands it into hierarchical B-spline
surfaces with explicit control points, and those tessellate to meshes at arbitrary precision. It
arrives from plant phenotyping rather than from rendering, so it has no pixel→element join and no
offset-versus-bake distinction — the two halves this design has to invent.

## 1. SpeedTree Modeler — the reference, in its own words

### 1.1 The two editing modes, and the sentence that decides this design

> "The SpeedTree modeling approach is a blend of procedural modeling and manual editing. Vegetation
> models are typically too complex to model efficiently by hand, so the backbone of the modeling
> system is procedural. Once you have the model roughed out using procedural techniques, you can
> fine-tune it with a combination of curve editing and directly manipulating each part of the model."
> [S1]

Generators create and control groups of **nodes**; a node is one branch, frond, leaf, zone or proxy
[S2][S3]. Switching from Generation Mode to Node Edit Mode makes the property editor show the *node's*
properties instead of the generator's, and this is the load-bearing fact:

> "When I select a node, all of the property values are 0.0. Why? **Node edits are offsets to the
> value it was already given by the generator.** So, 0.0 means 'just use the generator value'. Any
> values that aren't 0.0 are added to the generator value before the node is computed." [S1]

> "Will I lose my edits if I go back and change the generator value? **No.** The edits will remain in
> place, even if you go back and edit the generator further. The only time you'll lose a node edit is
> if you reset it yourself or **the node no longer exists due to changes to the model**." [S1]

That second sentence's tail is the whole risk, and SpeedTree answers it with silence: a node that
stops existing takes its edits with it and nothing reports the loss. An agent writing text cannot
afford that, because the failure is invisible — the picture simply stops answering to the sentence
that used to change it. [S1] also records the discipline's own counsel, which this design keeps:
*"You're typically better off to model procedurally as long as you can and save the node edits for the
end."*

Three more mechanisms from [S3]:

- a node may also carry an **offset curve**, "added to (or subtracted from) the computed profile
  curve" — so a per-node *shape* edit is a curve, not a set of numbers;
- a node's local **X, Y, Z** are exposed separately from the radial/lengthwise placement, "which gives
  the artist another level of control over node placement";
- hierarchy navigation is by relation (parent, first child, previous/next sibling on a shared parent),
  which is a *text-shaped* interface already — an agent can name those relations without a cursor.

### 1.2 Where SpeedTree's manual tools stop being transferable

| tool | what it is | why it does not survive the translation |
| --- | --- | --- |
| Hand drawing [S4] | a freehand stroke becomes a **Bezier spline with control points**; spacing, curve-fit and control-point style (corner/linear/smooth) govern how many; a drawn spine can be resampled; **generation parameters do not apply to a hand-drawn branch**; forces and perturbance still do; any spine can be toggled to and from hand drawn | the stroke does not transfer, **the control points do**: dictating a control polygon is strictly more precise than drawing one. The toggle is the mechanism worth copying — a spine is either shaped by its generator or authored, and the document says which |
| Vertex editing [S9] | move one vertex or a soft-selected area; falloff curve shape; branch-versus-blend skin selection; **edits can be lost when the resolution changes** | the representation is the defect: an offset on a *mesh vertex* is an offset on a tessellation. The same edit written against `(station along the spine, slot around the ring)` moves with the tessellation instead of being destroyed by it |
| Bending, paint displacement, paint vertices, paint vertex colors [S10] | a brush over a region of the model | a brush is a *region plus a value*. The region an agent can name is a **predicate** (a generation, a handle's descendants, a station band, a computed-value test, a world box) or a **rectangle of a rendered frame**. The value transfers unchanged; the gesture becomes a selector |
| Click place [S7] | put a node where the user clicks, with a per-generator policy for receiving clicks | the click is a pixel. A harness that answers `--locate x,y` with an element *and a station on it* turns a pixel into `(handle, t, θ)`, which is a placement the document can hold |
| Photogrammetry [S10] | scanned trunks, mesh converter, stitch, feature vertices | nothing in this framework reads the filesystem, and there are no scanned assets. Excluded, not deferred |

### 1.3 The parameter surface, and what is worth naming

The branch generator [S5] is the densest object in the survey, and its groups are a checklist rather
than a wish list: **Generation** (§1.4), **Forces** (allow, per-force strength, a profile curve of
influence along the branch, plus *alignment* of the branch's own orientation with a force, in
individual or group style), **Random seeds**, **Segments** (an exact `Accuracy` for the high-resolution
spine — with the warning that *"changing accuracy may drastically affect the shape of your spines"* —
an `Influence` scale, a polygon `Scalar`, length and radial segment counts as `Absolute + Relative`
with three relative bases, and an `Optimization` that adaptively removes segments by curvature),
**Features** (feature vertices from a material's cutout meshes or painted by hand, with stitching
options to pad around them), **Spine** (length as `Absolute + % of parent + % of radius`, with the
explicit advice to *favor the relative terms* because they randomize and reuse better; orientation as
start angle/roll/alignment/roll/unify/weld-start; shape as gravity/ancestor/straightness/twist/parent
curl in three axes/zigzag; prune by interior/up/down/general/ground; a tiling deformation **path**;
**early and late noise** with amount/turbulence/threshold/smooth/seed; **jink** to straighten a spine
into linear segments; **break** with chance/spot/seed), **Shape control** (§1.5), **Skin** (type:
polygons, two subdivision-surface variants, or spine-only; visibility as a threshold that hides thin
branches; radius as `Absolute + % of parent` with clamp-to-parent, length compensation and a
length-kill ratio; light-seam reduction by bending normals toward the spine; roll; smooth; **welding**
with keep/force-ray-casts/offset/spread/blend; subdivision junction parameters; **squash** to make a
cross-section elliptical; **splits** with chance/balance/seed/spread/expand/smooth), **Extensions**
and **UV mapping** (three lock styles, U around the branch and V along it, fractional U, twist, three
V styles with an anchor end and skew correction, and 0-1 patch modes).

Two of those are worth arguing with rather than adopting:

- **Welding is a search.** "When checked, branches will be pushed out and fused to their parent,
  creating a seamless union" [S5], with a `Force ray casts` option for heavily deformed parents and a
  `Keep failed` flag to inspect welds that did not take. That is exactly the class R4.1 excludes: two
  surfaces meeting because an intersection test decided they should. A branch that instead **names the
  station and ring slot it hangs from** and is *sunk* into its parent's skin needs no test at all, and
  "the base is not buried" becomes a measurable violation.
- **`Accuracy` couples shape to tessellation.** A spine stored as sampled points changes shape when the
  sample count changes, which is the same defect class as the vertex edits of §1.2 and the reason
  SpeedTree tells the user to pick the maximum fidelity early and live with the compute cost. A spine
  stored as **control points with tangents** has no such coupling: the shape is the curve, and the
  station count is a dial.

The leaf mesh generator [S6] is the second dense object, and its groups are the leaf vocabulary:
size (or actual size), weld onto the parent's skin, cross, flip, fix winding, initial orientation;
a three-stage orientation pipeline (**local orientation** with a 3D anchor style, unification against
an anchor at one of five levels, sky influence, fold, align, face; **sensitivity** to a parent anchor
and to the sky; **final adjustments** as three rotations); **deformation** (height-map displacement,
fold, curl, twist, per-vertex noise with amount/noise/seed, per-axis scale); **lighting** (smooth,
*parent puffiness* toward an anchor at one of six anchor choices, puffiness that "sphere-izes" normals
around the mesh centre, unify, three normal-only rotations, and **world up** for grass); **wind**
(a weight whose profile curve runs from the pivot to the farthest vertex and whose left end "must be
zero to avoid disconnecting from the parent", a cross term that weights the edges more than the
centreline, a group, and lock-to-parent); **dynamic LOD** (survival weight, a scale factor by which
survivors grow into the vacated space, a mesh-index curve); **vertex color** per channel as `Set` or
`Offset from parent`; and **collision** (four participation styles, take-parent, ignore-parent,
also-remove-descendants, a weight by which the winner is chosen, and pivot and sphere thresholds).

Three of those are design inputs rather than features:

- **The anchor levels** (Global / Deep / Intermediate / Local / None) are how one point of the plant is
  named as a reference for orientation *and* for normals. That is a handle with a level of indirection,
  and it is the mechanism behind both "leaves point away from the trunk" and "a crown shades as a
  volume".
- **`Offset from parent` vertex color** is inheritance with a manual override — the same relation as a
  node's property offset, applied to a per-vertex value. A plant's colour variation is authored as a
  relation, not as a number per leaf.
- **Collision removes.** "When leaves collide, the one with the higher weight wins" [S6] and the loser
  is deleted, optionally taking its parent and its descendants. Under R3/R4.1 that is a *search* whose
  result silently rewrites the model. The transferable half is the report; the removal has to become an
  authored prune the document holds.

### 1.4 Placement modes, and the one that is a plan rather than a distribution

Ten generation modes [S7]: Classic (legacy), Proportional, Proportional steps ("commonly seen on pine
trees"), Absolute, Absolute steps, Phyllotaxy, Interval, Bifurcation ("places nodes on spots where the
parent bends, simulating a split in the branch … often the easiest way to give models an organic
look"), Flood (an exact number placed randomly), Parent (an exact number at the parent's origin).
Shared properties: `Active` by a resolution curve, `Pass` ordering with the constraint that a
generator's pass number cannot be lower than its ancestors', **Boundaries** (`First`, `Last`, `Clip`),
`Rotation`, `Position`, `Size scalar`, `Sink`, a **pruning curve** that removes nodes below 0.5,
`Extend parent` (eight extension types including Split 1/Split 2/Skinny/Fat/Breaks only/Bottom),
`Click place` policy, `Knockout` with a resolution term, **force containers** that include or exclude
nodes by whether they fall inside a geometry force, mesh alignment, and masks.

`First`/`Last` is the one the architecture survey had no analogue for, and it is a *plan*: "How does it
know to only put roots near the bottom? … 'Last' on the roots is a low value" [S1]. It is also the
cheapest possible answer to the bamboo fact an agent run wrote down by hand — *"A grove drawn with
leaves down its whole height is a hedge; bamboo carries its leaves on the top third of the culm"*
(`worktrees/dsf/crates/gallery/src/courtyard.rs`, its own doc comment) — because a station band on the
parent is one authored pair of numbers.

### 1.5 Shape control: the silhouette is a authored object

> "Style: None / **Sphere** — branches are constrained to a circular shape / **Enabled geometry
> forces** — branches are constrained to the shape of any active geometry forces. Position … 0.0 at the
> base of the spine, 1.0 at the tip; going over 1.0 pushes the sphere placement off the spine.
> Distribution style: Branches only / All descendants. Distribution lets you push and pull the relative
> lengths from one end of the sphere to the other. Size: Absolute + % of size" [S5]

This is the single most valuable tool in the survey for an agent, because the silhouette is what an
agent *looks at*: a crown is read as an outline, and an outline is the one thing a picture states
unambiguously. An envelope that trims descendant lengths is a per-node computation (each node asks what
the envelope is at its own height) and not a search, so it is admissible under R4.1 — and it makes the
crate's central report possible, because the envelope is what was *asked for* and the expanded mesh's
own radius-versus-height profile is what *arrived*.

### 1.6 Curves: five kinds, and one setting that makes them a hierarchy

[S8] distinguishes **parent curves** (a value distributed by how far along its parent each child sits,
with a **Parent Level** setting that chooses *which* ancestor the curve reads — and compound curves
over several levels at once), **profile curves** (applied the same way to every node of a generator,
running down the node's own length — the taper of a branch is one), **distribution curves** (the spread
of a fixed number of nodes), **pruning curves** (one per generator, removing nodes wholesale),
**resolution curves** and **LOD curves** (left end = highest LOD, right end = lowest).

The metaphor the documentation itself uses is the right one for an agent: *"You can generally think of
curves as a 'volume control' on your tree properties. The static value of the property … can be thought
of as the maximum volume."* A curve is a **named function of a station**, which is text an agent can
write; and Parent Level is the fact that the station may be measured on any ancestor, which is what
makes "smaller toward the top of the tree" and "smaller toward the end of the bough" two different
sentences rather than one ambiguous one.

### 1.7 What was not read this session

Named from the documentation's own index [S10] and **not** fetched, so nothing below is a claim about
their contents beyond the index's own one-line summaries and what §1.3/§1.6 quote from other pages:
`kcforces` and `force_object` (force kinds), `geometry_forces`, `kcgameslod` (LOD generation),
`windgames`/`windvfx` (the exported wind model), `atclusters` (clusters), `atsplits`, `toolcollision`
(the collision tool as opposed to the leaf generator's collision group), `toolshapecontrol`,
`kcrandomizex` (the variance model), `random-seeds-properties`, `kcvfxresolution`, `kcvfxgrowth`,
`kcseasons`, `vertex_colors`, `uv_tiling`, `manipulate_mesh_sections`, `ctbendingx`, `ctclickplacex`,
`ctpaintdisplacementx`, `ctpaintverticesx`, `ctpaintvertexcolorsx`, `frond_generator`,
`card_generator`, `decal_generator`, `knot_generator`, `fin_generator`, `cap_generator`,
`shell_generator`, `zone_generator`, `target_generator`, `batchedleaf_generator`, `tree_generator`.
The generator taxonomy itself **was** read, from [S2]: main (tree, branch, leaf mesh, batched leaf,
frond, zone, card), decorators (cap closes an open end, fin is a thin flap for fungus or moss, knot is
an extra piece of geometry attached to a branch such as a knot, lump or cavity, shell wraps a parent in
moss, algae or low-poly vines), and photogrammetry (mesh, mesh converter, stitch, target — "used to
precisely locate children off any parent").

## 2. The literature, judged the same way

Bibliographic records only; the papers were not read this session, and each entry says what that means
for its use here.

- **Weber & Penn, "Creation and rendering of realistic trees", SIGGRAPH 1995** [S11]. The stochastic
  botanical-rule model whose parameter set a commercial branch generator is recognisably a descendant
  of — insertion angle, tropism, length/radius ratios, bifurcation. **Fit: as `derive` laws.** Its
  parameters are exactly the numbers a `Species` should hold as relations; its *representation* is a
  finished artefact with no edit handle, which R3 forbids relying on.
- **Runions, Fuhrer, Prusinkiewicz & Lane, "Modeling trees with a space colonization algorithm",
  Eurographics Natural Phenomena 2007** [S12]. Growth is driven by attraction points: a node extends
  toward the points that see it, and points within a kill distance are consumed. It is the best-known
  way to make a crown *fill a shape*, which is what §1.5's envelope asks for. **Fit: as an R3 default
  generator only.** Each iteration is a nearest-attraction-point search over the whole point set, so it
  is inadmissible as the expander, and its output is a spine — which is precisely what a `derive` may
  write into the document as ordinary editable control points, guarantee-free.
- **Bloomenthal, "Modeling the mighty maple", SIGGRAPH 1985** [S13]. A polygonal loft along a spline
  skeleton with a branching topology, in a system that also introduced the idea of a *cross-section
  polygon* swept along a curve. **Fit: this is the expander.** It is the closest published analogue of
  a ring-loft walk and is cited here as prior art for the mechanism rather than as a competing
  representation.
- **Greene, "Voxel space automata", SIGGRAPH 1989** [S14]. **Rejected as a representation**: a voxel
  grid holds no hierarchy to name, so there is no handle and no authored correspondence. Admissible in
  one narrow role only — as an occupancy test a `derive` may consult, which is what SpeedTree's
  geometry forces and force containers already are (§1.4).
- **de Reffye, Edelin, Françon, Jaeger & Puech, "Plant models faithful to botanical structure and
  development", SIGGRAPH 1988** [S15]. The botanically-faithful axis (AMAP): a plant's architecture is
  a developmental sequence of internodes and phyllotactic positions. **Fit: the vocabulary.** It is
  where "internode", "phyllotaxy" and "axis of order *n*" come from, and the vocabulary is worth
  borrowing because it names the levels a generation hierarchy needs. Its physiological fidelity is out
  of scope.
- **Prusinkiewicz & Lindenmayer, *The Algorithmic Beauty of Plants*** [S16]. Rewriting grammars.
  **Rejected as the representation** for the reason the architecture survey gives for graph grammars:
  rule applicability is a search, and the derivation is re-run globally. The **phyllotaxis patterns** it
  catalogues (decussate, distichous, spiral with the golden angle) are the placement modes' vocabulary,
  and the gallery already hard-codes one of them — `vegetation::leaf::GOLDEN_ANGLE`, the leaf field's default phyllotaxy (it was `geometry::leaf_spray`'s, since retired)
  turn.
- **Oppenheimer, "Real time design and animation of fractal plants and trees", SIGGRAPH 1986** [S17].
  **Rejected**: a fractal is a finished artefact with no edit handle, and its self-similarity is the
  opposite of the per-node offset that makes detail authorable.
- **Eloy, "Leonardo's rule, self-similarity and wind-induced stresses in trees"** [S20] and **"Tree
  Branching: Leonardo da Vinci's Rule versus Biomechanical Models", PLOS ONE** [S21]. The claim worth
  carrying into a validator: a mother branch's cross-sectional area equals the sum of its daughters'
  (an exponent of 2 on radii), attributed to Leonardo, and [S20]'s argument is that it follows from a
  constant wind-stress criterion. [S21] tests the rule against biomechanical models on real
  measurements. **Neither was read this session**, so the exponent's measured range in real trees is
  *not established here* — but the *relation* is a number a crate can compute for a junction in O(1)
  and report, which makes it the vegetation analogue of `building`'s signed volume: a botanical fact
  that turns an invisible error (a branch too thin for what it carries) into a named violation.
- **Zhao et al., "DeepTreeSketch: Neural graph prediction for faithful 3D tree modeling from
  sketches", CHI 2024** [S18]. A sketch becomes a tree by learned graph prediction. **Rejected**, in
  the same class as ProxyBuild in the architecture survey: it infers the structure the design requires
  to be authored. It is worth naming for the interaction it has and this design cannot — a sketch is a
  cheap way to *discover* a silhouette. The text-side equivalent is the envelope (§1.5) plus a `derive`
  that fills it, and an agent iterating on a picture.
- **Hadadi, Jubery, Schnable, Singh, Benes, Krishnamurthy & Ganapathysubramanian, "FloraForge:
  LLM-assisted procedural generation of editable and analysis-ready 3D plant geometric models",
  arXiv:2512.11925, submitted 2025-12-11** [S19]. Abstract read in full. Its shape: domain experts
  iterate in natural language ("Plant Refinements"), an LLM refines Python that generates
  **hierarchical B-spline surfaces with explicit control points and parametric deformation functions**
  under botanical constraints, human-readable "Plant Descriptor" files carry the parameters, and the
  surfaces tessellate to triangle meshes at arbitrary precision, with a second output carrying
  parametric metadata for analysis. Demonstrated on maize, soybean and mung bean, fitted to empirical
  point clouds. **This is the closest published corroboration of the role split this design adopts** —
  the model refines a descriptor, a deterministic generator emits geometry — and its differences are
  exactly the two halves a rendering-side design must add: no pixel→element join (its consumer is an
  analysis pipeline, not a picture an author looks at), and no distinction between nudging a computed
  value and authoring a shape outright.

## 3. What the workspace already holds

Recorded so the design reuses rather than rediscovers (all read on 2026-10-17 against `main` at
`cc7aaa5`):

| asset | where | what it is for here |
| --- | --- | --- |
| five foliage generators with **no caller** | `crates/gallery/src/geometry.rs` — `culm`, `blade`, `double_blade`, `leaf_spray`, `tuft` | a bamboo culm's ringed profile, a tapered one-sided ribbon, its two-sided sibling, a golden-angle spray, a grass tuft. This is a form library in the wrong crate, in the same sense `built.rs` was for `building`, and `double_blade`'s own doc records the defect class a validator exists for: reversing only the normals left the second ribbon wound against them, "which is a leaf lit as though it were edge on" |
| trees as primitives | `crates/gallery/src/terraces.rs` — 110 China firs as **two cones**, broadleaf crowns as **a squashed sphere** | the demand side. [sphere-is-a-hemisphere-twice.md](sphere-is-a-hemisphere-twice.md) already measured that a crown is "not a reason to spend 2592 triangles on", and that as a ball it now hangs half into the cones it sits on |
| three agent runs that hand-rolled bamboo | `agent-scene-iteration-loop.md` §"Two process gaps"; `worktrees/dsf/crates/gallery/src/courtyard.rs` | 600 placements of two meshes, with the leaves confined to the culm's top third by hand. Its cost model is recorded there as *"a pass draws one instance, so one shader per material is one pass per material"* — **false against `main`'s emitter**, which groups by `(stage, shader, mesh, images)` and draws every row of one instance buffer in one pass (`crates/scene/src/standard/emit.rs`, `Group` and `groups`) |
| a curve/patch model | `crates/patch` — `HermitePatch`, `HermiteSurface` (C1 inside a soft-edge component), `RectNet`, `SimplexNet` | the argument for a **Hermite** spine: the jet is the data, so continuity is structural and there is no reconciliation pass. `HermitePatch::boundary` already states the Hermite↔Bezier identity for a curve |
| a polyline sampler and a station vocabulary | `crates/mountain` — `Sampling`, `Station`, `End::On`, `CornerNormals`, `math` | `Station` and `End::On` are the attachment vocabulary verbatim; `math` is the one definition of which axis is up, which is what `building`'s P1 found it actually needed from `mountain` |
| the identity channel, in both directions | `crates/gallery/src/{identity,building_identity,locate}.rs`, `--locate`/`--where` in `crates/device-rasterize/examples/render.rs` | a third registration, not a new mechanism: `authored_id` as `r32float`, zero reserved, ids contiguous from a first, one walk producing both the numbers and the table |
| the cost model | [compile-cost.md](../proposals/compile-cost.md) | `monoliths` at 182k points compiles in 32 ms and renders in 10; cost scales with **bytes**, because points are `Value::Packed`. A half-million-point plant is affordable as a still frame |
| the vertex-stage gap | [photographic-scenes.md](../proposals/photographic-scenes.md) item 1 | a normal may be displaced in the fragment stage, but *"a leaf that turns … wants the silhouette to move too, which is a vertex-stage hook and a different decision"*. Wind needs that hook; the gallery's `Authored` extras carry values into the fragment stage and cannot move a vertex |

## Comparison table

Legend: **P** planned/authored · **n** noise/sampled · **L** re-expands locally · **G** globally ·
**D** deterministic · **W** watertight by construction · **H** has a handle an agent can write.

| Family | planned vs noise | editability | determinism / solidity | Rust feasibility | R4.1 fit | agent fit (H) |
|---|---|---|---|---|---|---|
| SpeedTree generator + node offsets [S1][S3][S5][S6] | P: generators, curves, modes, anchors · n: variance, noise, knockout, weighted alternatives | **L**, per node, as an offset on a computed value; a node's edits survive generator edits | D given seeds; solidity is not its concern (it welds by search) | the vocabulary is a rewrite; no runtime is reusable (commercial, and the SDK is a renderer) | **High** for the offsets; **low** for welding and collision, both searches | **High.** An offset keyed by a handle is a sentence. Its *loss* of edits on a vanished node is the gap to close |
| SpeedTree freehand [S4][S9] | P: the control points a stroke leaves · n: the stroke | L for control points; **fragile** for vertex edits (lost on a resolution change) | D | n/a | **Medium**: control points are authored data; vertex offsets are not | **Low as a gesture, high as a representation.** Dictating control points beats drawing them; painting vertices does not survive |
| Weber & Penn stochastic botanical rules [S11] | P: the rule set · n: everything within it | G: a re-run is a new tree | D given a seed | trivial to rewrite | **Low**: correspondence is derived from the rules, not authored | **Medium** as `derive` only (R3) |
| L-systems / rewriting [S16] | P: the grammar · n: rule choice, stochastic variants | L in the rule tree, **G** for the shape | D | rewriting engine is a rewrite | **Low**: applicability is a search | **Medium** as a vocabulary of phyllotaxy; **low** as a representation |
| Space colonization [S12] | P: the attraction-point set (which *is* an authored envelope) · n: the growth order | L per node, but the point set is consumed as it grows | D given a seed | a few hundred lines | **Low as the expander**: nearest-point search per iteration | **High as `derive`**: an authored envelope filled by a search, then written down as control points |
| Voxel / field automata [S14] | n: almost all | none | D | trivial | **None**: a grid holds no hierarchy | **None**, except as an occupancy test |
| Polygonal loft along a spline [S13] | P: the skeleton and the cross-section | **L**, per branch | D; watertight if the rings carry their correspondence | a direct walk; no kernel | **Highest**: ring *i* to ring *i+1* is authored, exactly mountain's belt | **High**: a station and a ring slot name a place on any branch |
| Learned reconstruction / sketch prediction [S18] | n: all | none (a re-prediction is a new tree) | not reproducible in kind | model weights, out of scope | **None**: inferred correspondence | **Low**: the one thing it has is a stroke |
| LLM-assisted descriptor refinement [S19] | P: the descriptor, refined in natural language · n: whatever the descriptor asks for | **L**, and the descriptor is human-readable | D on the generator side | a separate concern; the model writes code against an API | **High**: the descriptor names its own elements | **Highest**: it is this design's own role split, published |
| Botanical junction laws [S20][S21] | P: the law | n/a | n/a | O(1) per junction | **High as a check**: it names a violation, it does not repair one | **High**: it turns an invisible error into a number |

## Recommendation

Build on four families, in this order, and refuse three.

**1. A polygonal loft along an authored spine — the expander.** [S13]'s mechanism, mountain's
invariant: a branch is a run of rings, ring *i* is paired with ring *i+1* by construction, and a
junction is authored as a station on the parent's spine plus a ring slot plus a sink rather than found
by an intersection test. Watertightness of *one branch* is structural; a plant is deliberately not
manifold as a whole, which is `building`'s two-level rule (each solid closed and outward, every joint
authored) applied to a tree.

**2. SpeedTree's generator/node-offset split — the authoring model.** A node's manual value is an
offset on the value its generator computed, so a generator edit does not destroy a hand edit, and a
node's *shape* edit is a curve offset rather than a set of numbers. The handle must be a **derivation
path** (generation plus slot at each level), never an index into an expanded list, and an edit whose
handle no longer resolves must be a **reported violation** — the hole [S1] leaves open.

**3. An authored envelope plus a measured crown profile — the planning tool.** [S5]'s shape control as
the authored ask, and the mesh's own radius-versus-height profile as the answer, compared the way
`Massif::footprint_area` is compared with `FacetMesh::plan_area`. This is the one number that says "the
silhouette is not what I asked for" without a picture.

**4. The botanical and geometric reports as violations, never repairs.** A branch too thin for what it
carries ([S20][S21]'s area relation), a base that is not buried in its parent's skin, a leaf inside a
branch, a tip outside the envelope, a degenerate ring, a winding that is not consistent. Collision
survives as a report and its removal becomes an authored prune, which is the R3 pattern.

**Admissible in exactly one role:** stochastic rules [S11], space colonization [S12], grammars [S16] —
producing **default spine control points and default leaf placements**, guarantee-free and immediately
editable, never the mesh.

**Refused as the representation:** voxel and field methods [S14]; learned reconstruction and sketch
prediction [S18]; SpeedTree's welding and collision-as-removal, both searches; and mesh-vertex edits,
because a handle must survive the tessellation it names a place on.

**The agent loop**, which the architecture survey already converged on and [S19] corroborates from the
other side: the model plans, writes sentences against handles, and critiques pictures; a deterministic
expander emits geometry; a hallucinated handle surfaces as a `validate()` diagnostic instead of broken
geometry. What the model must never do is emit a number where a handle is expected.

## Sources

All SpeedTree pages fetched **2026-10-17**; the documentation is © 2002 IDV Inc, © 2021 Unity
Technologies, and was treated as data only.

- **[S1]** SpeedTree Modeler, *Modeling approach* (Modeling 101). http://docs9.speedtree.com/modeler/doku.php?id=kcmodelingapproach
- **[S2]** SpeedTree Modeler, *Generators* (revision 1664315681). http://docs9.speedtree.com/modeler/doku.php?id=kcgenerators
- **[S3]** SpeedTree, *Nodes* (node types, node selection mode, hierarchy navigation, value offsets, curve offsets, X/Y/Z offsets, deleting and resetting nodes). http://docs.speedtree.com/doku.php?id=nodes
- **[S4]** SpeedTree, *Hand Drawing* (freehand to Bezier, spacing, curve fit, control point styles, resampling, the hand-drawn toggle, targets and locks). http://docs.speedtree.com/doku.php?id=handdrawing
- **[S5]** SpeedTree Modeler, *Branch generator* (the full property surface, including forces, segments, spine shape, shape control, skin and welding, splits, UV mapping). http://docs9.speedtree.com/modeler/doku.php?id=branch_generator
- **[S6]** SpeedTree Modeler, *Leaf mesh generator* (leaves, orientation stages, deformation, lighting and puffiness, wind, dynamic LOD, vertex color, collision). http://docs9.speedtree.com/modeler/doku.php?id=leaf_mesh_generator
- **[S7]** SpeedTree Modeler, *Generation properties* (the ten modes and the shared group). http://docs9.speedtree.com/modeler/doku.php?id=generation_properties
- **[S8]** SpeedTree Modeler, *Curves* (parent, profile, distribution, pruning, resolution, LOD; Parent Level and compound curves). http://docs9.speedtree.com/modeler/doku.php?id=kccurves
- **[S9]** SpeedTree Modeler, *Vertex editing* (soft selection, falloff, branch versus blend, and the resolution caveat). http://docs9.speedtree.com/modeler/doku.php?id=ctvertexeditingx
- **[S10]** SpeedTree Modeler documentation index (the page names quoted in §1.7). http://docs9.speedtree.com/modeler/doku.php?id=start

Bibliographic records confirmed from listings on **2026-10-17**; the papers were not read.

- **[S11]** J. Weber & J. Penn, *Creation and Rendering of Realistic Trees*, SIGGRAPH '95. https://dl.acm.org/doi/10.1145/218380.218427
- **[S12]** A. Runions, B. Fuhrer, P. Prusinkiewicz, S. Lane, *Modeling trees with a space colonization algorithm*, Eurographics Natural Phenomena 2007. https://diglib.eg.org/items/b5d756ee-0ab3-436e-a5cb-617d50df78fb · https://dlnext.acm.org/doi/10.5555/2381384.2381395
- **[S13]** J. Bloomenthal, *Modeling the Mighty Maple*, SIGGRAPH '85. https://dl.acm.org/doi/10.1145/325165.325249
- **[S14]** N. Greene, *Voxel Space Automata*, SIGGRAPH '89. https://dl.acm.org/doi/10.1145/74334.74351
- **[S15]** P. de Reffye et al., *Plant models faithful to botanical structure and development*, SIGGRAPH '88. https://dl.acm.org/doi/10.1145/378456.378505
- **[S16]** P. Prusinkiewicz & A. Lindenmayer, *The Algorithmic Beauty of Plants*. https://search.worldcat.org/title/1086557957
- **[S17]** P. E. Oppenheimer, *Real Time Design and Animation of Fractal Plants and Trees*, SIGGRAPH '86. https://cir.nii.ac.jp/crid/1573668926429655936
- **[S18]** *DeepTreeSketch: Neural Graph Prediction for Faithful 3D Tree Modeling from Sketches*, CHI 2024. https://dl.acm.org/doi/10.1145/3613904.3642125
- **[S19]** M. Hadadi et al., *FloraForge*, arXiv:2512.11925, submitted 2025-12-11 — **abstract read in full**. https://arxiv.org/abs/2512.11925
- **[S20]** S. Eloy, *Leonardo's rule, self-similarity and wind-induced stresses in trees*. https://arxiv.org/abs/1105.2591
- **[S21]** *Tree Branching: Leonardo da Vinci's Rule versus Biomechanical Models*, PLOS ONE. https://journals.plos.org/plosone/article?id=10.1371/journal.pone.0093535

### Not established this session (do not treat as fact)

- The contents of every page listed in §1.7, in particular the **exported wind model**, the **LOD
  generation** pipeline, the **cluster** mechanism, and the **collision tool** as distinct from the leaf
  generator's collision group. What is quoted about wind and LOD comes from [S6] and [S8] only.
- The measured range of the branching exponent in real trees, and whether [S21] supports or refutes an
  exponent of 2 for the species a `validate()` check would name. The *relation* is computable; the
  *tolerance* is not established here.
- Whether SpeedTree's node edits survive a change to a **sibling's** count in practice, as distinct from
  a change to the node's own generator. [S1] promises survival for generator edits and names only
  "the node no longer exists" as the loss case; the renumbering behaviour was not measured and cannot
  be, from documentation.
- Whether FloraForge's B-spline hierarchy carries a per-level offset (a true hierarchy) or one control
  net with parametric deformation functions on it. The abstract states the latter's vocabulary and the
  distinction decides how much of its descriptor design transfers.
- Any performance figure for SpeedTree's compute, and the `Batched leaf` generator's actual mechanism
  beyond its index summary ("forgoes individual node editing for orders-of-magnitude faster
  computation"), which is itself a statement that per-node editability has a compute cost.

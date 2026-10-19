# Ten trees of the Hengduan: hero-grade species for the `vegetation` crate

Status: design settled, implementation starting. Written 2026-10-18 against `main` at `e8c7b44`.

Related: [agentic-vegetation-generation.md](agentic-vegetation-generation.md) is the crate this
builds on and its P1..P3 phasing is assumed throughout;
[shape-primitives.md](shape-primitives.md) supplies `Curve`, `Scalar`, `Section` and `Sweep`;
[investigate/vegetation-modeling-techniques.md](../investigate/vegetation-modeling-techniques.md) is
the survey the crate's own decisions were cut from.

## 1. Motivation

**The `vegetation` crate has no tree in it.** Its four species are a 2.2 m bamboo culm, a 0.5 m
shrub, a crowded clump of the same and a 2 cm grass tuft
([`derive.rs`](../../crates/vegetation/src/derive.rs)). Its gallery scenes are *law demos*: every one
of [`vegetation_laws.rs`](../../crates/gallery/src/vegetation_laws.rs) draws the same plant twice so
that one force is the subject, and its own doc says so — *"a law is a claim about a difference"*.
That is the right thing for a crate to prove first, and it is not a picture anyone wants to look at.

So the work here has two halves that cannot be separated:

1. **Ten trees, each at hero grade.** Ten species chosen from the mountains of Southwest China — the
   Hengduan range and its valleys — because that flora is the most species-rich temperate flora on
   Earth and because it contains, within one day's drive, a dry-hot valley of kapok and a subalpine
   spire of fir. A system that can draw ten of *those* can draw ten of anything.
2. **The system improvements those ten force.** No capability is added on spec. Each one below is
   pulled out of the crate by a named species that cannot be drawn without it, which is how a
   content deadline does the work of a design review.

The second half is the point. A species list is a week of numbers; *"what does a procedural tree
system have to be able to say"* is the question, and ten hard species answer it faster than a
survey does.

## 2. The bar: hero, then epic

Both words are given an operational meaning here, because *"make it look good"* cannot be handed to
an agent or checked by a probe.

### 2.1 Hero grade — the specimen

A species is at **hero grade** when one individual, drawn alone in one frame, passes all five:

| # | test | how it is checked |
| --- | --- | --- |
| H1 | **Silhouette recognition.** The frame reduced to a black shape on white is the right species, not "a tree". | The `authored_id` mask is already written by every render. Cut the species out of it, lay ten silhouettes side by side, and ask a reader who has seen the reference photographs and not the models to name them. Fewer than seven of ten correct is a fail, and the answer says which species look alike. |
| H2 | **Species-specific foliage.** At a range where one leaf is 3–20 pixels across, the foliage reads as a needle fascicle, a sclerophyllous oak leaf, a fern pinna, a white bract — and not as a generic spray. | A close frame, one branch, at a stated camera distance, beside the reference photograph it was cut from. Judged by eye and recorded as a judgement, not a number. |
| H3 | **Crown depth.** The crown has a lit outside and a dark inside, so it reads as a volume. | Measured, not judged, by `audit/frame_stats.py --crown` over the plant's own pixels inside the crown's identity-mask box: the **interior share** — plant pixels wholly surrounded by plant — at least **0.25** on the whole-tree plate, and the interior's mean luminance over the plant's own lit p90 at most **0.80** measured on the **bough plate**. The second clause moved to the bough plate on 2026-10-19, measured: at whole-tree distance a dense crown's interior is made of *lit* leaves, because the shadow page's penumbra is coarser than a needle's gap, so more foliage raises the interior toward the rim (the oak measured 0.778 sparse and 0.824 dense); at bough distance self-shadowing resolves and the same crown measures 0.715. A species therefore shows its mass at the tree plate and its depth at the bough plate, and each plate says which it is being judged by. |
| H4 | **Wood reads as wood.** The trunk has a flare at the ground, a taper that is not a cone, and a bark whose form — plates, flutes, peeling sheets, persistent leaf bases — is in the geometry. | One frame at the base of the trunk, one at the crown, and the taper law written in the species' note so the flare can be read as numbers. |
| H5 | **No artefact on the list.** | Every frame is checked against the defect list in §2.3 and the check is recorded. |

A species that passes H1–H5 is a **hero asset**: it can stand alone in the foreground of a picture
at ten metres and hold the frame.

### 2.2 Epic grade — the picture

A hero species is at **epic grade** when the same plant, drawn in its own site, passes all four:

| # | test | how it is checked |
| --- | --- | --- |
| E1 | **The site is the species' own.** The ground slopes the way its belt slopes, the light is a light that falls there, and the air carries the valley. | The frame names its altitude, its belt and its hour in the sidecar, and the numbers agree with the species' note. |
| E2 | **Scale is legible.** Something in frame is a known size — a person, a rock, a companion of a known height — so a 30 m fir is not a 3 m fir photographed close. | The sidecar states the companion's height and the frame's camera height. |
| E3 | **The tree is one of many, and still the subject.** Companions at a distance, of the same and of other species, without the hero losing the frame. | Judged by eye; the frame's subject is stated before it is rendered and the judgement is recorded against that statement. |
| E4 | **It survives the two backends.** The rasterised frame and the exported path-traced frame are the same picture. | The exporter already writes the scene's environment as Cycles' background; the two frames are placed side by side and any disagreement is either fixed or written down as a known difference. |

Epic grade is a property of a *scene*, not of a species: one scene per species, and one scene of all
ten together on a slope.

### 2.3 The defect list

A frame at either grade must not contain any of these. The first four are recorded defects of this
repository already; the rest are what a tree adds.

- a leaf lit as though edge on (`double_blade`'s recorded fault — one material, one normal family);
- a crown hanging half into its own trunk (an attachment resolved to a wrong surface frame);
- a leaf that does not sit on the twig it hangs from (scaling applied after the placement rather
  than in the template's frame, which [`LeafInstance::scale`](../../crates/vegetation/src/leaf.rs)
  documents and pins);
- a bare broom (a leaf size chosen as if a template were one unit long when it is 0.31 m);
- **a polygonal silhouette**: a straight trunk whose six rings read as a hexagon against the sky;
- **a lollipop**: one trunk, one sphere of leaves, no branch architecture inside the crown;
- **a cauliflower**: branch architecture present but every generation the same, so the crown has no
  first-order boughs and no twig texture — the two scales a real crown reads at;
- **a dead give**: foliage distributed evenly along a branch to its very base and its very tip, which
  no real shoot does;
- **a felt-pen tree**: wood and leaf in one tint, or two species in one scene sharing a tint;
- **a floating tree**: the trunk meets the ground on a flat disc, with no flare and no litter;
- **a plastic crown**: every leaf the same size, the same pitch and the same twist, so the crown has
  a moiré instead of a texture.

### 2.4 What "improve the vegetation system" means, measurably

At the end of this work the crate must be able to say, without a special case per species, each of:
a needle fascicle; a compound leaf; a leaf that is not green; a trunk at thirty metres with a flare
and a plated section; a cone on a branch tip; a crown raised clear of the ground; a crown trimmed to
a shape; a plant on a slope; and ten of these in one frame with ten palettes. §4 turns each of those
into a change with a size.

## 3. The ten species

Chosen to be *hard in different directions*: each one is the species that forces one capability, and
the set covers the four belts a Southwest China mountain actually has.

| # | species | belt | what it forces |
| --- | --- | --- | --- |
| 1 | **云南松** *Pinus yunnanensis* | subtropical montane, 600–2600 m | needle **fascicles** (2–3 needles in one sheath), an **irregular open crown** that is not radially symmetric, plated reddish bark, a trunk that clears its own lower boughs, cones on branch tips. The dominant tree of the whole region, and the hardest. |
| 2 | **华山松** *Pinus armandii* | montane, 1200–3300 m | five needles a fascicle, a broad flat-topped mature crown that was conical when young — one rule, two ages. Proves the fascicle is a number and not a pine special case. |
| 3 | **丽江云杉** *Picea likiangensis* | subalpine, 2400–3800 m | a spire: needles single and four-angled on pegs, dense and short; **pendant** cones; a crown that keeps its lower branches to the ground. The opposite of 1 in every dial. |
| 4 | **苍山冷杉** *Abies delavayi* | subalpine, 2800–4100 m | flat soft needles in two ranks (a phyllotaxy of one half), **erect** cones, a narrower darker spire than 3. Proves spruce and fir are one rule with different numbers. |
| 5 | **川滇高山栎** *Quercus aquifolioides* | subalpine to treeline, 1500–4200 m | the characteristic sclerophyllous evergreen oak of the eastern Himalaya: a **gnarled contorted trunk**, small stiff spiny-margined leaves, a dense rounded crown, and a habit that runs from a 20 m tree to a krummholz shrub on one rule. |
| 6 | **珙桐** *Davidia involucrata* | montane relict, 1500–2500 m | **a leaf that is not green**: the two large white bracts that give the dove tree its name, hanging under a head of small purple flowers. Forces more than one leaf template and more than one tint in one plant — the payoff of §4.3 and §4.4. |
| 7 | **大树杜鹃** *Rhododendron protistum* var. *giganteum* | montane, 2000–3300 m | **a terminal leaf rosette** (leaves crowded at the shoot tip, not spread along it) and a **flower truss** seated at the same tip. A 25 m tree whose architecture is a candelabra of rosetted shoots. |
| 8 | **桫椤** *Alsophila spinulosa* | warm humid valley, 400–1600 m | **a compound leaf**: the 2–3 m pinnate frond, modelled as a rachis that is wood with pinnae that are leaves — which is the unification this crate wants, not a second leaf kind. A trunk of persistent leaf bases. |
| 9 | **木棉** *Bombax ceiba* | dry-hot valley, 500–1600 m | **buttress roots**, conical spines on the trunk, **whorled horizontal** first-order branches, a **palmately compound** leaf (five to seven leaflets from one point — the fascicle again), and a tree that flowers **leafless**, so one rule must be drawable at two seasons. |
| 10 | **筇竹** *Qiongzhuea tumidinoda* | subtropical montane, 1500–2500 m | the existing bamboo pushed from a law demo to a hero: its namesake **swollen ring-like nodes** in the taper law, several culms a clump, and the leaf cluster a bamboo actually makes. The control: it must reach the bar with the *fewest* new capabilities, which is how the ten say whether the additions were necessary. |

Second wave, once the bar holds: 滇油杉 *Keteleeria evelyniana* (erect cones), 云南铁杉 *Tsuga
dumosa*, 西南桦 *Betula alnoides* (peeling white bark, drooping catkins), 云南红豆杉 *Taxus
wallichiana*, 青冈 *Cyclobalanopsis glauca*, 沙棘 *Hippophae rhamnoides* (silver-leaved dry-valley
shrub), 鹅掌楸 *Liriodendron chinense*.

The per-species facts and the reference photographs each of these is cut from live in
[investigate/trees/](../investigate/trees/), one note per species, and the photographs in
`out/refs/<slug>/`. Every number in a species' implementation cites its note.

## 4. What the crate cannot say yet

Each item states the species that pulls it, the change, its size, and the alternative that was
rejected. Nothing here is speculative: each is a capability one of the ten needs and the crate does
not have, verified against the source at `e8c7b44`.

### 4.1 A cluster of leaves at one station — the fascicle

**Pulls it:** 1, 2, 9, 6, 7. A pine's needles come two, three or five to a sheath at one point; a
kapok's leaflets are five to seven at one point; a dove tree's bracts are two at one point; a
rhododendron's truss is many flowers at one point. [`LeafField`](../../crates/vegetation/src/leaf.rs)
spreads leaves along a band in a spiral and has no way to put *n* of them at one station.

**Change:** two fields on `LeafField`, both with the identity as their default:

```rust
/// How many leaves share one station, and the extra pitch they fan through.
pub cluster: u32,   // 1 is the identity
pub fan: Scalar,    // radians of pitch spread across the cluster; zero is the identity
```

`LeafField::groups` then makes `count / cluster` stations instead of `count`, and at each station
emits `cluster` leaves whose slot advances by `1 / cluster` of a turn and whose pitch is offset
across the `fan` range. **The draw order is stated and frozen**: the cluster's own leaves are drawn
inside one another, after the station's phase and before the next station, so a change to `cluster`
moves the leaves of that cluster and no other number — the same rule the existing three draws follow.

**Rejected:** a second leaf-distribution type beside `LeafField` (two ways to place a leaf is two
edit surfaces, and a species would have to know which one it is using); and modelling a fascicle as
a *node* with a leaf field on it (a pine twig carrying two hundred three-node fascicles would put
six hundred nodes in one plant against a
[`PLANTING_STRIDE`](../../crates/gallery/src/vegetation_picture.rs) of four thousand, and a sheaf is
not wood).

**Size:** about forty lines in `leaf.rs` plus the two fields, and the identity case leaves every
existing plant byte-identical. This is the single highest-value change in the list: five of the ten
species need it.

### 4.2 Resolution by arc length, not by count

**Pulls it:** 1–5, 9. [`Detail::DEFAULT`](../../crates/vegetation/src/expand.rs) is six rings along
every spine, whether the spine is a 2 cm grass blade or a 25 m trunk. Six rings along a 25 m trunk
is a 4 m facet, and a straight conifer trunk against a sky is a hexagon — the *polygonal silhouette*
defect, guaranteed.

**Change:** `Detail` gains a third reading and `expand` chooses per node:

```rust
/// The longest ring spacing a spine may have, in the plant's units.
pub spacing: f32,
```

Rings for one node become `max(detail.stations, ceil(arc_length / spacing))`. A grass blade keeps six
because six is already finer than its spacing allows; a trunk gets what its length asks for. The
count stays the author's floor and the spacing becomes the law, which is the direction the rest of
the crate goes — *"a law, not a number per thing"*.

**Rejected:** a per-node `Detail` (a second authoring of the same fact, and a plant's nodes would
carry a number that belongs to the picture); and simply raising `stations` for the whole plant, which
multiplies the cost of every twig by the trunk's need.

**Size:** about ten lines, one field, and a change to every plant's triangle count — so the existing
pinned hashes move. That is accepted and stated: the change is recorded in the handoff, and the
existing scenes are re-pinned with an explanation rather than quietly.

### 4.3 Every leaf template drawn, not the first

**Pulls it:** 6 (a green leaf and a white bract), 7 (a leaf and a petal), 9 (a leaflet and a red
flower), 8 (two pinna ranks if wanted).

**Defect, verified:** [`plant_scene_with`](../../crates/gallery/src/vegetation_picture.rs) takes
`planting.species.templates.first()` and, when a planting has more than one template, silently drops
every template after the first. `LeafGroup` already carries the `template` index it wants, so the
mesh is chosen correctly for the group that asks — the picture just never built the other meshes. A
species with two leaf kinds therefore renders one of them and reports no error, which is the worst
shape a defect can have here.

**Change:** collect one mesh per distinct template across all plantings (the existing `blades`
deduplication already does this; only the `.first()` that feeds it is wrong), and index the leaf
loops by `group.template`.

**Rejected:** nothing. This is a fix, not a design.

**Size:** about thirty lines in `vegetation_picture.rs`.

### 4.4 A palette per planting, and per template

**Pulls it:** all ten. Today `CULM_TINT` and `LEAF_TINT` are module constants of
`vegetation_picture.rs`, so every plant in the gallery is the same two greens — a pine and a birch
cannot be in one frame. The scene's own comment already knows this is a limitation: *"one tint for
both makes a bamboo look like a tree drawn in felt pen"*.

**Change:** a `Palette` that travels with the planting:

```rust
pub struct Palette {
    pub wood: Surface,          // tint and roughness of every node's loft
    pub leaves: Vec<Surface>,   // one per leaf template, in template order
    pub seats: Vec<Surface>,    // one per decorator template, in template order
}
```

with `Palette::default()` being today's two constants, so every existing scene is unchanged and a
species that does not care says nothing. `Planting` gains `pub palette: Palette` and a
`with_palette` builder beside its existing `at` and `with_seats`.

**Rejected:** a palette on the `Species` in the `vegetation` crate — a colour is a scene's fact, not
a plant's, and the crate's own rule is that a light direction belongs to a scene
([`species.rs`](../../crates/vegetation/src/species.rs) says exactly this of `Field`).

**Size:** about sixty lines, and it touches every existing call site of `plant_scene` through the
default.

### 4.5 Reproductive structures seated by a rule

**Pulls it:** 1, 2, 3, 4 (cones), 6 (the bract head), 7 (the truss), 9 (flowers), 10 (nothing —
which is the control).

The parts exist: [`decorator::Template`](../../crates/vegetation/src/decorator.rs) is a small sweep,
and `Seat` places it at `(node, template, station, slot, sink, scale)`. What is missing is a *rule*
that seats them: today every seat is written by hand, and a fir with forty cones is forty lines.

**Change:** one function beside `decorator::placed`:

```rust
/// A seat for every live node of one generation, at a station along it.
pub fn scatter(
    plant: &Plant,
    generation: usize,
    template: usize,
    station: f32,
    count: (u32, u32),
) -> Vec<Seat>
```

`station = 1.0` is a cone at a branch tip and a truss at a shoot apex — which is where both actually
are — and `count` with a jitter drawn from the node's own stream is how forty cones on one fir differ
from forty on another. `count = (0, 0)` on a node is the absence the seed decides, so a fir does not
fruit on every branch.

**Rejected:** a `DecoratorField` mirroring `LeafField` (a decorator is sparse and named by a
generation, not dense along a band, so the density law would always be a constant and the phyllotaxy
would always be a slot — two dials that say nothing).

**Size:** about thirty lines.

### 4.6 The sky, the lights and the ground as arguments

**Pulls it:** every epic-grade frame. [`atmosphere`](../../crates/gallery/src/vegetation_picture.rs),
`KEY_POSITION`, `KEY_COLOUR`, `KEY_IRRADIANCE`, `FILL_POSITION`, `FILL_COLOUR` and `FLOOR_TINT` are
all module constants, so a vegetation scene can be drawn at one hour, under one sky, over one ground
colour, for ever. A kapok in a dry-hot valley at noon and a fir on a misty ridge at dawn are the same
light.

`Ground` is already a law — `Ground::Fielded { height, divisions }` takes a sampler, so a slope needs
no new type — but nothing can reach it except through the `Framing`'s own field, and the lights and
the sky cannot be reached at all.

**Change:** a `Lighting` struct carrying the key, the fill, the floor tint and the atmosphere, with
`Lighting::default()` being today's constants, and `plant_scene` taking it as an argument beside the
framing. A species' epic frame then states its own hour and its own air as numbers, which is what E1
asks for.

**Rejected:** folding the lights into `Framing`, whose doc says it carries the camera, the frame and
the floor's reach — three things a picture of plants differs in. A light is not a camera, and one
struct that holds both stops being either.

**Size:** about fifty lines, and it moves the existing scenes' constants into a default.

### 4.8 A whorl of branches — `Generation::cluster`

**Pulls it:** 1, 2, 3, 4, 9. Every conifer in the list branches in whorls: a season's leader bears one
ring of boughs at its top, and the rings stack into the tiers a pine or a fir is recognised by. A
`Generation` spread `count` children evenly over a station band, which is a spiral, and a band of one
point, which is one whorl — but a trunk is one node, so one generation could make one whorl and a
crown of five tiers was five generations of one child each, which is not a rule.

**Change:** `Generation` gains `cluster: u32`, the branch-side twin of §4.1's leaf `cluster` and the
same word for the same fact — how many siblings share one station. `count` children gather at
`count / cluster` stations spread over the band; the children of one station stand `1 / cluster` of a
turn apart and each next station advances by the slot step. `cluster = 1` is the spread the rule has
always made, exactly.

**Rejected:** a second generation list that repeats along a leader chain (a rule that recurses is a
generator, which is the thing this crate refuses); and quantising stations with an edit pass after
`grow` (a tier is a fact about the crown, but writing it as forty hand edits per tree makes the crown
uneditable as a crown).

**Size:** one field and four lines in `grow`.

### 4.9 A shape function sees where its child hangs — `Joint::station`

**Pulls it:** 1, 3, 4, 5. A conifer's lower bough is twice the length of its upper one and an oak's
profile is the other way about; both are laws over the station the child hangs at. `Joint` carried the
parent's radius, the depth and the sibling rank, so a shape function could not tell a bough at the
crown's base from one at its top and every crown came out a cylinder.

**Change:** `Joint` gains `station: f32`, the child's own station on its parent, which `grow` already
computes. A shape function that ignores it is unchanged.

**Size:** one field, two construction sites.

### 4.10 A crown is a count, and the count is large — recorded, not yet closed

**Found by:** the first render of 云南松 (2026-10-18). The frame drew a skeleton: five correct whorls,
correct bough angles, and a crown with no mass. The arithmetic is the whole lesson. A sixteen-metre
pine's crown presents about fifty square metres to a camera; a needle of the note's 22 cm × 1.1 mm
covers 1.2e-4 m² of it, so a crown that reads as foliage wants on the order of **two hundred thousand
needles**, against the five thousand the first rule made. No amount of art direction closes a factor
of forty; only the counts do, and the counts run into two limits this crate currently states:

- [`PLANTING_STRIDE`](../../crates/gallery/src/vegetation_picture.rs) is 4096, so a planting may have
  four thousand nodes — and a pine that reads wants on the order of ten thousand twigs. The stride is
  one constant and the ids are `f32`, so 65536 is exactly representable; raising it is a one-line
  change with a stated reason, deferred until a species actually needs it.
- The rasteriser samples a pixel at one point, so a blade narrower than a pixel is dropped rather than
  faded: at twenty-five metres a 1.1 mm needle is a tenth of a pixel. Supersampling the frame and
  box-filtering it down is the contained fix and would serve every scene in the gallery, not only the
  trees; it is deferred to arc 7, where the epic frames want it anyway, and until then a whole-tree
  frame is read for its silhouette and the foliage is judged at the bough plate's distance, where a
  needle is a pixel and a half wide and honest.

The first pine therefore lands at the largest count the current stride and the current render time
allow, and the note's per-twig fascicle density is honoured while the twig count is not. That gap is
written into the species' verdict rather than hidden: **arc 2's pine is a pine at the density the
system allows, and closing the density is arc 7's first job.**

### 4.11 Foliage must shadow foliage — the shadow bias is a crown-killer

**Pulls it:** H3 for every species, and the proposal's own D18 sentence (*"a close crown's
self-shadowing is most of what makes a canopy read as deep"*).

**Found, measured, 2026-10-19** ([investigate/leaf-self-shadowing.md](../investigate/leaf-self-shadowing.md)):
the leaf rows are in the shadowing bucket and are rasterised into the shadow page, but a `Point`
light's pool test displaces the receiver one page texel along its normal — 172 mm at the pine's
`key_extent` of 40 — and the depth bias adds 48–288 mm on top. The dead zone is 17–29 cm against a
blade a millimetre thick and a centimetre from its neighbour, so a caster in front of a leaf can
never occlude it; and the sky term is added with no shadow lookup at all. The crown's interior is
therefore exactly as bright as its rim, which is the number H3 catches (0.99 against a bar of 0.75).

**Change:** a plant-only custom-schema `shadow_bias_scale`, defaulting to one so that every existing
material is untouched, multiplying both the push and the bias, set to about 0.05 on plant materials;
and the pine's `key_extent` down to about eight so a page texel is millimetres. No decision record
forbids it and one requires it; `clustered_lights` walks `Gallery::names()`, which holds no vegetation
scene, so no pinned pass count moves.

**Size:** a shader uniform read, one schema field kept out of the shared building schema, and one
number in the pine scene.

### 4.12 A spray: the conifer crown's level of detail

**Pulls it:** 1, 2, 3, 4 — every conifer — and the emitter's per-instance cost.

The whole-tree plate cannot draw two hundred thousand true needles: the scene document pays per
instance, and 86 000 instances already cost forty seconds of scene compile against three of
raster. And a true needle at twenty-five metres is a tenth of a pixel, which a point-sampled
rasteriser drops whole. Widening the needle into a card buys coverage but reads as a strap at arm's
length.

**Change, content rather than crate:** `Foliage::Spray` — one blade whose width law is a **comb of
needle-teeth** on a plume envelope, forty-five centimetres long, instanced where a fascicle-plume
sits. Its teeth are one to two pixels at the whole-tree plate's distance, which is the grain the
reference photographs show at the same distance, and its area per instance lets eighteen thousand of
them cover a crown that two hundred thousand needles could not afford. The bough plate keeps
`Foliage::Needle`, the true 1.1 mm blade. Measured: crown coverage 0.29 → 0.42 and the H3 interior
share 0.33 → 0.48 between the needle crown and the spray crown at the same frame.

**Rejected:** supersampling the frame (helps edges, not coverage, and multiplies the fill rate that
is already the wall); alpha-textured billboards (no texture pipeline in this renderer, and a
billboard has no normal for the two-sided shading D18 bought); and accepting a sparse crown (H3
exists precisely to refuse it).

### 4.7 Already expressible — verified, and not to be rebuilt

Recording these matters as much as the gaps, because each is a capability a species brief could waste
a week reinventing:

| need | how the crate already says it |
| --- | --- |
| a crown raised clear of the ground, a trunk that sheds its lower boughs | `paint(&plant, &Selector::StationBand { from, to }, Change::Prune { reason: Prune::Dead })` — `Prune` already has `ByAuthor`, `Thinned` and `Dead` ([`node.rs`](../../crates/vegetation/src/node.rs)) |
| a plated, fluted or peeling bark | `Node::with_section(Section)` — a node's cross-section is not required to be a circle, so a twelve-sided jittered section is a pine's plates and a smooth one is a birch |
| a gnarled, contorted trunk | `Change::Spine { jets }` on the root handle, which is what `arching()` already does to a culm |
| a root flare | a taper law with knots at 0.0 and 0.05 — a `Scalar`, no new type |
| a crown of a given shape | `envelope::trim` with `Envelope::radial` and `Outline::of`, which `panelled()` already draws three ways |
| a compound leaf | a frond is a **node** whose wood is the rachis, with pinnae as a `LeafField` on that generation. No new leaf kind. §5 arc 5 tests this. |
| wind, gravity, a light a plant grows toward, a wall it grows around | `Field::Tropism` (of which `gravity` and `light` are the two directions), `Field::Ancestor`, `Field::Obstacle` |
| a plant on a slope | `Ground::Fielded { height, divisions }` |
| leaves aimed at the sky so a crown shades as a volume | `LeafField::aim_axis` and `LeafField::aim`, a `Scalar` weight |
| one plant, two leaf layouts | the seed, which reaches the leaf field and nothing else |

## 5. Phasing

Eight arcs. Each one ends with a rendered frame that is looked at, and a note that says whether the
bar was met. An arc that cannot be rendered is not finished.

| arc | what it does | species | system change |
| --- | --- | --- | --- |
| **1** | the picture can carry a tree | none new; the existing scenes must come out unchanged or with an explained change | §4.3, §4.4, §4.6 |
| **2** | one conifer at hero grade, and the bar is written against it | 1 云南松 | §4.1, §4.2, §4.5 |
| **3** | the spire conifers, to prove the pine was numbers and not a special case | 3 丽江云杉, 4 苍山冷杉, 2 华山松 | none intended; anything needed here is a defect in arc 2's design |
| **4** | the evergreen oak, and the gnarled-trunk habit | 5 川滇高山栎 | none intended |
| **5** | the flagships, where the leaf stops being green and stops being simple | 6 珙桐, 7 大树杜鹃, 8 桫椤 | payoff of §4.3, §4.4, §4.5 |
| **6** | the dry valley and the bamboo control | 9 木棉, 10 筇竹 | buttress roots if §4.7's flare is not enough |
| **7** | epic grade: all ten on one slope, in their belts | all ten | the slope, the mist, the hour — §4.6's payoff |
| **8** | the second wave, only if arcs 3–6 needed no system change | the seven of §3's tail | none |

Arc 2 is the one that decides everything: 云南松 needs the fascicle, the resolution law, the cones and
the raised crown at once, and if the bar can be met on it, the other nine are numbers.

## 6. Decisions

### D1: ten species, not one, and they are chosen to be hard in different directions

Rejected: (a) one hero tree done very well — it would prove the crate can draw *that* tree and
nothing else, and the whole claim here is about generality; (b) many easy species — a system that can
draw ten birches has learned one shape ten times. The ten are chosen so that each is the species that
forces exactly one capability, and 筇竹 is included as the *control*: it should reach the bar with the
fewest additions, and if it does not, the additions were not the necessary ones.

### D2: a fascicle is a leaf-field dial, not a node and not a second distribution type

See §4.1. The reason to record it: the tempting design is a `Cluster` node with leaves on it, because
that is how a plant actually grows one, and it fails on cost — six hundred extra nodes a plant against
a stride of four thousand — and on the crate's own rule that wood is what a `Plant` owns.

### D3: resolution becomes a spacing law, and the existing pinned hashes move

See §4.2. Recorded because it is the one change here that deliberately breaks a pin. The alternative
— raising `stations` globally — was rejected as multiplying every twig's cost by the trunk's need.
The re-pinning is written in the handoff with the before and after numbers, not done silently.

### D4: colour is a scene's fact and travels with the planting

See §4.4. The crate stays colour-free, which keeps its rule that a light direction belongs to a scene.

### D5: a reproductive structure is a decorator seated by a rule, never a leaf

See §4.5. A cone is a solid and a bract is a blade, and the crate already has one type for each;
the only thing missing was the rule that seats them. The exception is 珙桐's bracts, which are
*leaves in every sense that matters here* — flat, thin, lit from both sides — and are therefore a
second `LeafTemplate` with a white tint, not a decorator. That is a judgement and it is stated so
that it can be argued with.

### D6: hero and epic are two grades of two different things

A species is a hero; a scene is an epic. Conflating them would mean a species cannot be finished
before its site exists, and the ten sites are one arc (§5's arc 7) precisely so that the ten
specimens are done first.

### D7: a whorl is a cluster of branches, and a joint sees its station

§4.8 and §4.9. Recorded together because the pine pulled both in one reading of one photograph: the
tiers are a count of siblings per station and the crown profile is a law over the station, and
neither was expressible. The rejected alternative for the tiers — a leader chain of one-child
generations — is recorded in §4.8 because it is the design an L-system background reaches for first,
and it turns a rule into a generator.

### D8: a species has two foliages, and each plate says which one it drew

The rasteriser samples a pixel at one point, so a blade narrower than a pixel is dropped, not faded
(§4.10). The note's 1.1 mm needle is a tenth of a pixel in a whole-tree frame. The alternative —
widening the one blade until the whole-tree frame is dense — makes the bough plate a lie, and the
alternative of supersampling the frame is a renderer change that belongs to arc 7's epic frames, not
to a species. So a species carries a true blade and a card whose half-width is half a pixel at the
distance its whole-tree plate is framed from, **at the same density as the true blade**: the card's
extra area is what pays the sampling tax, and a card made sparser by the width ratio — the first
reading, which the first card frame refuted — leaves the crown exactly as bald as the needle's,
because the tax is on width and the count was never the whole debt.
**Each plate is judged at its own distance and neither is asked to
serve the other's**, which is what a level of detail is.

### D9: the bough plate draws a bough, not the tree

A camera inside the whole tree's needles shades every one of them: the first bough plate, framed
inside a quarter of a million instances, ran ten minutes and was killed. The plate therefore grows a
species whose trunk *is* a bough — one bough, its twigs, the true needle — which draws in seconds and
shows exactly what the foliage test asks. The cost is that the bough plate's ids are the bough's own
walk, so it carries its own resolver; that is stated in its `Naming` row rather than hidden.

### D10: the conifer crown's mass is a comb-toothed spray, and it is content, not a crate change

§4.12. Recorded because the tempting crate-level answers — a spray as a first-class leaf kind, a
compound template, an impostor system — would each have added a type for what a width law with
teeth already is. The spray is authored in the species, measured against the reference at the plate's
own distance, and the module doc says which plate draws which blade. If a later species needs a
spray of a different grain, it authors a different comb; that is the crate's own answer to every
such question.

## 7. Verification

- **Every arc renders.** A species that has not been drawn has not been built, and the frame goes in
  `out/` with the species' slug in its stem.
- **H1 is run once, at the end of arc 6**, on all ten silhouettes at once, and the result is recorded
  in the handoff — including the pairs that were confused, because a confusion is a fact about two
  species and the fastest route to fixing both.
- **H3 is a number** and is measured by a probe, not judged, so a regression in crown depth shows up
  as a number moving rather than as a frame looking slightly worse.
- **The existing scenes come out of arc 1 unchanged** except where §4.2 says they must not, and each
  change is stated with its reason before it is committed.
- **Each species gets a probe row**, not a test: a line in the species' own note giving its node
  count, leaf count, triangle count and crown reach at the default detail, so that a later change to
  a shared law shows up as ten numbers moving.

### 4.16 What the second passes asked for, decided

The second passes of arcs 3–6 and the second wave came back with six more wants. Each is decided
here, because a limitation that is not decided gets re-litigated by every later species:

| want | decision | reason |
| --- | --- | --- |
| a `Field` scoped to a generation, so a droop hangs the branchlets and not the bole | **won't fix** | three species (birch, spruce, fir) drew the droop in their shape functions and the fourth (oak) never wanted one; a field that reaches only some depths is a second author of the same curve, and a droop is a drawing |
| `Detail::around` silently below a section's authored point count, which drew a hard terminator across 330 px of birch bark | **fix**, §4.17 | the loss is silent and it is the polygonal-silhouette defect drawn by the plate whose subject is the wood |
| a leaf field that leaves a node's apex bare for the cone seated there | **won't fix** | the band is the dial that says where foliage stands and `scatter_in`'s band is the one that says where fruit stands; keeping two bands in step is authoring, and the alternative is a field that reads another field |
| the shadow page's fitted boundary drawing a straight line across the ground | **defer to the shadow subsystem** | the page is fitted to the subject by design and the line is its edge; widening it is the virtual-shadow-maps roadmap's question, not this proposal's |
| an emissive term on `Skin`, for a flower that is saturated without being lit | **won't fix now** | a flower's read is a tint and a roughness at every distance this project draws; emissive is a materials arc that nothing else here is waiting for |
| a decorator distribution along a node's own length (the fern's mantles of leaf-base scars) | **defer** | it is a decorator *field*, a new type mirroring `LeafField`; the fern's local `mantle()` helper is honest and small, and one species wanting it is not yet two |

### 4.17 A section's authored corners must survive the sweep's sampling

**Pulls it:** 4, 5, 9, 10 — every species whose bark is a section.

`expand::wood` samples every solid at `detail.around` vertices, and a section authored with more
corners than that loses them silently: a sixteen-lobed birch column drawn at fourteen came out with
a hard vertical terminator, and a twenty-four-fissured keteleeria at sixteen rounded its own fissures
off. Both were found by looking at a frame, which is the last place a silent loss should surface.

**Change:** `wood` samples at `detail.around` or the section's authored corner count, whichever is
finer. An ellipse has no corners and is unchanged; every existing round-section plant keeps its
twelve; only the sections that authored more corners than the dial draws move, which is exactly the
set that was being lied about.

**Size:** one `max` and a `Section` accessor for its authored corner count, nought for an ellipse.

## 8. The site: how a species stands in its belt (arc 7's design)

A hero specimen is a tree; an epic frame is a place. The ten species live in four belts of one
mountain, and a frame that puts a kapok in mist or a fir in a dry valley is a frame that lies, so the
site is authored per belt and the bar's E1 checks the frame's own sidecar against the species' note.

**One slope, four frames.** A single heightfield law — a ridge descending south-west into a valley,
authored as a plane plus three swells and given to `Ground::Fielded` — is the ground of every site
frame, so the four frames are one mountain seen at four heights rather than four inventions. The
belts, from the notes' altitude ranges:

| frame | belt | altitude drawn | species in it | light |
| --- | --- | --- | --- | --- |
| `site_valley` | dry-hot valley | 800 m | 木棉 in flower and in leaf, 桫椤 in a damp gully, 筇竹 as a clump | hot noon, thin shade, haze 0.010 |
| `site_montane` | subtropical montane | 2000 m | 云南松 in stands on the sunny slope, 川滇高山栎 in the draws, 珙桐 in a moist north gully | warm morning, raking, haze 0.005 |
| `site_subalpine` | subalpine | 3200 m | 丽江云杉 and 苍山冷杉 in a stand, 大树杜鹃 under them, 川滇高山栎 as krummholz on the ridge | cold high sun, deep shadow, haze 0.002 |
| `site_transect` | the whole slope | 800–3200 m compressed | all ten, each at its own belt's height on one compressed slope | one low sun along the valley |

The transect states its compression in the sidecar and in its own doc: it is a diagram that is also a
picture, and it is the frame the ten silhouettes are finally judged in together.

**Companions are the same species at a coarser dial.** `Planting::with_detail` and the spray/plume
foliages exist precisely so that a background tree is the same rule at a coarser resolution: a site
frame holds one hero planting at full detail and eight to twenty companions at `Detail::scaled(1)`
with the spray foliage, seated on the heightfield by `vegetation::ground::seat` at seeded positions.
The companion count is what the scene compile allows, and the emit hoist of §4.13 is what raises it.

**Scale is a stone, not a person.** E2 wants a known size in frame. A person would need authoring a
person; a standing stone of 1.8 m is one `Prop` cuboid with a taper, and the sidecar states its
height. A stone also belongs in every one of these belts, which a person in a kapok valley does not.

**Mist is the haze dial and the sky together.** A valley frame carries `haze` 0.010 and a pale
horizon; a subalpine frame carries 0.002 and a deep zenith. The two numbers are the whole of the
difference between a humid gully and a clear ridge at 3200 m, and both are already dials (§4.6).

**E4, the two backends, is deferred with a reason.** The exporter writes the scene's sky as a Cycles
background and the document carries both, but no Cycles binary is installed on this machine, so the
comparison cannot be run here. Arc 7 delivers the frames and the exported documents, and records E4
as unverified-with-reason rather than passing it silently.

### 4.14 What the dry valley and the fern asked for, and what they get

Arc 6's two species came back with nine things the crate cannot say
([the valley author's report, 2026-10-19]). Each is decided here rather than left to drift, because
a limitation that is not decided gets re-discovered by every later species:

| limitation | decision | reason |
| --- | --- | --- |
| `scatter` seats at one station of the node, so "fruit on the upper crown only" is unwriteable | **fixed**, §4.15: `scatter_in` with a band of the parent's station | a fir's cones and a kapok's flowers are both terminal; two species needed it in one arc |
| `decorator::Template` is one sweep, so a corolla was a ten-pointed star section | **fixed**, §4.15: a decorator template may be a sprig | the leaf side already had the join; the decorator side was the same want one type down |
| `envelope::trim` measures about the root's spine, so a buttressed root trims against two and a half metres of trunk | **defer** | a crown law about the root is the right frame for a single-stem tree; a buttressed species shapes its crown from `Joint::station` laws and the kapok does. Revisit when a third species hits it |
| a node's section cannot vary along its length, so packed leaf bases became a ripple in the taper | **won't fix** | a section varying along a spine is a second sweep; the taper is the radius law and a ripple in it is the same fact told once |
| `Detail` is per planting, not per node, so a buttress and a bole want two constants | **won't fix** | the planting is the level-of-detail unit; a node-level dial would make one plant two resolutions and break the rule that a walk is one geometry |
| a `Placement` carries no scale, so a pinna's taper is eighteen blade geometries in a sprig | **defer** | eighteen blades are authored once per species; a scale law would save authoring, not rendering |
| a leaf template is one arrangement, so "five to seven leaflets" is two templates | **won't fix** | a part count that varies per instance varies the instanced mesh, which is the one thing instancing forbids; two templates is the honest spelling |
| a leaf's two faces share one skin, so "paler beneath" is not drawn | **defer to a materials arc** | needs a per-face material channel; nothing else in the crate wants one yet |
| a frond or a buttress cannot be `trim`med without crushing | **defer**, with the trim item | same cause, same revisit |

### 4.15 A scatter that reads a band, and a decorator that is a sprig

**Pulls it:** 3, 4 (erect cones on the upper crown), 9 (flowers at every twig tip of a bare crown),
7 (a truss inside a rosette).

`scatter` seats on every live node of a generation at one station of that node, and
`decorator::Template` is one sweep. The first cannot say *where on the parent* the fruiting nodes
are, and the second cannot be a corolla of five petals. Both get the smallest addition that says it:
`scatter_in(…, band)` seating only the nodes whose attachment station lies in the band, with
`scatter` as the full band; and a decorator template that is a sprig of parts, joining lofts exactly
as [`LeafTemplate::sprig`] does, so `Template::loft` stays the one entry the gallery calls and a
one-part template's loft is bit-identical to today's.

**Size:** one band parameter and one constructor; the seed's draw order is untouched, so every
existing seat is where it was.

### 4.13 The scene compile pays per instance row, and most of it is per material

**Pulls it:** every site frame, and every dense crown.

Measured on this branch: a pine of 87 000 instance rows compiles in 51 s and rasterises in 3 s; a
bamboo of 5 000 rows compiles in 0.1 s. The cost is per row, and inside a row the compiler re-does
five things that depend only on the row's material and shader — verifying the material's custom data
against the shader's custom schema, resolving its images, stripping its image fields, and merging the
row schema — so a scene with 3 000 materials and 260 000 rows computes them 260 000 times. The fix is
to compute them once per material and borrow them per row; the emitted document does not change, so
every pixel and every pinned hash stays where it is. Until it lands, a species' whole-tree plate is
budgeted at 90 000 rows and the site frames at companions enough to read as a stand.

## 9. The verdicts, species by species

A living table: `pass` is measured or read against the bar by the author and checked by the artist;
`weak` passes with a named caveat; `—` is not yet run. H3's second clause is read on the bough plate
(§2.1). The epic columns name the site frame the species stands in and whether E1–E3 were judged on
it; E4 (the two backends) is unverified repo-wide, no Cycles binary on this machine.

| species | H1 | H2 | H3 | H4 | H5 | epic frame | notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 云南松 *Pinus yunnanensis* | pass (sheet) | pass | 0.628 / 0.769 pass | pass (base7) | pass | `site_montane`, `site_transect` | the arc-2 reference species; cones faint at tree distance |
| 华山松 *Pinus armandii* | — | — | — | — | — | `site_montane` | in flight (wave 3); the two-pine separation is its H1 |
| 丽江云杉 *Picea likiangensis* | pass (0.158 ratio) | pass | 0.663 / 0.479 pass | pass | pass | `site_subalpine`, `site_transect` | second pass; bough reframed to 1.09 m |
| 苍山冷杉 *Abies delavayi* | pass (0.254 ratio) | pass | 0.789 / 0.622 pass | pass | pass | `site_subalpine`, `site_transect` | cones pale-glaucous to read; `FIR_CROWN_TOP` is a stated compromise |
| 川滇高山栎 *Quercus aquifolioides* | weak (mid-crown hole) | pass | 0.792 / 0.715 bough pass | pass (base) | pass | `site_montane`, `site_transect` | H1 polish in flight; sprig crown at the note's leaf size |
| 珙桐 *Davidia involucrata* | pass (sheet) | pass | 0.899 / bough | pass (base) | pass | `site_montane`, `site_transect` | bracts as two templates, unequal pair |
| 大树杜鹃 *Rhododendron protistum* | pass (sheet) | pass | 0.848 / bough | pass (base) | pass | `site_subalpine`, `site_transect` | rosettes and truss; bole wander authored |
| 桫椤 *Alsophila spinulosa* | weak (palm-like from one angle) | pass | 0.690 / 0.650 pass | pass | pass | `site_valley`, `site_transect` | fronds at the photograph's crown spread, note's length broken and stated |
| 木棉 *Bombax ceiba* | pass (sheet) | pass | 0.561 / 0.747 pass; flower exempt | pass (base) | pass | `site_valley`, `site_transect` | two seasons; corollas of five petals on the twigs that carry them |
| 筇竹 *Qiongzhuea tumidinoda* | — | — | — | — | — | `site_valley` | in flight (wave 3) |
| 西南桦 *Betula alnoides* | weak (reads conifer-strip) | pass | 0.521 / 0.724 pass | pass, bark pattern stated absent | pass | `site_montane` | H1 polish in flight; red-brown bark, never chalk |
| 滇油杉 *Keteleeria evelyniana* | weak (rough spruce) | pass | 0.757 / 0.677 pass | pass | pass | `site_montane`, `site_transect` | H1 polish in flight; cones at the note's ceiling |

## 10. Open questions

**H1's first running, 2026-10-19** (`out/h1-sheet2.png`, ten silhouettes, read by the author of the
bar against the reference photographs): eight of the ten are nameable at a glance — 桫椤's parasol on
a pole, 木棉's flat plate of heavy limbs, 大树杜鹃's candelabra of discrete stars, the two spires
told apart by ratio and tip attitude, 云南松's tiered feathering, 珙桐's rounded mass, and 滇油杉's
rough cone. The bar's failure line is seven. The confusions are the middle three: 川滇高山栎's dome,
滇油杉's crown and 西南桦's weeping column each read as *a broadleaf* or *a narrow conifer* before
they read as their species, and the oak's silhouette carries a hole at mid-crown that no photograph
of it has. Those three are the remaining H1 work, and the sheet is the instrument that will say when
it is done. 华山松 and 筇竹 join the sheet when arc 3's and arc 6's last species land.

1. **Does H3's ratio need to be a stated number before arc 2, or can arc 2 set it?** — **Closed, 2026-10-19: arc 2 set it, from two measured regimes rather than from one candidate.** The p90/p10 luminance ratio the first draft proposed does not separate a deep crown from a sparse lit one (a bamboo's separated leaves scored 1.82 against the pine's 1.70), and a rim mask does not either, because silhouette-edge pixels face away from the key as often as toward it (interior over rim measured 0.99 on a crown that was plainly not self-shadowing *and* 0.97 on one that was). The bar is therefore the interior share at least **0.25** and the interior's mean over the plant's lit p90 at most **0.80**: the pine under a frontal key with foliage shadowing disabled measures **0.99**, and under a raking key with the foliage shadow bias of §4.11 measures **0.76**, so 0.80 sits between the two regimes with margin on both sides. A species that cannot reach it has either a see-through crown or a light that flattens it, and the number says which.
2. **Bark as texture.** §4.7 says plates and flutes are a `Section`. That is form, not colour
   variation, and a hero frame at three metres may want a mottled bark that no geometry gives. Whether
   a per-vertex tint along a trunk is worth a change is deferred to arc 4, when the oak's bark is the
   subject.
3. **Two seasons.** 木棉 flowers leafless and 珙桐's bracts are seasonal. Whether a species carries a
   `season` dial that switches a leaf field off, or whether a season is a second species, is deferred
   to arc 6.
4. **Does the rasteriser's shadow map survive a crown of forty thousand leaves?** The handoffs on
   shadow texel push and virtual shadow maps suggest this is live elsewhere. Arc 2 measures it on one
   pine before arc 3 puts ten firs in a frame.

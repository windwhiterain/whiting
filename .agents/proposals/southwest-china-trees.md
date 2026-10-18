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
| H3 | **Crown depth.** The crown has a lit outside and a dark inside, so it reads as a volume. | Measured, not judged: the frame's own histogram of lighting-stage luminance inside the crown's silhouette must span more than a stated ratio between its 10th and 90th percentile. A bag of leaves lit from one side is narrow; a self-shadowing canopy is wide. |
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

## 8. Open questions

1. **Does H3's ratio need to be a stated number before arc 2, or can arc 2 set it?** The first
   reading is that arc 2 sets it and every later species is held to the same one, which is a bar set
   by the first candidate. The alternative is measuring a known-good reference frame — but there is
   no known-good tree in this repository, which is the whole problem.
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

# A Chinese vernacular level: the four assumptions the building crate has to give up

Status: design proposal, 2026-10-18. It rests on four research notes in
[`investigate/`](../investigate/): [chongqing-stilt-houses.md](../investigate/chongqing-stilt-houses.md),
[yunnan-dwellings.md](../investigate/yunnan-dwellings.md), [guizhou-dwellings.md](../investigate/guizhou-dwellings.md),
[chinese-timber-frame-and-roof.md](../investigate/chinese-timber-frame-and-roof.md) — which answer
separate questions and are, per the repository's rule, not a basis for implementation on their own.
No code has been written against this document.

## Scope: the vernacular, not the official tradition

The owner's direction of 2026-10-18: take the framework to **Chinese traditional architecture**, and
specifically to the **dwelling** — 民居 — rather than the palace and the temple. Named: the south-west
(西南民居), the Chongqing stilt house (吊脚楼), the Yunnan and Guizhou minority dwellings (云南贵州等地的
少数民族民居), and the old town (古城, 古镇) fabric those stand in.

Out of scope unless it measures something: 官式 construction (the 营造法式 and 工程做法则例
traditions), palaces, 牌坊, and the axial temple compound. Those are a different design system with a
published module (材分°) the vernacular does not use; where a vernacular note cites them it is as a
contrast, and this document keeps that distinction.

Why the distinction is not cosmetic, in one line: the official tradition is a **module-driven**
architecture whose proportions are derived from a graded timber section, while the vernacular is a
**frame-and-infill** architecture whose proportions come from the bay, the purlin spacing and the
available timber. A generator for one is not a generator for the other.

## What the model is, measured against the code

A building is a `BuildingSkeleton` (`crates/building/src/skeleton.rs:42`) holding seven lists:
masses, roofs, grids, openings, parts (templates and placements), walls, slabs. `build` walks them
into closed planar-facet solids and every face names the authored element that made it
(`mesh.rs:193`).

| Solid | What it is | Where |
| --- | --- | --- |
| `Mass` | a room: an outline carrying a ground elevation per node, a **level** wall top, a wall thickness, a floor slab under it, optional courtyard loops | `mass.rs:124` |
| `Void` | the wall ring one of a mass's voids is built as — the courtyard itself | `courtyard.rs`, `mesh.rs:286` |
| `Roof` | an eave loop, a crest network, planes, a thickness; names the mass it sits on | `roof.rs:133` |
| `Wall` | one straight run, thickness split about the line, level top, bottom following the ground | `wall.rs:69` |
| `Slab` | a level prism over a plan polygon; the roles are `Plate`, `Floor`, `Roof` | `slab.rs:88` |
| `Part` | a template's masses and roofs, placed in a bay by slot and pin | `parts.rs:250` |

Five invariants are the crate's capital, and every extension below is judged by whether it keeps them:

1. **Planarity.** Every face is planar within `PLANARITY_TOLERANCE`; an authored surface is one
   shading group, so its facets are smooth across and hard at its edges (`mesh.rs`, `validate.rs`).
2. **Two-level watertightness.** Every solid is closed and outward (positive signed volume), and two
   solids are two solids: a joint between them is **authored**, never welded by coincidence
   (`expand.rs` module contract, D5).
3. **Authored correspondence.** No boolean, no plane-intersection search, no triangulation of a
   point cloud: two faces meet because they were built from the same authored row or corner.
4. **Slots, not places** (D10): a bay is `(row, column)` in a grid's stop lists, so a re-split
   carries what hangs in it (`grid.rs`).
5. **A part's frame is a rotation.** A template maps into a bay by a frame that can be reproduced by
   a yaw about `up`, which is what lets a scene instance it (`parts.rs`, `expand::part_frame`).

## The four assumptions

Each of these is a sentence the code says plainly, and each is false of a Chinese dwelling. They are
ordered by how much of the type's identity depends on them.

### 1. A roof is a strip of one slope

`RoofRow` (`roof.rs:72`) pairs a crest vertex index with **one** station on the eave loop, and a
`RoofPlane` is a strip of such rows — so each plane is **one straight slope from the ridge to the
eave** ("A roof plane is a strip of rows, and a row is a crest point paired with an eave station",
`roof.rs:12`). `derive` builds hips and gables from it and a probe measures every plane departing its
fitted plane by under 3e-7 m (`derive.rs`).

A Chinese roof is the opposite: the purlins step down at **unequal** rises — 举架 in the Qing
tradition, 举折 in the Song, 提栈 in 江南 — and the profile through those purlins *is* the
building's silhouette. A straight tent is not a Chinese roof at any scale.

The planarity contract is **not** the obstacle, and this is the observation the design turns on: a
purlin profile is a chain of **planar strips**, one per 步架, which is exactly what the roof already
builds for one step. What is missing is a plane that holds *k* strips — not a surface that curves.

### 2. A wall top is one elevation

`Mass::top: f32` (`mass.rs:128`) and `Wall::top: f32` (`wall.rs:69`). What the crate does in its
place is close the gable with the roof's own fascia, "the band around the skin's boundary … which is
what closes a gable's ends as well as its eaves with one rule and no case analysis"
(`mesh.rs:162`, `RoofSurface::Fascia`). That is a 悬山 reading with the wall left out, and it is a
good one — but it is the only one available.

A Chinese dwelling uses the whole family: 硬山 (the wall rises flush to the roof surface), 悬山 (the
roof oversails a gable wall that stops below it), 马头墙 (a stepped gable wall above both roof and
neighbours), the parapet of a 一颗印, and the shaped top of a 照壁. Every one of them is a wall whose
top is a **profile**, and none of them can be stated.

### 3. A member is vertical

The only line elements in the document are `Wall` — a vertical prism with a level top — and level
`Slab`s. 穿斗 construction is made of the members that are neither: `穿枋` ties threaded through the
columns, `挑枋` cantilevers carrying a gallery, `撑弓` diagonal brackets under them, `檩` purlins and
`椽` rafters on the roof. A cantilevered gallery with nothing under it, and a gable frame with no
columns and no ties, are the two pictures this assumption forbids.

It is worth saying what this is *not*: it is not a request for detail geometry. Five members —
column, tie, cantilever, bracket, purlin — carry the type's identity, and they are all one construct:
a prism over a 3D segment.

### 4. A floor stands on its outline's ground

`mass.rs:119` says the ring's base "stands on the outline's own elevations", and `site::seat` gives
every mass **one level: the highest ground under its outline plus an offset**, with the reason
written out — "what a builder does with a slope — cut and fill to a level floor, and let the downhill
side show its slab" (`site.rs:24`).

That is a correct answer to a slope for masonry, and the wrong one for a 吊脚楼, where the floor is
set by the **uphill** grade, the downhill side is open to the air, and the building stands on posts
with the lower storey half-hanging. Today a stilt house can only be authored by pretending its posts
are a plinth: a solid box where the architecture has air. The type the owner named first is the one
the site model refuses.

## What the crate already gives us

- **The courtyard is solved**, including the hard part: a void is a ring of wall built *outward* from
  a loop inside the outline, standing on the plate, closing itself, with the rooms' faces and the
  courtyard's faces derived from the same two loops (`courtyard.rs`). 天井, 院落 and 厢房 are this.
- **Deep eaves are free.** The eave loop is authored apart from the mass outline (`roof.rs:135`), so
  an 出檐 is a number, and 腰檐 / 披檐 / 重檐 are further roofs hosted on the same mass.
- **An intermediate floor exists and nothing draws it.** `derive::floor_slab` (`derive.rs:787`)
  builds a hosted plate on the mass's inner loop at an elevation strictly inside the mass, refusing
  one at or below its ground and one at or above its top. It has one call site, in `derive`, and no
  gallery scene uses it. 楼板 is a completion, not a new concept.
- **The freestanding wall is drawn.** `yard` authors five runs as a yard enclosure and `authored`
  authors one; `Wall` is the second building element that is not a room (`wall.rs`), and it is what a
  照壁, a 院墙 and a 封火墙 already are.
- **A part is a small skeleton**, so a door leaf, a stair, a canopy or a balustrade needs no new
  geometry — only a template (`parts.rs`).

## The gap table

| What the type needs | In the model today | The smallest construct that closes it |
| --- | --- | --- |
| 举架 / 举折 roof profile | one slope per plane | a section of *k* steps on `Roof` |
| 硬山 gable, 马头墙, 照壁 top | `top: f32` | a crown (a top profile) on `Mass` and `Wall` |
| 穿枋, 挑枋, 撑弓, 檩, 椽 | nothing with a sloped axis | `Member`: a prism over a 3D segment |
| 吊脚 posts, open underside | terrace seating | a stilt seating mode, plus `Member` posts |
| 楼板, a second storey | `derive::floor_slab`, unused | use it, at a grid level |
| 天井, 院落, 厢房, 走马廊 | void ring, deep eaves, hosted slabs | nothing — authoring |
| 瓦垄, 瓦当, 滴水, 小青瓦 | nothing (a surface has a colour) | a shader question, not geometry — see the forks |
| 台基, 踏步, 柱础, 石砌勒脚 | `Slab`, `Part`, `Wall` | nothing — authoring |

## What the buildings ask for

Numbers to come from the four research notes: bay and depth ranges, purlin steps and their rises,
eave depths, the 吊脚 drop as a function of slope, courtyard dimensions, tile and slate sizes, and
the pitch minima each covering needs.

## The design

Six moves. Four are new constructs and two are completions; each names what it buys, the invariant it
must keep, and the sentence it refuses to build.

### D1. A roof holds a section, and the section is a fraction **and** a rise

`Roof` gains one field: `profile: Vec<Step>`, each `Step { fraction, rise }` — the fraction of the
slope's plan run from the eave, and the rise above that row's own eave elevation. The list is strictly
increasing in both, every step strictly inside `(0, 1)` in fraction and strictly below the crest in
rise. Expansion turns each authored row into a **polyline** through those steps, and each plane into
`profile.len() + 1` strips of quads. The strips are the 步架.

Why a section and not one of its two halves — each spelling fails in a different corner, and the two
failures are why this is one datum and not two:

- **Rises alone fail on a sloping eave.** A cut "at elevation e" lands at a different fraction on two
  rows whose spans differ, so the cut line stops being parallel to the rows and the sub-quad stops
  being planar — and a level ridge with a level eave hides this completely, because then the two
  profiles are the same up to a change of parameter.
- **Fractions alone have no heights.** A fraction says where the purlin stands along the run; the
  rise says how high. 举架 is the pair — each 步架 has a width and a rise, and the roof's concavity is
  the ratio changing between them.
- **The pair is also what keeps the strips planar.** Rows of one plane are parallel in the ordinary
  case — a gable's two slopes, a shed's single slope — so a step's two segments are parallel, and the
  quad between them is a planar trapezoid. A **fan** (a hip's triangular end, rows meeting at one
  crest point) has no such pair of parallel segments, and a level purlin cannot cross a fan as a
  straight line: that plane is refused by name rather than given a second rule.
- **Watertightness owes the profile nothing.** The vertices a step adds are identified by *(the row's
  own identity, the step index)*, so two planes sharing a row share every vertex of it by the crate's
  existing welding rule, not by a coordinate test.

And what the row's polyline means physically: the roof surface bends at every purlin. It is not one
slope with marks on it — it is a chain of planes, which is what a 举架 roof *is*, and why the profile
belongs on the roof rather than being derived per plane.

What it buys: the silhouette. A Chinese roof is read from its profile — shallow at the eave, steep at
the ridge, with a distinct break at every purlin — and that profile is the one thing a straight tent
cannot fake.

Refusals: `ProfileOutOfOrder` (a fraction not strictly increasing inside `(0, 1)`, or a rise not
strictly increasing above the eave), `ProfileReachesTheCrest` (a step at or above the crest's own
rise), `ProfileOnAFan` (a plane whose rows are not parallel, which a level purlin cannot cross
straight), and the existing `SolidOpen` if two planes still disagree.

The way a 起翘 corner stays legal is worth one sentence: a raised eave node tilts its row, which is
the sloping-eave case, and the *same fraction* then lands at a different elevation along the eave —
a rising eave, correctly. A corner built as its own 翘角 piece is its own small plane or its own
member.

### D2. A wall top is a profile

`Mass::tops: Vec<f32>`, one elevation per outline node, and `Wall::crown: Vec<CrownPoint>` — a
profile along the run. A level top is the degenerate case of both, so the common wall is written as a
one-element crown rather than the special case.

Why it stays planar, computed rather than assumed. Take one edge from `A` to `B`, its inner offset
`A'`, `B'` a wall thickness `w` in, and tops `t_A`, `t_B`, with the inner nodes taking their outer
node's elevation. In a frame along the edge the four points are `(0,0,t_A)`, `(L,0,t_B)`,
`(L,w,t_B)`, `(0,w,t_A)` — all four satisfy `z = t_A + (t_B − t_A)·x/L`, which does not contain `y`:
**one tilted plane holds the whole top strip.** At a corner the two strips share the horizontal
segment `B–B'`, so they meet along an edge rather than a gap, and the crease between them is a hard
edge because an authored corner is hard by contract. The outer and inner wall faces are vertical
planes whose top edges follow the node elevations, so they are planar too. Nothing needs
triangulating and no tolerance is spent.

What it buys: 硬山 (the wall flush to the roof), 悬山 (the roof oversailing a wall that stops below
it), the stepped gable, the parapet, and a 照壁 whose top is not straight. It also gives the **gable
under a pitched roof** the wall it actually stands on, which today is a 0.2 m roof fascia.

Refusals: `CrownCountMismatch` (one top per node, and a count that disagrees is a wall in the wrong
place), `CrownBelowBase` (a top at or under the wall's own base), `CrownNotASequence` (a `Wall` crown
whose stations are not strictly increasing within the run).

### D3. A member is a prism over a 3D segment

`Member { from: Vec3, to: Vec3, across: Vec2, width: f32, depth: f32, host: Option<SolidId> }`.
Six faces, all planar by the same argument as a slab's: four sides are rectangles and the two ends are
the section. `across` is the horizontal direction the section's `width` runs, which is what makes a
vertical column's orientation authored rather than undefined (`up × axis` is zero for a vertical
member, and a silent default there is a section that spins with the wind).

`host` is the authorization the crate already demands of a roof: a member embedded in a wall overlaps
it, and two solids may overlap only when the overlap is authored.

What it buys: 柱, 穿枋, 挑枋, 撑弓, 檩, 椽, a railing, a stair's stringers, and the 吊脚柱 of D4 — one
construct, because every one of them is a prism over a segment. It also buys the **palette**: a material
belongs to an instance and not to a face (`gallery::house`'s own comment), so dark timber against a
whitewashed wall is not a colour on a face — it is a member and a wall being two solids.

Refusal: `MemberAxisDegenerate` (a segment of no length, or an `across` parallel to the member's plan
direction, which collapses the section).

### D4. A floor need not stand on the ground: stilt seating

`site` gains a second seating mode beside the terrace. Where the terrace sets every node of a mass to
the highest ground under it, the stilt mode keeps the mass's **authored** elevations (the floor) and
emits posts from the floor's underside down to the sampled ground at a stated spacing, each post a
`Member`, each seated on a 柱础 if the author asks for one. It reports the largest drop, because that
number is the building's own proportion: a 吊脚楼's character is how far the downhill side hangs.

This is the mode the current one refuses by construction. A terrace is the correct answer to a slope
for masonry and the wrong one for a timber house on a river bank: the house does not cut the hill, it
stands on it and hangs over the water.

Refusal: `StiltPostTooLong` when a post passes a stated limit (a post that long is a different
building), and `StiltGroundBelowSea` when sampling finds no ground under a post.

### D5. The intermediate floor is a completion, not a construct

`derive::floor_slab` already builds a hosted plate on the mass's inner loop at an elevation strictly
inside the mass. What is missing is the *place*: a floor at the grid's second level, carried by the
face's own storey lines. Nothing new is authored — a scene calls it at `levels[1]` and the two-storey
house becomes a two-storey *building* rather than a two-storey facade.

### D6. A starting point, in `derive`

`derive::starter` offers `RoofStyle::{Hip, Gable, Flat}`. The vernacular level adds one derivation:
a **three-bay, one-or-two-storey timber house** with a purlin-profiled 悬山 roof, deep eaves, gable
frames that carry their columns and ties, and a plinth — every element an ordinary one, as the
module's contract demands.

The point is not convenience. A derived element that carries no guarantee and is immediately editable
is how this crate makes a new architecture *authorable* by an agent in one call, and how the gallery
gets a scene whose document is 60 lines instead of 600.

### Table: what each move costs

| Move | New fields | Code that changes | The gate it must pass |
| --- | --- | --- | --- |
| D1 profile | `Roof::profile` | `expand` (row polylines, strips), `roof` (accessors), `validate` (three refusals) | a probe that every strip is planar and the strips tile the loop |
| D2 crowns | `Mass::tops`, `Wall::crown` | `expand` (ring top strips, wall faces), `validate` | a probe that a crowned ring is closed and planar |
| D3 members | `Member`, a list in the document, a `SolidId` | `expand`, `mesh` (element), `identity`, `validate` | a probe that a member is closed and outward, and that two members overlapping without a host are reported |
| D4 stilts | a seating mode | `site` | a probe on an analytic slope: posts land on the ground, the floor stays level |
| D5 floors | none | a scene | the floor's own faces appear in the identity table |
| D6 starter | none | `derive` | the starter builds and its roof's profile measures the profile |

## The forks, for the owner

Each is a real choice with a cost somewhere; none is "obviously right".

**F1. The roof's profile — three ways, one recommendation.**
- (a) *A section of fraction and rise, cut into planar strips* (D1). Keeps planarity, keeps
  watertightness, and matches how the roof is built (a 举架 roof is a chain of planes, not a curve).
  Costs: every row becomes a polyline, a fan plane cannot take a profile, and a 起翘 corner is a
  separate piece.
- (b) *A curved, tessellated roof surface.* Truer to a tile surface, and it breaks the crate's
  one-face-one-plane contract, its shading groups, and the fascia derivation — three invariants for
  one silhouette.
- (c) *A silhouette in the shader.* Cheapest; the roof would cast the shadow of a tent, the id channel
  would name a plane that is not there, and the picture would disagree with the document.

**F2. The crowned wall top — three ways.**
- (a) *Per-node tops on `Mass` and a profile on `Wall`* (D2). One rule, no new solid, and every gable
  form is authored. Costs: the ring walk's top strip stops being level, which touches the most
  exercised code in the crate.
- (b) *Crown walls as separate `Wall` solids stacked on the mass*, with an authored joint. Smaller
  change; every gable becomes two solids, every joint needs authorizing, and a wall that overlaps the
  mass is the overlap the validator reports.
- (c) *Deep eaves only.* No change at all; 硬山, 马头墙 and the parapet are refused, and every roof in
  the gallery is 悬山 with a wall that stops short — which is a real Chinese roof, and only one.

**F3. The timber frame — three ways.**
- (a) *`Member` as a first-class solid* (D3). The frame is built, cast, shadowed and named; and it is
  a new element class, so it is a new material — the palette follows for free.
- (b) *Members as templates of masses.* The existing `Part` mechanism can place a column, because a
  column is a small mass. A **diagonal** is impossible — a mass is a prismatic shell standing on its
  outline — so 撑弓, 斜撑 and 椽 are out, and so is every rafter.
- (c) *Texture only.* No geometry; the frame is a colour. The id channel cannot name a column, so
  "move the third column" is not a sentence this system can answer.

**F4. How deep the detail goes.** Recommended line: columns, ties, cantilevers, brackets and purlins
are geometry; rafters are geometry **only under the eave**, where a Chinese roof's underside is the
picture; tiles, 瓦当 and 滴水 are the shader's. Above about 8 cm a member is a couple of pixels at the
frames this gallery draws; below it, it is noise that costs faces and shadow rays.

**F5. Which building first.** The 吊脚楼 is the owner's first-named type and the one that tests D4;
三坊一照壁 is the richest and tests D1, D2 and D5 at once; a 古镇 street tests the settlement scale.
The road below assumes the order, but it is the owner's to change.

**F6. Scope of the old town.** A street of houses is not a building: it needs a *settlement* layer —
frontage, blind walls on party lines, the arcade, the slope of the street, water. That layer does not
exist in this workspace (the gallery draws one building per scene). This proposal stops at the
building; the settlement is a separate question and should be named before an arc promises it.

## The road

Each arc ends green through the audit, with one new scene and its pinned hash, and each is shippable
alone. The gate cost per scene is real and known: a `.png` hash pin in `[6/8]`, a pass-count row in
`crates/scene/tests/clustered_lights.rs`, and — when the scene's id channel changes — rows in
`scene_identity_probe`'s table that `[7/8]` checks two ways.

1. **Arc 1 — the profile (D1).** A single three-bay house with a purlin-profiled 悬山 roof and a deep
   eave. Scene: `stilt`'s predecessor, a `timber_house`. Proves the silhouette.
2. **Arc 2 — the crown (D2).** A 硬山 gable and a stepped gable on the same house. Proves that a wall
   can meet a roof.
3. **Arc 3 — the members (D3).** The gable frame exposed: columns, ties, cantilevers under a gallery.
   Proves 穿斗 and the bracket.
4. **Arc 4 — the stilts (D4).** The 吊脚楼 on a slope: floor on the uphill grade, posts and braces
   downhill, a lower storey half-open. Proves the site.
5. **Arc 5 — the compound (D5, D2).** 三坊一照壁 or 四合五天井: wings, galleries, a 照壁, 天井.
6. **Arc 6 — the variations.** 竹楼 (steep short ridge, deep eaves), 石板房 (stone slate pitch),
   屯堡 (stone court), 一颗印 (high blank walls, parapet) — the same constructs, different numbers,
   and each one is the proof that the parameters are the type.
7. **Arc 7 — the settlement.** Only if F6 says yes: a street of three or four houses on a slope.

## What would make this proposal wrong

- If a roof needs a **hip** (庑殿, 歇山) *and* a profile, D1 refuses the fan plane and the roof has to
  be built as two roofs or with a different section rule. The vernacular south-west is overwhelmingly
  悬山/硬山, so this is a boundary rather than a hole — but it is a boundary, and it is not measured.
- If a crowned mass's miter at a corner cannot be given one elevation per node without folding a top
  strip, D2 is wrong and (b) is the answer. Not measured; the algebra above says it is fine, and
  algebra is not a probe.
- If the vernacular turns out to need a wall that is not a prism (a battered 土墙 that leans, a
  竹编 wall that curves), no move here expresses it and the design stops at the prism.

## Not established

- Whether the crate's planarity tolerance holds a *crowned* mass's top strip at a corner. The
  geometry says a sloped top strip over a straight run is planar (four points on one tilted plane);
  the miter at a corner, where two crowned edges meet at different slopes, has not been computed.
- Whether a 举架 profile is the same for every plane of a hip roof. A gable's two slopes have the
  same 步架 count and the same profile; a hip's short ends have fewer, and the design must say
  whether the profile is per plane or per roof.

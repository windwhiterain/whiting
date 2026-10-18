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
purlin profile is a chain of **planar steps**, one per 步架, which is exactly the shape the roof
already builds for one step. What is missing is a *row* that can break before it reaches the eave —
and a surface that bends at the purlins is a chain of planes, not a curve, so nothing here asks the
crate to give up planarity.

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

### The fifth difference is a hierarchy, not an assumption

The four above are sentences the code says. This one is a sentence the code does not say and these
buildings do: **the frame is the building and the wall is infill.** The Bouyei stone house states it
outright — "房屋的承重结构是穿斗式木构房架，石料只作围护结构和屋面防水材料", the load-bearing structure is the
穿斗 timber frame and the stone is only enclosure and roofing (Guizhou note §Verdict, [S21]) — and the
Dong long house says it in numbers: the same frame carries walls of **0.03 m** slab infill or
**0.40 m** rubble, a factor of thirteen that no single wall-thickness parameter can hold, and that is
only possible because the wall carries nothing.

In this crate a `Mass` **is** the wall ring: the walls are the solid and the frame does not exist. That
is not wrong — the ring is what a viewer sees — but it fixes what D3 is for. The members must be
solids *in front of* the ring, and for this architecture they are not decoration on a building: they
are the building, and the ring is the panel hung on it.

## What the crate already gives us

- **The courtyard is solved**, including the hard part: a void is a ring of wall built *outward* from
  a loop inside the outline, standing on the plate, closing itself, with the rooms' faces and the
  courtyard's faces derived from the same two loops (`courtyard.rs`). 天井, 院落 and 厢房 are this.
- **Deep eaves are free.** The eave loop is authored apart from the mass outline (`roof.rs:135`), so
  an 出檐 is a number, and 腰檐 / 披檐 / 重檐 are further roofs hosted on the same mass.
- **A second storey is already a building.** `derive::floor_slab` (`derive.rs:787`) builds a hosted
  plate on the mass's inner loop at an elevation strictly inside the mass, refusing one at or below
  its ground and one at or above its top; `authored` calls it at **2.7 m** (`authored.rs:153`) and
  `workshop` at its own level (`workshop.rs:301`), and both push the plate into `slabs`. So a
  two-storey house is a grid with two levels and a floor between them, and it is drawn today. **This
  proposal first recorded this as a missing capability, and that was false** — the constructor has one
  call site because the *derivation* is what a scene calls, which is exactly what the two scenes do.
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
| 楼板, a second storey | **already drawn** (`authored.rs:153`, `workshop.rs:301`) | nothing — a grid level and a floor |
| 楼板开洞 (火塘通高, 楼梯井) | `Slab` has one plan polygon | `Slab::voids`, filled by `fill` |
| 屋面覆盖 (小青瓦, 石板瓦, 杉皮) | a surface has a colour | a `Covering` enum on `Roof` — the ridge and the pitch minimum differ |
| 使用开间 = 2 × 结构开间 | one grid per face | a ratio in the starter, not a construct |
| 天井, 院落, 厢房, 走马廊 | void ring, deep eaves, hosted slabs | nothing — authoring |
| 瓦垄, 瓦当, 滴水 | nothing | a shader question, not geometry — see the forks |
| 台基, 踏步, 柱础, 石砌勒脚 | `Slab`, `Part`, `Wall` | nothing — authoring |

## What the buildings ask for

Two of the four notes were complete when this section was written — the frame and the roof, and
Guizhou–Xiangxi. The Chongqing and Yunnan notes follow, and what they change is not the design but its
numbers.

**A caution about the notes' own language, because this proposal caused it.** All four were briefed
with "every roof is a flat slab", and each opens by naming that as the assumption that breaks. The
crate's roofs are **not** flat: a plane is a strip between a crest row and an eave station
(`roof.rs:12`), which is already one straight slope. Read every "flat plate" in the notes as **"a
single straight slope"**, and every "a roof is not a plate" as "a roof is not one slope". The
conclusions survive the correction; the words do not.

### The roof profile: a list of purlin stations, and only one of the three rules is implementable

The primary text settles the shape of D1: **the profile is not a pitch, it is the sequence of purlin
elevations** (frame note §"The profile"). Three named algorithms generate it, and they are three
different algorithms rather than three numbers:

| rule | period | how it generates the curve | can this crate implement it today? |
| --- | --- | --- | --- |
| 举折 jǔzhé | Song, 《营造法式》1103 | one total rise `H`, then a **halving fold series** `0.1H · 2^−(i−1)` from the ridge | **no.** The series sums to at most `0.2H`, so it cannot produce the whole rise, and the rule that closes it is not in the primary text. The note refuses to implement it, and so does this proposal. |
| 举架 jǔjià | Qing, 《工程做法则例》1734 | **one coefficient per step**, `rise = step × X/10`, increasing toward the ridge | **yes**, for the coefficients the worked example attests arithmetically: 五举 `0.5`, 六举 `0.6`, 八举 `0.8` (26.6°, 31.0°, 38.7°) |
| 提栈 tízhàn | Jiangnan, 《营造法原》 | one coefficient per step, steeper than Qing | **no.** The text was not obtained; its range is not established. |

What is safe to implement is narrower than the literature suggests and it is enough: **the 举屋 table**
(official coefficients, 1/3 to 0.2575 of the eave-to-eave span, as a contrast) and **举架's three
attested coefficients**, applied as `Step { fraction, rise }`. Everything else is `measured` — the
modeller types the purlin heights — and that is the variant a vernacular roof will actually use, which
is the honest shape for the field and the honest shape for the code.

So the authoring helper is `Step::from_widths_and_coefficients(widths, coefficients)`: the widths are
the 步架 (what a 掌墨 sets on a 丈杆) and the coefficients are the 举 (what the craft names), and the
fractions D1 stores are their normalised cumulative sums.

### The frame's proportions: official ratios are contrast, and there is one vernacular dataset

The official ratios are self-consistent — 柱高 = 0.8 × 面阔, 柱径 = 0.07 × 面阔, 出檐 = 0.3 × column
height (Qing, [S2]) — and they are **not** the vernacular's: a south-western 深出檐 is *larger* than the
official 0.3, because the eave is the rain screen. What this session obtained for the vernacular itself
is one measured dataset (桥梁民居, 黔江, 1916, 穿斗 + 悬山 + 青瓦): **进深 8.11–11.55 m**. Everything else —
开间, 柱径, 椽档, 出檐, tile — is `not established` beyond two sites.

That asymmetry is itself a finding and it constrains the road: **the first scene's dimensions will be
authored, not surveyed**, and the proposal must say so on the scene rather than pretending to a
precision the sources do not have.

### The south-west dwelling's own numbers (Guizhou–Xiangxi)

Four rules, each directly programmable, and each one a sentence rather than a range:

| rule | value | source |
| --- | --- | --- |
| **used bay = 2 × structural bay** (Dong) | the 7间屋 at 平流寨 is **14 structural bays** | [S1], academic survey |
| bedroom clear height vs hearth room | **2–2.5 m** vs **> 3.5 m with no ceiling** | [S4] |
| the dimensions a 掌墨 sets | **column height, bay width, depth, eave projection** — and nothing else | [S4] |
| the drum tower's plan and frame | even-sided polygon (4/6/8), **4 inner + 12 outer posts**, odd eave count, section contracting inward per eave | [S1] |

And the best-measured structure in the slice, which is a *settlement* piece rather than a dwelling:
程阳风雨桥, deck **77 × 3.8 m**, four spans of **14.8 m** at **17.3 m** pier spacing, piers **3 % battered**,
five four-tier towers, and a **multi-layer corbelled cantilever bracket** carrying the span.

Roof pitch for the Guizhou types is **`not established`** — the single largest gap in that note — which
is why the profile above is `measured` and not derived from an ethnic-style coefficient.

## The design

Seven headings: **five new constructs** (D1, D2, D3, D4, D7), **one derivation** (D6) — and **one
correction** (D5), a capability this proposal first recorded as missing which the crate already had.
Each names what it buys, the invariant it must keep, and the sentence it refuses to build.

### D1. A roof holds a section, and the section is a fraction **and** a rise

`Roof` gains one field: `profile: Vec<Step>`, each `Step { fraction, rise }` — the fraction of the
slope's plan run from the eave, and the rise above that row's own eave elevation. The list is strictly
increasing in both, every step strictly inside `(0, 1)` in fraction and strictly below the crest in
rise. Expansion turns each authored row into a **polyline** through those steps, and each authored
plane into `profile.len() + 1` **faces**, one per 步架.

Faces and not strips, and the difference is the whole point: **the surface bends at every purlin**, so
the steps of one authored plane are *not* coplanar. Each step is therefore its own planar surface and
its own shading group — a strip list would smooth the normals across the bend and draw a curve where
the building has a crease. `RoofSurface::{Skin, Soffit}` gain a **step** index for that reason, and the
planarity check measures each step rather than the authored plane.

Why a section and not one of its two halves — each spelling fails in a different corner, and the two
failures are why this is one datum and not two:

- **Rises alone fail on a sloping eave.** A cut "at elevation e" lands at a different fraction on two
  rows whose spans differ, so the cut line stops being parallel to the rows and the sub-quad stops
  being planar — and a level ridge with a level eave hides this completely, because then the two
  profiles are the same up to a change of parameter.
- **Fractions alone have no heights.** A fraction says where the purlin stands along the run; the
  rise says how high. 举架 is the pair — each 步架 has a width and a rise, and the roof's concavity is
  the ratio changing between them.
- **The pair is also what keeps every step planar.** Rows of one plane are parallel in the ordinary
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

Two things it buys without a line of new code, both measured from the crate's own structure:

- **The bargeboard.** The fascia band is derived from the **skin's own boundary** and not from the
  eave loop (`mesh.rs:162`), so a bent gable edge — a row that is now a polyline — makes the band bend
  with it: that band *is* the 博风板 a 悬山 hangs on its gable. The same derivation is why a gable's
  end is already closed today, and it keeps closing it.
- **The weld.** `expand_roof` already welds a crest by its index and an eave station by its normalised
  bits **across planes** (`expand.rs:3111-3150`), so a step vertex keyed by its row's own identity and
  its step index lands in exactly the same map and the same rule. The steps are watertight because
  the roof's three keys are, not because two constructions agree.

One companion parameter, because the covering changes the roof and not only its colour:
`Roof::covering` is an enum of 小青瓦 / 石板瓦 / 杉皮. Slate has **no drainage channels and no ridge
tile** — its slabs lap into the curve themselves (Guizhou note §Verdict, [S21]) — so a slate roof's
ridge is a bare joint where a tiled one carries a roll, and slate is heavier, so the purlin spacing
follows the covering (§5.1, [S9]). Two of those three facts are shape, and a shader cannot state
either.

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

### D3. A member is a prism between two anchored points

`Member { from: Anchor, to: Anchor, across: Vec2, width: f32, depth: f32, host: Option<SolidId> }`,
where

```
Anchor = Free { at: Vec3 }
       | OnMember { member: MemberId, t: f32 }          // a station along that member's axis
       | OnFace   { face: FaceRef, station: f32, elevation: f32 }
```

Six faces, all planar by the same argument as a slab's: four sides are rectangles and the two ends are
the section. `across` is the horizontal direction the section's `width` runs, which is what makes a
vertical column's orientation authored rather than undefined (`up × axis` is zero for a vertical
member, and a silent default there is a section that spins with the wind).

**The anchors are the crate's own correspondence rule, applied to members.** A 檩 does not rest near a
柱 — it passes through a mortise at a named place, and a 穿枋 threads a row of columns at one
elevation. A member built from two `Free` points and eyeballed to touch a column is exactly the
"two independent constructions that have to agree by luck" this crate exists to refuse: the first edit
that moves the column separates the tie, and the picture keeps drawing two solids that no longer meet.
With anchors, moving the column **moves** the tie, because the tie's end *is* a station on the column.

`host` is the authorization a roof already needs: a member embedded in a wall overlaps it, and two
solids may overlap only when the overlap is authored. An anchor `OnMember` is itself that
authorization for the pair it joins — the joint is the statement.

What it buys: 柱, 穿枋, 挑枋, 撑弓, 檩, 椽, a railing, a stair's stringers, and the 吊脚柱 of D4 — one
construct, because every one of them is a prism over a segment. It also buys the **palette**: a material
belongs to an instance and not to a face (`gallery::house`'s own comment), so dark timber against a
whitewashed wall is not a colour on a face — it is a member and a wall being two solids.

Refusals: `MemberAxisDegenerate` (an axis of no length, or an `across` parallel to it, which collapses
the section), `MemberAnchorUnresolved` (an anchor naming a member that is not there, or a `t` outside
`[0, 1]`, or a face whose loop cannot answer the station), and `MemberAnchorCycle` (two members whose
anchors need each other: the expansion resolves anchors in dependency order, so a cycle is a name
rather than a stack overflow).

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

### D5. The intermediate floor is not missing, and this proposal said it was

`derive::floor_slab` is called by `authored` at 2.7 m (`authored.rs:153`) and by `workshop`
(`workshop.rs:301`), so the two-storey building the vernacular needs is **already a document**. An
earlier draft of this section recorded it as "exists and nothing draws it", inferred from the fact
that `Slab::floor` has one call site — and the inference was wrong for the reason this repository
keeps re-learning: **the constructor is called inside the derivation, and the derivation is what the
scenes call.** A count of one site was a count of one construction, not of no users.

What stays true: a storey is a grid level plus a floor, and the *non-uniform* storey — the hearth bay
with no ceiling over it — is D7, which is a hole rather than a height.

### D6. A starting point, in `derive`

`derive::starter` offers `RoofStyle::{Hip, Gable, Flat}`. The vernacular level adds one derivation:
a **three-bay, one-or-two-storey timber house** with a purlin-profiled 悬山 roof, deep eaves, gable
frames that carry their columns and ties, and a plinth — every element an ordinary one, as the
module's contract demands.

It carries two rules that are rules and not numbers: the **used bay is two structural bays**
(Guizhou note §Verdict, [S1]), so the frame is laid out on one grid and the rooms on twice it; and the
frame is **translated vertically** as one object, which is what makes each post's length a terrain
query rather than a design decision (§Verdict, [S1]).

The point is not convenience. A derived element that carries no guarantee and is immediately editable
is how this crate makes a new architecture *authorable* by an agent in one call, and how the gallery
gets a scene whose document is 60 lines instead of 600.

### D7. A slab can have a hole

The double-height hearth bay — clear height **over 3.5 m with no ceiling** while the bedrooms beside
it are **2–2.5 m** (Guizhou note §Verdict, [S4]) — is a floor with a slot in it, and so is a
stairwell. `Slab` is a level prism over *one* plan polygon (`slab.rs:88`), so a floor with a hole
cannot be authored.

The hole is small to add because the crate already builds an annulus: `fill::triangles` fills two
convex loops, and it is what makes a courtyard's ring closed. A `Slab::voids` list gives the top and
bottom caps an inner loop each, and two inner side bands — the same four surfaces a courtyard ring
has, without the wall. The one constraint `fill` already states is convexity, which a hearth slot and
a stairwell satisfy.

What it buys: a hall that is tall because the floor above it has a hole, which is how a 火塘 room is
tall; and a stair that arrives through the floor rather than beside it. It is also the cheapest
possible answer to "the storeys are not uniform", which is the last thing in the Guizhou note's list
of ten.

### Table: what each move costs

| Move | New fields | Code that changes | The gate it must pass |
| --- | --- | --- | --- |
| D1 profile | `Roof::profile` | `expand` (row polylines, one face per step), `roof` (accessors), `mesh` (`RoofSurface` gains a step), `validate` (three refusals) | a probe that every step is planar and the steps tile the loop |
| D2 crowns | `Mass::tops`, `Wall::crown` | `expand` (ring top strips, wall faces), `validate` | a probe that a crowned ring is closed and planar |
| D3 members | `Member` with `Anchor`, a list in the document, a `SolidId` | `expand` (anchors resolved in dependency order), `mesh` (element), `identity`, `validate` | a probe that a member is closed and outward, that two members overlapping without a host are reported, and that moving a column moves the tie anchored on it |
| D4 stilts | a seating mode | `site` | a probe on an analytic slope: posts land on the ground, the floor stays level |
| D5 floors | none — already there | nothing | already gated: two scenes draw one |
| D6 starter | none | `derive` | the starter builds and its roof's profile measures the profile |
| D7 slab holes | `Slab::voids` | `expand` (caps, inner bands), `validate` | a probe that a holed slab is closed and its annulus area is the difference |

### What this design does not add, and why

Naming these matters more than adding them, because each is the kind of thing an arc quietly promises
and then cannot ship:

1. **A semantic space layer** — 堂屋, 火塘, 廊, 美人靠, 吞口, 阁楼. These are *named places* that are not
   solids, and the Guizhou note lists them as a construct (§5.2 item 9) because several cannot be
   derived from the geometry: the veranda is the largest space in a Dong plan, and the grain storey is
   staggered against the rooms below it. This crate's named place is the **bay**, and a room is a set
   of bays. So the smallest honest form is a *named set of bays*, not a new kind of solid — and it
   should be added when a scene needs to hang something on "the hearth bay" rather than on bay
   `(1, 2)`. Until then, a scene names its bays by comment.
2. **A settlement generator.** 鼓楼, 风雨桥, 粮仓, 禾晾 and a street's frontage are placed by village
   rules, and the Guizhou note's own §5.1 lists them under a separate heading. A 鼓楼 is *authorable*
   from what this design adds — an octagonal mass stack, one roof per eave, posts as members — but
   *placing* it is a different layer, and F6 keeps it out of this proposal.
3. **A corbel as a construct.** 程阳桥's multi-layer cantilever bracket is a stack of members with
   anchors, one per layer, each reaching further out; it needs the `Member` construct and nothing else.
   A `derive::corbel` is a convenience for a later arc, not a capability.
4. **The covering's laying law.** Tile courses, corbelling, slate laps and their curvature are the
   shader's (F4). The `Covering` enum carries only the two facts that are *shape*: whether the ridge
   carries a roll, and the minimum pitch the covering needs.
5. **A frame graph that validates the load path.** D3's anchors make pairwise joints; nothing checks
   that every purlin is carried by a post or that a cantilever is balanced. That is a structurally
   different instrument — a graph with a carried/carrier relation — and it is the natural *next*
   proposal if the frame turns out to need validation rather than only drawing.

## The forks, for the owner

Each is a real choice with a cost somewhere; none is "obviously right".

**F1. The roof's profile — three ways, one recommendation.**
- (a) *A section of fraction and rise, cut into planar steps* (D1). Keeps planarity, keeps
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
- (a) *`Member` as a first-class solid, with anchored joints* (D3). The frame is built, cast, shadowed
  and named; moving a column moves the tie that meets it, because the joint is a station rather than a
  coincidence. And it is a new element class, so it is a new material — the palette follows for free.
  Costs: anchor resolution in dependency order, and three refusals to write and gate.
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
三坊一照壁 is the richest and tests D1, D2 and D7 at once; a 古镇 street tests the settlement scale.
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
5. **Arc 5 — the compound (D1, D2, D7).** 三坊一照壁 or 四合五天井: wings, galleries, a 照壁, 天井.
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

About the model:

- Whether the crate's planarity tolerance holds a *crowned* mass's top strip at a corner. The
  geometry says a sloped top strip over a straight run is planar (four points on one tilted plane);
  the miter at a corner, where two crowned edges meet at different slopes, has not been computed.
- Whether a 举架 profile is the same for every plane of a hip roof. A gable's two slopes have the
  same 步架 count and the same profile; a hip's short ends have fewer, and D1 answers by refusing the
  fan rather than by having measured it.

About the rules, inherited from the notes and not to be invented later:

- **举折's closing rule.** The halving series `0.1H·2^−(i−1)` sums to at most `0.2H` and cannot
  produce the total rise; the rule that does is not in the primary text, and the commentaries that
  would settle it were not obtained. **Do not implement 举折 as a profile rule** until they are.
- **提栈.** The 《营造法原》 text was not obtained at all: no coefficients, no range. The south-west
  descends from it, which makes this the most consequential single gap in the four notes.
- **The full Qing ladder.** 五举, 六举 and 八举 are attested and arithmetically verified; the complete
  five-step ladder is repeated in the general literature and has no primary source in this session.
- **营造尺 and 鲁班尺 in metres.** The one document giving both a 尺 and a millimetre figure is
  self-inconsistent (314 mm against 309 mm). Every official dimension above is therefore in units of
  the 尺, not in metres.
- **Whether a south-western 穿斗 roof has a 望板 layer** under the tile, which decides whether a
  rafter is visible from below and so whether F4's "rafters under the eave" is the right line.
- **Vernacular dimensions.** The session obtained exactly one measured dataset for a vernacular
  dwelling (桥梁民居, 黔江, 1916: 进深 8.11–11.55 m). 开间, 柱径, 椽档, 出檐 and tile sizes are not
  established beyond two sites, and the Guizhou note records roof pitch as not established for
  **every** type in its slice. The first scene's dimensions will be authored, and the scene must say
  so rather than borrow the official ratios quietly.

About the research itself:

- All four notes were briefed with a premise this proposal has since corrected ("every roof is a flat
  slab"). Their sections on what breaks are sound; their *vocabulary* for the roof is one step behind
  the code, and a reader of the notes alone would think the crate has no pitched roof at all.

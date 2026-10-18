# A Chinese vernacular level: the four assumptions the building crate has to give up

Status: design proposal, 2026-10-18. It rests on four research notes in
[`investigate/`](../investigate/): [chongqing-stilt-houses.md](../investigate/chongqing-stilt-houses.md),
[yunnan-dwellings.md](../investigate/yunnan-dwellings.md), [guizhou-dwellings.md](../investigate/guizhou-dwellings.md),
[chinese-timber-frame-and-roof.md](../investigate/chinese-timber-frame-and-roof.md) — which answer
separate questions and are, per the repository's rule, not a basis for implementation on their own.
No code has been written against this document.

## The owner's decisions, 2026-10-18

Four answers, taken before any code, and they decide the shape of this document rather than its taste:

| fork | the owner chose | what it commits this proposal to |
| --- | --- | --- |
| the roof's profile (F1) | **a section per slope, folded into planar steps** | D1 as written; planarity is kept and the roof stays a chain of planes |
| how much structure is geometry (F2, F3) | **members with anchored joints *and* crowned walls** | D2 and D3, both; the frame is a solid and a gable wall can meet a roof |
| the first building (F5) | **the 吊脚楼** | the road is one scene in stages, starting at the site |
| the covering (F4) | **geometric tile** — not a texture | a new construct, D9; and the face budget to go with it |

The fourth is the one that went against my recommendation, and the numbers in D9 are why it is
defensible rather than merely brave: a tile course is the same size on screen as the members D2 and D3
already draw, so a covering that is built is a covering that reads.

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
| 坊 as an instance placed in the plan | `Placement` targets a face bay | `Placement::Free` (a position and a yaw) |
| 封火墙 passing through the roof | `Wall` has no host | `Wall::host` |
| two slopes pitched differently (傣楼, 一颗印 second tier) | one slope per plane | `RoofPlane::profile`, per plane |
| 楼板开洞 (火塘通高, 楼梯井) | `Slab` has one plan polygon | `Slab::voids`, filled by `fill` |
| 屋面覆盖 (小青瓦, 石板瓦, 杉皮) | a surface has a colour | **D9** — owner's decision, 2026-10-18 |
| 使用开间 = 2 × 结构开间 | one grid per face | a ratio in the starter, not a construct |
| 天井, 院落, 厢房, 走马廊 | void ring, deep eaves, hosted slabs | nothing — authoring |
| 瓦垄, 瓦当, 滴水 | nothing | a shader question, not geometry — see the forks |
| 台基, 踏步, 柱础, 石砌勒脚 | `Slab`, `Part`, `Wall` | nothing — authoring |
| 夯土收分 (a battered wall) | one thickness per wall | **named non-goal** — see below |
| a different infill material per bay | a material belongs to a solid | **named non-goal** — see below |

## What the buildings ask for

All four notes are in. What they change is not the design — the six constructs and the two completions
below were already the right shape — but three of the numbers, and two of the constructs (D8, and the
per-plane profile in D1) came out of them.

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

### The Chongqing house: a 4.5 m bay over a purlin every 1.0–1.3 m

| quantity | value | source |
| --- | --- | --- |
| bay 开间 | ≈ **4.5 m** ordinary, **5.5 m** in an ancestral hall, minimum ≈ 3 m | [S1], academic |
| depth-direction column spacing | **0.9–1.3 m** — the purlin spacing, and so the 步架 width | [S1] |
| 落地柱 / 吊脚柱 diameter | **20–25 cm** / ≈ **20 cm**, slenderness over **1:30** against the official **1:11** | [S1] |
| roof pitch | **四分水 0.4:1 (≈22°)** to **五分水 0.5:1 (≈27°)** | [S1] |
| eave projection 出檐 | front and gallery ≈ **3 m**, sides **> 1 m**, gable sides **> 1.5 m** | [S1] |
| 椽 | 10 × 3 cm; spacing given as 1:1.5, and the same source elsewhere as 250 cm — **it contradicts itself** | [S1] |

Two things this note records as negatives that matter to the road: the house is a **穿斗 column grid on
independent stone pads at individually chosen heights**, so "the ground is fitted to the building"
rather than the other way round; and **no source gives a stilt's count, height or spacing** — the type
is named for its stilts and the one number the literature does not print is the stilt.

Its town-scale figures are the only statutory ones obtained: the 磁器口 2020 flood peaked at
**192.65 m** and ran **150 m up 正街**, and new work in the 洪安 protected zone is bounded at **9 m**.

### The Yunnan house: the module is a wing, not a bay

| quantity | value | source |
| --- | --- | --- |
| the module | **the 坊** — a three-bay, two-storey wing; 三坊一照壁 is three of them plus a 照壁, 四合五天井 is four around four corner skywells | [Y2], [Y6] |
| 厦子 (front gallery) depth | **1.8–2.0 m**, measured centre to centre from the front eave column to the 金柱 — the only framework-internal module found | [Y1] |
| Bai bays / 堂屋 / depth | **3.3–3.6 m** / **3.6–3.8 m** / **5–6 m** | [Y1] |
| 一颗印 | **12 × 15 m** site, **3 m** square 天井 — **≈5 %** of the site — and it grows by adding 耳房 while the rectangle stays fixed | [Y3] |
| Dai roof | **45°–60°**, the two slopes pitched **unequally**, 歇山 ends | [Y6] |
| a second tier | 一颗印 and Dai both carry one (腰檐 / 重檐) at a **deliberately different pitch** from the main roof | [Y6] |
| rafter / tile | ⌀60–80 mm at 220–240 mm; the tile module is **两椽支一**, lap **¼–⅓** | [Y6] |

And the sentence that makes the hierarchy in the assumptions section: 建筑物重量全部由构架承担，墙壁只起围护作用
— the whole weight is carried by the frame and the walls are only enclosure ([Y1]).

Negative results worth carrying: **no Naxi number at all** was obtained, 蛮楼 appears in no source read,
and 一颗印's measured dimensions are behind a 1938 monograph the session could not reach.

### What the four together say about the numbers

The pitch range across the four regions is **22° to 60°**, and the same *type* changes pitch between its
first and second tier. That is the strongest argument for D1's `measured` variant: there is no global
coefficient to derive a vernacular roof from, and any single "Chinese pitch" would be wrong somewhere.
The bay range (**3.3 m** in Dali to **4.5 m** on the Jialing) and the depth range (**5–6 m** to
**11.55 m**) say the same thing about plan dimensions, and the one thing every source agrees on is the
*rule* — column spacing sets the purlin spacing, and the used bay is twice the structural bay.

## The design

Nine headings: **seven new constructs** (D1, D2, D3, D4, D7, D8, D9), **one derivation** (D6) — and
**one correction** (D5), a capability this proposal first recorded as missing which the crate already
had. Each names what it buys, the invariant it must keep, and the sentence it refuses to build.

### D1. A roof plane holds a section, and the section is a fraction **and** a rise

`RoofPlane` gains one field: `profile: Vec<Step>`, each `Step { fraction, rise }` — the fraction of the
slope's plan run from the eave, and the rise above that row's own eave elevation. The list is strictly
increasing in both, every step strictly inside `(0, 1)` in fraction and strictly below the crest in
rise. Expansion turns each authored row into a **polyline** through those steps, and each authored
plane into `profile.len() + 1` **faces**, one per 步架.

**On the plane and not on the roof**, because a slope's 步架 count and its rises belong to that slope:
the two sides of a Dai house are pitched differently and the 一颗印's second tier deliberately takes a
different pitch from the first. Two planes of one gable share the crest *vertices* and no row at all —
a row is (crest index, eave station) and the two slopes name different stations — so independent
profiles cannot disagree about a shared row, and the watertightness below is the crate's existing weld
rather than a new agreement.

Faces and not strips, and the difference is the whole point: **the surface bends at every purlin**, so
the steps of one authored plane are *not* coplanar. Each step is therefore its own planar surface and
its own shading group — a strip list would smooth the normals across the bend and draw a curve where
the building has a crease. `RoofSurface::{Skin, Soffit}` gain a **step** index for that reason, and the
planarity check measures each step rather than the authored plane.

Why a section and not one of its two halves — each spelling fails in a different corner, and the two
failures are why this is one datum and not two:

- **Rises alone fail on a sloping eave.** A cut "at elevation e" lands at a different fraction on two
  rows whose spans differ, so the cut line stops being parallel to the rows and the step stops being
  planar — and a level ridge with a level eave hides this completely, because then the two profiles
  are the same up to a change of parameter.
- **Fractions alone have no heights.** A fraction says where the purlin stands along the run; the
  rise says how high. 举架 is the pair — each 步架 has a width and a rise, and the roof's concavity is
  the ratio changing between them.
- **The pair keeps every step planar, on both shapes a plane comes in.** A trapezoid strip (a gable's
  side, a shed) has parallel rows, so a step's two segments are parallel and the quad between them is
  planar; a **fan** (a hip's triangular end, rows meeting at one crest point) has four points that all
  lie in the plane the fan itself spans. Neither needs a tolerance. An earlier draft of this section
  refused a profile on a fan; that was wrong — a fan with a level eave gives level purlins, and it
  takes a profile without a second rule.
- **Watertightness owes the profile nothing.** The vertices a step adds are identified by *(the row's
  own identity, the step index)*, so a row named by two planes carries every step vertex once, by the
  crate's existing welding rule, not by a coordinate test.

And what the row's polyline means physically: the roof surface bends at every purlin. It is not one
slope with marks on it — it is a chain of planes, which is what a 举架 roof *is*.

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

The covering used to sit here as a companion parameter — a `Covering` enum with a ridge roll or without.
The owner's decision of 2026-10-18 made it geometry instead, so it is D9, and what stays here is only the
fact the profile needs: **the covering sets a minimum pitch and a purlin spacing** (slate is heavier than
tile), so a roof's steps and its covering are chosen together.

Refusals: `ProfileOutOfOrder` (a fraction not strictly increasing inside `(0, 1)`, or a rise not
strictly increasing above the eave), `ProfileReachesTheCrest` (a step at or above the crest's own
rise), and the existing `SolidOpen` if two planes still disagree.

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

A party wall does more than stand taller. A 封火墙 rises above its own ridge and **passes through the
roof's eave** on its way there, so `Wall` needs the same authorization a roof already has —
`Wall::host: Option<SolidId>` — or that overlap is reported, correctly, as two solids standing in one
another. One field, and without it the party wall is the one wall a market town cannot have.

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

### D8. A wing is placed in the plan, not in a bay

A `Placement` names a template, a face, a bay `(row, column)` and a pin (`parts.rs:250`), so a part is
**attached to a wall**. A courtyard house is the other way round: its module is the **坊** — a whole
three-bay, two-storey wing — and 三坊一照壁 is three of them plus a screen wall, while a 一颗印 grows by
adding or removing a 耳房 with its site rectangle unchanged (Yunnan note §Verdict, [Y2]). A wing has no
host wall to hang on; it stands beside the court.

So `Placement` gains a second target: `Placement::Free { template, at: Vec2, elevation: f32, yaw: f32 }`.
The yaw, and only the yaw, keeps the invariant that a part's frame is a rotation; two wings around a
court do not overlap, so nothing needs authorizing, and the court between them is simply plan space
that no mass occupies.

What it buys: the courtyard as an **arrangement** rather than as a subtraction — which is what both the
Yunnan and the Chongqing notes say the type actually is. The void ring stays, because a 一颗印's 天井 *is*
a hole cut into one mass; but a 三坊一照壁 is four objects around a gap, and today it is not authorable
without writing the same wing's document three times.

Refusals: none new — a yaw is a number, and a template with no masses is already a `Template` fault.

### D9. The covering is geometry — one construct, periodic in one of two directions

The owner chose geometric tile over a texture. The construct that makes that affordable is **one prism
per period, not one per tile**, and it covers both materials this arc needs because **they are periodic
in opposite directions**:

```
Covering { direction: Across | Down, period: f32, section: Vec<Vec2>, lap: f32 }
```

- 小青瓦 is periodic **across** the slope: a field of ridges (瓦垄) running from ridge to eave, with a
  concave channel between them. `Across` at 0.20 m over a 10 m plane is **50 ridges**.
- 石板瓦 is periodic **down** the slope: courses laid across the slope and lapped, with **no channels and
  no ridge roll** (Guizhou note §Verdict, [S21]). `Down` at a 0.30 m course is **10 courses**, each a
  thin plate with a lap.

Either way each period is laid **one prism per step**, because the skin bends at every purlin (D1): a
three-step plane is `periods × 4` prisms, each an ordinary closed prism with six planar faces. Fifty
ridges on a three-step slope is 200 solids and about 1 200 faces — two orders of magnitude below one
solid per tile, and the same order as the members the frame already draws, which is exactly the size at
which the choice pays for itself.

Three things follow, and the first two are invariants rather than conveniences:

- **The covering is hosted by the roof.** A ridge sits *on* the skin and overlaps it, so the overlap is
  authored the way a member's is (`host`), and `validate` does not read 200 prisms standing in a plane.
- **The eave end is the picture.** 瓦当 and 滴水 are about 12 px at these frames, which is the size that
  reads. The first covering arc decides whether the ridge's own end face is enough or a cap part is
  worth its solids; the slate covering has no cap at all, and that difference is the covering's, not the
  roof's.
- **Tiles are their own element class**, so they are their own material: the skin under them can be a
  dark mortar colour beneath a lighter tile field, which is how a real eave reads.

Refusals: `CoveringOutOfPhase` (a period that does not divide the plane's width or its run, so the field
would end in half a ridge), and `CoveringThinnerThanItsLap` (a lap longer than the course).

### Table: what each move costs

| Move | New fields | Code that changes | The gate it must pass |
| --- | --- | --- | --- |
| D1 profile | `RoofPlane::profile` | `expand` (row polylines, one face per step), `roof` (accessors), `mesh` (`RoofSurface` gains a step), `validate` (two refusals) | a probe that every step is planar and the steps tile the loop |
| D2 crowns | `Mass::tops`, `Wall::crown` | `expand` (ring top strips, wall faces), `validate` | a probe that a crowned ring is closed and planar |
| D3 members | `Member` with `Anchor`, a list in the document, a `SolidId` | `expand` (anchors resolved in dependency order), `mesh` (element), `identity`, `validate` | a probe that a member is closed and outward, that two members overlapping without a host are reported, and that moving a column moves the tie anchored on it |
| D4 stilts | a seating mode | `site` | a probe on an analytic slope: posts land on the ground, the floor stays level |
| D5 floors | none — already there | nothing | already gated: two scenes draw one |
| D6 starter | none | `derive` | the starter builds and its roof's profile measures the profile |
| D7 slab holes | `Slab::voids` | `expand` (caps, inner bands), `validate` | a probe that a holed slab is closed and its annulus area is the difference |
| D8 free placement | `Placement::Free` | `parts`, `skeleton` (resolution), `expand` | a probe that two free-placed wings land where their yaw says and neither moves when a face is re-split |
| D9 covering | `Covering`, a list in the document, a `SolidId` | `expand`, `mesh` (element, and a material class of its own), `identity` | a probe that a 50-ridge field is 50 closed prisms per step and that a half-period field is refused |

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
4. **The covering's laying law** — *crossed off on 2026-10-18*. The owner chose geometric tile, so the
   laying law is D9: one prism per period, `Across` for 小青瓦, `Down` for 石板瓦. What is still not
   modelled is the *corrugation's* curvature — a ridge is a section given as a polygon, so a round tile
   is a polygon approximation, which is what a planar-facet mesh is.
5. **A frame graph that validates the load path.** D3's anchors make pairwise joints; nothing checks
   that every purlin is carried by a post or that a cantilever is balanced. That is a structurally
   different instrument — a graph with a carried/carrier relation — and it is the natural *next*
   proposal if the frame turns out to need validation rather than only drawing.
6. **A battered wall.** 夯土 thickness goes from **0.6–0.7 m at the base to 0.4–0.5 m** at the top
   (Yunnan note, [Y6] measured), and `Mass::wall` is one number. A per-node *outer* outline could
   express it by leaning the outline in — the wall is offset inward from the outline, so a leaning
   outline leans the outer face. That is a cheap trick and it changes what the outline means, so it is
   named here and not proposed: the stone and timber houses this arc starts with are vertical.
7. **A different material per bay.** A material belongs to an instance and not to a face
   (`gallery::house`), so a wall that is board in one bay and rubble in the next needs one solid per
   bay. Two wings of one house are already two solids; one *wall* with two materials is not. Named,
   not proposed: split the wall into masses if a scene needs it.
8. **Joints beyond a pair.** D3's anchors join two members at a station; a mortise is a notch in one
   member that another passes *through*, and "notched" against "butted" is a distinction the Yunnan
   note asks for ([Y9]). A joint type is a later vocabulary, for when a drawing must say which.

## The forks, and what the owner chose

Each is a real choice with a cost somewhere; none is "obviously right". Five were answered on
2026-10-18 — F1 **(a)**, F2 **(a)**, F3 **(a)**, F4 **geometric**, F5 **the 吊脚楼** — and the reasoning
stays because the rejected option is the record of what this design is not. F6 is still open.

**F1. The roof's profile — three ways.**
- **(a) A section of fraction and rise, cut into planar steps** (D1). *Chosen.* Keeps planarity, keeps
  watertightness, and matches how the roof is built (a 举架 roof is a chain of planes, not a curve).
  Costs: every row becomes a polyline, and a 起翘 corner is a separate piece.
- (b) *A curved, tessellated roof surface.* Truer to a tile surface, and it breaks the crate's
  one-face-one-plane contract, its shading groups, and the fascia derivation — three invariants for
  one silhouette.
- (c) *A silhouette in the shader.* Cheapest; the roof would cast the shadow of a tent, the id channel
  would name a plane that is not there, and the picture would disagree with the document.

**F2. The crowned wall top — three ways.**
- **(a) Per-node tops on `Mass` and a profile on `Wall`** (D2). *Chosen.* One rule, no new solid, and
  every gable form is authored. Costs: the ring walk's top strip stops being level, which touches the
  most exercised code in the crate.
- (b) *Crown walls as separate `Wall` solids stacked on the mass*, with an authored joint. Smaller
  change; every gable becomes two solids, every joint needs authorizing, and a wall that overlaps the
  mass is the overlap the validator reports.
- (c) *Deep eaves only.* No change at all; 硬山, 马头墙 and the parapet are refused, and every roof in
  the gallery is 悬山 with a wall that stops short — which is a real Chinese roof, and only one.

**F3. The timber frame — three ways.**
- **(a) `Member` as a first-class solid, with anchored joints** (D3). *Chosen.* The frame is built, cast,
  shadowed and named; moving a column moves the tie that meets it, because the joint is a station rather
  than a coincidence. And it is a new element class, so it is a new material — the palette follows for
  free. Costs: anchor resolution in dependency order, and three refusals to write and gate.
- (b) *Members as templates of masses.* The existing `Part` mechanism can place a column, because a
  column is a small mass. A **diagonal** is impossible — a mass is a prismatic shell standing on its
  outline — so 撑弓, 斜撑 and 椽 are out, and so is every rafter.
- (c) *Texture only.* No geometry; the frame is a colour. The id channel cannot name a column, so
  "move the third column" is not a sentence this system can answer.

**F4. How deep the detail goes — and the tile was not a detail.** *Decided: geometric.* Columns, ties,
cantilevers, brackets and purlins are geometry; rafters are geometry **only under the eave**, where a
Chinese roof's underside is the picture; and the covering is D9.

The reasoning that made it a fork is worth keeping, because it is the argument the arc will be measured
against: the Yunnan note says "every one of these roofs is read as a texture of discrete units; that is
most of what a camera sees" (§"constructs", item 6), and the Guizhou note's slate roof has **no channels
and no ridge tile** — a different picture from the same geometry. At 1024 px a tile course is 8–12 px.
I recommended a texture and the owner chose the geometry; D9's per-period prism is what makes the choice
affordable, and the probe that will say whether it was right is the face count and the frame time of the
first covering arc.

**F5. Which building first.** *Decided: the 吊脚楼.* The owner's first-named type and the one that tests
D4; 三坊一照壁 is the richest and waits for Arc 7.

**F6. Scope of the old town — still open.** A street of houses is not a building: it needs a *settlement*
layer — frontage, blind walls on party lines, the arcade, the slope of the street, water. That layer does
not exist in this workspace (the gallery draws one building per scene). This proposal stops at the
building; the settlement is a separate question and should be named before an arc promises it.

## The road

The owner chose the **吊脚楼** first, and chose the frame, the crowns and the tiles as real geometry. So
the road is **one scene built in stages**, not seven scenes in sequence: every arc adds one construct to
the *same* building, re-pins its hash, and by the sixth arc the scene is the type.

The gate cost is real and known: creating the scene costs a `.png` hash pin in `[6/8]`, a pass-count row
in `crates/scene/tests/clustered_lights.rs`, and — when its id channel first exists — rows in
`scene_identity_probe`'s table that `[7/8]` checks two ways. Every later arc re-pins the hash and updates
the pass counts, which is the cheap half of the same list.

1. **Arc 1 — the ground is a query (D4).** *Done, 2026-10-18.* A three-bay body at the uphill grade, posts down to a sampled
   slope, a plinth on the uphill side, a lower storey that is half open. The roof is the crate's existing
   hip and the body is a box, because *this arc is the site*: the 吊脚 silhouette exists before any
   Chinese detail does, and the scene exists to be improved.
2. **Arc 2 — the profile (D1).** *Done, 2026-10-18 — see below.* The hip becomes a 悬山 whose two planes carry a 举架 section, with a deep
   eave. The straight tent becomes a Chinese roof, and the bargeboard arrives with it.
3. **Arc 3 — the crown (D2).** Gable walls that reach the roof (硬山) and one party wall that passes
   through the eave (封火墙). This is the arc after which the house reads from the side.
4. **Arc 4 — the frame (D3).** Columns under every purlin, ties threading them, cantilevers carrying the
   front gallery, brackets under the cantilevers — anchored, so moving a column moves its tie.
5. **Arc 5 — the covering (D9).** Fifty 瓦垄 a slope, hosted by the roof, with the eave end decided.
6. **Arc 6 — the interior (D7).** The hearth bay with no ceiling over it, and the stair through the floor.
7. **Arc 7 — the compound (D8).** A second scene: 三坊一照壁, the arc that proves D8 was needed at all.
8. **Arc 8 — the town.** Only if F6 is answered yes; the settlement layer does not exist yet.

### Arc 1 is closed, with its numbers

`site::posts(skeleton, ground, spacing, section, embed)` appends one `Wall` run per post under a seated
mass and moves nothing, and `crates/gallery/src/stilt.rs` draws it. Measured:

- **the site**: 17 posts on a 35 % bank, lengths **0.2550 .. 3.2550 m**, feet **0.00e0 m** from the
  analytic ground, **0** posts uphill of where the ground reaches the floor, **0** on a flat site, and
  11 of 22 refused on a ground that answers on one side only;
- **the picture**: **1152 × 768**, silhouette **693 × 589 px (1.177)**, hash `64685171182193EC`;
- **the suite**: the gallery's hash pins are **ten**, `scene_identity_probe` holds **975** rows and the
  channel is two-way for every scene, and the pass counts are five in each mesh stage — one for each of
  the four classes a building scene draws plus the posts.

Two findings the picture produced, both worth carrying into later arcs:

1. **A plate's rim is coplanar with the wall's inner face.** The starter seats the plate on the inner
   loop on purpose; a camera below the floor therefore sees two coincident planes fighting. The scene
   draws the plate's two caps and lets the wall be the rim. The crate-level fix — a plate inset by a
   millimetre — moves every pinned hash and waits for an arc that can re-pin them.
2. **A member whose top meets another solid's face must be embedded in it.** A post butted exactly
   against the plate's underside put two faces at one depth and the first render combed along the edge;
   `0.02 m` of tenon is what fixed it, and it is a parameter rather than a constant.

### Arc 2 is closed, and the nine pinned pictures did not move

`RoofPlane` has a `profile: Vec<Step>` — a fraction of the slope's plan run and a rise above the
row's own eave — and expansion cuts each authored band into `profile.len() + 1` strips. Each strip is
its own `RoofSurface::Skin { plane, step }`, because the surface **bends** at every purlin and a
shading group that spanned the bend would draw the curve the architecture does not have.
`Roof::profile_fault` is one reading with three names: `expand`'s problem, `validate`'s
`ProfileOutOfOrder` and `ProfileReachesTheCrest`, and the probe's.

Measured on `roof_profile_probe`, a 10 × 6 gable with four 步架 of 0.9 m and the coefficients the Qing
worked example attests:

| number | value |
| --- | --- |
| the ladder | **五举 0.500 (26.57°), 六举 0.600 (30.96°), 七五举 0.750 (36.87°), 八举 0.800 (38.66°)** — steepening to the ridge, which is the whole of 举架 |
| the ridge's height | **2.385 m** above the eave = `0.9 × (0.5 + 0.6 + 0.75 + 0.8)`, so the profile places the ridge and not the other way round |
| the strips | **8** over two planes, **16 skin triangles** against 4 for one straight slope |
| planarity | the largest departure from a plane is **2.35e-8 m** |
| the tiling | the skin's plan area equals the eave loop's, gap **0.00e0** |
| the purlins | **12** mesh points stand exactly at an eave plus a step's rise |
| the solid | **68 faces, euler 2, manifold, 17.740812 m³, 0 violations** |
| the refusals | `ProfileOutOfOrder { step: 1 }` and `ProfileReachesTheCrest { step: 0 }`, each also reaching `expand` as `ProfileFault` |

**The finding worth carrying: the nine pinned pictures did not move.** The audit's other nine hashes
are byte-identical with the profile in the crate, because a plane without a profile emits exactly the
quads it emitted before — the construct is a strict extension, and the only picture that changed is the
one scene that asked for a 举架 roof (`stilt`, `F8630F6784B70585`). Getting there cost one correction:
the first version walked each strip's corners from the eave up, which put the quad's diagonal on the
other pair of corners and moved **all ten** hashes for a difference no viewer can see. The corners run
crest-to-eave in the authored rows' order, and the nine came back.

## What would make this proposal wrong

- If a roof needs a **hip's dress** — 推山, where the hip line is pushed back and the end plane is not
  the simple fan this crate derives — D1 is not the problem and the crate's hip is: its end plane is a
  fan from the ridge's end point, which is a pyramidal hip. The vernacular south-west is overwhelmingly
  悬山/硬山, so this is a boundary rather than a hole — but it is a boundary, and it is not measured.
- If a crowned mass's miter at a corner cannot be given one elevation per node without folding a top
  strip, D2 is wrong and (b) is the answer. Not measured; the algebra above says it is fine, and
  algebra is not a probe.
- If a covering's face count or its shadow cost makes a frame unaffordable — 200 prisms a slope, every
  one of them a caster — then D9 is too heavy at the size the owner asked for, and the fallback is the
  texture the device layer already has. Not measured: the numbers the first covering arc prints are the
  ones to judge it by, and that arc should print them whether or not they are bad news.
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

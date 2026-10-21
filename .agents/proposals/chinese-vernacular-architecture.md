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

**F6. Scope of the old town — answered: a layer.** A street of houses is not a building: it needs a *settlement*
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
3. **Arc 3 — the crown (D2).** *Done, 2026-10-18 — see below.* Gable walls that reach the roof (硬山) and one party wall that passes
   through the eave (封火墙). This is the arc after which the house reads from the side.
4. **Arc 4 — the frame (D3).** *Done, 2026-10-18 — see below.* Columns under every purlin, ties threading them, cantilevers carrying the
   front gallery, brackets under the cantilevers — anchored, so moving a column moves its tie.
5. **Arc 5 — the covering (D9).** *Done, 2026-10-18 — see below.* Fifty 瓦垄 a slope, hosted by the roof, with the eave end decided.
6. **Arc 6 — the interior (D7).** *Done, 2026-10-19 — see below.* The hearth bay with no ceiling over it, and the stair through the floor.
7. **Arc 7 — the compound (D8).** *Done, 2026-10-18 — see below.* A second scene: 三坊一照壁, the arc that proves D8 was needed at all.
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

### Arc 3 is closed: the 山墙 rises to its roof, and a warped strip is two planes

`Mass::crowns` is **one elevation per outline node, and empty is the level case** — exact rather than a
row of copies of `top`, the same shape `FaceRef::void` already had. `Mass::top_at` is the one reading
(the ring's `Ring::top_at` is the same line), `expand` lifts the walk's top per node, and `site::seat`
carries a crown with its mass. `stilt`'s gable ends are split at their middles and the two nodes under
the ridge are crowned to **the roof's underside less a centimetre**, so the 山墙 is a triangle that
meets its roof instead of an open band under it.

The finding is what the first measurement produced: **a crowned strip of wall top at a plan corner is
not planar.** The strip's four corners are the edge's two outer nodes and its two inner, mitered ones;
at one elevation any three span a plane the fourth lies in, but a crowned strip is planar only if the
two ends' **offset directions are parallel**, and a rectangle corner's miter is never parallel to a
straight node's perpendicular. The warp is **0.037 m** — 15 % of the 0.24 m band — and it arrives as
four `FaceNotPlanar` violations.

The resolution applies D1's rule to a warp: a bend was its own element, so a **warped strip is its two
triangles with their own elements**, `WallTop { edge, half: 0 }` and `{ edge, half: 1 }`. A level strip
stays one element, so every recorded picture and every recorded name is unchanged — the second half is
named and the first is not.

Measured on `wall_crown_probe`, a 10 × 6 gable with two crowned apexes at 5.27:

| number | value |
| --- | --- |
| the ring | **48 faces, euler 0** (a ring, not a box), manifold, **26.693367 m³**, **0 violations** |
| the wall-top elements | **10** over six nodes, **4** of them the second half of a warped strip |
| planarity | the largest departure of any element is **7.8e-8 m**; the two level edges keep one element and are flat to `0.00e0` |
| the lean | the crowned strips' normals run **0.6869 .. 0.8237 up** — they lean, which is the whole point |
| the apex | the mass reaches **5.2650**, exactly the roof's underside at the ridge |
| the refusals | `CrownCountMismatch`, `CrownBelowBase`, `GridOnACrownedFace`, and `CrownWithACourtyard` for the case the ring cannot express |

**Two boundaries, both by name rather than by silence**: a crowned mass that has a courtyard (a crown is
authored per node of the mass's outline and a courtyard's ring per node of the *void's* loop — two node
counts with no rule between them), and a crowned edge that carries an opening (`cut_edge` reads the
ring's level top at its stations). Both are one arc's worth of work when a scene needs them: the void
ring grows the parameter, and `cut_edge` interpolates the top along its stations.

`stilt`'s picture moved for the first time since the profile: `2DB5EE0D9E0E1297`, and the identity
probe's rows went **999 → 1009** — the four warped strips' second halves and the strips the split
added.

### Arc 4 is closed: the frame, and the 挑廊 it carries

`Member` is the crate's **sixth kind of solid**: one straight piece of frame from one
[`Anchor`](crate::member::Anchor) to another, with a rectangular section swept along its own axis.
`Anchor::Free` is the point as authored; `Anchor::OnMember` is a **fraction of another member's axis**,
which is what a joint is. The anchors are resolved to a fixed point **before anything is expanded**, so
a bracket on a beam lands on the beam's own line rather than at a point that nearly agrees.

`site::posts` emits members now, and that changed the model rather than the picture: a post used to be
a short `Wall` run whose foot followed the ground across its own section, and a real post stands on a
**stone**, which is level. The scene's picture moved by one hash (`F90C153B1605E750`) and the identity
probe's rows did not move at all — a post had six surfaces as a wall and has six as a member.

Measured on `member_probe` — two posts, a beam across their heads, a bracket on the beam:

| number | value |
| --- | --- |
| each member | **12 faces, loops 0, euler 2, manifold** |
| the volume | **0.120000005** against `0.2 × 0.2 × 3` = 0.12; the beam **0.037500013** against 0.0375 — the largest error over four members is **7e-8 m³** |
| the joint | the beam's axis is the post heads' elevation, off by **0.00e0 m** |
| the refusals | `MemberDegenerate` (an axis with no length *and* an `across` along the axis), `MemberNotThick`, `MemberAnchorUnresolved`, `MemberAnchorCycle` (reported once per member of the cycle) |

**What this arc deliberately does not do yet**, and it is the visible half: the **挑廊** is a plate
outside the mass's outline standing on brackets, so it needs `Anchor::OnMember` in a scene and a plate
that `site::posts` did not place. `Anchor::OnFace` (a bracket pinned to a wall) is not a variant yet
either — a variant is added when a scene draws it, not before.

**The 挑廊 is what the frame was for, and it is in the picture.** A `Floor`-role plate hangs
`1.2 m` outside the downhill wall and `1.0 m` under the eave that shelters it; under the plate, one
**joist** runs from each post's head out to the plate's edge and one **strut** runs from a fixed drop
below that head to the joist's own outer end, so every bracket is a closed triangle whose three
vertices are a fraction of a member that already exists. On the plate's edge stand the balusters and
across their tops one rail.

The numbers the scene moves: the picture's hash is `532FF169FCB14F04`, and the identity probe's rows go
**1009 → 1111** — nineteen members at six surfaces each, and the deck's own six. The document is one
plate, nineteen members, a crowned mass, a profiled roof and the posts: **no face of the gallery is
authored as a coordinate that some other face has to agree with.**

### Arc 5 is closed: the tiles are geometry

`Roof::covering` is a three-number spec — `period`, `fill`, `height` — and the expansion derives the
rolls from it: **one straight triangular prism per roll per 步架**, standing on the skin it covers.
`stilt`'s roof carries 296 of them in 2 368 triangles.

**The bend cost the arc its first design, and the measurement is the record.** A roll that crosses a
purlin is bent, and a bent body is one solid with a creased surface — topologically fine, and not
writable by the crate's rule, which winds a solid by one consistent corner order and checks the body's
own signed volume. The hand-wound bent roll left **24 boundary edges**; the same roll as one prism per
step closes, and it is also what a tiled roof is: a course laid up to the purlin and the next course
above it.

| number | value |
| --- | --- |
| the rolls | **296** over the roof (2 planes × 37 × 4 steps), **2 368 triangles** |
| every roll | **8 faces, euler 2, manifold, closed** — a body, not a surface |
| the volume | **1.499899 m³** over 296 rolls of 6 cm × 19 cm × ~0.9 m |
| the tile bed | the base stands **1 mm** off the skin, for the same reason a post is tenoned into its plate |
| the refusals | `CoveringFault` for a period of zero, a fill of one and a height of zero, each also reaching `expand` |

`stilt`'s hash is `4833AA82E627DD0E`, and the identity probe's rows go **1111 → 2591**: 296 rolls at five
surfaces each is 1 480 of them, which is the price of drawing tiles as solids rather than as a texture —
stated here rather than discovered later.

### Arc 7 is closed: the compound is three buildings and a wall

The order of the last two arcs changed, and the reason is the third one's: **the interior needs a
second storey**, and the compound is where a second storey lives. So the compound came first.

`crates/gallery/src/compound.rs` is a 三坊一照壁: a main hall 18 × 6 at the back, two wings 6 × 15 down
the sides, a screen wall across the front, three gable roofs with the 举架 ladder and its tiles, and an
intermediate floor in every range. **Sixteen single-storey constructs compose into a compound, and not
one of them was added for it.**

The finding is the **order of the courts**: a 四合院 with four ranges is one mass with a courtyard void,
which the crate has had since it had courtyards — and this type is not that, because its fourth side is
a **single-storey screen** and a mass's walls are all one height. So the court is a *gap* between three
masses that touch at two corners, and the fourth side is a `Wall`. Two ways to one square of ground, and
the second is the one that lets a wall be lower than the ranges.

| number | value |
| --- | --- |
| the frame | **1280 × 800**, hash `19CA92920AB35238` |
| the passes | **5 meshes × 3 stages + 1 post = 16** — the covering's 652 roll segments ride in the roofs' own mesh, so sixty metres of tiles cost no pass |
| the identity rows | **6 685** for the compound alone, and **9 276** over the eleven scenes |
| the violations | **0**, on three masses that touch, three roofs, six plates and a wall |

**What it does not draw, and what that costs the type**: no openings (every range is a closed ring,
where the doors and windows are), no stair between the storeys, and no 厦子 — the Yunnan front porch.
The first two are the interior arc's work now that the second storey exists; the third is a plate on
two rows of posts, which is the frame arc again.

### Arc 6 is closed, and its evidence is a document rather than a picture

`Slab::void` is **one hole through a plate**, and a slab with a hole is a **prism over a ring**: its two
faces are annuli and the hole's wall is a surface of its own that looks *into* the hole. The annulus
triangulation was already in the crate — `fill::ring_fill`, which a courtyard's own base stands on — so
what this arc added is the third surface, the document check that knows a holed slab is a **torus**
(euler `0`, not `2`), and one rule the whole arc turned on:

**No winding is written by hand.** The first version decided each face's order from the body's own
middle and left **48 boundary edges**; the second passed each face through
`Solid::oriented_triangle`, which decides by the face's own up component, and left **0**. The lesson is
the same one the roof's skin already carried, and this arc is where it became a rule to write down:
*a planar face's winding is a fact about the surface and not about the code that emits it.*

| number | value |
| --- | --- |
| the probe's ring | a 6 × 4 floor with a 1.2 × 1.2 stairwell, 0.3 m thick: **32 faces, loops 0, euler 0, manifold, volume 6.767998** against 6.768 |
| the hole's wall | **8 of 8 sides look into the hole**, and none away |
| the refusal | an L-shaped (non-convex) hole is `SlabVoidNotFilled`, because `ring_fill` bridges convex loops |
| the compound | the main hall's floor carries a **1.5 × 1.4 stairwell**: the document's identity rows go **6 685 → 6 689**, and the picture's hash **does not move at all** |

**That last row is the arc's real result.** The interior is invisible from outside the ranges, so an
interior construct's evidence cannot be the frame — it is the document, the probe, and the four surfaces
the identity table names. A scene that *showed* the hole would be a camera inside the hall, which is an
interior view this gallery does not yet have; drawing one now would be a picture of a room whose walls
the document closes.

### Arc 8 is closed: the settlement layer is a frontage, and F6 is answered

**F6 is answered by the owner: the settlement stays in scope, as a layer and not as a diorama.** So this
arc adds the smallest layer that answers a street's three questions and nothing else.

`building::frontage` is a line, a side, a depth and a list of widths. Walking the line by those widths
gives one `Plot` per building: the segment of the line it faces and the segment behind it. **The plots
tile the line** — their widths and the gaps between them sum to the length walked — which is what makes
a party wall a party wall rather than two walls with a street between them, and what makes `gap = 0` the
statement "these two houses share a wall".

| number | value |
| --- | --- |
| the probe | five 4 m plots on a 20 m line: fronts tile to **20.000 m**, area **120.000 m²**; the other side mirrors by **12 m**; a bent line turns the last two plots from `-z` to `+x`; **six refusals** each named |
| the scene | `street`: five plots of 6 × 5, one two-storey house on each, a 3-bay × 2-storey facade with a door and three windows, the 举架 ladder and its tiles on every roof, the row climbing at 5.5 % |
| the frame | **1280 × 800**, hash `247F1868C99BD982`, **13 passes** (4 meshes × 3 stages + the post) |
| the identity rows | **4 940** for the street, **14 220** over the twelve scenes |

**Three rules this arc cost, each measured.** A plot's quad is built and then reversed when its own plan
area is negative — *the winding of a plot is the crate's business and not the frontage's*. A plot's
direction is read at its own **middle**, because a plot that starts exactly on a corner would otherwise
be turned by the segment it left. And **a house is built, seated, then cut**: a face's grid holds
absolute elevations, so a grid cut before `site::seat` keeps the level the house had before it climbed
the hill, and every opening in it ends up 0.33 m below its own wall — one house's worth of the slope, and
that is the number the expansion reported.

**What the street does not draw**, and each is a named next step rather than a gap: no pavement or
arcade (the 骑楼 needs a first floor standing out over the walk — the 挑廊's plate again), no blind party
walls (the joint is two walls touching), and no plot behind a plot (a block's own depth polygon is a
second derivation).

### The two gables: 悬山 and 硬山 are one derivation with two numbers

The roof arc left a gap that every 西南 民居 names: **a roof took one overhang for every edge**, so a
硬山 — the gable end flush with the wall, which is what a terrace of party-walled houses is — could not be
written at all. It is written now.

`Eave::read_by(mass, sides, ends, sink)` builds the eave loop from **one amount per edge**, with the two
gable ends chosen by the axis rather than by a caller's guess. A corner is the **crossing of the two
edges' own slid lines**, so a 硬山's corner is held in `x` by the end edge (which did not move) and pushed
in `z` by the long edge (which did) — which is the one thing a corner of a 硬山 *is*.

| number | value |
| --- | --- |
| the probe | over one 10 × 6 mass: 悬山 **11.400 × 7.400, 84.360 m², 18.559214 m³**; 硬山 **10.000 × 7.400, 74.000 m², 16.279993 m³**; both **20 faces, euler 2, manifold, 0 violations** |
| the difference | the run is **1.400 m** shorter, which is exactly two ends × 0.70, and the long sides do not move |
| backwards | `gable_roof(x, y)` and `gable_roof_by(x, y, y)` are **the same roof, field for field** — the eleven pinned plates could not move, and the only uniform-case change is a rounding that was deliberately left alone |
| the street | five houses become 硬山: hash `BD46F1ACB223C05F`, and **the five roofs stop reaching into their neighbours** — the interpenetration the 悬山 row drew was the party wall being crossed twice |
| the rows | street **4 940 → 4 340**, twelve scenes **14 220 → 13 620**: a shorter eave carries four fewer rolls per slope, and 160 fewer roll segments is 800 of the 600 measured — the rest is the bands' own re-count |

### The court under a crown: the wall top is one surface

`CrownWithACourtyard` is gone, and what replaced it is one sentence rather than a new field: **the court's
wall top is the mass's own crown surface, read at the court's own nodes** by the station of the nearest
point of the mass's outline. A crown is authored per node of the outline, so it is a piecewise-linear
function of the outline's station; a court's node does not lie on the outline, and *the wall top over
this node* is the crown at the nearest point of the loop — which is what a wall top between two authored
corners already is. The two node counts never have to agree, which was the whole objection.

The second half was a defect the first half exposed: a court's top strip under a crown **warps**, exactly
as the mass's own wall top does when it is crowned at one end, and the two triangles it has always been
are two planes. `VoidTop` grew the same `half` the mass's `WallTop` carries, so a warped court strip is
two elements and a levelled one keeps the name it had.

| number | value |
| --- | --- |
| the levelled court | top **3.2000** — the mass's own top, unchanged; **0 violations**; mass and court both **euler 0**, manifold |
| the crowned court | top **5.0000**, the crown's own high side; the south wall tops at **3.2000** and the north at **5.0000**, so the court rises with the gable; **0 violations**; both rings close |
| what it cost | one violation variant deleted, one surface field added, and the crown threaded through one call — no new authored field, and no scene moved |

### A 一颗印: the type the last three arcs were for

`crates/gallery/src/yikeyin.rs` is the Yunnan seal: thirteen by fifteen metres, four ranges round a
three-metre well, the 正房 at the back, two 厢房 down the sides, and a gate house across the front with
the one gate in it. It is the first scene built from **硬山** gable ends (so each range's roof stops at
its own gable wall) and **山墙冠** (so each gable wall rises to its ridge), and the two are why the type
reads at all.

| number | value |
| --- | --- |
| the frame | **1280 × 800**, hash `C2A71A8F65326125`, **13 passes** (4 meshes × 3 stages + the post) |
| the identity rows | **5 091** for the house, **18 711** over the thirteen scenes |
| the document | four masses that touch at their corners, four crowned 硬山 roofs, four intermediate floors and one gate — **0 violations** |

**One crate defect fell out of it, and it is the round's other half.** `ops::split_outline` inserted a node
into a mass's outline and left `crowns` alone, so a crowned mass could not be split without falling out of
step with `CrownCountMismatch` — and a crown has to be set *after* a split, because the height a gable's
middle node takes is the roof's own apex. The split now interpolates the crown exactly as it interpolates
the elevation, which is what makes "split the gable end, then crown it" one verb pair instead of two
readings that have to agree.

**What the picture shows and what it cannot**: a 一颗印 is a sealed block with a slot of sky, and that is
what the frame is. The court itself is dark at the bottom of its well — three metres of sky over six-metre
walls, which is the type's own light. It does not draw doors or windows behind the gate, the 夏子 porch,
or the court's paving.

### The last named boundary: `Anchor::OnFace`

A bracket can be pinned to a **wall's own face** now: `Anchor::OnFace { face, along, up }`, where `along`
is metres from the edge's own start node and `up` is metres above the wall's base **at that station** —
the base being the two outline nodes' elevations interpolated, so a wall on a slope answers the point on
its own base line and not on a level one.

| number | value |
| --- | --- |
| the corner | a bracket at `along = 0, up = 0` lands on the mass's own corner: box `x −5.0100..−4.9900`, `y −0.0053..2.4053`, `z −4.5085..−2.9915` |
| the station | `along = 2.5, up = 1.8` on a ten-metre wall: box `x −2.5100..−2.4900`, `y 1.7900..1.8100` |
| the refusals | past the edge's end, past the wall's top, and a face on a mass that is not there: **all three `MemberAnchorUnresolved`** |
| the difference from `OnMember` | **no other member is needed**, so a face anchor resolves in the first pass and a cycle through one is impossible |

**One naming fault the probe caught and the code now does not make**: the resolvability test was weaker
than the resolution — it asked whether the mass existed, not whether the wall had the point — so an anchor
past the edge was reported as a `MemberAnchorCycle`. It is the same question now, and the fault named is
the fault that happened: *a point the wall does not have can never resolve, which is not a cycle*.

`OnFace` has no user in the thirteen scenes yet, and that is stated rather than hidden: its first scene
use is the **夏子** (the Yunnan front porch), which needs a *lean-to* roof — one plane from the porch's
front edge up to the wall — and a roof whose skin covers a single slope is the next derivation, not this
one. What this round owes the drawing is the anchor, and what it owes the record is that the drawing is
still waiting for it.

### A lean-to, and the 夏子 it was for

A 夏子 is the porch across a 正房's front, and its roof is **one plane**: high at the wall, an eave at the
porch's own edge. The crate could not derive it, and the measurement that mattered was whether it could
*express* it — a gable and a hip both put the crest **inside** the eave loop, so the question was a crest
**on** the loop's own back edge.

**It can, and the reason is the model's own rule**: a roof's skin must cover its eave loop, and a single
plane from the eave edge up to the back edge covers exactly that loop. Measured over a 9.3 × 1.35 porch:
skin plan area **12.5550 m² against the loop's own 12.5550**, euler **2**, manifold, volume **1.506599 m³**,
**0 violations**. Two things the probe cost and the derivation keeps: the loop must be **wound
positively** (the other way round is a roof inside out — it measured a volume of −0.753299), and the
plane's rows stand at **the eave edge's two ends**, not at two arbitrary stations (stations `0` and `0.5`
covered a diagonal half, 6.2775 m²).

`derive::lean_to(host, plan, low, high, thickness)` is that derivation, and the 一颗印 now wears a 夏子:
nine metres of lean-to across the 正房's court wall, two posts at its edge, and **two struts whose wall ends
are `Anchor::OnFace`** — the face anchor's first use in a scene, and its own proof that an anchor follows
the wall: the porch's court edge is found *by position*, because the 正房's outline was split when its gable
ends were crowned and edge indices move when a loop grows.

| number | value |
| --- | --- |
| the identity rows | the 一颗印 **5 091 → 5 276**, thirteen scenes **18 711 → 18 896** |
| the frame | **unchanged** — `C2A71A8F65326125` |
| why the frame is unchanged | **the 夏子 stands inside the sealed court**, and the block's own frame cannot see into a three-metre well. Its evidence is the document, the probe and the rows, and an interior camera still does not exist. |

### The court's four faces, and the check that refused them

A 一颗印's court faces are where the house is *used*: the court is the circulation, and every range opens
onto it. The scene cut them — three bays by two storeys, a door in the middle bay of the ground floor and a
window in every bay above — and it could not, because `GridOnACrownedFace` refuses **every** grid on a mass
that has **any** crown.

That is the wrong question, and the round's other half is asking the right one. **A crowned mass has level
edges** — every wall between two corners at the same height, which is precisely what a court's four faces
are — and a rhythm divides a rectangle. `edge_crowns` answers the face's own two ends (the mass's outline's
crowns for its own walls, the court's derived crowns for its ring), and the violation now fires only when
those two differ.

| number | value |
| --- | --- |
| the openings | four court faces: 3 bays × 2 storeys on the three ranges, 3 × 1 with one door on the gate house |
| the identity rows | **18 896 → 18 932**, and the 一颗印's frame is re-pinned at `FDFC5862BC8DF946` |
| the exterior | the composition does not change, because the openings are **inside** the sealed block — the picture shows the gate's own bay moved and nothing else |

**A finding this round paid for and did not fix.** An interior camera was written for the court, rendered,
and **rejected before landing**: the well is three metres across and six deep, its only light is one
directional sun, and the frame came back almost black — a plate that would have been pinned as "the court"
while showing nothing. The reason is not the camera: **the shading has no sky ambient term**, so a well can
only ever be a dark hole under one sun. That is the next round's work, and until it exists the court's
openings are document evidence and not picture evidence.

### The sky is a light, and the court finally has a camera

The last round found that a three-metre well under one directional lamp is a hole. The cause was not a
missing shader term: **the lighting stage already carries a sky term, and reads it out of the scene's own
environment map** — and a scene that declares no environment answers zero there. A 一颗印 declared a sun
and nothing else.

`crates/gallery/src/yikeyin.rs` now declares the air: an `Atmosphere` (zenith, horizon, ground, density,
scale height) baked into a 256 × 128 latlong map, handed to the lighting stage as an `environment_light`
and to the post stage as the frame's own sky. **One air, three readers**, so the bounce and the background
cannot drift apart — and the grade reads `environment_sky(ray)` instead of a sky law of its own.

| | before | after |
| --- | --- | --- |
| the exterior plate | `FDFC5862BC8DF946`, shadowed walls at zero | **`19D6B13ADEC8B470`**, the shadowed side is sky-lit and the court's own walls are visible down the well |
| the court plate | *rejected a round ago as unreadable* | **`B88182140B4C4AD7`** — the sky slot, the 夏子's tiled edge, the 正房's door and the ambient-lit walls, all in one frame |
| the identity rows | 18 932 over 13 scenes | **24 244 over 14** — the court view is the same document, so its 5 312 rows are the same rows |

**The two plates are one document and two cameras**, which is the point: `yikeyin_court` re-renders the
expansion the exterior plate renders, from a person's height in the corner the sun does not fill. That is
the evidence the 夏子 and the courtyard arc never had.

### The valley's three scenes get their air

The 一颗印's sky was one scene's fix for a defect every scene had: **the shading's sky term reads the
scene's own environment map, and a sun-only scene answers zero there**, so every shadowed face in the
gallery was black. `kit::sky(atmosphere)` hands back the two halves of one air — the map a path tracer reads
and the light this renderer bounces — and the three vernacular scenes now each own an `Atmosphere` whose
numbers are the ones their old hand-written sky law carried.

| scene | its air | hash |
| --- | --- | --- |
| `street` | a clear afternoon: zenith `0.24 0.42 0.68`, the sun's bloom `10.0 / 0.30` | `82B1EAF35E21F0D0` |
| `compound` | a bright highland afternoon: `0.22 0.40 0.66`, bloom `8.0 / 0.24` | `3F8FF1A57FD70BCE` |
| `stilt` | a hazy river morning: `0.28 0.42 0.62`, bloom `6.0 / 0.30` | `580E69E31DE732FF` |

**What it bought, and it is legibility rather than decoration**: the street's gable ends, the compound's
照壁 and court walls, and the stilt house's posts, gallery and hillside were all near-black before and are
now readable. A black shadow is not a look; it is an absent light, and the architecture it hid was the
thing these scenes exist to show. The identity rows do not move — 24 244, unchanged — because this is a
change to the light and not to the document.

### The last nine buildings

Every building scene now owns an air. The nine that remained — `house`, `hall`, `hall_large`, `shed`,
`terrace`, `wing`, `workshop`, `yard`, `identity` — were converted to the same pattern as the valley's
three: one `Atmosphere` carrying the numbers their own sky law already had, baked into the map the shading
reads, declared as an `environment_light` beside the sun, and read by the grade through
`environment_sky`. The identity rows do not move — this is the light and not the document.

| scene | hash | scene | hash |
| --- | --- | --- | --- |
| `house` | `2C18FA24229A832E` | `workshop` | `D869C688B924047D` |
| `hall` | `0E369523A20D972B` | `yard` | `B6FB314FC1721046` |
| `hall_large` | `F6A821084C92768B` | `identity` | `3029BDBFE9E3E7EC` |
| `shed` | `2BE00AF7BD9C658B` | `terrace` | `8A0E101DB18C8AFB` |
| `wing` | `1A0FC6D69838214D` | | |

**Two things the conversion found rather than caused**, both recorded rather than fixed:

- **Three scenes' sky glow points at a different sun than the light they declare.** `hall`, `terrace` and
  `wing` carry `sun = normalize(-22, 30, 20)` in their sky law while their `SUN` constant is
  `[-26, 34, 24]` (or `[-30, 36, 26]`). The baked map keeps the glow where the *sky law* put it, because
  that is the look the plate was drawn with; the light is where the light is. Whether the two should agree
  is a scene question nobody has answered.
- **`density` and `scale_height` are inert in these scenes**: they feed `air_closure`, which no building
  shader calls. They are carried for the day a scene's air is drawn rather than approximated.

### The court is where a courtyard house is read

The 三坊一照壁's court had never been photographed and its ranges' fronts were blank walls. Both are fixed,
and the two fixes are one story: **a court face is a façade**, and it is what the court camera exists to
show.

`crates/gallery/src/facade.rs` is the one place that says what a range's front is: **three bays, two
storeys, a door in the middle bay of the ground floor and a window in every bay above** — the 明间/次间
division the sources describe — with the gate house as the type's own exception. Both courtyard scenes call
it, so the 一颗印 and the 三坊一照壁 cannot disagree about what a range's front is, and the 一颗印's plates
prove the refactor reproduced the document exactly: same two hashes, same 5 312 rows.

| number | value |
| --- | --- |
| the compound's court | three faces opened; hash **`DDA7A8DBBBE9B6F3`** |
| the court camera | **`F183A16EECEADE87`** — the two wings' inner faces with their windows, the paving, the 照壁 at the end and the sky slot above |
| the identity rows | the compound **5 276 → 6 725**, fifteen scenes **31 005** |

**And one defect the metre margins exposed**: an opening's jambs are measured against the *inner* face,
whose ends the wall's own miter pulls in by the wall thickness. A half-metre margin on a half-metre wall
therefore leaves the jamb exactly on the corner, and the expansion refuses it by **one millimetre**
(`OpeningOutsideWall { past: 0.0009999871 }`). The margin is now the wall's own thickness plus five
centimetres — which for the 一颗印's 0.45 m wall is the half metre it always had, so its plates did not move.

### The schema a builder carries, and the measurement that closed it

For five rounds this proposal carried a named hazard: **`shaders::lighting_with_normal` and
`shaders::textured` hard-code the gallery's custom schema**, so a scene whose rows carry a field the gallery
does not — a plant's shadow-bias scale — would shade through a schema that never declared it, and the
shading would be silently wrong rather than refused.

**Measured, and the hazard was latent rather than live.** The one scene in the gallery with a different
schema is `vegetation_picture`, and it does not use either builder: it calls
`stage_set_over_schema(..., &plant_custom_schema())`, which takes the schema as an argument and always did.
The two builders' own callers — `lighting` (every building scene, through `mesh_shaders`), `sweep_picture`
and `tiles` — all use the gallery's, the last two with an `image` field appended **at the end**, where it
shifts nothing.

So the fix is the one that makes the hazard unrepresentable rather than the one that "repairs" a defect
that was not happening: **both builders now take `custom: &Schema`**, every existing caller passes
`&gallery_custom_schema()`, and a scene with another schema can now say so. The proof that it changed
nothing else is the audit: **15 of 15 plates unmoved, 31 005 identity rows unmoved**.

### The street becomes a lane

A row of houses facing nothing is a terrace, not a street. The 西南 old street is a **space between two
walls**: shops on both sides, a 骑楼 over each walk, and the hill carried down the lane between them.

`crates/gallery/src/street.rs` now builds **two frontages across a seven-metre lane**. The second frontage
is the same line mirrored (`side = -1`), so its plots come out wound the other way without being told to,
and the same seating, floor and opening loops walk both rows.

**The 骑楼 is derived from each plot and not from a plan line**, which is what makes one function serve both
rows: the canopy's outward direction is `front − back` of the plot it belongs to, so the mirrored row gets a
walk facing its own street. Its posts stand at the grade under them, because the lane climbs.

| number | value |
| --- | --- |
| `street` document | `07E1337D733A0725` |
| `street_arcade` document | `0AAED8AD9C1EE823` |
| identity rows | the street **4 805 → 9 610**, sixteen scenes **36 275 → 45 885** |

The lane is *not* paved, and that is deliberate: a paving slab's ends have to be authored where the hill is
not level, and the terrain is already the floor of a lane that climbs.

### Standing on the 挑廊

The 吊脚楼's gallery was built, meshed and never stood on: every plate of the type was from down the bank,
where the posts read as a texture under a floor. It now has a camera on the deck — the fourth scene to be
**one document with two cameras**, after the 一颗印's court, the 三坊一照壁's court and the street's 骑楼.

| number | value |
| --- | --- |
| `stilt_gallery` document | `B8EDEBDC592E2FDE` |
| the stilt's rows | 1 735, the same in both views — one document |
| seventeen scenes | **47 620 rows** |

The frame holds what the type is: the deck underfoot, the rail at hip height, the posts the deck hangs
between, the dark of the eave above, and the valley the whole thing is hung over.

### The water: a paved court with one hole

四水归堂 says the four roofs shed inward, and the proposal has said so since the courtyard arc — but the
court itself was the terrain, so the sentence ended in air. `derive::court_paving` is where it ends now: **a
plate over the court's own rectangle, with the drain's grate cut through it at its middle**.

Two invariants decide whether it builds at all: the hole must stand `margin` clear of every edge (a grating
at the lip is a gutter, so such a court answers `None` rather than paving a basin whose outlet is its own
edge), and **the plate carries no host** — a court is the space *between* masses, and a plate hosted by one
of them would move with that one and leave the rest behind on the first seat.

| number | value |
| --- | --- |
| the 一颗印's court | 3 × 3 m, grate 0.45 m, paving 0.08 m thick; rows 5 312 → **5 322** |
| the 三坊一照壁's court | 6 × 15 m, grate 0.6 m; rows 6 725 → **6 735** |
| documents | `yikeyin` `D150D4C141F7DCF9`, `yikeyin_court` `21BD72E24CC886C5`, `compound` `6AAD1D2CD464B03C`, `compound_court` `7C99C4BDA8890F27` |
| seventeen scenes | **47 660 rows** |

**A renderer finding, measured and corrected here**: the 一颗印's court draws almost black through Cycles
while it reads in the rasterising pipeline, and this proposal first said the tracer did not read the
environment light. **That was wrong, and the measurement says so**: with the sun's power at 0, raising the
declared radiance tenfold raises the court wall's mean luminance by **10.0132** (0.014962 → 0.149819); a
light that is dropped does not scale. The emitted document carries the sky as

```xml
<background>
  <environment_texture name="world_texture" filename="air.png" />
  <background name="world" strength="1.06, 1.06, 1.06" />
  <connect from="world_texture color" to="world color" />
  <connect from="world background" to="output surface" />
</background>
```

so the image *is* carried and the radiance *is* on the strength. The real difference is **what the two
renderers do with it**: the rasteriser's ambient is an unoccluded map lookup (`environment_map` sampled by
the normal), while the tracer's is traced and therefore respects the well. A 3 × 3 m court under 6.2 m of
wall sees ≈0.34 sr of sky, and `albedo·L·Ω/π ≈ 0.015` — which is the measured wall. **The dark court is the
correct picture and the bright one was the approximation.**

`crates/device-cycles/examples/environment_probe.rs` is that measurement, kept: it rewrites the declared
radiance, renders at two of them, and prints the region's luminance beside the document's own world block,
with `--light-power 0` to switch the other lights off.

**One related defect found and not fixed**: a **tinted** environment light loses its tint. The colour ×
radiance triple lands on Cycles' `Strength` socket, which is a float, so only the first number survives —
`[1.0, 0.2, 0.2]` renders identically to white, and the court's sky slot reads `41.57, 48.81, 60.23` in both
cases. No gallery scene is tinted (all seven declare `[1.0; 3]`), and the fix is to put the radiance alone
on `Strength` and multiply the texture by the tint with a `vector_math` node; it is left for a round that
wants it, because the world block is a pinned output other notes have measured.

### The rafter tails: what an eave shows from below

A Chinese eave is not a roof edge — it is a **row of rafter ends**, and every plate so far drew the roof's
underside as a smooth soffit, which is the one thing an eave is not.

`derive::rafter_tails` derives them from the roof the mass already carries: **a tail runs from the wall's own
top — the outline at `mass.top` — out and down to the eave node's line**, so its slope is the roof's slope by
construction rather than by a second number that could disagree with the first. Positions are interpolated
between each eave edge's own two nodes, the same two nodes the plane's rows stand on, and `spacing` is
honoured as a **maximum**: a side whose length is not a whole number of spacings gets the count that fits and
an even division, because a gap at one end of an eave is a mistake a reader can see.

| number | value |
| --- | --- |
| the 一颗印 | 4 ranges × 87 tails = **348 members**; rows 5 322 → **5 670** |
| documents | `yikeyin` `273D958F73EE63AD`, `yikeyin_court` `681FB58AB7AE8A29` |
| seventeen scenes | **48 356 rows** |

Only the 一颗印 carries them so far. The street's ranges would take the same call, but its canopies are
lean-tos whose tails would have to be derived from a lean-to's single plane — a different reading of the
same idea, and not one to guess at.

### The rafter tails, and the rule that says where they go

The tails reached the street's ten ranges and the stilt house this round, and doing so found the first
version's mistake: **it hung a tail under every edge of the eave loop**, which on a gable puts them in the air
above the gable slope. The rule is geometric and is now read from the roof itself — **a side is an eave side
when it runs along the ridge**, the ridge being the roof's own two crest points. A gable has two such sides, a
hip has two, and a lean-to has one; a gable end has none, because its edge is perpendicular to the ridge.

| scene | rows | document |
| --- | --- | --- |
| `street` (ten ranges, two rows) | 9 610 → **11 530** | `7AF4130F7541F00D`, arcade `D0947AF8263A984C` |
| `stilt` | 1 735 → **2 071** | `CAF75052AA593363`, gallery `2E359A3D718A60AF` |
| `yikeyin` (re-derived under the new rule) | 5 670 → **6 402** | `F57D07D988AE3AB1`, court `3E15895308BA44E5` |
| seventeen scenes | **54 332 rows** | |

The 一颗印's count grew because its crowned outline splits each long side into two, and both halves are
parallel to the ridge — two runs a side where there was one, which is what a crowned gable actually is.

### The barred window: 直棂窗

Every opening in this repository was a hole. A southwest Chinese window is not a hole — it is a hole with
**bars in it**, and the oldest form of that is the 直棂窗, the straight-barred window: vertical timbers standing
in the opening, three lights where there was one.

`facade::lattice` stands two bars in every upper window the shared court face cuts, and it does so as
**members anchored with `Anchor::OnFace`** — the face, how far along its edge, how far above the wall's base —
so a bar travels with its wall when the building is seated, and a wall on a slope carries its bars at the
slope's own height.

Two mistakes were made and are recorded because both are instructive:

- **The bars were placed from the mass's height at first, and stood in the wrong place.** A crowned mass's
  `top` is its **apex**, not the top of the wall the window is cut in, so `height / 2 + 0.9` put the bars above
  the hole on every range whose gable end was crowned. The fix is to place them from the **opening's own
  resolved rectangle** (`Opening::extents` against the bay) and the face's own ground line
  (`face_extent`): the bars now run between the sill and the head of the hole by construction.
- **A document that carries members and a scene that does not draw them is a bar nobody can see.** The 一颗印's
  scene packed masses, roofs and floors only; the bars existed, numbered 24 in the identity table, and rendered
  as nothing at all. `Element::Member` now has its own mesh and a timber material, the way the street's frame
  does.

| number | value |
| --- | --- |
| the 一颗印 | 24 bars (4 ranges × 3 bays × 2), rows 6 402 → **6 474** |
| the 三坊一照壁 | the same call on its court faces, rows 6 735 → **6 807** |
| seventeen scenes | **54 620 rows** |

The bars are not yet on the **street's** shopfronts or the stilt house, which carry different fronts (five bays
of shop, one door) and would want their own reading of the pattern.

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

### The 走马廊's rail is a frame, not a fence

The stilt house's gallery had posts, joists, struts, a baluster at every post and one rail across their tops.
That is a fence: two horizontals and nothing between them, with a gap wide enough to fall through, and it read
as a railing only because it was in shadow.

It is now a **frame**: a baluster at every post, three slats at the quarters of every bay, a middle rail per
bay and the top rail across the lot. The section is the gallery's own (`GALLERY_SECTION`), narrowed to 70 % for
the slats so the uprights read against the rails rather than merging with them. Rows 2 071 → **2 167**.

**A door onto the walk was attempted and refused, and the reason is worth keeping.** A gallery with no door
behind it is a balcony with nothing to walk out of, so the downhill wall was cut as a front — a door in the
middle bay, a 直棂窗 in every upper one. `validate` answered `OpeningOutsideWall { past: 1.4 }`: the wall the
gallery hangs from is the house's **gable end** as well, and that wall is split at its middle before the seat so
the ridge can crown it. A three-bay grid laid on it either spans two half-walls of two metres (bays too narrow
for their own margins) or has the crown's own node standing inside a bay. The window band therefore waits for a
type whose gallery fronts a wall that is not also its gable.
### The 一颗印's gate is a door, not a hole

The compound had a 门房 — a three-metre gate house on its front — and the gate itself was a hole in it. A 一颗印
is entered through that door, and what makes an entrance is three pieces: a **threshold** to step over, **boards**
that close it, and a **门罩** over it so the rain never falls on a person waiting for the gate to open.

Three measurements, and the first two were found by the document refusing to agree:

- **the gate house is one bay, not three.** At three bays a 3 m front leaves a 0.30 m middle bay, and the boards
  in it were skipped by the very guard that refuses bars in a hole too narrow for them — the frame showed a hole
  and the identity table grew three rows for a gate that was never put in;
- **the margin is the wall's thickness plus five centimetres.** The gate's first inset used 0.35 on a 0.45 wall
  and overran the inner face by **0.101 m**: `OpeningOutsideWall`, exactly, and the rule every other opening in
  this crate already follows;
- **the threshold is the wall's own foot.** The opening starts 0.15 above the ground rather than at it, because a
  member standing in an opening that reaches the floor splits the wall's shell into two loops — `SolidOpen`,
  measured on the street's shopfront two rounds earlier. The stone below the door is what a person steps over.

Rows 6 519 → **6 550**, and the gate carries its own 门罩 hosted by the gate house, shedding away from the wall.
### The ranges stand on their 台明

The four ranges stood level with the court's paving, which is not how a Chinese courtyard is built: the ranges
stand on a **stone platform** — the 台明 — and the court is the floor below it, which is what makes the court a
room and the ranges its walls.

Three things, and only the first is a number:

- **the seat's own offset is the platform's height** (`0.45`), so the ranges and their platforms cannot disagree:
  the seat moves the building up by that much and the platforms fill exactly the gap it opens;
- **each platform is its range's own plan, proud by `0.30`**, wound positively — the same inflation-about-the-
  centre and the same `loop_area` test the village's house plinths use;
- **the court stays where it was** (`0.02 .. 0.08`), so the paving is now the well floor of a four-sided room
  rather than the level the ranges stand on.

Identity: 6 550 → **6 580 rows**. What the compound still lacks is the **踏步** — the three or five stone steps
from the court up to each range's door, which is the other half of a 台明 and is not drawn.
### The 踏步, and why there is no picture of them

Two stone steps now stand at every range's door: the range's court-facing wall is found by taking the edge whose
middle stands nearest the court's centre, and the flight is laid out along that edge's outward normal — two treads
of `0.34`, `1.9` wide, from the court's level up to the platform. Four ranges, two steps each.

**Measured, and exact:** identity 6 580 → **6 628 rows**, which is 4 × 2 × 6 — the steps exist, one flight a door,
and the arithmetic of the count is the evidence because the frame cannot give it.

**Two frames were tried and neither shows them.** The court's own camera looks steeply up at the sky-well, and an
eye-level camera inside the court renders **almost black**: the sun stands low and the ranges' four roofs leave the
court in their own shadow, which is what a 一颗印's court *is* at that hour — 四水归堂 means the light comes down
the one hole in the middle. A plate of the 踏步 therefore needs either the sun raised for the court's own scene or
an interior light, and neither is a thing to change in the middle of a round that was about the steps.
### The court's own plate: a sun, a lens, and an aim

The 踏步 were in the identity table and in no picture, because the court's own frame was **aimed at the sky-well**
and lit by a sun that leaves a 3 m court in shadow all afternoon. Three scene numbers, none of them a change to any
building:

- **a sun of its own** (`COURT_SUN`, nearly overhead) so the shaft reaches the platform rather than stopping on the
  four roofs. It is a scene's sun and not the building's: the same document, lit for the frame that is about the
  court;
- **the environment turned up** for that frame (`2.6` against the outside view's `1.06`), because a well's own
  opening is bright and the sky is the court's only other light;
- **a 97-degree lens looking *down***. The court is three metres across and six metres deep in walls, so the camera
  stands on its south side; the earlier aim was 60 degrees *up*, which is a photograph of the sky-well and not of
  the floor. What the frame now shows is the 正房's platform edge with its two steps, the porch's posts and their
  shadows, and the sky-well above them.

**What this is not**: the court is still genuinely dark, and the picture says so — that is the type.
## Round 47: the court's paving is its own surface

A courtyard's paving was drawn as a *floor* — the same mesh and the same material as the four 台明 and the storeys — and
it is not one. It is the one surface in a 一颗印 that rain reaches, laid in the open with the grate through it, and the
crate now says so:

- **`SlabRole::Paving`** and **`Slab::paving`**, a fourth role beside plate, floor and roof. `derive::court_paving`
  lays a paving and no longer a plate.
- **The two checks that ask "is this the plate a mass stands on" both read the role**, so a paving is skipped by both:
  it neither defines a mass's floor coverage nor cuts the ground out from under a room.
- **The court scene draws it as its own mesh with its own stone** — darker and much smoother than the platforms, which
  is what wet stone reads as.

Identity: **25 scenes, 105 876 rows, unchanged** — a role is not geometry, and the paving is the same prism it was.

### What was tried and rejected, in one frame

**Three-sided galleries round the court** were built first, on the reasoning that a courtyard is a gallery on three sides
and a screen wall on the fourth. The court frame came back **black**: a 一颗印's court is 3 × 3 m, and a 0.85 m gallery on
each side leaves a 1.3 m slot, so the posts stood in the middle of the well. The premise was the 三坊一照壁's, not the
seal's — **a 一颗印 has one 厦子, on the 正房** — and this scene already had it. The change was reverted whole.
## Round 50: the 傣族竹楼, the gallery's tenth type

**A 竹楼 is a floor, not a wall.** The other nine types in this crate are enclosed boxes standing on the ground; this one
is a platform lifted `1.95 m` on piles with a room on part of it, a veranda along the front, a drying deck at the gable
end, a steep thatched hip over all three, and a stair of ten treads up to the veranda. Its ground floor is open air, and
that is the type.

Two facts, and both of them are machinery this crate already had:

- **The floor is the building.** The room is seated `FLOOR` above the ground, the veranda and the deck are slabs at the
  room's own floor level hosted by it — so the seat moves them with the room — and `site::posts` fills the gap the seat
  opened under all of them. Nothing touches the ground but the piles.
- **The roof sheds by slope.** `RISE` is 2.70 m over a 4.2 m room, the eaves run 1.35 m past the wall and the corner is
  cut as a **hip**, because a thatched roof with a gable end would tear in the first storm.

**What it cannot draw yet**: the thatch itself. A hip's two **corner rolls** leave their own plane by `1.10e-3`, against a
tolerance of `1e-3` — **the same millimetre that kept the 蘑菇房's straw out of the village in round 45**. Two types, two
hip roofs, the same two corner rolls: the fault is in `covering`, and it is now recorded twice at the place the roll is
built. The roof in this document is bare, and it should not be.

Identity: 26 scenes, **106 060 rows**, of which `bamboo_house` is **184** — a small house, and the type's cost is in its
decks and its stair rather than its walls.
## Round 51: the hip's top rolls, measured and not yet fixed

The covering's guard was to be made stricter so that a hip's corner rolls stop leaving their plane. **Three changes were
tried and none of them moved the reported number** — the same two faces, the same departures, to the last digit:

| the change | the result |
| --- | --- |
| the guard keeps a tenth of the tolerance in hand | `1.1063814` and `1.1048317`, unchanged |
| the plane is taken from the fattest of the quad's four triangles instead of its first three corners | unchanged |
| a degenerate quad is refused, and a roll that collapses to a spike at either end is not drawn | unchanged |

**An unchanged number after three different guards is the finding**: those two rolls are not reaching the mesh through the
guarded path, and the guard's own reading of them is not the validator's. What the probe printed is the clue:

```text
warped 274: Roll(RollId { roof: RoofId(0), index: 24 }, Slope { side: 0 })
  corners [[2.5709066, 4.3910003, -4.35], [2.475, 4.301, -4.35], [0.21571869, 7.001, 0.0]]
```

**The third corner is on the crest** (`y = 7.001`) while the other two are at the eave: the roll's section has collapsed to
a point at the ridge, so the prism has a triangle where a quad should be. A degenerate quad has no plane to depart from,
which is why every guard here called it flat — but the element's group holds **four** vertices (the pair of triangles that
draws it) and the two triangles are not coplanar, which is the `1.106e-3` `validate` reports.

**The next step is therefore one instrument and not another guard**: count, inside `push_roll`, the rolls whose section
span is under a centimetre at either end, and see whether those two are among them. If they are, the refusal belongs
there, where the section is computed, rather than in the guards above it. This round did not build that instrument, and it
did not land a fix: the two documents are back as they were, and the 竹楼's thatch stays off.
## Round 52: an element is one plane, and the 竹楼's two rolls are still unexplained

**Four hypotheses about the 竹楼's two twisted rolls, all four falsified by measurement**:

| the hypothesis | the test | the result |
| --- | --- | --- |
| the guard's tolerance is a whisker loose | margin at 0.9 and at 0.5 of the tolerance | same two faces, same departures |
| the guard's plane is ill-conditioned on a tapering band | the plane taken from the quad's fattest triangle | unchanged |
| the roll collapses to a spike at the crest | the spans printed from inside `push_roll` | **no roll has a span under 5 cm at either end** |
| the element groups a *bent* roll | `RollSurface` names its profile **step** | unchanged — the two rolls already sat in one step |

The fourth was worth doing on its own account, and it is what this round lands: **an element is one plane, so a surface that
bends is more than one element.** A roll is a chain of prisms, one per profile step, and it bends at every purlin; naming
them all `Base` or `Slope { side }` put a bent strip into one planarity group. The name carries the step now, in the
element and in the identity row (`its near slope, step 2`).

**Identity: 26 scenes, 106 060 rows, unchanged** — every other roof in the gallery has a one-step profile, so the naming
is an identity on them and a *correction* for any roof that ever gets a 举架 ladder under a covering.

### So what is left

The 竹楼's thatch stays off and the two rolls stay unnamed. What is now known about them:

- their element is `Slope { side }` at **one** profile step, so the group is one prism;
- their departure is `1.106e-3` and it did not move when the plane's construction, the tolerance, the spike check or the
  naming changed — **a number that survives four different readings of the same quad is measuring something the readings
  do not share**;
- the printed corners put one corner at `y = 7.001` (the crest) and two at the eave, which is a roll that reaches the
  ridge — and the covering's own span at the ridge is zero *by geometry*, not by arithmetic.

The next instrument is not another guard: it is a print of `low`, `high` and the guard's own `departure` for **every**
roll of that roof, so the two can be read against the rest.
## Round 53: the census, and the fifth falsification

The instrument is built and it settles the question by **refusing the premise**: `ROLL_DEBUG=1` prints every roll the
guard sees, and on the 竹楼's hip that is **240 rolls, every one of them flat**:

```text
  roll 24 step 0: spans 0.1918/0.1918 departures [4.7683716e-7, 4.7683716e-7, 7.1525574e-7]
  roll 39 step 0: spans 0.1918/0.1918 departures [0.0, 0.0, 4.7683716e-7]
```

The two rolls `validate` names as `FaceNotPlanar { departure: 1.106e-3 }` are, at the guard, an ordinary pair of prisms
whose three faces each depart their own plane by **half a micrometre**. So:

- the guard is not wrong about what it measures;
- the sections do not collapse (spans `0.1918` at both ends);
- the group is one step (round 52's naming did not change the number);
- and the fitted-plane departure of the same four corners is `1.1e-3` **only in the validator**.

Five hypotheses, five falsifications. What is left is a disagreement between **two readings of the same four points**, and
that is a different kind of fault from every one this arc has chased: not a wrong roll, but a wrong *passage* between the
expansion and the validator. The instrument that follows from this is not another guard — it is a check that the validator's
`departure_of` and the expansion's `departure` agree on a set of four points they both see, and that is where the next
round should start.

**Landed**: the census itself, behind `ROLL_DEBUG`, and **26 scenes at 106 060 rows, unchanged**. The 竹楼's thatch stays
off until the disagreement is understood; a covering that one reading calls flat and another calls twisted is not a
covering to ship.
## Round 54: the validator's plane fit was cancelling itself to zero

Five rounds of hypotheses, and the sixth was not about rolls at all. **The validator's least-squares plane was wrong**, and
the whole coverage story follows from it:

```
rotation_of:  tangent = shifted - sign * sqrt(shifted^2 + 1)
```

For a covariance matrix — whose entries are **square metres** — `shifted` is routinely large enough that both terms are the
same `f32`. The difference is then exactly **zero**: the rotation becomes the identity, Jacobi never diagonalises the
matrix, and the "smallest direction" it hands back is a **coordinate axis** rather than a normal. A plane is then measured
against the wrong normal, and a surface that is flat to `0.08 mm` reads `1.106e-3`.

The reciprocal form is algebraically the same root and has no cancellation:

```rust
let tangent = -sign / (shifted.abs() + (shifted * shifted + 1.0).sqrt());
```

### What it settles

- **the 竹楼's thatch is back**, and the roof has its bundles: `bamboo_house` 184 → **764 rows**;
- **26 scenes at 106 640 rows**, and the other twenty-five are **unchanged to the row** — the fit only ever reported
  false positives, and it now reports none;
- the two rolls that five rounds could not explain were never twisted: they were **measured against an axis**.

It also closes the arc that began in round 45, when the 蘑菇房's straw was dropped from the village on the same
`1.085e-3`. That measurement was the same defect, and the straw can come back.

### The lesson worth keeping

Every guard this arc added was a guard on the *expansion* — the tolerance, the plane's construction, the spike test, the
element's name — and none of them could ever have worked, because the reading that was wrong was **the one doing the
accusing**. When a measurement disagrees with a hand computation, the disagreement is a defect in the measurement.
## Round 57: the Dai hamlet, the gallery's second settlement

**A settlement of one type repeated.** The mountain village is several types held together by a square and a tower; a Dai
hamlet is neither — it stands on the valley floor, it has no square, and what makes it a settlement is the **lane** and the
**well**. `dai_hamlet` is a row of three 竹楼 along a lane that falls 4 % one way and is level the other, each house on its
piles with its veranda and stair turned to the road, and the well-head in the open ground in front.

**Nothing is re-authored.** `bamboo_house::document()` is a whole house, and the hamlet translates three copies and seats
each on the ground under it — the same `seat` + `posts` pair the single-house scene makes, read once per house. The scene
shell (`village::scene_of`) is shared with the mountain village and made crate-visible for the purpose: one list of meshes,
one set of earthy materials, two settlements.

**The well is a mass.** A well-head is a ring of stone with an inside, and this crate has no ring primitive, so it is a
small mass with a `0.28 m` wall — the expansion draws a curb with a hole in plan, which is what a well looks like from the
lane.

Identity: **27 scenes, 110 005 rows**, of which `dai_hamlet` is **2 305** — one more copy of a 764-row house plus the well
and the extra meshes.

### What it does not draw

- **no vegetation**: the palms and bamboo a Dai hamlet lives among are the vegetation arc's work;
- **no fences or gardens**;
- **the three houses all face the same way**, because the scene transforms are translations and this crate has no rotation
  for a skeleton. A hamlet of houses at right angles needs one.
## Round 58: a skeleton turns, and `translate` had been moving only the foot

**`ops::rotate`** turns a whole document about a plan point: every section of every mass, a roof's eaves and crest, a
slab's outline, a member's free ends **and its `across` direction**, and the site's walls. Elevations never move, which is
what keeps a stair a stair.

Two defects came out of building it, both of them older than the rotation:

- **`ops::translate` moved only a mass's foot.** It called `Mass::outline_mut`, which is `levels[0].outline`, so a mass
  drawn in several bands would have had its bottom moved and its top left where it was authored. No scene translated such a
  mass when the op was written — the village's houses were one band each — so the defect was invisible until the hamlet
  turned houses, and the fix is to walk every level.
- **A member's `across` is a plan direction**, so a rotated body whose members keep a world-frame `across` has every
  section turned against its own axis: measured on a turned 竹楼 as `MemberDegenerate { length: 1.1 }` from the stairs.

### The hamlet round the water

The Dai hamlet's three houses no longer all face the lane: the middle one faces the road and the two outer ones turn a
quarter turn round the well, stairs inward. **A row that all faces one way is a street; three houses whose stairs come
down toward the same well is a hamlet** — and it took a rotation to say so.

Identity: **27 scenes, 110 005 rows, unchanged** — a turn moves geometry and not a single element's name, which is exactly
what the identity table is supposed to be blind to.
## Round 59: the 挑檐枋 the posts carry

A 骑楼's walk is roofed by the floor above it and by nothing else, and what a walker sees from under that floor is the
**beam it rests on** and the posts that meet it. The posts were there; the beam was not, so the projecting slab read as
something floating above two columns.

The beam runs between each plot's two posts at `base + FLOOR - 0.22`, on the projection's own outer edge, `0.20` deep and
`0.16` wide — as deep as the post it lands on and no wider, which is what a 挑檐枋 is. **`street` 12 550 → 12 610 rows**:
six faces a plot, ten plots.

**27 scenes, 110 245 rows**, the other twenty-six unchanged.
## Round 60: plants in a settlement — the bridge

The gallery has had plants since the vegetation arc, and no *building* scene has ever shown one: a plant lives in
`vegetation`'s world of lofts and leaf placements, a building in `building`'s world of solids, and nothing carried a plant
into a settlement. **`settlement_plants` is that bridge**, and it is small because both halves already existed:

- **`vegetation_mesh::wood_points_of`** packs the culms into one `PackedList`, with the placement baked in — the wood is
  one mesh for the whole row of clumps;
- **`vegetation_mesh::template_points`** packs one blade, and every leaf of every clump is an **instance** of it, with the
  transform `vegetation_instancing::standing_at` already knew how to make;
- **`vegetation::expand::expand`** grows one species once, so six clumps cost one plant.

`village::scene_of` takes an `Extra` — meshes, materials, and every instance of them per stage — and the village passes
`Extra::none()`, which adds nothing at all: **the mountain village's scene is byte-identical to yesterday's**.

The Dai hamlet grows six bamboo clumps behind the row, by the well and at the lane's ends.

### What reads and what does not

The clumps are **bare culms with their leaves at the top**, which is the species: `derive::bamboo` puts a handful of blades
on the upper nodes, and a *picture* of bamboo is dense because it draws a grove of hundreds. Getting a single clump to
read as a clump is a **leaf-field** question (density and the nodes a field is allowed to reach), not this module's, and it
is recorded as the next step.

Drawings are scaled for the distance: clumps at `1.3`, blades at `3.0`, because a blade at the species' own scale is under
a pixel from the lane.

**Identity: 27 scenes, 110 245 rows, unchanged** — a plant is not a building element and does not enter the building's own
table. The hamlet's document grew from 7.75 MB to 12.57 MB, which is the wood and the leaves.
## Round 61: a clump that reads as a clump, and the hill village's plants still to do

Round 60's clumps were **bare culms with a handful of leaves at the top**, because `derive::bamboo` is a *culm*: it puts a few
blades on the upper nodes, and a picture of bamboo is dense because it draws a grove, not because one plant is bushy. The
gallery's **`derive::clump`** is the species the name promised — six limbs a node, dense leaves, gravity to droop them — and
the hamlet grows that now, at `SCALE 3.4` (a clump three metres high) with blades at `1.8`.

Two numbers were measured and one of them confirmed:

| dial | document | the frame |
| --- | --- | --- |
| `Detail::DEFAULT.scaled(2)` | **57.29 MB** | no visible gain over the coarse dial |
| `Detail::DEFAULT` | **20.63 MB** | the same clumps |

So the detail dial stays at its default: a settlement's plants are seen at ten to thirty metres, and the two times finer
dial cost thirty-seven megabytes of document to change nothing anyone can see.

### The hill village got the same bridge and it did not hold

`hillside_village` was given five clumps through the same call, and **they drew above the horizon** — the terrain under
them is not the terrain in the frame, because the camera stands outside the hill's shoulder and looks along the slope, so
the seats I wrote by hand landed on ground that is off-frame. The change is reverted, and the note is:

- **a plant's seat on a terrain must be chosen in the terrain's own frame**, not by reading the pin from a frame; the fix is
  to scatter on the same field the ground mesh is built from, and to refuse a seat whose ground is outside the frame.

**Identity: 27 scenes, 110 245 rows, unchanged** — plants are not building elements. The hamlet's document is 20.63 MB,
against 12.57 MB for the bare culms and 7.75 MB for no plants at all.
## Round 62: the hill village's scrub, and a plant drawn twice

Round 61 put five clumps round the hill village and every one drew against the sky. Two faults, and the second is the one
that mattered:

- **the seats were read off a frame.** They are now **scattered on the terrain's own field** and refused twice: the seat's
  ground must be within a metre and a half of the platform **and** its plan inside the square plus three metres. The level
  alone is not enough — the massif carries ground at the platform's own height elsewhere on its flank, and that is exactly
  where the hand-written seats landed. A terrace is a *place* as well as a height.
- **the wood was translated twice.** `wood_points_of` bakes each placement into the points it packs, and the bridge also
  pushed one instance **per placement** with that same translation. Every clump was drawn at twice its plan coordinate and
  twice its height — on the hillside forty metres above a plateau standing at twenty, and in the flat hamlet so far out
  that only the clumps nearest the origin were in frame at all. The wood's instance is now **the identity**, which is what
  a world-space mesh wants.

Both scenes now have their plants where they were meant to stand: nine clumps on the hillside's shelf between the houses,
six in the hamlet's yards.

`SEATS_DEBUG=1` prints every kept seat with the ground under it, which is how the twenty-metre plateau was found.

**Identity: 27 scenes, 110 245 rows, unchanged.** The hillside's document is **110.32 MB** against 94.86 MB, and the
hamlet's is 20.62 MB.
## Round 63: a bridge for two species, and no tree to grow

The bridge was one species hard-coded. It is now **parameterised by a prefix, a species, and two scales**, and an `Extra` can
absorb another, so a settlement holds a clump and a tree in one scene:

- **every mesh label carries the prefix** — `clump-wood`, `tree-wood`, `clump-leaf-0` — so two species never collide;
- **each instance names its own material in the tuple**, and that was a defect worth finding: the name had been *derived*
  from the mesh, so a leaf drew with `clump-leaf-0-leaf-depth`, which no scene declares. The first scene with two species
  refused to write at all: `instance 'clump-leaf-0-depth-0' names material 'clump-leaf-0-leaf-depth', which the scene does
  not declare`.

### There is no tree species, and that is the finding

The vegetation crate has five: **bamboo** (a culm), **tuft**, **clump**, **shrub**, and **sculpted** — which is a *bamboo*
with a hand's edits, not a tree. Measured: a shrub at `11.0` is **nine metres of bare wood with a shrub's leaves on it**,
and it draws as a twig above the roofs. No dial makes a crown out of a species that has no crown.

`settlement_plants::trees` is the seam and is **unused**: a settlement's tree needs a species of its own — more generations,
a bough that divides at the top, and a leaf field dense enough to be a canopy — and that is a vegetation-crate job, not a
scene's.

**Identity: 27 scenes, 110 245 rows, unchanged**; the hamlet's document is 20.63 MB, as before.
## Round 64: `derive::tree`, the species round 63 found missing

The vegetation crate had five species and no tree. It has six now, and the difference from a stretched shrub is the one the
name promises — **a bole**:

- **`bole`**: a clear four metres to the first bough, tapering `0.150 → 0.045`, with a slight lean. `stem` is half a metre
  with the shrub's limbs from `0.35` of it, which is why a shrub at `11.0` was nine metres of bare wood with a bush at its
  foot.
- **`bough_one` / `bough_two` / `bough_three`**: the shrub's three generations at a tree's scale — 1.15 m boughs off the
  bole's top third, 0.78 m branches, 0.46 m twigs, the departure easing from a radian to a little over.
- **Leaves on every generation**, 18, 40 and 90 a metre: a crown is one mass seen from outside, and leaves on the twigs
  alone left the boughs bare — measured at 26.62 MB of document for a figure of twigs.

`derive::tree(seed)` is the species; `settlement_plants::trees` — the seam round 63 left unused — now grows it. The Dai
hamlet has three trees between its houses, and their boles stand a head above the roofs.

**Identity: 27 scenes, 110 245 rows, unchanged**; the hamlet's document is 28.39 MB against 20.63 MB for clumps alone. The
trees are **sparse** — a crown of blades at a settlement's distance still reads more as branches than as foliage — and the
dial that would thicken it is the leaf field's density, not its geometry.
## Round 65: a crown that reads, and trees in both settlements

**The crown was a parameter and not a shape.** Three dials moved it, and every one of them was measured:

| the change | the hamlet's document | the frame |
| --- | --- | --- |
| three leaf fields at 18 / 40 / 90 a metre | 28.39 MB | branches with a few sprays |
| the same at 40 / 90 / 200, blades half again as large | **32.90 MB** | leaves along every bough |
| **five to seven boughs** a generation instead of three | **43.17 MB** | a crown with a silhouette |

So the geometry was the lever and not the leaf count: a bough generation of three can be as leafy as it likes and still be
three branches. The tree's rule now puts five to seven boughs on the bole, four or five branches on each, and three or four
twigs on those.

**Both settlements plant now.** The hill village takes the same scatter it already had and gives **every third seat to a
tree**, the rest to clumps: a canopy over the yards and brush along the wall.

**Identity: 27 scenes, 110 245 rows, unchanged** — plants are not building elements. Documents: the hamlet at 43.17 MB
(from 20.63 with clumps alone) and `hillside_village` at **126.28 MB** (from 110.32).
## Round 66: the 藏族碉房, the gallery's eleventh type

**A stone fortress a family lives in.** The other ten types are one to two storeys because their materials are light —
bamboo, earth, timber, boards. This one is **six hundred millimetres of stone**, which is why it can be three storeys, and
its windows are small and high because a window is a hole in a wall that must still carry.

It is built the way this crate allows a tall house to be built — **two masses, the third storey standing back by `0.40 m`
on the lower one's roof and carried through `Join::Storey`** — with a flat threshing floor on each tier, a parapet round
each, a 煨桑炉 in the top roof's corner and one timber gallery on the sunny face.

Two things were measured on the way:

- **a mass inside another mass's plan is refused**, and the refusal is asked of every pair: the burner as a mass read as
  `UnattachedOverlap { mass: 0, other: 2 }` *and* `{ mass: 1, other: 2 }`, because its plan lies inside both tiers', and an
  attachment to one of them does not answer the other. It is drawn as **four walls of members** now, which is a ring of
  stone with a fire in it and has no plan to overlap.
- **a balcony with rails and no deck is a frame in the air**, which is what the first gallery drew: two posts, two rails
  and nothing to stand on. Its floor is three planks across the brackets, and its posts run from the **first roof** rather
  than hanging off the wall.

**Identity: 28 scenes, 110 529 rows**, of which `tibetan_house` is **284** — a tower of two tiers, four windows, a door,
two parapets, a burner and a gallery.
## Round 67: the plateau village, the gallery's third settlement

**Four towers in a row with the sun on their galleries, and the shelf falling away behind them.** A mountain village is a
terrace village round a square; a Dai hamlet is a row on a valley floor; a plateau village is neither — its ground is one
shelf at one height, its houses are **one type at one size**, and what holds it up is a wall.

- **four `tibetan::document()`s**, all facing south: a plateau household puts its only timber where the sun is, whatever
  the lane does;
- **two trees and three clumps** from the same bridge the other two settlements use;
- **one retaining wall** at the shelf's north edge, one metre of stone holding a terrace a metre and a half high.

Three refusals were measured on the way, and two of them are the shared scene shell's own:

- **a retaining wall inside a house's plan** is `UnattachedOverlap` against all four houses, because a mass inside a mass is
  a mass with a host — the wall stands clear of the row now;
- **a mesh with no points is not a mesh**: a tower village wears no roof, so `roofs` was declared empty and the document
  writer refused it (`mesh roofs has no points`), and after the mesh was skipped the *instance* outlived it
  (`instance roofs-depth names mesh roofs, which the scene does not declare`). **One table of what the building has now
  decides both**, which is a fault any scene with an element kind missing would have hit;
- **an outline drawn the other way round** answers `OffsetInverted` — the wall's four nodes were listed clockwise when the
  reorder was made, and the crate says so with an area of `-157.85`.

**Identity: 29 scenes, 111 966 rows**, of which `plateau_village` is **1 437** — four towers at 284 rows each, the wall, the
trees and the clumps. The village's document is 28.91 MB.
## Round 68: the 屯堡石板房, the gallery's twelfth type

**A house that quarries its roof.** The 碉房 carries a timber gallery and an earth roof, the 土掌房 is earth, the 木楞房 is
logs — this one is **all stone**: a wall of `0.55 m`, a yard of the same stone, and a gable roof laid in thin slates. A
屯堡 is a Ming garrison's descendants' village, and what a Ming garrison built with was the ground under it.

The type's whole subject is one number — **a `Covering` with `period` 0.42 and `fill` 0.94**, a wide flat course and not a
roll — on a low pitch (`RISE` 1.5 over 6.4, a third of the 竹楼's), because a slate roof is heavy. The yard is a slab of
the wall's own material in front of the door, which is what a 屯堡's 院坝 is.

**Three refusals, all from the crate and all measured**:

- a wall that stops at the eaves leaves the loft open — the first frame showed the roof's underside as a hole — so the
  gable ends are split and crowned, as the 一颗印's are;
- **a crowned face cannot be gridded**: naming the front edge by index before the crown answers
  `GridOnACrownedFace` and then `FaceNotPlanar` at `0.303`;
- and **an `Opening` cuts inside a bay**, so an unsplit face answers `None` and an inset asked of the wrong edge answers
  `OpeningOutsideWall { edge: 3, past: 0.411 }`.

**So the house has its door and not yet its windows**, and that is this round's honest state: the type's form — stone wall,
crowned gable, slate courses, yard — is landed, and the two windows are the next round's work.

**Identity: 30 scenes, 112 218 rows**, of which `slate_house` is **252**.
## Round 69: the 石板房's windows, and a transposed pair that cost two rounds

Round 68 landed the twelfth type with a door and no windows and recorded three refusals. **All three were one pair of
arguments in the wrong order, read three different ways:**

```rust
Opening::new(face, row, column, inset)   // the row comes first
```

Every attempt passed `(column, 0)` — the door asked of row 0 or row 2 of a **one-row** face, and the third bay never cut at
all. What that produced, in order:

| the symptom | what it was |
| --- | --- |
| `None` from `add_opening`, silently | the face had not been split yet |
| `GridOnACrownedFace`, then `FaceNotPlanar` at 0.303 | the front edge named by index after the gables were crowned |
| `OpeningOutsideWall { past: 0.411 }` | the *row* index standing where the *column* belonged |
| `OpeningOutsideWall { past: 0.331 }` | **the mitre**: a window 0.22 m from the outer corner is 0.33 m *past* the inner one, which is the wall's 0.55 less that 0.22 |

So the last fault is a real invariant of the model and not a mistake in the call: **an opening stands at least a wall's
thickness clear of the face's own ends**, because the loop it is cut into is mitred inward. The slates' house windows are
`0.70 m` in from their bays' ends and the door the same, and the face is split into its three bays first — after the gables
are crowned, because a split moves every station after it.

`slate_house` **252 → 260 rows**: two windows of four reveal faces each.

**Identity: 30 scenes, 112 226 rows**, the other twenty-nine unchanged.
## Round 70: the 屯堡村, the gallery's fourth settlement

**One street, four of the same house, a wall and a 朝门.** The mountain village is walled houses round a square, the Dai
hamlet is a row on a valley floor, the plateau village is four towers on one shelf — a 屯堡 is a **garrison's compound**, and
its one building that is not a house is the gate at the lane's end.

- **four `tunpu::document()`s** in a row, all facing the lane, so their yards are the lane's edge;
- **the compound's wall in two lengths** with the gate's gap between them — a wall with a gap is two walls;
- **the 朝门**: two stone piers and a lean-to roof over the gap, `2.7 m` clear, which is the one place a 屯堡's lane is a
  door.

Two refusals came from the model, and both are about a mass's own thickness:

- **a wall's wall must be thinner than half the wall's depth.** The first draft was 0.9 m of stone with a 0.55 m wall and the
  crate answered `OffsetEdgeFolded { edge: 1 }` and `OffsetInverted { area: -4.02 }` for each length: the inner loop is
  offset inward by the wall's own thickness, and two of those fold through each other. The wall is 1.1 m deep with a 0.40 m
  wall now, and each pier is 0.45 m square with a 0.18 m wall for the same reason;
- **the piers stand in the gap and not on the wall's line**: on the line they are masses inside masses
  (`UnattachedOverlap { mass: 4, other: 6 }` and `{ mass: 5, other: 7 }`), and set back by their own width they touch the
  wall's end without sharing any area with it.

**Identity: 31 scenes, 113 324 rows**, of which `tunpu_village` is **1 098** — four houses at 260 rows, the two walls, the
two piers and the gate's roof. The document is 7.10 MB.
## Round 71: the 碉楼 at the lane's end

A 屯堡's one building that is neither a house nor a wall is the **watchtower**, and the type's own record said the scene did
not draw it. It does now: a square stone tower of two storeys at the lane's west end — `3.2 m` square, `5.6 m` tall, an
arrow slit in every face of both storeys and a **merlon on every edge of the top**.

Built the way this crate allows a tall mass: **two masses, the upper carried by the lower through `Join::Storey`**, so each
takes its own cuts. The slits are cut the way round 69 taught — a face split into one bay first, then an inset that stands a
wall's thickness in from the bay's ends, because the loop an opening is cut into is mitred inward.

One refusal, and it was mine: the merlon's `across` was written as an expression over the edge's own direction, which
produced a section laid *along* the member — `MemberDegenerate { length: 3.2 }` four times, the length reading like a
degenerate length and being the merlon's own. The section is the perpendicular of the edge now.

**Identity: 31 scenes, 113 406 rows**; `tunpu_village` 1 098 → **1 180** — two storeys' walls, eight slits' reveals and four
merlons.
## Round 72: the 石板房's court, and the shape of a wall

A 屯堡 house does not stand on the lane; it stands **behind a wall in its own court**. The type's scene showed the house and
its yard and stopped there. `slate_court` closes the yard: the house at its head, three lengths of the same stone round the
other sides, and the **门楼** over the gate — two piers and a lean-to, the one piece of the court that is not stone.

The wall pieces were where every refusal came from, and each names something about a wall's own shape:

- **a plan's winding depends on its run's direction**, and the same four corners listed for a wall running east and one
  running west wind opposite ways: `OffsetInverted { area: -15.81 }` and `{ area: -7.82 }`. One `wound` helper decides it,
  and it **keeps the first node**, so a wall's start stays the corner its neighbours were measured from;
- **two lengths that each run to the corner share the corner's area**: `UnattachedOverlap { mass: 1, other: 3 }` and
  `{ mass: 2, other: 4 }`. The side walls now stop at the south wall's inner face and the south wall spans the whole width —
  meeting face to face is the join a wall has;
- and the round-70 pair still hold: the wall is `0.35 m` in a `1.0 m` depth so the inner loop cannot fold, and the piers stand
  **in the gate's gap** with `wall = 0.18` so they are neither masses inside masses nor folded.

**Identity: 32 scenes, 113 750 rows**; `slate_court` **344** — the house's 260, three walls' tops and faces, two piers and
the gate's roof. The document is 7.18 MB.
## Round 73: three invariants that had only ever been read in an error

Rounds 66, 69, 70 and 72 each lost time to a rule the crate enforces but never wrote down. They are in the documentation now,
each with the round that measured it and the refusal it produces:

- **`Mass::wall`** — the wall's thickness must be **less than half the plan's narrowest extent**, because the inner loop *is*
  the outline offset inward by it and two such offsets meet in the middle (`OffsetEdgeFolded`, then `OffsetInverted`). Round
  70's compound wall and round 72's pier were both as thick as they were wide.
- **`Level::outline`** — **a plan winds positively**, and which way that is depends on the direction the run is walked, so the
  same four corners wind opposite ways for a wall running east and one running west (round 72: `OffsetInverted` at areas
  `-15.81` and `-7.82`).
- **`Inset`** — an opening stands **at least one wall's thickness clear of the face's own ends**, because the loop an opening
  is cut into is mitred: a hole 0.22 m from a corner of a 0.55 m wall is 0.33 m past the inner corner, reported as
  `past: 0.331` (round 69).
- **`Opening::new`** — **the row comes before the column**, and both are `usize`, so transposing them compiles and asks for a
  bay the grid does not have (round 68, which read one transposition as three different faults).

The winding rule is also **code now and not a habit**: `ops::wound(plan)` is the one place the order is decided, and
`slate_court` uses it instead of the private copy it grew in round 72. A refactor owes a measurement as much as an addition
does, and this one has the strongest kind: **the frame is byte-identical** to round 72's (`38c99288…`) and the identity is
unchanged at **32 scenes / 113 750 rows**.
## Round 74: the 白族三坊一照壁, the gallery's thirteenth type

**A courtyard with three ranges and one screen wall.** The 一颗印 is a Yunnan courtyard that is a *box*; this one is a court
with a **照壁** closing its approach — the wall that gives the type its name, white plastered and wearing a **tiled coping**,
which is the type's whole silhouette.

- **the 正房** is two storeys at the head of the court, carried through `Join::Storey` so each storey takes its own cuts, and
  its front is three bays with a door in the middle;
- **the 厢房** are one storey each, flanking the court, one door apiece facing across it;
- **the 照壁** is a mass `0.5 m` thick with a `0.18 m` wall in it and a gable roof for its coping — a wall with a little roof
  on it is exactly what a screen wall is, and it is the same `gable_roof` call as the 正房's at a different span;
- **the court is paved**, which is what makes four walls a court.

Two things the frame taught, both about which face is which:

- **the front face is named by the plan's winding**, not by its position in space: the 正房's door was cut on `edge 2`, which is
  the `+z` side, so the house faced *away* up the hill and the first frame showed a blank wall to the court. The face that
  looks into the court is `edge 0`;
- **a 照壁 3.1 m high hides the court from its own approach**, which is right — the view the type was built for is the wall's
  coping in the foreground and the 正房's upper storey behind it.

**Identity: 33 scenes, 115 488 rows**; `bai_court` **1 738** — two storeys, two wings, the screen wall, four roofs, a paved
court and eight openings' reveals. The document is 7.59 MB.

The third 坊 and the corner 门楼 are not drawn, and the type says so: the camera stands where the third range would be.
## Round 75: the 门楼, and a sentence of the type's own that was wrong

Round 74's note said the court does not draw **"the third 坊 and the 门楼 in the corner"**. The second is true and the first is
not: **the fourth side of a 三坊一照壁 is the 照壁**, which is what the name says, so a scene of the type has exactly three
ranges and the camera does not stand where a range would be — it stands on the screen wall's approach. The note is corrected
and the court is entered now.

**The 门楼** is the corner between the east 厢房 and the 照壁: a short wall on the court's east line, a door through it and a
lean-to hood over the door. It runs from the wing's south face to the screen's inner face, so it **meets both and shares area
with neither** — the join a wall has (round 72). Its door is cut the way round 69 measured: one bay, an inset a wall's
thickness clear of the bay's ends, which the gate wall's own `0.40 m` depth demands and no more.

**Identity: 33 scenes, 115 550 rows**; `bai_court` 1 738 → **1 800** — the gate wall's faces and top, one door's reveals and
the hood. The document is 7.61 MB.
## Round 76: the 门楼's own frame

Rounds 74 and 75 drew the gate as **a hole in a wall**. A 白族 gate is framed — a threshold, two jambs, a lintel and a **门罩**
over it — and the frame is what a person reads at a door, so it is drawn now: six members, each laid **in the wall's own
thickness and proud of it**, because a frame stands in front of what it frames.

The frame's own measurements come from the opening it frames, and not from a table: its clear span is the door's
(`GATE_HIGH - 1.05` to the threshold), its jambs stand clear of what the opening already leaves, and the 门罩 stands a little
further out than the lintel because that is the one member that is decoration and not structure.

The scene's camera moved with it. The approach view that showed the 照壁 as a screen now stands off its east end, along the
wall to the gate, so the frame is the foreground and the 正房 rises behind — **the shot is part of the type's record, and a
detail added where no shot can see it is a detail no one will check**.

**Identity: 33 scenes, 115 580 rows**; `bai_court` 1 800 → **1 830** — six members, thirty rows. The document is 7.62 MB.
## Round 77: the court's well and its planting

A courtyard with four walls and a paved floor is a yard. A 白族 court has its **井** and its flowers against the 照壁, so the
court has both now:

- **the well is a mass whose wall is thick enough to be a ring** — `0.22 m` in a `1.24 m` plan, so what the expansion draws is
  a curb with a hole in plan. It is the same answer the Dai hamlet's well-head gave, and it satisfies round 73's invariant
  without being told to: the inner loop cannot fold in a plan four times the wall;
- **three clumps at the corners the wings leave**, from the bridge the settlements use, and the court's own paving under them.

The camera moved **inside** the court, and that is the round's real measurement. Every shot from beyond the gate — rounds 74,
75 and 76 — showed the wall that closes it and nothing of what it closes: the well and the plants were invisible for two
rounds of work. The court is 8.4 m across, so the inside shot has to be a corner's diagonal with a wide field of view, and the
well was moved to the corner that shot can see.

**Identity: 33 scenes, 115 593 rows**; `bai_court` 1 830 → **1 843** — the well's curb, its top and its hole, thirteen rows.
The document is **13.91 MB** against 7.62, which is the planting's price and is the same trade the settlements made.
## Round 78: the 白族巷, the gallery's fifth settlement

**A lane with a courtyard house behind each gate.** The mountain village is walled houses round a square, the Dai hamlet is a
row on a valley floor, the plateau village is four towers on one shelf, the 屯堡 is a garrison's compound — a 白族 village is
a **巷**: a lane, and the courts turn their one formal face onto it.

- **three `bai_court::document()`s**, two west of the lane and one east, each turned so its **gate opens onto the same strip of
  paving**. The court's gate faces `+x`, so the west plots take no turn and the east plot is turned a half — the whole
  arrangement is that one fact about which way a gate looks;
- **the lane is a `Slab::paving`** from one end to the other: it has no wall and nothing stands on it, so it is not a mass;
- **five clumps along it**, from the bridge the settlements use.

**Identity: 34 scenes, 121 128 rows**; `bai_lane` **5 535** — three courts at 1 843 each and the lane's paving. The document is
20.34 MB.

The lane's other end and the village's 本主庙 are not drawn, and the type says so: three plots are this scene's frame.
## Round 79: the 白族本主庙, the gallery's fourteenth type

**The village's own temple, and the first public building in the gallery.** A 本主庙 has two buildings because the village has
two duties: the **大殿** where its 本主 is, and the **戏台** where the god is *entertained*. The pair facing each other across a
court is the whole type, and it is what this scene draws:

- **the 大殿 stands on a plinth** a half-metre above the court, ten metres across with a two-metre roof rise —
- **the 戏台 stands on a higher plinth**, seven metres across, with a **mouth** the width of the stage on the face that looks at
  the 大殿, because it is a stage and not a room;
- **the court between them is paved** and closed by two side walls, so the two buildings are one temple and not two houses;
- **the 大殿's front is a colonnade**: three bays, each cut to the plinth so the piers between them are what the village stands
  behind.

Two things the frame taught:

- **the plinth is a floor slab and not a mass**, so nothing stands inside anything: `Slab::floor` from `0.0` to the plinth's
  height is exactly a stepped floor, and the two buildings sit on their own.
- **the 戏台's mouth faces the 大殿**, so a camera from the court's south sees the stage's back. The shot is high and
  three-quarters now, over the stage's roof into the court and down onto the colonnade — the type's axis is only legible from
  above.

**Identity: 35 scenes, 122 240 rows**; `benzu_miao` **1 112** — two masses, two plinths, two roofs, a paved court, two side
walls and four openings' reveals. The document is 7.39 MB.
## Round 80: the 碉房's batter, and the chain a stack actually is

The type's own record has said since round 66 that **the batter is not drawn**: a real 碉房's walls lean in as they rise, a
banded mass cannot be cut, and so the tower was vertical. It leans now — and the answer is the one the tradition gives:

- **the wall is laid in courses stepped one on the next — 叠涩**. A ground course of `2.60 m` carries the door and the two
  windows, and three courses above it are each set in by `0.09 m`, so the wall leans `0.27 m` over `5.2 m`. **Every course is a
  one-band mass, which is cuttable**; a banded mass would have refused the openings by name.
- **round 6's verdict is the rule obeyed here**: *model the relationship, not the overlap*. The courses share an edge and share
  no volume, and each is carried by the one below through `Join::Storey`.

Three refusals came out of it, and the third was a real fault in my own reading of the crate:

- **`UnattachedOverlap` against every intermediate course, and `StoreyNotOnTop`** — the third storey was attached to the
  *bottom* of the four courses. The validator reads a stack as a **chain** (`stacked_above`), so a mass that names the bottom
  of four leaves three pairs unrelated. Naming the course directly below it answers all of them at once: the chain *is* the
  relation.
- **a door is two metres and a course of four is 1.3** — `OpeningUnresolved` on the ground face. So the courses are not equal:
  the ground course is a storey and only the batter above it is stepped.
- **an opening's sill needs a margin like every other side**: a sill at `0.0` in a course whose floor slab hangs `0.30` below
  reaches the face's boundary. The door's sill is `0.30` now, which is the slab's own thickness.

`plateau_village` had asserted that a 碉房 *is two stone tiers*, and it is five masses now. The assertion names what it needs —
a house with no seated mass floats — instead of a count that has to be rewritten whenever the type is laid differently.

**Identity: 35 scenes, 123 172 rows**; `tibetan_house` 284 → **288**, `plateau_village` 1 437 → **2 365** (four towers, each with
a battered wall). The tower's document is 4.30 MB, unchanged to the hundredth.
## Round 81: `derive::stepped_courses`, and the course count a door decides

Round 80 laid the 碉房's batter out inline. It is **`derive::stepped_courses` now** — one place that lays a plan in courses,
attaches each to the one below through `Join::Storey` and returns the ids — and the type calls it with three numbers instead of
a loop of twenty lines.

Getting there cost four refusals, and every one was the same mistake in a different dress: **the ground course must be a whole
storey.**

| what the code did | what the crate answered |
| --- | --- |
| four equal courses, a door in the ground one | `OpeningUnresolved` — a door is 2 m and the course is 1.3 |
| a hand-cut door and windows, insets sized for a 5.2 m face | `OpeningUnresolved`, twice |
| the ground course wears `Front::RANGE` | `OpeningsOverlap { left: 0, right: 5 }` — a row per storey on a 2.6 m face |
| `Front { storeys: 1, bays: 3, door: true }` **and** the hand-cut pair | `OpeningsOverlap` again — the same face cut twice |

So the ground course is `BASE_HIGH` (3.45 m, the height at which a three-bay façade resolves), the two courses above it split
what is left, and the openings are the façade's alone. **A batter starts above a door**, which is what a stone wall does and
what the crate's own cut rules insist on.

**Identity: 35 scenes, 123 107 rows**; `tibetan_house` 288 → **323** (the third course and the façade's small high windows),
`plateau_village` 2 365 → **2 265** (four towers, each laid differently). The lean is `0.15 m` a course — steeper than a real
碉房's 3–5 %, and the module says so rather than pretending the number is surveyed.
## Round 82: the tower gains a storey so its batter can be honest

Round 81 had to choose between a real ratio and a visible lean, and it chose **17 %** — a wall steeper than any 碉房. The
reason was arithmetic: the batter region was one storey, and three courses over 1.75 m cannot be both.

**So the tower is four storeys now.** The batter region is two storeys, the ground course still carries the door, and the four
courses above it step in by `0.07 m` each — **0.28 m of lean in 5.2 m, which is six per cent** and is what a coursed stone
tower actually does. The frame still reads it: the wall's silhouette narrows visibly over its height, and the bands are there
to be counted.

The camera rose with the house — a four-storey 碉房 is 10.4 m and the plate is about its whole height.

**Identity: 35 scenes, 123 909 rows**; `tibetan_house` 323 → **349** (the fourth storey's walls, its slits and its roof
floor), `plateau_village` 2 265 → **3 041** (four towers, each a storey taller). The tower's document is 4.14 MB, and the
module's own note no longer has to apologise for the number.
## Round 83: the stranded top storey, and a NUL byte in the module's own doc

Round 82 gave the tower a fourth storey and left its top floor **reached by nothing**: the one external flight climbs to
`lower_top`, which is the threshing roof, and the storey above it had no way in.

- **a second flight now climbs the east wall** from the threshing floor to the top roof, `SECOND_STEPS` treads to a storey
  instead of the first flight's `STEPS` to two. It lands through the top roof's parapet, which is where the storey above the
  threshing floor is entered.
- **and a defect in the file itself is fixed**: `crates/gallery/src/tibetan.rs` carried **one NUL byte** inside its own module
  documentation, left by a line-surgery edit in round 81 — `At \0.15 m a course` — which made the file *binary* to every tool
  that reads it and silently swallowed the backtick that was meant to quote the number. It is gone, and the sentence it broke is
  rewritten: the lean is six per cent in four courses and that is why the tower is four storeys tall.

**Identity: 35 scenes, 124 089 rows**; `tibetan_house` 349 → **385** (six treads), `plateau_village` 3 041 → **3 185** (four
towers, each with both flights). The tower's document is 4.35 MB.
## Round 83b: the same corruption in a second module

The NUL byte found in `tibetan.rs` was not alone: `crates/gallery/src/southwest_wave3.rs` carried **two**, both inside a
comment — `A colour component, not a constant: \0.318 is the green of a rhizome's section` — where a backtick-and-zero pair from
a scripted edit in an earlier round had collapsed into a NUL. Both are `0` again, the file reads as UTF-8, and a sweep of every
scene source finds **no file carrying a NUL byte**.

`cargo build --workspace --examples` clean, **35 scenes / 124 089 rows** unchanged — a comment repair owes a measurement too.
## Round 84: the parapet's gaps, and the side that had none

Round 83 left the second flight climbing **through** the top roof's parapet, and fixing that turned up a worse fault underneath it.

The parapet was built by listing four sides, then — for the top roof only — **popping the west side** and pushing **two north
pieces** in addition to the north side already in the list. So the top roof's parapet had **no west edge at all** and its north
edge was drawn three times: one full length and two pieces over it. The stair climbed through the east side because there was no
gap to land in.

Each side is built once now: south and west whole, east in two pieces with the second flight's landing between them, north in two
pieces with the balcony's bay between them. The gaps are the two ways onto the roof, and the parapet is complete round the rest
of it.

**Identity: 35 scenes, 124 119 rows**; `tibetan_house` 385 → **391** (the west edge restored, the east edge split), and
`plateau_village` 3 185 → **3 209** — four towers, each with a parapet that goes all the way round.
## Round 85: the parapet's cap

A stone parapet is not a wall that stops; it is a wall under a **cap**, and the cap is what sheds the water a flat roof would
otherwise take at its edge. The parapets on both roofs of the 碉房 have one now: a second member over each side's own run and
section `PARAPET + 0.10` wide, so it stands `0.05 m` proud of the wall on each face, `0.09 m` high.

It follows the sides rather than replacing them, so the two gaps round 84 made — the gallery's bay and the stair's landing — are
gaps in the coping as well, and the cap is continuous everywhere the wall is.

**Identity: 35 scenes, 124 419 rows**; `tibetan_house` 391 → **451** (ten cap members: four on the lower roof, six on the upper
where the sides are split), `plateau_village` 3 209 → **3 449** (four towers, each with both roofs capped). The tower's document
is 4.37 MB.
## Round 86: the 一颗印's 照壁

A 一颗印 opens **straight onto the lane**, and what a Yunnan courtyard sets against the lane is a **照壁** — a white screen
wall, head high and wider than the gate, wearing a tiled coping. The type has one now, standing `1.70 m` out from the gate
range: a mass `0.50 m` deep with a `0.18 m` wall in it, so the inner loop is a third of the depth and cannot fold, and a gable
over a half-metre span for its coping — the same `gable_roof` call the ranges wear at a different span.

It is the piece the type was missing at its own front: the gate had a threshold, boards and a 门罩, and beyond them nothing,
where a 一颗印's gate is always seen against a screen.

**Identity: 35 scenes, 124 845 rows**; `yikeyin` 6 828 → **6 841** (the screen's faces, its top and its coping), and
`yikeyin_court` unchanged — it is the court's interior and the screen is outside it. The document is 6.29 MB.
## Round 87: the 土掌房's second flight

Round 56's own record said the 土掌房's stair **reaches the terrace and stops**: "goes on to the room above by a second flight
this document does not draw". The top roof has waited thirteen rounds, and its parapet has had a **gap in it since round 62** —
a gap made for a stair that did not exist.

- **the second flight climbs from the terrace to the top roof**, `SECOND_STEPS` treads for the storey above the 晒场.
- **it is on the same flank as the first**, so the house is climbed by one straight run broken at the terrace, which is how a
  土掌房 is climbed. Its treads run across the terrace's own walkway — exactly `SETBACK` wide — and it comes out through the
  parapet's gap.
- **and it is on the camera's side**, because a flight added where no shot can see it is a flight no one will check (round 77's
  lesson, learned the hard way).

**Identity: 35 scenes, 125 181 rows**; `earthen_house` 182 → **230** — eight treads, their sides and their reveals. The
document is 4.27 MB.
## Round 88: the lane's well

A 白族巷 has a well, and it is the lane's institution: the courts keep their own water in their yards, and **the well is where
the lane meets**. It stands at the lane's south end — a stone curb `1.44 m` across whose wall is `0.24 m`, so what the expansion
draws is a ring with a hole in plan and the inner loop cannot fold — with two clumps of bamboo beside it, because a lane's
planting is watered from the well.

The well is a **mass** and not a member, for the same reason the Dai hamlet's well-head is: a well has an inside, and a block
with no inside is a block.

**Identity: 35 scenes, 125 194 rows**; `bai_lane` 5 535 → **5 548** — the curb's four faces, its top and its hole. Two clumps
were added and the identity does not count them, because plants are not building elements. The document is 20.35 MB.
## Round 89: the 巷门, and how this crate carries a thing over a gap

Two rounds of notes said a gateway could not be modelled: a lintel over a lane is a mass over a void with nothing under it, and
`UnattachedOverlap` refuses every attempt to stack one.

**It is `Join::Under`, read the other way round.** The validator's own words: a mass with `Some(host) if join == Under` claims
that **its top meets the host's foot and its plan stands inside the host's** — so the *mass carries the host* and the host is
the thing above. A storey names the mass *below* it; a gateway names the mass *above* it, with both piers declaring `Under` the
lintel. Every overlapping pair then has a relation and the overlap question is dropped for it.

So the 白族巷 has its gate: a lintel `8.8 m` across standing `3.2 m` up, two piers under it outside the lane's paving — a cart
is not stopped by them — and a lean-to roof over the lintel. The lane passes under it, and the shot is taken through it.

**Identity: 35 scenes, 125 239 rows**; `bai_lane` 5 548 → **5 593** — the lintel's faces and top, the two piers, and the gate's
roof. The document is 20.37 MB.
## Round 90: the 巷门's leaves and its 题额 — and a rule that was already written

The round began as a documentation task: put round 89's reading of `Join::Under` into the crate. **It is already there**, and it
has been since the variant was added — "the mirror of `Storey` … a bridge's masonry pier and the timber gallery over it share a
plan for as long as the bridge stands; without a word for that, `validate`'s overlap question reports the pair as two masses in
each other's way". Two rounds were spent deriving a paragraph that sits in `mass.rs`, four hundred lines from where the failure
appeared.

So the round drew the gate's two missing pieces instead:

- **two leaves, standing open against the piers.** A 巷门 is a gate and not an arch: it closes, and when it is open each leaf lies
  back along its pier. The first version drew them as **rods**, because a member whose two anchors differ in elevation is a post —
  a leaf's run is its own width along the lane and its section is a hand's thickness across by the opening's height up;
- **the 题额**, the board over the opening: the one piece of a gate that is written rather than built, `0.12 m` proud of the
  lintel's stone.

**Identity: 35 scenes, 125 257 rows**; `bai_lane` 5 593 → **5 611** — two leaves and the board. The document is 20.37 MB.
## Round 91: the 穿斗式民居, the gallery's fifteenth type

**The one structural tradition the other fourteen had left out.** The 抬梁 frame carries its roof on beams that span bay to bay;
**穿斗** carries it on a **row of thin posts set close together**, tied by purlins pushed through them, and the boards between the
posts are not structure. That is why a 穿斗 house's front is a colonnade: the posts *are* the wall's rhythm.

- **six posts at two-metre centres** down the front, standing `1.60 m` out from the body's own wall, with one beam across their
  heads — the purlin at this scale;
- **deep eaves**: the roof's overhang is `1.30 m` on three sides and `1.30 + 1.60` at the front, because a board wall needs the
  rain kept off it, and that need is what a colonnade is for;
- **a board body** `0.22 m` thick behind the posts, with a door and two windows cut in three bays.

Three refusals framed the work, and the third is the one worth keeping:

- **`OpeningOutsideWall { past: 0.048 }`, three times** — an inset of `0.34 m` in a `0.22 m` wall is short of the **mitred end**;
- **`OpeningOutsideWall { past: 3.574 }` and `{ past: 7.241 }`** — one bay each, which is what a **one-column grid** looks like:
  the split had been made on an edge that was half a face;
- **`GridOnACrownedFace`** — because the crown had run *after* the split. The order the crate asks for is: **crown the gables,
  find the front edge by its midpoint's `+z`, split it, then cut.**

`crown_gables` was copied in two documents; it is `derive::crown_gables` now, with the gable-station reading beside it, and both
the 石板房 and this type call it.

**Identity: 36 scenes, 126 323 rows**; `chuandou_house` **1 066** — a body, six posts, a purlin, a roof and three openings'
reveals. The document is 7.38 MB.
## Round 92: the porch's floor, and the 腰檐 this crate will not build

A 穿斗 house's porch is a **floor** and not a pavement: the posts stand on it, and it is where the work that needs daylight
happens. It is a `Slab::floor` a step above the ground, one bay deep, and the six posts stand on its edge.

**The 腰檐 is not drawn, and the refusal is worth recording.** A Sichuan 穿斗 house is often **一挑二重檐** — the main roof over
the body and a lower eave over the colonnade — and the obvious way to build the lower one is `derive::lean_to` hosted by the
body, exactly as the 白族 gate's hood is. Every attempt came back the same way:

```
FaceNotPlanar { face: 28, departure: 0.11000013 }   warped face 28: Mass(MassId(0), WallTop { edge: 3, band: 0, half: 0 })
```

**A `lean_to` hosted by a gable-crowned mass warps that mass's own wall top.** It was tried with the plan crossing the wall and
clear of it, with the eave side first and last, and with the two additions in isolation: the warp is the roof's, not the
ordering's. The 白族 gate's hood works because its host is an uncrowned lintel. So the second eave is the next round's work and
it needs a host this crate can give it — a dedicated fascia mass, or a roof that does not touch its host's wall top.

**Identity: 36 scenes, 126 329 rows**; `chuandou_house` 1 066 → **1 072** — the porch floor's top and its four edges. The
document is 7.39 MB.
## Round 93: the wall plate, and where the 腰檐's warp really comes from

Round 92 blamed the 腰檐's refusal on its **host**: "a `lean_to` hosted by a crowned mass warps that mass's wall top". This round
put the roof on a **plate of its own** — a thin uncrowned mass against the body's wall — and the same `FaceNotPlanar
{ departure: 0.11 }` came back on **`Mass(0)`**, the crowned body. So the host is not what is raised.

Then the plate alone, with the roof dropped:

| what is in the document | what the crate says |
| --- | --- |
| body + porch floor + **plate** | clean, 7 392 939 bytes |
| body + porch floor + plate + **腰檐** | `Mass(0) WallTop { edge: 3, half: 0 }` departs `0.11 m` |

**It is the lean-to roof itself, not its host, that raises the wall top it stands against** — the roof reads the mass its high
edge abuts and lifts that wall's top to the crest, and a crowned wall top has no room for the lift. A dedicated host does not
help, because the abutting mass is found from the plan and not from the `host` field.

So the plate stays and the eave does not. **And the plate is worth keeping on its own account**: it is the 穿斗 frame's wall
plate, the beam a purlin sits on, standing `0.22 m` deep against the boards from the posts' heads to the eave line. The second
eave needs one of two things this crate does not have yet — a roof that does not lift its neighbour's wall top, or a wall top
that can be lifted above a crown.

**Identity: 36 scenes, 126 342 rows**; `chuandou_house` 1 072 → **1 085** — the plate's four faces and its top. The document is
7.39 MB.
## Round 94: the lane laid in flags

The lane's paving was **one slab from end to end**, which reads as concrete and is not what a village street is. It is
`LANE_SLABS` flags now, `3 m` each with a `0.045 m` joint between them — the joint is where the water goes and where a mason's
day ended, and it is the whole difference between a street and a runway.

`derive::court_paving` is *not* this: read before used, it builds a rectangle **with a square drainage grate at its centre**,
which is a courtyard's floor and not a run of flags. So the lane stays a list of `Slab::paving`s and the joints come from the
loop.

**Identity: 36 scenes, 126 420 rows**; `bai_lane` 5 611 → **5 689** — fourteen flags in place of one, thirteen joints. The
document is 20.39 MB.
## Round 95: the 挑枋 and its 撑弓

A 穿斗 frame carries its eaves on **brackets pushed out through the posts** — a short beam cantilevered forward at the post's
head, and a strut under it taking the load back down the post. Six of each now, one pair at every post: `CANTILEVER` of reach,
`STRUT_DROP` of drop, and their shadows on the pavement read the frame's own rhythm.

A post with a roof on its head and nothing else is a post holding a roof up by itself, which is what the frame drew until now.
The bracket is the joint the tradition is built out of, and it is the reason the eaves can reach `2.9 m` past the wall.

**Identity: 36 scenes, 126 492 rows**; `chuandou_house` 1 085 → **1 157** — six brackets and six struts, with their sections and
their ends. The document is 7.42 MB.
## Round 96: the well's 辘轳 — and the 土掌房's batter closed as impossible

**The 辘轳 comes first.** A lane's well is not a hole with a curb: it is a curb with a **frame** over it — two posts, a beam, and
a drum between them for the rope. Four members, and they are what makes the well an object a village uses rather than a drain.

**The 土掌房's batter is closed, and this time by reading rather than by refusing.** Five rounds deferred it; opened, it cannot
be built in this crate: the type is two storeys and the **door and windows occupy both of them** (`Front::RANGE` cuts a row per
storey), while a batter in courses needs a **storey-high ground course above the door** — which this house has nowhere to put.
Battering only its top course would inset a wall 0.5 m above the openings and read as a mistake, not as 夯土. So the type keeps
its vertical wall and this record says why, instead of carrying the item for a sixth round.

**Identity: 36 scenes, 126 516 rows**; `bai_lane` 5 689 → **5 713** — two posts, a beam and a drum. The document is 20.39 MB.
## Round 97: the 照壁's framed panel

A white screen wall is not blank: it carries a **framed panel** where the household's motto is written, and the frame is what
makes the wall a page rather than a surface. Four members `PANEL_PROUD` proud of the plaster, `PANEL_BAR` square, a margin off
the wall's own edges — and **on the lane face**, because this screen stands *outside* the gate (a 外照壁), so the face it is
for is the street's.

The first attempt panelled the gate side, which is the reading a 内照壁 gets, and the plate showed a blank wall: the camera
stands in the lane. Turning it round is the whole round's small lesson — **which face a panel belongs on is decided by which side
of the gate the wall stands.**

**Identity: 36 scenes, 126 564 rows**; `yikeyin` 6 841 → **6 865** — four mouldings with their sections and their ends. The
document is 6.30 MB.
## Round 98: the 屯堡石板街, the gallery's sixth settlement

**A street and not a compound.** The 屯堡 village is a garrison's yard with a wall and a 朝门; a 屯堡 *street* is what a growing
one becomes — houses on both sides of a flagged lane, each turning its 院坝 onto it, and the lane itself the village's front
room.

- **six `tunpu::document()`s in two rows**, the north row turned a half so both rows face the lane. A 石板房's yard is at `+z`,
  so the south row takes no turn and the north row does — the same single fact the 白族 lane was arranged by;
- **the lane laid in eighteen flags** across its whole length: a 屯堡's street is the same stone as its houses;
- **a wall at each end with the lane's width left open**, so the street has two mouths and no third.

Two faults of my own, both caught by the frame rather than by the crate:

- **the street ran the wrong way.** The rows are at `±ROW` in `z`, so the lane between them runs **east-west** and its mouths are
  at `±x`; the first version laid the flags and the end walls along `z` and put the camera at a mouth that was not there. What
  the plate showed was a row of house **backs**.
- **and the camera then stood outside a mouth**, where the end wall is the whole foreground. It stands in the street now, at
  its west mouth, and the lane runs away to the far wall with the six yards along it.

**Identity: 37 scenes, 128 284 rows**; `slate_street` **1 720** — six houses at 260 each, eighteen flags and four wall lengths.
The document is 7.55 MB. A scene is not in the suite until three lists name it: `Gallery::names`, `build`'s match and
`naming::building_of` — the probe skipped this street until the third was added, and a scene the suite skips is a scene nobody
measures.
## Round 99: the street's channel

The lane was one flagged surface with joints across it. It has a **channel down its middle** now: each flag is three bands — two
walked strips at `FLAG`, and a gutter `CHANNEL` wide standing `GUTTER` below them — which is how a stone street drains without a
pipe, and why the middle of an old street is the part that is always wet.

**Identity: 37 scenes, 128 500 rows**; `slate_street` 1 720 → **1 936** — eighteen flags become fifty-four bands, and the
channel's own two edges are counted on every one of them. The document is 7.68 MB.

The culverts the channel would empty into are still not drawn: a slab drains to its lowest edge and this crate has no word for a
mouth in a wall.
## Round 100: a door leaf of its own, and the gallery at a hundred rounds

Round 90 measured that **a leaf is a panel and not a post** — a member whose two anchors differ in elevation is a rod leaning
against a pier, which is what the first 巷门 drew. That finding is `derive::leaf` now, and two documents hang their gates with
it: the 白族巷's gate, unchanged at **5 713 rows**, and the **屯堡村's 朝门**, whose opening had been drawn empty for thirty
rounds. A 朝门 closes — that is what makes a garrison's street closable — and a gate whose opening is drawn empty is a hole in a
wall.

The helper's own first refusal is worth the line it costs: `MemberDegenerate { length: 0.45 }`, because the section's `across`
was passed as the run's own direction instead of its perpendicular. Round 71 learned the same thing about a merlon.

### What a hundred rounds hold

| | |
| --- | --- |
| **types** | 15 — from the 竹楼 to the 穿斗式民居, and the 本主庙 as the first public building |
| **settlements** | 6 — 山村, 傣寨, 高原小村, 屯堡村, 白族巷, 屯堡石板街, plus the 风雨桥 |
| **courts and pieces** | the 一颗印 and its 照壁, the 石板房院 and its gate, the 三坊一照壁 and its 门楼, the 本主庙's hall and stage |
| **scenes / rows** | **37 / 128 512** |
| **invariants written down** | 12, from `Mass::wall` being under half the plan to the mitre that a door's inset must clear |

**Identity: 37 scenes, 128 512 rows**; `tunpu_village` 1 180 → **1 192** — two leaves, six rows each. The document is 7.52 MB.
## Round 101: the street's water

A 屯堡 street's houses have no wells of their own: the water is at **one place**, where the street has room for it. Two masses
stand in the lane's mid-ground now —

- **the well**, a mass whose wall is thick enough to be a ring — `0.24 m` in a `1.44 m` plan, the same answer the 白族 lane's
  well gave, and the inner loop cannot fold;
- **the trough** beside it: a well laid on its side, the same ring of stone `1.85 m` long and `0.62 m` deep, which is what a
  street's animals drink from.

The first position put both at the east mouth, `15 m` up the lane, where they were three pixels of grey. They stand at `7.4 m`
now, in the middle of the street's own view — **a detail placed where the plate cannot show it is a detail nobody sees**, which
this project has now learned three times (rounds 77, 97 and here).

**Identity: 37 scenes, 128 538 rows**; `slate_street` 1 936 → **1 962** — a curb's four faces, its top and its hole, and a
trough's. The document is 7.69 MB.
## Round 102: the 井台

A well's ring does not sit on the street: it sits on a **platform a hand's height up**, so that the water running down the
channel cannot run back into it. The street's water stands on one now — a floor slab `PLINTH` high carrying both the curb and
the trough, whose nodes are at that height and not at nothing.

**Identity: 37 scenes, 128 544 rows**; `slate_street` 1 962 → **1 968** — the apron's top and its four edges. The document is
7.69 MB.
## Round 103: the 朝门's frame

Round 100 hung the 朝门's leaves; a gate whose leaves hang on bare stone has nothing to hang on, and a gate you walk through
without stepping over anything is not a gate. The opening is **lined in timber** now: two jambs just inside the stone piers, a
head across them, and a **threshold** a hand's height off the paving — four members `FRAME` square, and the leaves lie against
them.

**Identity: 37 scenes, 128 568 rows**; `tunpu_village` 1 192 → **1 216** — four frame members with their sections and their
ends. The document is 7.53 MB.
## Round 104: the 抱鼓石

A gate frame standing on nothing but its own two posts has no foot. The 朝门 has two now — **抱鼓石**, one at each jamb's foot
inside the opening: `DRUM` square, knee high, the stone a leaf swings back against and the stone that holds a jamb upright.

They stand **inside the opening** and against the jamb's inner face, so neither overlaps a stone pier: a drum that reached past
the jamb would be a mass inside a mass.

**Identity: 37 scenes, 128 594 rows**; `tunpu_village` 1 216 → **1 242** — two drums' four faces and their tops. The document is
7.54 MB.
## Round 105: the 本主庙's court furniture

A temple's court is arranged **around** something, and what it is arranged around is the 香炉. The court has one now — four stone
walls on a `1.25 m` plan, a metre high — and two **旗杆石** flanking the 大殿's steps, where the flags stand on festival days. A
court with two buildings and nothing between them is a yard with a stage in it.

The 香炉's first version was four **posts**: its members' two anchors differed in elevation, so each wall ran *up* instead of
*along*, and a box became four sticks at a box's corners. That is the third time this project has made that particular mistake
(rounds 90, 100 and here), and the module now says it in the code: **both anchors at one elevation, the section's height is the
wall's.**

**Identity: 37 scenes, 128 644 rows**; `benzu_miao` 1 112 → **1 162** — four burner walls and two flagstones' faces and tops. The
document is 7.41 MB.
## Round 106: the 香炉 corrected, and its step and lid

Round 105 said the burner's walls were panels. **They were not**: the formatted call had kept the second anchor at
`BURNER_FOOT + BURNER_HIGH`, so the box was still four posts standing at a box's corners, and the round's own note claimed a fix
that had landed only in the shorter of two copies of the line. Corrected here — both anchors at one elevation — and the `across`
direction with it: it was passed as the run's own direction, which the crate answers as `MemberDegenerate { length: 1.25 }`, the
same lesson the door leaf learned in round 100.

A burner is **two slabs and four walls**, and it has both now: a floor slab `BURNER_FOOT` high that it stands on, the four walls
between, and a roof slab `LID` thick over the top. A box with a lid is a burner; a box without one is a planter.

**Identity: 37 scenes, 128 656 rows**; `benzu_miao` 1 162 → **1 174** — the step's top and four edges, the lid's two faces and
four edges. The document is 7.42 MB.
## Round 107: the step up to the 井台

A platform a hand's height up with no step is a kerb you climb. The 井台 has **two shallow steps** in front of it now — each a
slab `STEP` deep at a third and two thirds of the platform's height — and the platform itself is raised from `0.15 m` to
`0.30 m`, which is what a well platform is: not a kerb but a step up out of the street's water.

At `0.15 m` the two steps were sub-pixel from the plate's own camera, and a detail that cannot be seen is a detail that cannot be
checked — the same rule as rounds 77, 97 and 101, applied before landing this time rather than after.

**Identity: 37 scenes, 128 668 rows**; `slate_street` 1 968 → **1 980** — two steps' tops and edges. The document is 7.69 MB.
## Round 108: the 石敢当 at the lane's mouth

Where a lane meets a wider way a 白族 village sets a stone at the corner — **石敢当**, "the stone that dares" — and it is the one
object in the street that is written and stands alone. It stands at the south mouth now, in the plate's own foreground: a mass
`STELE_HALF` by `STELE_DEEP`, a metre and a bit high, set into its own foot slab.

A stele is a **mass** and not a post, and that is the whole difference between this round and the three that drew sticks: two
faces, two edges, a foot and a top, all of them a box's.

**Identity: 37 scenes, 128 687 rows**; `bai_lane` 5 713 → **5 732** — the stele's four faces and its top, and its foot slab's
top and edges. The document is 20.40 MB.
## Round 109: the 本主庙's 山门

The module has said since round 79 that the **山门** "stands where the camera is". It stands there now: a wall across the court's
south end, `12.4 m` wide and `0.6 m` thick with a `SHANMEN_WALL` in it, its door cut in three bays with an inset a wall's
thickness clear of the ends, and a lean-to over it that sheds into the court — which is what makes a wall a gate house.

The temple reads as a compound for the first time: **山门 → 戏台 → court → 大殿**, which is the order a 本主庙 is walked, and the
plate is backed off to take the gate in — a plate of a temple that leaves its own entrance out of the frame is a plate of a
courtyard.

**Identity: 37 scenes, 129 044 rows**; `benzu_miao` 1 174 → **1 531** — the wall's faces and top, four reveals round its door, and
the gate's roof. The document is 7.52 MB.
## Round 110: the 山门's leaves and threshold

Round 109 closed the temple with a gate wall whose opening was drawn empty, and an empty opening in a gate wall is a hole. The
gate has its **two leaves** and its **threshold** now: the plates stand open against the jambs, swung out into the lane so the
plate can see them, and a foot steps over a `0.26 m` stone between them.

The leaves are `derive::leaf` — round 100's helper, which is now the third document to hang its gate with it (the 白族巷, the
屯堡's 朝门, and this), and the reason the helper exists: **both anchors at one elevation, the section's height the opening's**,
so a leaf is a panel and not a post.

**Identity: 37 scenes, 129 062 rows**; `benzu_miao` 1 531 → **1 549** — two leaves and the threshold. The document is 7.53 MB.
## Round 111: the 街沿石

A street laid in flags and stopping at the yards' own paving has **no edge** — it bleeds into them. Two strips of stone run the
lane's whole length now, a hand's height proud of the flags: `KERB` wide, `KERB_HIGH` high, and they are the line the street is
read by — the flags between them, the channel down the middle, the yards beyond.

**Identity: 37 scenes, 129 074 rows**; `slate_street` 1 980 → **1 992** — two kerbs' tops and their four edges each. The document
is 7.69 MB.
## Round 112: the 傣族缅寺, the gallery's sixteenth type

**The second public building, and the first temple of the south-west's wet tropics.** Where a 白族本主庙 is axial and walled, a
缅寺 is **one great roof**: the hall is a single volume, the roof takes two thirds of its height, and the walls under it are a
screen and not a structure.

- **the roof is the steepest in the gallery** — `RISE` 4.6 m on a 4.1 m wall — because the rain of the tropics is the building's
  first problem and a steep pitch is a fast drain;
- **the hall stands on a platform** a metre up, with three treads at the porch's middle, because every approach to a 缅寺 is up;
- **the porch is four slender posts and a beam**: the front of the hall is open and the roof comes down over it — `2.3 m` of
  eaves past the posts.

**The proportion is a structural fact and not a taste.** The hall is **wider than it is deep**, so the ridge runs along the long
pair of edges and the gable ends stand at `±x`; the face the village enters by — the one at `+z` — is then a long side. Built the
other way round, the porch stood against a gable and the crate answered `GridOnACrownedFace { edge: 3 }`: a crowned gable cannot
take an opening grid, because a grid is cut into a plane and a crown has already bent it.

**Identity: 38 scenes, 130 566 rows**; `dai_temple` **1 492** — a platform, a hall, its crowned roof, four posts, a beam, three
openings' reveals and two treads. The document is 7.51 MB.
## Round 113: the 缅寺's compound

A 缅寺 is not a hall standing in a field — it is a **wat**, and a wat is an **enclosure**: a wall a metre and a half high, a gate
in its front, and the hall on its platform in the middle of the yard. The temple has one now: five wall lengths round a paved
yard, and the platform standing `2.40 m` inside it.

**The sides stop at the front and back pieces' inner faces**, so no two lengths share a corner. Two lengths that meet at a corner
are two masses both owning the same square metre, which `validate` names `UnattachedOverlap` — round 72 measured it on the
石板房院's walls and this round measured it again, in the same shape, four pairs at once.

**Identity: 38 scenes, 130 637 rows**; `dai_temple` 1 492 → **1 563** — five wall lengths' faces and tops, and a yard's paving.
The document is 7.53 MB.
## Round 114: the 缅寺's 寺门

The compound's wall left a gap and the gap was empty. It has a gate now, and it is **round 89's reading**: a **lintel is the
host** and the two piers are `Under` it — a mass whose top meets the host's foot and whose plan stands inside the host's plan is a
mass that *carries* the host, so the thing over the gap names the things under it. That is the only relation this crate has for a
gateway, and it is why a gate is a lintel with piers beneath rather than two piers with a beam between them.

**The gate porch stands `0.62 m` in front of the wall and not in its gap.** A lintel wide enough to contain two piers reaches
past the gap's own edges, and the wall lengths either side of that gap own the square metre it reaches into: `UnattachedOverlap
{ mass: 4, other: 6 }` and `{ 5, 6 }`, measured on the first attempt. Set out in front, the same gate overlaps nothing.

**Identity: 38 scenes, 130 847 rows**; `dai_temple` 1 563 → **1 773** — a lintel's faces and top, two piers' and their joins, and
the gate's own roof. The document is 7.60 MB.
## Round 115: the 缅寺's 戒堂

A wat is not one hall. The **戒堂** is where the monks are ordained: it stands beside the 大殿 and clear of its platform, it is
smaller, its platform is lower, and it is **closed** — the rite it holds is not a public one. Two roofs of different size on one
yard, one open and one shut, is what an ordination hall is for.

**Where a second building goes is decided by the plate, not by the map.** Built behind the 大殿 it was invisible: the hall's own
roof is 4.6 m of pitch across the whole axis. Set to its **east** it stands beside it, and the compound is widened from `2.4 m` to
`5.6 m` because a wat's yard is not a corridor — and because a yard that does not hold both buildings is a yard with one building
in it.

**Identity: 38 scenes, 131 366 rows**; `dai_temple` 1 773 → **2 292** — a second platform, a second hall with its crowned gable,
and its own roof and covering. The document is 7.6 MB.
## Round 116: the 塔

This crate has no **solid of revolution**, and a 塔 is round — but a Dai stupa is **built in square courses that batter inward**, and
that is a stack of masses the crate already has a word for. `derive::stepped_courses` lays exactly that, each course carried by the
one below through `Join::Storey` and sharing an edge with it and no volume; on its top stand the **bell** and the **spire**, each
carried by the one before. Four courses, three bands, `7.4 m` to the spire's point: a base, a bell and a needle, which is what a 塔
is.

**A square stupa is not a compromise**: it is the form a brick 塔 of the upper Mekong takes, and the crate's own batter is the
reason it can be drawn at all.

Two refusals worth the record, both from the bell's first version:

- `OffsetEdgeFolded` — a wall wider than half the band's plan folds its inner loop;
- `MassThickness { wall: -0.0102 }` — a plan computed to shrink by a fraction of the base went **through** zero and came out
  negative. The bands now have their own half-widths, and their wall is a fraction of that width and never a number of its own.

And one chain: naming the **bottom course** as host for all three bands left every pair unrelated — `UnattachedOverlap { 14, 15 }`,
`{ 14, 16 }`, `{ 15, 16 }` and three `StoreyNotOnTop`, because a `Storey` mass must stand with its base at its host's top. Each band
names the one under it.

**Where it stands was decided by the plate again**: built in the yard's far corner it was behind the hall's roof and invisible. It
stands in the near corner now, where a wat's 塔 actually is — the first thing you see at the gate.

**Identity: 38 scenes, 131 463 rows**; `dai_temple` 2 292 → **2 389** — four courses' faces, three bands' and the platform's. The
document is 7.79 MB.
## Round 117: the hamlet's wat

A Dai hamlet is not a row of houses: it is a row of houses **and a temple**, and the temple stands **behind** the row so that its
roof rises over theirs — which is what a Dai village looks like from its lane. The whole `dai_temple::document()` is translated
into place and merged, nothing re-authored, the same answer the hamlet gave for its three 竹楼 in round 40.

Three buildings, a well and a wat on one plate: the 竹楼s' pyramidal roofs in front, and behind them the wat's long steep roof,
the 戒堂's smaller one, and the **塔** standing between the houses' roofs as a village's landmark should.

The shadow extent goes from `40 m` to `64 m`: the wat adds twenty metres of depth behind the row, and a shadow map that stops at the
row draws a temple with no shadow.

**Identity: 38 scenes, 133 857 rows**; `dai_hamlet` 2 305 → **4 699** — the wat's two buildings, its compound, its gate, its 塔 and
its yard. The document is 46.0 MB.
## Round 118: the wat's trees

A court with two buildings and no tree is a yard. The wat has three now — a bodhi inside the gate's sight, and two along the wall —
carried by the **vegetation bridge** the hamlet already uses, merged rather than re-authored.

**The identity table does not move, and that is the measurement**: 38 scenes / 133 857 rows before and after, `dai_temple` still
**2 389**. A tree is not a building element and the bridge's meshes carry no identity rows, so a scene that gains vegetation gains
no ids — the same answer the hamlet's own palms gave. What moves is the **document**: 7.79 MB → **31.4 MB**, because three trees
are three crowns of leaf geometry, and the channel that names masonry does not name a leaf.

**Identity: 38 scenes, 133 857 rows** (unchanged); `dai_temple` **2 389** (unchanged); the document 7.79 → **31.41 MB**.
## Round 119: the spire's lotus and needle

The 塔's spire was **one column** — a chimney on a bell. It is six bands now: the bell, its shoulder, the **lotus seat** that pinches
the bell in, the spire, and two bands of needle above it. `(0.62, 0.95), (0.46, 0.55), (0.44, 0.18), (0.30, 0.85), (0.20, 0.55),
(0.11, 0.62)` — each carried by the one under it, and the pinch is the whole difference between a stupa and a stack.

**Identity: 38 scenes, 133 935 rows**; `dai_temple` 2 389 → **2 428** — three more bands' faces and tops. The document is 31.4 MB.
## Round 120: the wall's 压顶

The compound wall stopped in a **bare plane** while the gate above its gap was tiled: a compound whose gate is roofed and whose wall
is not is a gate standing in a field. Five caps now run along the five lengths — a roof slab `COPE` thick and a hand wider than the
wall on either side, the same answer the 石板街's kerb gave at the other scale.

**Identity: 38 scenes, 133 995 rows**; `dai_temple` 2 428 → **2 458** — five caps' tops and four edges each. The document is
31.4 MB.
## Round 121: the hall's inside, and a plate that cannot show it

A wat's hall is not an empty shell: the image stands on a **佛坛** at its far end, and the roof is carried by a rank of **nave posts**
the room is walked between. Both are there now — a dais `4.60 × 1.70 m` a step high at WALL + 0.20 off the back wall, and three pairs
of `0.22 m` posts down the spine.

**The dais is a floor slab and not a mass**, because a mass inside the hall's own plan is a mass that must name a host it does not
have — a plinth is what a dais is, and this crate draws a plinth as a slab.

**And the inside cannot be plated.** A camera inside the hall drew a **black frame** (measured, then measured again from the porch:
a dark doorway with one post against it), because this catalogue has **no interior light** — one directional sun and a roof over the
nave is a room in shadow, and the scene registered for it was withdrawn rather than shipped as a black picture. What the round
leaves is the geometry and a recorded limitation: **an interior needs a light this catalogue does not have**, the same class of
finding as the 腰檐's warp in round 93.

**Identity: 38 scenes, 134 079 rows**; `dai_temple` 2 458 → **2 500** — the dais's top and four edges, and twelve nave posts with
their sections. The document is 31.4 MB.
## Round 122: the 经幡柱

Every Dai wat has them: **two tall poles either side of the gate**, each with a cross-arm, and a banner hanging from each arm. Six
members — a post, an arm and a cloth, twice — and they are the first thing that tells a traveller this is a temple and not a house.

**Members and not masses**, because a pole has no inside: the crate's `Mass` is a ring with a wall and a pole is not, and the arm and
the cloth are the sections a member already carries.

**Identity: 38 scenes, 134 151 rows**; `dai_temple` 2 500 → **2 536** — six members with their sections and their ends. The document
is 31.5 MB.
## Round 123: the 放生池

A wat has a basin where live things are set free, and it is a **mass whose wall is thick enough to be a rim** — `0.32 m` in a
`3.4 × 2.4 m` plan, the same answer the two lanes' wells gave. Its top is the water's level a hand above the paving, which is the only
way this crate can say *water* at all.

**It stands outside the gate and not inside the wall.** Built in the yard it was invisible: a `1.55 m` wall hides everything low
behind it from a camera ten metres up, and a basin nobody sees is a basin nobody dug. Beside the gate it is the first thing the
village walks past, which is also where a wat's release pond belongs.

**Identity: 38 scenes, 134 177 rows**; `dai_temple` 2 536 → **2 549** — a rim's four faces, its top and its inner loop. The document
is 31.5 MB.
## Round 124: the 界石

An ordination hall is not just a building: it stands inside a **consecrated precinct**, and the precinct is marked by a ring of
stones set in the ground. Six **界石** stand round the 戒堂 — along its south and east sides, where the plate sees them, and not on
the west, which is the 大殿's platform: a stone set inside another building's plan is a stone on its floor.

The stones are `SIMA` square and knee high, a mass each, because a boundary stone has a foot and a top and is set *into* the ground
rather than resting on it.

**Identity: 38 scenes, 134 333 rows**; `dai_temple` 2 549 → **2 627** — six stones' four faces and their tops. The document is
31.5 MB.
## Round 125: the 街门 at the east mouth

A street with two mouths and no gate is a street a garrison cannot close — and this is a 屯堡. The wall's gap was the one thing the
street was missing, and it has a gate now: a lintel over two piers, `GATE_HEAD` to the lintel's underside, at the east mouth the
camera looks down the lane to.

**Both of the crate's lessons about gateways are in it**, and it is the third gate built from them:

- the **lintel is the host** and the piers are `Under` it (round 89) — a mass whose top meets the host's foot and whose plan stands
  inside the host's plan is a mass that carries the host;
- the gate stands **`GATE_OUT` in front of the wall's face** (round 114) — a lintel wide enough to contain two piers reaches past the
  gap's own edges, and the wall either side owns the square metre it reaches into.

**Identity: 38 scenes, 134 372 rows**; `slate_street` 1 992 → **2 031** — a lintel's faces and top, two piers' and their joins. The
document is 7.71 MB.
## Round 126: the 街门's leaves

A gate that cannot close is a doorway, and a 屯堡's street gate is the thing the garrison closed. Two **leaves** hang from the
lintel's underside to a hand's width above the flags, swung open into the street, and a **threshold** lies between them.

It is the **fourth gate** built from `derive::leaf` — after the 白族巷's, the 屯堡村's 朝门 and the 本主庙's 山门 — and the reason that
helper exists has not changed: **both anchors at one elevation, the section's height the opening's**, or the leaf is a rod.

**Identity: 38 scenes, 134 390 rows**; `slate_street` 2 031 → **2 049** — two leaves and the threshold. The document is 7.72 MB.
## Round 127: a 街门 at each mouth, from one implementation

Round 126's gate stood at the east mouth only, which is a street that closes at one end and not the other. There is a gate at **both**
now, and it cost **one function**: `fn gate(skeleton, gate_x, inward)` in the module, called twice with the two mouths and the two
swing directions. A second copy of sixty lines would have been the easy answer and the wrong one — the crate's own rule is
*unification against variants*, and the two gates differ in a sign.

The function's doc carries both invariants it depends on, so the next gate does not have to rediscover them: **the lintel is the host
and the piers are `Under` it** (round 89), and **the gate stands in front of the wall's face** (round 114).

**Identity: 38 scenes, 134 447 rows**; `slate_street` 2 049 → **2 106** — the second gate's lintel, its two piers, two leaves and its
threshold. The document is 7.74 MB.
## Round 128: the 竹楼's rail

A floor two metres up with no rail is a platform you fall off, and the **晒台** — the drying deck — is where a Dai family sits. The
type has shown three of them bare in every plate since round 40; they have rails now, on the veranda's front and the drying deck's
three open sides: a post every `RAIL_BAY`, a top rail at `RAIL_HIGH` above the floor and a mid rail, which is the shape a 竹楼's own
rail takes.

**The rail is a closure over an edge, called three times**: one run's posts are laid at the edge's own spacing and its two rails are
one member each, so a rail and a rail's posts cannot disagree about a length.

**Identity: 38 scenes, 134 927 rows**; `bamboo_house` 764 → **884** and `dai_hamlet` 4 699 → **5 297**, because the hamlet is three of
them. A change to a type is a change to every scene that holds it, which is why it owes a measurement in each. The document is 4.46 MB.
## Round 129: the stair's handrails

A steep run of ten treads with nothing to hold is a stair a household climbs carefully; with a rail it is a stair it climbs carrying
a basket, which is what a 竹楼's stair is for. Two rails now follow the flight's own slope, a hand's height above the treads, each
with a **newel** at the foot and one at the head — and they take the same `RAIL_HIGH` and `RAIL_POST` the deck rail does, so the
stair and the deck it climbs to are the same hand's work.

**Identity: 38 scenes, 135 071 rows**; `bamboo_house` 884 → **920** and `dai_hamlet` 5 297 → **5 405**. The document is 4.47 MB.
## Round 130: the 石栏 round the 塔

A stupa's platform is a **sacred square**, and a square a village walks round is bounded: a low rail of stone posts and two rails on
the platform's own edge. Twenty-four posts and eight rails — four edges, bays of `STUPA_RAIL_BAY` — at a third of the 塔's height,
so the fence reads as a boundary and not as a wall.

It is the **third rail** in three rounds and the same shape each time: round 128 gave the 竹楼's deck one, round 129 its stair, and
this one a 塔's platform. **A rail is an edge that has been given a rail**, and the difference between the three is the material's
section and how high a hand grips it.

**Identity: 38 scenes, 135 431 rows**; `dai_temple` 2 627 → **2 807** — twenty-four posts and eight rails, with their sections and
their ends. The document is 31.6 MB.
## Round 131: the 台口栏杆, and the court's own plate

A stage a metre up with an open mouth is a stage a child walks off the front of. The 戏台 has its rail now: across the mouth, on the
stage's own floor, a post every `STAGE_RAIL_BAYS`-th of its width and two rails — **the fourth edge given a rail in four rounds**, after
the 竹楼's deck, its stair and the 塔's platform, and here in timber where the audience sees it.

**And the compound plate cannot show it.** The mouth faces the 大殿 and the compound camera stands behind the stage — so the temple
gets a **second scene**, `benzu_miao_court`, taken from the 大殿's step where the audience sits. This is not the round-121 dead end: a
court has daylight on it, so the plate is legible, and the rail reads against the shaded mouth exactly as it does on site.

**Identity: 39 scenes, 137 088 rows**; `benzu_miao` 1 603 → **1 657** — eight members' sections and ends — and the new
`benzu_miao_court` **1 603**, the same document under a second camera. The document is 7.55 MB.
## Round 132: the 白族巷's 井台

Round 130 gave the street's well a platform; this is the same answer in the same vocabulary — a floor slab `WELL_PLINTH` up, **a step
in front of it** on the side the lane comes from, and the curb and the windlass standing on its top. Two wells in two scenes and **one
way of standing a well on the ground**: what a plate reads as the same thing is the same thing, and the two scenes now agree about it.

**Identity: 39 scenes, 137 100 rows**; `bai_lane` 5 732 → **5 744** — the platform's top and edges and the step's. The document is
20.4 MB.
## Round 133: the 庑

A temple's court is flanked by **covered galleries**, not by a fence. The two side walls were the last of the yard: they are the
galleries' **back walls** now, with `GALLERY` posts standing in front of each, a beam across their heads, and a **lean-to shedding
from the wall's head into the court** — six posts and a beam a side, and their own roofs.

Without them a court is a yard between two buildings; with them it is a 本主庙, which is what the type is arranged as.

The beam's section cost the round's first refusal: `MemberDegenerate { length: 7.9 }`, because its `across` was passed as the run —
the lesson rounds 100, 106 and 128 have each paid for, now written beside the line that pays for it a fourth time.

**Identity: 39 scenes, 137 512 rows**; `benzu_miao` 1 657 → **1 809** — twelve posts, two beams and two gallery roofs — and
`benzu_miao_court` with it, the same document under its second camera. The document is 7.61 MB.
## Round 134: the 坐凳栏杆 under the 庑

A 庑 is where the village sits to watch the 戏台, and a gallery with no bench is a corridor. Each gallery has a **bench rail** now:
a plank `BENCH_WIDE` across at `BENCH_HIGH`, carried on a leg at every post — so the rail's top is the seat, which is what a
**坐凳栏杆** is.

**And the round paid a fifth time for one mistake.** `MemberDegenerate { length: 7.9 }` on the planks, because a member's `across` was
written as its own run — after round 100's door leaf, round 106's burner, round 128's deck rail and round 133's gallery beam. The rule
is in the code as a rule now, not as an anecdote: **a member's across is the run turned a quarter**, and never the run.

**Identity: 39 scenes, 137 680 rows**; `benzu_miao` 1 809 → **1 893** — twelve legs and two planks, and `benzu_miao_court` with it.
The document is 7.64 MB.
## Round 135: the 匾额 and the 楹联

A 大殿's colonnade is not bare: over the middle door hangs the **plaque** that names the god, and on each flanking bay a pair of
**couplet boards**. Three members `BOARD_PROUD` off the front face — the one the compound plate looks at, which is why they are worth
drawing at all.

**The court's plate cannot show them**, and that is the reverse of round 131: the court camera stands on the 大殿's step looking at the
戏台, so the hall's own front is behind it. Two plates of one temple, each with the face the other cannot see — which is what the pair
was for.

**Identity: 39 scenes, 137 716 rows**; `benzu_miao` 1 893 → **1 911** — a plaque and two couplet boards with their sections and ends.
The document is 7.65 MB.
## Round 136: the 门簪, and the board turned to the lane

Four **门簪** now pin the 题额 to the lintel — the gate's one purely ornamental joint, two either side of its middle, a hand apart. They
cost nothing to draw and they are what a 白族 gate is recognised by.

**But the first attempt put them, and the board they pin, on the face the plate cannot see.** Round 90 hung the 题额 on the gate's
`+z` side; this scene's camera stands at the lane's north end and looks south, so the lane's own face is the `-z` one. Both moved —
**round 97's lesson read a second time**: *which face an object belongs on is decided by which side the view is.*

**Identity: 39 scenes, 137 740 rows**; `bai_lane` 5 744 → **5 768** — four studs with their sections and ends. The document is
20.41 MB.
## Round 137: the 白族巷's 抱鼓石, and what the validator actually compares

Round 104 gave the 屯堡 朝门 a pair of **抱鼓石**; the 白族巷's gate has the same two stones now, for the same two reasons — a leaf swings
back against one, and the jamb is held upright by one.

**They could not stand on the gate's own line**, and the reason is worth the record: `validate` compares **plans and not volumes**, so a
stone anywhere under the lintel's outline is a stone *inside* the lintel — `UnattachedOverlap { mass: 22, other: 25 }` and `{ 22, 26 }`,
measured twice while they stood at the gate's centre, once at the pier's face and once pulled in. Set `DRUM_Z` in front of the gate the
same pair overlaps nothing, and that is also where the plate sees them.

**Identity: 39 scenes, 137 766 rows**; `bai_lane` 5 768 → **5 794** — two drums' four faces and their tops. The document is 20.42 MB.
## Round 138: the 上下场门, and a fascia that did not exist

The round opened with a **封边** — a board closing the 竹楼's deck edge — and it was **withdrawn after the plate came back with the same
hash it had before the change** (`c684389e…`, byte for byte). The deck's edge already reads at the plate's distance; a fourth member
along it is a member nobody sees, and this project's own rule says a detail placed where the plate cannot show it is a detail nobody
sees. **A change that does not move the frame and does not move the table is not a change**, and the honest thing to do with it is to
take it out.

What landed instead crosses a stage that has never had a back door. An actor enters by one and leaves by the other — **出将** and
**入相** — and they are cut where the compound plate looks: `edge 0`, the stage's far wall, in the outer two of three bays, each as
tall as a man. A 戏台 with no back doors is a stage nobody can walk onto.

**Identity: 39 scenes, 137 778 rows**; `benzu_miao` 1 911 → **1 917** — two openings' reveals. The document is 7.65 MB.

**What the plate shows and does not**: the doors sit on the stage's shaded side, so they read as two dark patches on a dark wall — the
frame is honest about them but not generous. They are measured in the table, which is the part this round can claim.
## Round 139: the 侗寨, the gallery's seventh settlement

**The first settlement built around a single vertical.** Every other one here is a row: the 傣寨 a line of houses on a well, the
屯堡石板街 two rows facing each other, the 白族巷 three courts on a lane. A 侗寨 is not a row — it is a **square with a tower on it**,
and the houses stand round the square.

Three reused documents and two new slabs:

- **`drum_tower::document()` at the origin** — seven storeys of skirted eaves, the settlement's one purpose-built object;
- **three `chuandou::document()`s round it**, each turned so its porch faces the tower: a Dong house **is** a 穿斗 frame, so the
  gallery already had the type and this scene is a composition and not a new building;
- **the 鼓楼坪**, paved as one slab, because the space round a drum tower is a floor where the village meets and sings, and not a road;
- **the lane**, laid in flags and running past the square's south side — the same joint vocabulary the 石板街 uses, at the other end
  of the province.

**The camera was too close on the first plate**: the houses cropped at the edges and the tower filled the frame. A tower in the middle
of a village is a picture of **the village**, not of the tower, and the second camera is backed off to take the square in.

**Identity: 40 scenes, 145 516 rows**; `dong_village` **7 738** — a tower, three houses, a square and twenty flags. The document is
9.70 MB.

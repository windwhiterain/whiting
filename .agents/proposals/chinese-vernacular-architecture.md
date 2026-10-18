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
| 举架 / 举折 roof profile | one slope per plane | a profile of *k* steps on `RoofPlane` |
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

To come, after the numbers.

## The forks, for the owner

To come.

## The road

To come.

## Not established

- Whether the crate's planarity tolerance holds a *crowned* mass's top strip at a corner. The
  geometry says a sloped top strip over a straight run is planar (four points on one tilted plane);
  the miter at a corner, where two crowned edges meet at different slopes, has not been computed.
- Whether a 举架 profile is the same for every plane of a hip roof. A gable's two slopes have the
  same 步架 count and the same profile; a hip's short ends have fewer, and the design must say
  whether the profile is per plane or per roof.

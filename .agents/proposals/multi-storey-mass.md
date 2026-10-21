# 多层体量 — a mass with levels, so storeys are one building and not a stack of relations

## The verdict this starts from

A multi-storey building, and an upper floor that **projects** past the one below it, belong to the building's own
primitive. They are not a relation between masses. The third relation that was proposed for a cantilevered storey
was the wrong shape, and the owner said so in one sentence: *"isn't this what the multi-storey building's
primitive should model? Why add a stacked relation?"*

## What exists today

`Mass` is **one outline and one top elevation**: a shell whose wall is vertical from the outline up to `top`, with
`crowns` able to raise individual nodes (which is how the 土掌房's gable walls rise into their roof). A building of
several storeys is therefore drawn as **several masses**, and the 鼓楼 is seven of them chained by `Join::Storey` —
a relation that exists only because the primitive cannot say "one building, seven floors".

## The primitive

```rust
pub struct Mass {
    /// The shell's horizontal sections, bottom first, at least two: the first is the foot, the last the top.
    pub levels: Vec<Level>,
    …
}

pub struct Level {
    /// The elevation this section is at.
    pub elevation: f32,
    /// The outline **at that elevation**: the outer face of the wall there.
    pub outline: Vec<PlanNode>,
}
```

The wall between two consecutive levels is the band between their outlines. Three things follow, and each one is
what an existing feature had to fake:

| the two outlines | the wall | what it gives |
| --- | --- | --- |
| the same | a vertical band | today's mass, exactly |
| the upper **inside** the lower | a band leaning in | **收分** — the battered earth wall the 土掌房 has been missing since round eight |
| the upper **outside** the lower | a band leaning out, plus a **soffit** where the floor projects | **挑出** — 骑楼's upper floor, 吊脚楼's gallery, the bridge's pavilion floor |

The storeys' floors are plates at the interior levels, and they hang off the mass the way its own plate does now.
Nothing about `Roof` changes: a roof still names the mass it sits on and its eave is still a plan loop.

## Invariants

- **the levels ascend strictly**: `levels[i].elevation < levels[i+1].elevation`;
- **every level's outline is closed, simple and positively wound**, and its `plan_area` is positive — the same
  three statements the one outline makes today, made once per level;
- **consecutive levels differ** — an outline equal to the one below is not an error but is also not a level, and
  the constructor folds it away so a document cannot carry a zero-height band;
- **the shell stays closed**: a projecting level adds a soffit face between the two outlines' shared boundary, and
  a receding level adds a horizontal face on top of the one below. Both are the wall's own, and
  `SolidOpen`/`SolidNonManifold` are what catch a mistake, unchanged;
- **no relation is added.** Two masses never overlap because a building is one mass with levels; `Join::Storey`
  stays for the case it really is — two *buildings* on one site, one standing on the other, which is a site
  question and not a storey question.

## What it costs

- **the wall expander** goes from one band per edge to one band per edge per level pair, plus the faces at the
  levels. That is the whole of the change; the plates, the openings, the grids and the roof derivation all keep
  reading "a mass's face" and "a mass's elevation", now with `levels.len()` of each;
- **openings name a face and a row**, and a row is a band: the rows of a face become the intervals between
  consecutive levels, so an opening's `row` already means what it should;
- **migration is an identity.** A mass built as it is today is *one* level pair — foot and top, same outline — and
  the expansion of a single vertical band is exactly what it is now. **All twenty-three scenes should be
  unchanged to the row** until one of them authors a second level, which is the test the change has to pass.

## What it unlocks, in the order it would be worth doing

1. **鼓楼 as one mass of seven levels**, with `Join::Storey` then used for nothing at all in that scene;
2. **骑楼 street: the upper floor projects** over the arcade — the type's definition, and the one thing the street
   still cannot say;
3. **吊脚楼: the gallery's floor projects** over the bank it hangs on;
4. **土掌房: 收分**, the battered wall, from a level whose outline is inset a few centimetres per metre;
5. **风雨桥: the pavilion's floor projects** over the deck, instead of its eaves doing all the work.

## What this note does not decide

- whether a level carries its own `wall` thickness or whether the wall is one thickness for the shell — a
  battered wall's *thickness* is usually constant and its *outline* moves, which suggests the outline is the whole
  of the answer and `Mass::wall` stays single;
- whether the plate at an interior level is a `Slab` the scene authors (today's way) or something the mass carries
  — the first is enough and is what the drum tower already does;
- the cost of the change, which is a round of its own and cannot be measured from a note.
## Landed: `Mass` is a stack of sections

The migration is in, whole, on one branch — and **it changes nothing that is drawn**, which was the test it had to
pass.

| before | after |
| --- | --- |
| `Mass { outline, top, crowns }` | `Mass { levels: Vec<Level> }`; a level is a loop whose every node carries its own elevation |
| `mass.outline` | `mass.outline()` / `mass.outline_mut()`; `mass.foot()`, `mass.crown()` |
| `mass.top` | `mass.top_at(node)`, `mass.highest_top()`, `mass.lowest_top()`, `mass.crown_top()` |
| `mass.crowns` | `mass.crown().outline[i].elevation`, `mass.crown_elevations()`, `mass.top_is_level()` |

**Two faults became unrepresentable.** A crown can no longer disagree with the outline — the top section carries one
elevation per node by construction — so `CrownCountMismatch` is no longer reported (the variant is left in the enum
for a caller that still names it). And a seat can no longer move half a building: `site::seat` shifts **every**
section with one `shift(delta)`, where it used to write the outline, the top and the crown vector separately.

**Measured.** The migration touched **30 files** across `building` and `gallery`, and the number that matters is not
that count: **all 23 scenes have exactly the same row count as before — 85 167 — and the drum tower's frame is
byte-identical** (`sha256 8d49906…`, the same file the seven-open-storey round produced). The audit's closure grew
by `whiting-document` and is green in 3.89 s.

**What is refused by name.** The multi-section wall is *authored* and not yet *expanded*: a mass with more than two
sections answers [`Problem::SectionsNotExpanded`] rather than being drawn as one band from its foot to its top with
its middle sections dropped. Building those bands — the battered wall and the projecting floor, with the soffit a
projection needs — is the next round's work, and this note is what makes it the next round rather than a surprise.
## Landed: the bands, and the first battered wall

The expander builds a **strip of wall per pair of sections**, so a mass's sections are geometry and no longer a
promise. The three cases the note named are one case now — the same four points and the same quad:

| the two sections | the strip |
| --- | --- |
| equal plans | vertical wall, which is every mass the gallery had |
| the upper **inside** | leaning in — **收分**, the battered earth wall |
| the upper **outside** | leaning out — 挑出, a projecting floor |
| **equal elevations, different plans** | horizontal, which is the soffit a projection stands on |

Four things the walk had to get right, each measured on the way:

- **the strips share their loops**, so the only faces a band cannot supply are the foot's annulus and the crown's
  wall top — and there is nothing between two bands to close;
- **points are shared, not recomputed.** `Solid::point` pushes and never welds, so a band that made its own copy of a
  section left the ring open: `SolidOpen { loops: 8 }` and `SolidNonManifold` on the very first pier;
- **the band is in the element.** `OuterWall`, `InnerWall` and `WallTop` carry a `band`, because two leaning strips
  in one planarity group are two non-coplanar faces in one group — which the validator would have reported as a
  real fault. `band: 0` reads exactly as it always did, so every recorded sentence still holds;
- **a banded mass carries no openings and no grids**, and says so by name (`BandedMassCut`): both are placed against
  one strip between a ground line and a top, and a hole put on the wrong strip is worse than a refusal.

**The first battered wall in the gallery is the 风雨桥's two abutments** — each a mass of three sections, each inset
`0.28` from the one below. Identity: `rain_bridge` 1 602 → **1 632 rows**, and **the other twenty-two scenes are
unchanged to the row**.

### What the piers taught, and why they are still posts

The masonry piers this round wanted to draw were refused, and the reason is a real limitation of another check:
`validate`'s `UnattachedOverlap` compares **two plans and carries no elevation**, so a masonry pier under a timber
gallery is an overlap however far apart in height the two stand. Two battered piers were built and reported
(`MassThickness` on a zero wall first, then the overlap), and the bridge's piers went back to the posts they were.
The abutments are the same masonry in the one place on that bridge where no other mass shares their plan. A
**`StoreyUnder`** relation — "this mass stands under that one" — is what the piers would need, and it is a relation
and not a level.
## `Join::Under`: the piers come back

The masonry piers the last round could not draw are drawn, and the reason they could not is now a variant instead
of a warning: `validate`'s overlap question compares two plans and carries no elevation, so a pier under a gallery is
an overlap however far apart the two stand in height. **A relation says otherwise, and a relation is what the pair
actually has.**

`Join::Under` is `Join::Storey`'s mirror, and it is checked as strictly:

| claim | read for a storey | read for an under |
| --- | --- | --- |
| the touching elevations | the storey's base is the host's top | the carrier's top is the host's foot |
| the contained plan | the storey's outline inside the host's | the carrier's outline inside the host's |
| the refusal | `StoreyNotOnTop`, `StoreyOverhangs` | `UnderNotCarrying`, `UnderOverhangs` |

The 风雨桥's two piers are **one mass of three sections each**, inset `0.42` from the one below — the batter the
abutments got last round, now where it belongs. Identity: `rain_bridge` 1 632 → **1 638 rows**, and **the other
twenty-two scenes are unchanged to the row**.

**What the note said was wrong is now recorded as wrong in the right place**: a shared plan is not an overlap when a
relation says why the two are one thing.
## Stage A landed: the walk takes its points and its ends

The first of the four stages is in, and it is a **pure refactor**: `ring_walk` no longer makes its own vertices and
no longer decides which ends of a ring it closes.

- **`RingPoints`** is the four point arrays a walk draws on — `authored`/`offset` × `base`/`top` — and it is built by
  the caller. A mass of several sections has one set of vertices per section, and the two bands that meet there must
  name the same four; a walk that called `point` again left the ring open at that joint (`SolidOpen { loops: 8 }`, the
  first banded pier).
- **`RingEnds { base, crown }`** says whether the walk closes the foot with the base annulus and the crown with the
  strip of wall top. An interior band of a mass closes **neither**, because its neighbours supply those faces; drawing
  them twice would put two coincident faces in one solid.
- **`RingEnds::BOTH`** is a ring standing on its own, which is every ring that existed before: the mass's whole wall
  and a courtyard's.

**All twenty-four scenes are unchanged to the row — 85 489 — which is the only evidence a refactor can offer.**

### The three stages that remain

| stage | what it needs |
| --- | --- |
| **B** | `cut_edge` carries the crown flag so a cut edge's strip is drawn only on the crown |
| **C** | `banded_mass` builds one `RingPoints` per section and walks each band with the right ends, so a band whose sections share a plan can be cut |
| **D** | the 骑楼's projecting upper floor: foot plan, a horizontal soffit band, then a vertical band with the shop's windows |

Until C lands, a banded mass still answers `BandedMassCut` by name.
## Stage B landed: the crown flag is real

`cut_edge` now carries the ring's crown flag, and the strip of wall top is drawn **only on a crown**. Before this, the
cut path drew that strip unconditionally — which is right for a mass's whole wall and wrong for any band that has a
section above it: the strip would be a face inside the solid.

**All twenty-four scenes unchanged to the row — 85 489** — because every ring walked before this has both ends.

### Why stage C is not a walk through a leaning band

The next stage was to route each band of a mass through `ring_walk`, and it found the real blocker, which is not the
walk but the **identity of a face**.

`FaceRef { mass, edge, loop_side, void }` names a face by its edge. A banded mass has **one face per band** over one
edge, and a face's openings and grids are keyed by that reference — so an opening written against a banded mass would
be cut on **every** band with that edge, at whichever elevation each band happens to have. On the 骑楼 that means one
drawing of a window cut twice, once in the shopfront's plan and once in the projecting floor's.

So the change that unlocks the 骑楼 is `FaceRef` gaining the band it belongs to, and that is a different kind of round
from this one: it is not a walk, it is the **identity** of every face, grid and opening in the crate. Stages A and B
stand on their own as the parts of the walk that were wrong; C waits for that identity.

**What is not blocked**: everything with one band per edge — which is every type in the gallery today.
## Stage C landed: the band is part of the face

The identity change is in, and it is the enabler the last two rounds were circling:

| before | after |
| --- | --- |
| `FaceRef { mass, edge, void, loop_side }` | `FaceRef { mass, edge, band, void, loop_side }` |
| one face per edge | **one face per band per edge**, `band: 0` being the foot's |
| `skeleton.openings.by_face()` keyed by edge | keyed by the strip a hole is in |
| `Ring` built from a loop and a top | `Ring` **carries its band**, and `resolve_holes` matches on it |

Three things follow, and each was needed:

- **`banded_mass` walks a vertical band through `ring_walk`** with that band's own `RingPoints` and `RingEnds`, so a
  strip of a multi-section mass takes openings and a rhythm like any other face. A band that **leans** or lies flat is
  four quads and takes none: the cut machinery reads a face as one strip between a ground line and a top, and a leaning
  strip has no such reading.
- **`Ring::surfaces` names the band** on every element it emits, so two strips' faces are two planarity groups and two
  identity rows.
- **`FaceRef::in_band(k)`** is the one way to name a strip other than the foot's.

**All twenty-four scenes unchanged to the row — 85 489 — and the 风雨桥's frame is byte-identical** to the one the
piers were merged with (`sha256 7261c099…`). A bug in the first pass found both numbers: the leaning bands' base and
crown faces were built with `RingSurfaces::of(loop, 0, band)`, which collapsed every edge's crown strip into one
identity row — 12 rows short, and the picture unchanged. The rows were the instrument that caught it.

### What is left for the 骑楼

Nothing in the crate. A 骑楼's projecting upper floor is **one mass of four sections** — the shopfront's plan, a
horizontal soffit band at the floor's level, then the projecting plan up to the eaves — with the windows authored as
`FaceRef::in_band(2)` on the strip that projects. That is a scene's work and it is the next round.
## The 骑楼's conversion: what it taught, and where it stands

Converting the street's five plots into four-section masses — the plot's plan, the plot again at the floor, the
projecting plan at the floor's own level plus the soffit, and the projecting plan up to the eaves — got as far as
**the shopfront and the upper window being cut in different bands of one mass**, which is the thing three rounds were
for. It did not close the solid, and the two causes are named here so the next round starts where this one stopped.

**Landed, inert, and verified** (all twenty-four scenes unchanged at 85 489 rows):

| change | why it is needed |
| --- | --- |
| **a corner two sections name at the same plan position is one vertex** | a 骑楼 moves its front two corners and leaves the party walls: those walls are the *same* corner written twice, and making a second vertex there opens the solid |
| **a band side whose ends coincide is a triangle** | the same fact from the other side: two of the quad's four corners are one point, and a zero-area face is worse than a triangle |
| **`resolve_holes` matches the band** | without it every strip of a banded mass cuts every hole in that edge — the bug that put the upper window in the shopfront's strip |
| **`face_loop` and `face_extent` read the band's own section** | a hole in the strip above a projecting floor must be measured between *that strip's* ground line and top, not the mass's |

**Not landed, and why**: with the bands cut, the masses came back `SolidOpen { loops: 2 }`, whose open edges sit at the
mass's **base** (`y` at the foot) and are owned by band `0`'s walls and by `WallTop { edge: 1, band: 0 }` faces whose
three corners are all at the *base* elevation — which says the crown vector handed to band `0`'s ring carried the
**section below's** elevations where the section above's were meant. That is the next thing to read, and it is one
instrumented run away: print the `crown` vector and the level elevations for a four-section mass after `site::seat`,
which is the one step between authoring and expansion that this scene applies and the bridge's abutments do not.

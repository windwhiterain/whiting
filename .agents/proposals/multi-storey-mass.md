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

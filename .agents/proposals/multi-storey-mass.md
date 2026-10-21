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
## Round 40: the probe, and the defect it found in one run

`crates/building/examples/band_probe` builds **one mass of four sections** — a 骑楼's — and prints the boundary edges
nothing claims. It is the smallest document that can answer "does the band walk close such a solid at all", and it
answered a defect in its first run:

**A shared corner was compared by plan position alone.** Two consecutive sections that stand on the same plan at two
*elevations* are two different corners, and sharing them collapsed the band between them to no height at all: the probe
reported `open boundary edges: 12` with the slanted inner edge `[2.6, 0.0, -2.1] -> [2.6, 3.16, 4.3]` — the side of a
band that had no floor. The test is the whole node now, position **and** elevation, and the count went to 16 open edges
of a different shape: the *soffit band*'s two horizontal seams, which is a real question about which band closes which
seam and not a defect in sharing.

**All twenty-four scenes are unchanged at 85 489 rows** — the sharing is inert for every mass the gallery has, because
no scene yet carries two sections on one plan at two elevations.

### What the probe says the next step is

With every band closing its own crown the seams move but do not close, and the warped faces name the reason:
`WallTop { edge: 0, band: 0 }` departs its plane by `0.2` and `OuterWall { edge: 2, band: 0 }` by `0.99`, on a band
whose two sections are **the same plan at two elevations** — a band that cannot warp at all. That is the thread to pull
next, and the probe is the instrument that shows it in one run instead of in a five-building street.
## Round 41: two regressions repaired, and the seam bounded

The probe's own reading — planarity measured across each **element group** rather than read off the validator's index —
found two defects, both of them mine from round 39's line-numbered surgery:

- **`Ring::surfaces` was passing `0` instead of `self.band`.** Every band's faces were named `band: 0`, so two bands'
  wall tops landed in one planarity group and reported a `3.2 m` departure — a group that was two planes because it
  was two strips. With the band restored the groups went **21 → 29** and every departure to zero.
- **`ring_walk`'s uncut branch drew its wall-top strip unconditionally.** `cut_edge` has had the crown flag since
  stage B; the uncut branch never got it, so an interior band drew a face *inside* the solid. The probe's faces went
  **80 → 72** and its open edges **16 → 8**.

**All twenty-four scenes unchanged at 85 489 rows** — the strip was only ever drawn for a two-section mass, whose
crown is the crown.

### The seam that is left, bounded from both sides

The eight remaining open edges are all at **one elevation**, the soffit band's crown:

| crown rule | open edges | at |
| --- | --- | --- |
| the last band only | 8 | `y = 3.16` |
| **every band** | 8 | `y = 3.00` |

So whichever band is asked to close the seam does not close it, and the seam simply moves. The next instrument is
narrow and written down here: **print band `0`'s ring corner indices against the loop's own**, because a strip that is
drawn with four points the wall below does not name is exactly this shape of answer.
## Round 42: the base annulus was the seam, and the probe closes

The instrument the last round asked for was built, and it said something the earlier one could not: **`holes 0`**. The
solid was never open. Sixteen edges were claimed **twice in the same direction** — a doubled face, not a hole — and the
probe's old criterion counted both as "open", which is why the seam appeared to move with the crown rule instead of
staying put.

**The root cause was `ring_walk`'s base annulus, unguarded.** `RingEnds { base, crown }` existed since stage A and the
annulus ignored it, so **every** vertical band drew a base at its own foot: band 0 at `y = 0`, which is the foot's and
correct, and band 2 at `y = 3.16`, which is a face inside the solid at the seam between two strips. Guarding it with
`ends.base` took the probe to

```text
expands: 64 faces
open boundary edges: 0 (holes 0, doubled 0)
```

— a **four-section mass with a projecting soffit that closes completely**, which is what four rounds were for. **All
twenty-four scenes unchanged at 85 489 rows.**

### Two things the probe now does that it did not

- **it counts holes and doubled faces apart**, because a hole has one claim and no partner while a doubled face has two
  claims the same way round and no partner either — and the two demand different fixes;
- **it measures planarity across each element group itself**, which is what found the `Ring::surfaces` regression in
  round 41 instead of reading a face index off the validator.

### What the cut mass still does

The street's conversion — four sections, shopfront in band `0`, window in band `2` — was tried again on top of this and
came back `SolidOpen { loops: 4 }`: the **uncut** band walk now closes, and the **cut** one does not. The next
instrument is the probe with one opening in band `0` and one in band `2`, which is a two-line change to a file that
already reports exactly this.
## Round 43: the cut band, measured

The probe grew a second mode — `-- cut` — that opens a shopfront in the foot's band and a window in the projecting one,
and it answers the question the plain mode could not:

| mode | faces | unbalanced |
| --- | --- | --- |
| plain | 64 | **0** (holes 0, doubled 0) |
| cut | 116 | **16, all holes** |

Both seams are at the foot's crown and at the soffit's crown (`y = 3.00` and `y = 3.16`), and the hole edges name the
reason: `[-0.55, 3.0, 2.5] -> [-3.0, 3.0, 2.5]`. **A cut subdivides the band's top row into stations, and the band
above starts from one straight edge per side.** The two do not meet, so the wall's top is open over every span the cut
touched — not a subtle defect, a missing face with a name.

`validate` reports it (`SolidOpen { loops: 4 }`), which is the repository's rule holding: a wrong shape is named and not
silently drawn. What is *not* yet built is the thing that would close it — **the cut's stations have to reach the
section above**, which means the band walk subdividing the loop it hands upward. That is a real piece of work and it is
the whole of what stands between this crate and a multi-storey mass with openings.

### The alternative, and the second gap it found

A 骑楼 can also be **two masses**: the ground floor carrying the projecting upper floor through `Join::Under`, both of
them single-band walls whose holes are cut the way every hole in this gallery is cut. It was built and it is blocked by
**`site::seat`**, which moves *every* mass onto the ground under it: an upper mass authored at the floor's elevation is
seated onto the bank, so `UnderNotCarrying { top: 3.33, foot: 0.33 }` names a carrier three metres above the mass it
claims to carry. So the seat's contract — "each mass stands on the ground under it" — needs to say "**a mass joined to
a host takes the host's shift**", which is a change to `site` and not to the street.

Both gaps are now written down with the measurement that found them.
## Round 44: the seat follows the join, and the 骑楼 stands on its own arcade

`site::seat` no longer presses every mass onto the ground under it. **A joined mass takes its root's shift**, where the
root is the mass that stands on its own ground:

| join | who stands on the ground |
| --- | --- |
| `Storey`, `Wing` | the host — the mass follows it |
| **`Under`** | **the carrier** — the *host* follows *it* |

**Every delta is decided before anything moves**, because a follower's delta is its root's and a root may come later in
the list than the mass that follows it. `Seat` reports `followed: Option<MassId>`, so the record says which mass
decided a seat rather than implying it.

### The 骑楼, drawn

A 骑楼 is **two masses**: the ground floor is the plot, the upper floor is a plan projected `2.2 m` over the walk, and
the ground floor carries the upper one through `Join::Under`. Both are single-band walls, so every wall takes its
openings the way every wall in this gallery does — **no cut ever lands on an interior band**.

- **the shopfront** is cut in the ground floor's face, three bays, boards and a transom rail;
- **the upper floor's window** is cut in the mass that projects, so it sits out over the walk;
- **the canopy is gone.** A 骑楼's overhang *is* the shelter, and the lean-to this function used to build had its crest
  at `base + 3.55` under a soffit at `base + 3.00`: a second roof inside the first. The posts stand under the
  projection's own outer edge and run up to the slab.

Identity: `street` and `street_arcade` 11 950 → **12 550 rows**, `village_square` 17 426 → **18 026** (it holds a street
row), and **twenty-one of twenty-four scenes unchanged**.
## Round 49: a banded mass's wall is not cut, and now it says so

`Problem::BandedMassCut` has been declared since the band walk was written and **raised nowhere**: a face of a mass
drawn in several bands could be cut, the cut subdivided the loop it landed on, and the shell came back open —
`SolidOpen`, `SolidNonManifold`, and sixteen holes on the probe. It is raised now, in `resolve_holes`, for any mass with
more than two sections:

```text
mode: cut
refused: BuildError { problems: [BandedMassCut { mass: MassId(0), face: FaceRef { edge: 2, band: 0,
                             void: None, loop_side: Authored }, openings: 1 }] }
```

**Why a refusal and not a fix.** The measurement says the seam is three faces on one edge: the band below's wall top, the
band above's wall bottom, and the strip of wall top between them. The strip is emitted **per station span** (it always
was — `cut_edge` walks the stations), so with no cut the two walls meet on one edge and the strip is only the crown's;
with a cut the wall's top edge is broken at every jamb, the band above's bottom edge is not, and drawing the strip over
the stations makes the third face. **The stations the cut adds must reach the sections above**, which is a change to the
band walk and not to this function: every section above a cut band would have to carry the same nodes, and a refusal is
better than a shell with holes in it until it does.

### What this unblocks and what it costs

- **measured**: no scene cuts a banded mass — the bridge's piers and abutments are banded and uncut, the 骑楼's two
  masses are one band each — so **all twenty-five scenes are unchanged at 105 876 rows** and the bridge, street and
  一颗印 documents all still build.
- **it costs the tiered 土掌房 its openings**, which is exactly what round 48 measured: the setback band walks and
  closes, and the door in the wall below it is the cut that opens the shell.

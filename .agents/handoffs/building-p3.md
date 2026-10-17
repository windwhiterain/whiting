# P3: the site — buildings on ground, enclosures, and the unit an agent composes

Started 2026-10-16. Branch `feature/building-p3` (worktree `.worktrees/building-p3`, from `main` at
`8f78701`, which is P2 closed). The design is
[`agentic-building-generation.md`](../proposals/agentic-building-generation.md); P2's record is in its
*What P2 found* section, and P1's in the one before it. **This file and the proposal are the whole
record** — P2's handoff was deleted when it closed, and this one will be too.

## What P3 is

The proposal's phasing: *"Several buildings, enclosure walls and courtyards, ground seating against
`terrain` heightfields (D8), the site as the unit an agent composes."*

Slices, in order:

1. **Ground seating** (this slice) — write the terrain's ground into a building's own outline, so the
   document holds the ground rather than assuming a plane.
2. **Enclosure walls and courtyards** — a wall as an authored element, and a courtyard as the space
   they close.
3. **Several buildings as one site** — the site as the unit: a document of several skeletons, one
   ground, one place.
4. **A site scene** — the shed and its yard on a slope.

## Slice 1: ground seating

**The crate stays terrain-independent.** P1's D-list says the crate takes no `terrain` dependency, and
seating does not need one: a height field is a function from plan to elevation, so
`site::seat(skeleton, ground: impl Fn(Vec2) -> f32, offset: f32)` takes the **function** and a caller
with a `HeightField` passes `|plan| field.sample(plan)`. That keeps the dependency decision true, and
it makes the slice testable against an **analytic** ground — which is how the numbers below are exact
rather than plausible.

**One level per mass, and it is the highest ground under it.** The mass model's floor top is a polygon
at its nodes' own elevations, so a quad with four different elevations is a **ruled surface, not a
plane** — and the crate's planarity contract would report it, correctly. A building therefore sits on
a **terrace**: `level = the highest sampled ground under the outline + offset`, written into every node
of that mass. That is also what a builder does with a slope — cut and fill into a level floor, with the
downhill side showing its slab — and it keeps every invariant the shell already has.

**The sample set is stated**, because "the highest ground under the outline" has no exact answer from
a distance function: the outline's **nodes**, the **midpoint of every edge**, and the **centroid**.
`O(nodes)` per mass, and the probe measures that it agrees with the analytic maximum of a plane to
rounding.

**`SeatReport` is the check.** `validate` cannot check a document against a terrain the document does
not hold, so the verb reports what it did: how many masses were seated, the level each was given, and
the ground's own range under that mass — the cut on the uphill side and the fill on the downhill one,
which is the number an author needs to see and the only place they can see it.

**Acceptance**, measured by `examples/seat_probe.rs` against the analytic plane `z = 0.3 x`:

- the level of a 5 × 8 mass equals the plane's maximum over the sample set, to `1e-4`;
- every node's elevation equals the level, so the mass's floor is one plane — the mass still reads
  `loops 0`, Euler 2, manifold, and its volume is the analytic one for that level (slab plus walls);
- a **second** mass elsewhere on the slope is seated at its own level, so a site is several terraces
  and not one;
- seating is a **function of the ground and the plan**, so two runs give the same document;
- and P1/P2's acceptance does not move: `census` at three starters, 0 violations, volumes
  `64.2080 / 24.0000 / 36.2880 / 12.2500`.

## Conventions that bind (unchanged from P1 and P2)

- Plan `(x, z)`, space `(x, y, z)`, `y` up, through `crate::math`.
- A face is wound so its right-handed normal points **out**; a positively wound outline puts the solid
  on the walker's **right**; the global check is positive signed volume.
- Points are welded by **authored identity**. A face's `Element` is its provenance. Solids stay
  separate. The eave is authored once. **The document is the skeleton**: seating *bakes* the ground
  into the document and no live rule survives the call.
- `validate` reports everything in a deterministic order and repairs nothing; ops refuse only a
  dangling index or a domain question, and a derivation's answer is never "nearly right".

## Status log

- **2026-10-16** — phase opened; worktree and branch from `8f78701`.
- **2026-10-16** — **slice 1 (ground seating) implemented and measured.** `site.rs`: `seat(skeleton,
  ground: impl Fn(Vec2) -> f32, offset) -> SeatReport`, `Seat { mass, level, lowest, highest, delta }`
  with `fall()`, `SeatReport::{len, is_empty, deepest_fall, highest_level}`; registered in `lib.rs`.

  The two decisions that made it correct rather than plausible:

  - **Rigid, not resized.** The delta goes to every node of a mass's outline, to its `top`, and to the
    eaves and crest of every roof that names it as host. Measured on a 5×8 shed: volume `36.832001`
    before and after, mass bounds moved by exactly `0.750000` (the delta), and the roof's rise stayed
    `1.850000`. Moving only the outline would have shortened every wall and flattened every pitch, and
    nothing in the closure predicates would have said so.
  - **A half-known ground is not a ground.** Any non-finite sample refuses the mass outright — the
    first implementation skipped the bad sample and used the rest, which is exactly the guess this
    crate refuses; measured after the fix, a ground unknown over one half seats **0 of 1** masses and
    leaves the document untouched.

  Against the analytic plane `z = 0.3 x`: level `0.750000` = the plane's maximum over the outline
  (`0.3 × 2.5`), lowest `−0.750000`, fall `1.500000`, and every node standing at the level. Two masses
  `12 m` apart come out as two terraces `3.600000` apart (`0.3 × 12`); an offset of `1.5` lifts the
  level to `2.250000`; seating a second time changes nothing; and the seated document reports **0
  violations** with both solids closed and manifold.

- **Next**: slice 2 — **enclosure walls and courtyards**. A wall has to be an authored element of its
  own (a mass is a room, and a freestanding wall is not one), and a courtyard is the space the walls
  close; the open question the design has not answered is whether an enclosure is one mass with a
  thickness or a chain of wall segments, and whether a gate is an opening in it (the answer that would
  reuse the most is: a wall is a mass with a thin outline and its gate is an opening, which the openings
  slice already knows how to cut).

- **2026-10-16** — **slice 2 (walls and courtyards) implemented and measured**, and the open question
  above answered **against** its own suggestion. A wall is **not** a mass with a thin outline: `Mass` is
  the room model (a shell with a floor disc and a slab), so a thin mass either leaves a cavity — a gate
  would look into the wall's inside instead of through it — or degenerates its floor onto its top, the
  fault the shed's canopy already paid for. And a gate is **not** a hole through a wall: it is a **gap
  between two runs**, which needs no cutting at all.

  So: `wall.rs` — `WallId`, `Wall { from, to, thickness, top }` (one straight run: a box whose bottom
  follows the ground and whose top is level), `WallSurface { Side { left }, End { start }, Bottom, Top }`,
  `length`/`direction`/`normal`/`highest_ground`/`lowest_ground`/`mean_height`/`volume`. Wired through
  `SolidId::Wall`, `Element::Wall`, `expand_wall` (six quads, twelve triangles), three document checks
  (`WallTooShort`, `WallNotThick`, `WallTooLow`), and `identity`'s naming ("wall 0, its left side,
  looking out"). `site::seat` also seats walls now — a **wall follows the ground** at both ends while a
  **building sits on a terrace**, and the report counts each.

  Measured by `examples/wall_probe.rs`, where every number can be written down as well as measured:

  - a 10 × 6 yard is **5 walls, 29.6 m of wall, 17.76 m³ written down**; every run is **12 faces, loops
    0, Euler 2, manifold**, and the measured total is `17.760012` against the written `17.760000` (off by
    `1.1e-5`), with each run compared in its own line (`6.000001` against `6.000000`);
  - **the gate is the gap**, measured at `2.4000 m` between the two south runs;
  - **each corner is shared by two runs that interpenetrate, and `0` walls are unclosed** — the
    two-level watertightness rule is what makes a courtyard work without a cleverer element;
  - seated on the plane `z = 0.3 x`, the north run stands on `−1.5000` at one end and `+1.5000` at the
    other with its top at `2.0000`, its volume **unchanged at `6.000000`** (exact for a linear slope,
    because the bottom turns about its mean) and its box now running from `y −1.5000` to `2.0000`, still
    closed;
  - the three faults fire and are refused by the build, each named: `WallTooShort { length: 0.0 }`,
    `WallNotThick { thickness: 0.0 }`, `WallTooLow { top: 0.0, highest: 0.0 }` — and the document's own
    `DidNotExpand` carries the same three.

- **Next**: slice 3 — **several buildings as one site**: the site as the unit an agent composes. The
  question to answer first is whether a *site* is a new document type (several skeletons, one ground)
  or whether the existing skeleton is already a site (it holds several masses and walls today) — and the
  answer changes what "the unit an agent composes" means.

- **2026-10-16** — **slice 3 (the site as a unit) implemented and measured**, and the question above
  answered: **the skeleton is already a site** — it has held several masses and walls since P1 — so a new
  document type would be a second place to hold the same thing. What was missing was a way to **look at
  it as one place**, and two things that only a site can say:

  - `Mass.name: Option<String>` with `Mass::named(..)`, because "mass 3" is not how an author speaks of
    their buildings. **A name is for a reader and never a handle**: the crate resolves nothing by name,
    two buildings may share one, and the doc on the field says so.
  - `site::report(&skeleton) -> SiteReport { buildings, walls, wall_length, ground, overlaps }`, with
    `Building { mass, name, footprint, top, lowest, roofs, openings, parts }`, `Overlap { first, second,
    area }`, and `SiteReport::{building, footprint, relief}`.

  Measured by `examples/site_probe.rs` on a place of three buildings (two sheds 12 m apart and a store)
  and a five-wall courtyard with a gate, then seated on `z = 0.3 x`:

  - before seating: **3 buildings, 5 walls, 38.0000 m of wall, footprint 92.0000 m²**, and the store's
    box overlapping the first shed's by **4.5 m²** — `1.5 × 3` by hand, and the report agrees;
  - the overlap **is legal** because the store is attached to the shed as a wing: the first version of
    the probe left it unattached, and `UnattachedOverlap` said so — the validator caught the author, not
    the report;
  - seated: **3 masses seated and 5 walls grounded**, the ground running `0.7500 .. 4.3500` (relief
    `3.6000`), the sheds standing on `0.7500` and `4.3500` — which is what `0.3 × 2.5` and `0.3 × 14.5`
    say by hand — and their tops moving with them (`3.5500`, `7.1500`);
  - the seated place **builds** (88 points, 144 faces) and **validates with 0 violations**;
  - a **derived** house is a site without being told: the starter reports `1 building`, footprint
    `77.0000 m²`, `1 roof`, ground `(0.0, 0.0)` — so the report works on what P1 already made.

  `Bounds2` (a plan bounding box) and `overlap_of` live in `site.rs` with the overlap's doc stating the
  bound plainly: two L-shaped buildings can have overlapping boxes and no shared wall at all, which is
  why the report **lists** overlaps and nothing **judges** them.

- **Next**: slice 4 — **a site scene**: the shed and its yard on a slope, drawn. That is the last item
  in the phase's own scope, and it is the one that shows whether the site reads as a place.

- **2026-10-16** — **slice 4 (the site scene) drawn**: `crates/gallery/src/yard.rs`, registered as
  `"yard"`, 13 passes. A shed and a store up a `0.16 x` slope, inside a five-run courtyard with a gate,
  everything seated by one `seat` call and expanded from one document.

  Three things the frame had to get right, each of which was a decision rather than a tweak:

  - **One law for the ground.** `ground(x, z)` is called once by the terrain mesh and once by `seat`, so
    what the frame draws is exactly what the document was seated on. Two laws would draw buildings
    hovering over a slope or buried in it, and the picture would still look plausible — the one failure a
    render cannot show and a shared function cannot have.
  - **The scene refuses to draw a site it could not seat**: it asserts `2` masses and `5` walls were
    seated, because a mass the ground did not answer for stays at its authored elevation, which on this
    slope means floating.
  - **A building on a slope needs a deep foundation, and the frame proved it.** The terrace is cut to the
    *highest* ground under a building, so its downhill edge stands the relief above the ground; with the
    starter's 0.3 m slab the whole downhill side floated, and the scene's buildings were given a `1.20 m`
    slab — which is what a real building on a slope has, and the reason the slab exists.

  First render (`out/yard.png`) read as a beige plane: the 0.09 slope was too gentle and the four
  materials too close in value. Second (`out/yard2.png`) with `0.16` and separated materials reads as a
  site; the remaining artifact is the shadow terminator's toothing, which is the shadow subsystem's.

- **Next**: **closing P3.** All four slices are in. What remains is book-keeping: fold this handoff's
  durable findings into the proposal as P3's record, delete the handoff, and let the phase branch be
  merged and removed — the same closing P2 got.

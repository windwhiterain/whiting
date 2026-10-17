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

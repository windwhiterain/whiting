# P2: the facade — the grid, the openings, the parts vocabulary

Started 2026-10-16. Branch `feature/building-p2` (worktree `.worktrees/building-p2`, from `main` at
`b8233a5`, which is P1 merged). The design is
[`proposals/agentic-building-generation.md`](../proposals/agentic-building-generation.md) revision 2
plus its *What implementation found* section; P1's own handoff was deleted when that phase closed, so
**this file and the proposal are the whole record**.

## What P2 is

Levels and bays with the grid invariants; openings mounted in bays; the parts vocabulary with authored
sockets; instance emission (D4); the identity **sibling** mechanism (D6); one coherent style set; and
`foldyard`'s shed and enclosure rebuilt as the second demonstration, whose door and gate are
bay-mounted openings.

Slices, in order:

1. **The grid** (this slice): the S2 document, its verbs, its invariants, its probe.
2. **Openings**: a bay carries an authored opening (a gap or a part), and the mass's wall is cut by it
   rather than being composed from piers and a lintel by hand.
3. **Parts**: templates with named sockets, placed into bays, instanced (D4).
4. **Identity**: the sibling of `gallery::identity` over this crate's expansion (D6).
5. **The second demonstration scene**.

## Slice 1: the grid

- **`FaceRef`** — which face: a mass and one outline edge. The face's extent is that edge's own plan
  length, horizontally, and `[lowest node elevation, mass.top]` vertically.
- **`FaceGrid`** — the authored rhythm of one face: levels (elevations) and divisions (arc-length
  stations along the edge). The two lists are the *stops*, so a bay is the cell between two
  consecutive stops of each.
- **`Bay`** — one cell, named by its **slot** (`row`, `column`) and not by its coordinates. That is
  D10's rule and the reason it exists: a re-split replaces the family, and a placement keyed by slot
  survives it where a placement keyed by position could not.
- **Invariants**, and each is a check rather than a hope: the stops are strictly increasing; the
  levels **span the face exactly** (first at the base, last at `top`) and the divisions likewise
  (first at `0`, last at the edge's length); and a family has at least two stops, because one stop is
  no bay at all.
- **`Rhythm`** — how a family is laid out: `Even { count }`, `Absolute { stops }` (which must span the
  extent exactly or be refused), and `Rhythmic { bookend, module, flexible }` where the remainder is
  either absorbed by one **declared** middle module or refused. The layout is a function with a
  domain: `Rhythm::stops(extent) -> Option<Vec<f32>>`.
- **Verbs**, with D10's **replace semantics**: `ops::split_face(skeleton, face, columns, rows)`
  replaces the face's whole family and returns how many bays it made, and `ops::clear_face` removes
  it. Nothing merges, and a rhythm the face cannot carry is refused (`None`) rather than guessed at —
  `mountain::derive_valleys`' rule, which is the one this crate applies everywhere.

**Acceptance for this slice**, measured and printed by `examples/grid_probe.rs`:

- for each rhythm and each of a house's four faces: the columns, the rows, the bay count, and the
  **tiling sum** — the bay widths added up against the edge's own length, and the bay heights against
  the wall's own height, both to within `GRID_TOLERANCE`;
- the **replace-semantics measurement**: a face divided twice carries the second family and not the
  sum of both, and a bay's slot still names the same cell after a re-split with the same rhythm;
- the **refusals**: `Absolute` stops that do not span the extent, an `Even` count of zero, a
  `Rhythmic` module larger than the face, `split_face` on a face of a mass that is not there;
- and `census` unchanged: P1's three starters still report **0 violations** and their volumes at
  `64.2080 / 24.0000 / 36.2880 / 12.2500`.

## Conventions that bind every module (unchanged from P1)

- Plan is `(x, z)`, space is `(x, y, z)`, `y` up, through `crate::math`.
- A face is wound so its right-handed normal points **out** of the solid; a positively wound outline
  puts the solid on the walker's **right**; the global check is positive signed volume.
- Points are welded by authored identity, never by coordinate. A face's `Element` is its provenance
  and its shading group. Solids stay separate. The eave is authored once.
- **The document is the skeleton**: a grid is data in the document, a rhythm is a construction move
  and never a live rule, and a derived family carries no guarantee.
- `validate` reports everything in a deterministic order and repairs nothing; ops refuse only a
  dangling index (and a rhythm outside its domain, which is a domain question rather than a fault).

## Status log

- **2026-10-16** — phase opened; worktree and branch from `b8233a5`.
- **2026-10-16** — **slice 1 (the grid) implemented and measured.** `grid.rs` (FaceRef, FaceGrid,
  Bay, Rhythm, Grids, `GRID_TOLERANCE`, `MAX_STOPS`), `BuildingSkeleton.grids` with the lookups a
  placement needs (`grid`, `bay`, `face_length`, `face_extent`, `faces_of`), the two verbs
  (`ops::split_face`, `ops::clear_face`) and four validator checks (`GridFaceMissing`,
  `GridTooShort`, `GridNotOrdered`, `GridOutsideFace`).

  Measured by `examples/grid_probe.rs` on a 7×11 starter house, every edge divided 3 across by 2 up:

  - **the bays tile the face exactly**: widths summing to the edge's own length and heights to the
    wall's own height, both **off by `0.00e0`** on all four faces, against a `GRID_TOLERANCE` of
    `1e-3` m;
  - **replace semantics**: 6 bays → 10 after a second division of 5 by 2, so the family was replaced
    and not accumulated, and a slot read before and after a re-split with the same rhythm names the
    same cell `[2.8000, 4.2000] × [0.0000, 1.6000]`;
  - **the remainder case**, which is why `Rhythmic` has a `flexible` flag at all: with bookends of
    0.5 m and modules of 1.3 m on a 7 m edge, 0.8 m is left over — an inflexible rhythm is
    **refused**, and a flexible one lays `[0.5, 1.3, 1.3, 2.1, 1.3, 0.5]`, the middle module carrying
    the remainder by name;
  - **refusals**: stops that do not span the edge, no bays across, a module wider than the face, a
    **module so small it asks for millions of stops**, a hundred thousand bays, and a face the
    document does not hold — all `None`, none of them panics;
  - **P1 is untouched**: `census` still reports 0 violations with volumes `64.2080 / 24.0000 /
    36.2880 / 12.2500` and the square's 16-face roof, and every other probe still exits 0.

  `MAX_STOPS` (4096) was added after the first run, by the chief designer rather than by a review:
  a rhythm's stop count is a function of two authored numbers, so `module = 1e-6` asks for eleven
  million stops on an eleven-metre face and the allocation is reachable by arithmetic alone. The
  precedent is `symbios-shape`'s own bound on rhythm expansion, and the answer is a refusal rather
  than a clamp.

- **Next**: slice 2 — **openings**. A bay carries an authored opening, and the wall is cut by it
  instead of being composed from piers and a lintel by hand (`built.rs`'s `WallGap` is the hand-made
  version of this). The interface is the chief designer's to freeze before any work is dispatched:
  the opening has to name its bay **by slot** (D10), the cut has to keep the wall a closed solid, and
  the fascia/wall-top ring has to follow the cut rather than being authored twice.

## Slice 2: openings

**The interface is frozen and committed** (`fcf5afa`): `openings.rs` — `Opening` (face, row, column,
`Inset`), `Extents` (the rectangle in arc length × elevation, with `area()` and `overlaps()`),
`RevealSide`, `Openings` (an author-ordered list, with `of_face`/`by_face` lookups); the
`MassSurface::Reveal { edge, opening, side }` variant; `BuildingSkeleton.{openings, aperture,
apertures_of}`; and the two verbs `ops::{add_opening, remove_opening}`.

The contract, and the two decisions inside it:

- **A hole is strictly inside its wall face.** The face is a vertical strip in `(u, y)`: `u` over the
  edge's own length, `y` from the **ground line** (`node elevation − slab`, linear between the edge's
  ends) to `mass.top`. A hole that reaches that boundary is a different shape — the wall stops being a
  rectangle with holes and becomes pieces joined at the author's corners — and is refused by name.
- **A door needs no second mechanism.** The grid's storey lines are elevations from the *ground*, and
  the wall face's floor is `slab` below it, so a hole whose sill inset is **zero** starts at ground
  level with the slab's thickness of wall under it. (The chief designer's first brief said the sill
  inset should *equal* the slab thickness; that is wrong and was corrected in flight — it would put
  the hole 0.4 m above the ground.)

**Two works were dispatched** from that interface, in their own worktrees: `bo-cut`
(`expand.rs` — the band decomposition, the inner face's hole located by projecting the outer hole's
corners, the four reveals per hole, and a refusal for any hole that is not strictly inside) and
`bo-check` (`validate.rs` — `OpeningBayMissing`, `OpeningExceedsBay`, `OpeningOutsideWall`,
`OpeningOverlaps`, each with a positive control).

**`bo-check` is merged and measured** (ranks 24–27 in `order_key`, called from `document()`). Its
controls are worth recording because two of them are what a wrong implementation would pass:

- **the ground line needs a sloping wall to be tested at all** — insets only move a hole inward and a
  bay's first row starts at the node elevation, a slab *above* the wall's own foot, so on level ground
  no hole can reach the ground line. The control that earns the `max`-over-the-hole's-two-ends is a
  sloping wall where the line at the midpoint reads `+0.0500` (would pass) and at the far jamb reads
  `−0.7000` (fires);
- **a touching pair must not fire** — two holes meeting exactly at one `u` remove no wall twice, and
  the overlap comparison is therefore strict on both axes. A non-strict one would report every
  divided-and-opened facade;
- **a door must not fire** either: sill inset `0` leaves a `0.4000` m margin over the ground line, and
  the brief's own "sill = slab thickness" door leaves `0.8000` m. Both read clean, which is the
  correction in flight being right for the second time.
- **Census unchanged**: three starters, 0 violations, volumes `64.2080 / 24.0000 / 36.2880 / 12.2500`.

`bo-cut` is the remaining half of the slice: until it lands, the acceptance probe prints the wall's
volume unchanged with **0 reveal faces and the removal `2.4860` m³ short**, which is exactly the gap
this slice exists to close.

### Two contract facts `bo-cut` found before writing the cut, both now in the interface

1. **The decomposition's `y` breakpoints are global to the face, not per band.** Every hole's `low`
   and `high` must split *every* band of that wall, so a hole-free band beside a hole comes out as
   stacked quads sharing the same two `u` values and the same elevations as its neighbour. Taken
   literally as first briefed — "for each band, emit the complement of the holes that span it" — a
   hole-free band is one quad from the ground line to the wall top, while the band beside it is two,
   and the vertical edge at the hole's side is then used once as `(ground, top)` and once as
   `(low, high)`: two edges that do not match, and an **open** mass. The brief specified the bands and
   the complements and never said the splits are shared across them.
2. **A lookup that filters must have a companion that names what it dropped.**
   `apertures_of(face)` returns what *can* be cut, so a hole whose slot the grid no longer holds — or
   whose insets leave it no area — is simply absent from it. An expander reading only that would build
   a wall with a hole missing and **no complaint**, a mesh quietly disagreeing with its document.
   `BuildingSkeleton::unresolved_openings(face)` is now that companion, and **both go through
   `aperture`**, so there is one definition of "resolved" rather than one per caller — the same shape
   as the coverage lesson from P1, where a check that reads a filtered set cannot see what the filter
   removed.

A third finding is about the chief designer's own reading rather than about the code: file sizes and
mtimes in a worktree are the **checkout's**, not the work's. A work reported as "40.9 KB, written
minutes ago" was byte-identical to its base commit; progress was being inferred from a checkout
timestamp. Only a commit is evidence of work.

### The cut, measured, and the two things it taught

`bo-cut` committed its work (its session then died mid-report — the commits survived, which is the
whole reason the brief demanded early ones) and the acceptance found two facts:

1. **A hole's jamb margin is the miter's own offset, not the tolerance.** A door inset `0.3` m from an
   edge whose 90° miter moved the inner rim inward by exactly the wall's thickness (`0.3` m) is
   refused by the expander with `OpeningOutsideWall { past: 0.0010001659 }` — `GRID_TOLERANCE` plus
   `f32` rounding, i.e. the jamb landed *exactly on* the inner rim's end. The refusal is **necessary**:
   an inner wall's corner edge runs the full height and is not split at a hole's elevations, so a
   reveal whose inner edge landed there would be a sub-segment nothing welds to, and the mass would be
   open. The domain is therefore "strictly inside the inner rim", and the reference case is inset
   `0.5` m; the boundary value is itself measured, as a refusal.
2. **The wall's volume came out exactly right while its topology did not.** For a wall carrying a door
   in bay 0, a window above it, and a window in bay 1: `removed 2.3480 m³` against the analytic
   `2.3480` (off by `8e-6`) and `reveal faces 24` for three holes — with `loops 8`, `euler −12` and
   `manifold false`. The validator named all three (`SolidOpen`, `SolidNonManifold`,
   `SolidNotASphere`), which is the checks half doing its job on the cut half's defect — the same
   shape as P1's lesson, one level down: **two halves measured separately are not a system**.
   Every case the previous work measured passes (one hole, a door, two stacked in one bay, an
   L-shaped outline, a rising wall); the failing shape is **holes in different bays of the same wall**,
   which is exactly the `u`-band/`y`-split interaction its own pre-code finding had named. A fresh work
   (`bo-cut2`) carries that localized symptom and the acceptance number `euler = 2 − 2 × 3 = −4`.

### Slice 2 closed: the defect, fixed, and the check that had outlived its assumption

`bo-cut2` died before producing anything, so the chief designer finished the defect directly. Two
changes, and the second is the more interesting one:

1. **A jamb is split at every cut of its wall, because the wall's own faces are.** The cut elevations
   are global to the wall — that is what makes neighbouring bands share whole edges — but a reveal was
   emitted as **one quad from its sill to its head**, so its jamb drew a single segment where the wall
   beside it drew two. The mismatch was exactly the cuts that belong to a hole *in another bay*: the
   window above the door spans `[1.8, 2.7]` while a window in the next bay made `2.5` a cut of the
   whole wall, so the wall had `[1.8, 2.5]` and `[2.5, 2.7]` on that line and the reveal had one edge.
   The fix splits the two **jamb** reveals across the row intervals between their sill and head; the
   head and the sill stay single quads, because their edges run along the wall and no cut divides a
   horizontal edge. Acceptance, measured: `loops 0`, **`euler −4`** (`= 2 − 2 × 3`), `manifold true`,
   volume `61.8600`, `removed 2.3480` against the analytic `2.3480` (off by `3e-6`), `reveal faces 32`,
   **0 violations** — and the un-cut mass is still 28 faces, loops 0, Euler 2, volume `64.2080`.
2. **`SolidNotASphere` had outlived its assumption.** The check said every solid must have Euler
   characteristic `2`, which is a statement about a sphere and was true while a mass was one. An
   opening is a **handle through the material**, so each one takes the characteristic down by two: a
   mass with three openings is a genus-3 surface reading `−4`. The check now compares against
   `2 − 2 × the openings that resolve`, counted from the document — and only resolved ones, because an
   opening whose slot the grid no longer holds builds nothing at all (`DidNotExpand` says so), so
   counting it would name a shape the mesh does not have. This is P1's *Floor* collision in a new
   dress: **a check that encodes a shape outlives the shape**, and the fix is to make it read the
   quantity the contract actually names.

With that, slice 2 is complete: `fcf5afa` (interface) → `bacc44f` (cut + checks merged) →
`4d63f67` (acceptance) → this fix. The phase's remaining slices are parts, identity and the second
demonstration scene.

**The acceptance probe is the chief designer's** (`examples/opening_probe.rs`), and its strongest
number is the one no predicate can fake: a wall with a hole holds less material by exactly
`hole area × wall thickness`. Measured on the document half before the cut landed — three holes (a
window, a door at ground level, a window above it) totalling `7.3 m²` of wall, so **`2.486 m³`** of
material to remove, with the verb refusing an off-grid slot, an inset that eats its bay, and a
negative inset, and the document reporting **0 violations**.

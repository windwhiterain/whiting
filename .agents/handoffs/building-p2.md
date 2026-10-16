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

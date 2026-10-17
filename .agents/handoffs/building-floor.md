# Floors: the plate inside a mass's own walls

Started 2026-10-16. Branch `feature/building-floor` (worktree `.worktrees/building-floor`, from `main` at
`352795a`, which closed the instancing slice). The design is
[`agentic-building-generation.md`](../proposals/agentic-building-generation.md). **This file and the
proposal are the whole record.**

## Why this slice

Every building the crate can author is a **single storey**: `shed` and `yard` are one room each, and the
grid's `rows` can split a wall into storeys, but nothing puts a floor between them. The vocabulary to do it
exists — `Slab` is a level plate and `host` makes it move with its building — and the *authoring* does not:
an intermediate floor is the mass's **inner loop** at an elevation, and the inner loop is `expand`'s private
`offset_loop`. An author who computed it themselves would be writing down a second opinion about where the
inside of the building is, which is precisely the duplication this crate exists to remove.

So the derivation goes where the other derivations live: **`derive::floor_slab(mass, id, elevation,
thickness) -> Option<Slab>`**.

## The invariants

- **The plate is the mass's own inner loop.** It is the same inset the walls are built on, so the floor
  meets its walls and cannot disagree with them about the inside.
- **Hosted by the mass**, so `seat` moves it with the building.
- **The elevation must be strictly inside the mass's height** — above the lowest ground and below the wall
  top — and the thickness positive; every other case answers `None` rather than a plate that would be
  invisible, buried or sticking out.
- A wall the miter refuses answers `None`: the mass's own expansion reports that fault, and a floor derived
  from a wall with no inside has no inside either.

## Acceptance, measured

`examples/floor_probe.rs` on a 7 × 11 starter with a 0.35 wall, a floor at `2.0` and `0.2` thick:

| measurement | value |
| --- | --- |
| the plate's plan | `6.3000 × 10.3000 = 64.889999 m²` by hand, and `64.889999` from its own loop |
| its volume | `12.978000` written, `12.978002` measured |
| its topology | **12 faces, loops 0, Euler 2, manifold** |
| the inset, on each of four sides | `0.3500 / 0.3500 / 0.3500 / 0.3500` against the wall's `0.3500` |
| the document | **0 violations** with the floor in it, and **0** when seated on `z = 0.3 x` |
| seating | the floor moves with the mass (`y 2.8500 .. 3.0500`, the flat one plus the `1.0500` delta) |
| the refusals | at the ground, below it, at the wall top, above it, and of no thickness — **all `None`** |
| the storeys it leaves | `1.8000 m` below its underside and `1.6000 m` above its top in a `3.6000 m` building |

## Status log

- **2026-10-16** — worktree and branch from `352795a`; `expand::offset_loop` made `pub(crate)`;
  `derive::floor_slab` added; probe written and run; gate green (16 probes).
- **2026-10-16** — **the two-storey scene added** (`crates/gallery/src/workshop.rs`, "workshop"): a 6 × 9
  two-storey building with three windows above, a window, an **open doorway** and a window below, and a
  mezzanine plate at `2.2` derived by `floor_slab`. 13 passes; the scene asserts its own document is clean.

  ## A real defect the scene found, reduced and recorded

  The scene's first configuration put the plate at `2.8` and the doorway's head at `2.05`, and the assert
  fired: `SolidOpen { loops: 4 }`, `SolidNonManifold`, `euler -14` on the mass. **The fault is in the
  openings' cutting and predates this slice.** Reduced by `examples/multi_row_probe.rs` to a **minimal
  reproduction**:

  | configuration on one wall | result |
  | --- | --- |
  | one ground-sill opening ("a door"), alone | `loops 0, euler 0` ✓ |
  | a door and a window **beside it** (same row) | `loops 0, euler -2` ✓ |
  | a door and a window **above it**, same column, **the same insets** | `loops 0, euler -10` ✓ |
  | six openings in two rows, **the same insets per row** | `loops 0, euler -10` ✓ |
  | a door and a window above it, **different insets** (u spans overlap, jambs not aligned) | **`loops 4, euler -6`, non-manifold ✗** |

  The diagnosis, measured rather than guessed: the cut elevations are **global to the wall** (that is what
  makes neighbouring bands share whole edges — the P2 fix), so the wall's vertical edges are split at
  **every** cut; but a hole's jamb reveals are emitted over `low..high` **of that hole** only. When a second
  hole's jamb station falls **inside** the first hole's `u` span — which is exactly what different insets
  produce — the wall's band splits at that station leave segments inside the first hole's height that no
  reveal covers, and the surface reads open. The wall's own edges are right; the *jamb reveals* of the hole
  that spans the extra station are what is missing.

  **The scene was then rebuilt with column-aligned insets** so it renders (13 passes), and the fault is
  recorded here as the next slice's subject rather than worked around silently: the fix belongs in
  `cut_edge`'s reveal emission, where a jamb must be split at **every** cut of the wall that lies strictly
  between the hole's own sill and head — the same rule the wall's bands already follow — so that two holes
  whose spans overlap close even when their jambs disagree.

  The scene's doc says plainly what the frame shows (two storeys) and what it does not (the plate, of which
  about seven centimetres are visible as the doorway's own ceiling), because a frame that overclaims is the
  one kind of documentation this crate cannot check.

- **Next**: the reveal-splitting fix above, with `multi_row_probe` as its acceptance (the failing row of
  that table must read `loops 0, euler -6, manifold true`).
- **2026-10-16** — **fixed, and the fault was the mirror of the one P2 fixed.** The boundary-edge ownership
  dump named it in one step: the door's **sill reveal** owns one edge `(0.553, 0, 3.7) → (-0.553, 0, 3.7)`
  while the **floor top** owns `(0.483, 0, 3.7) → (0.553, 0, 3.7)` — the sill's inner edge is **one** edge
  across the doorway, and the floor's rim is **two** there because a station (the window's jamb) falls
  inside the doorway's span. The two faces disagree about whether a horizontal edge is broken, and since the
  crate welds by authored identity they never meet.

  So the head and the sill reveal must be split at **every station the hole's own span holds**, exactly as
  P2 had already made the jambs split at **every cut of the wall**. P2's comment — *"the head and the sill
  are single quads: their edges run along the wall rather than up it, and no cut divides a horizontal
  edge"* — was true of **cuts** and false of **stations**, and is corrected in place.

  New table from `multi_row_probe`, the two rows that failed now the first two that pass:

  | configuration | before | after |
  | --- | --- | --- |
  | a door below and a window above, jambs not aligned | `loops 4, euler -6` | **`loops 0, euler -2, manifold true`** |
  | six holes, a different inset per row | `loops 12, euler -22` | **`loops 0, euler -10, manifold true`** |
  | every configuration that closed before | closed | **closed, and by the same counts** |

  The failing mass now has **0 boundary edges**. The workshop's doorway is restored to its **wider** inset
  (`0.28` against the windows' `0.34`) — the configuration that could not be drawn when the scene was
  written — and the scene re-renders at 13 passes with its own document assert clean.

  The gate is re-run on all seventeen probes and green; `census` reads 0 violations on all three starters.
- **2026-10-16** — **the reduction tightened, and the diagnosis narrowed.** `multi_row_probe` was rewritten
  around a table of configurations, and it now measures which ones close:

  | configuration | result |
  | --- | --- |
  | 1/2/3 holes in one row, one storey | `loops 0`, `euler 0/-2/-4`, manifold ✓ |
  | 1/2/3 holes in one row, two storeys | the same ✓ |
  | a door below and a window above, **jambs aligned** | `loops 0, euler -2` ✓ |
  | six holes, the same inset in every column | `loops 0, euler -10` ✓ |
  | a door below and a window above, **jambs not aligned** | `loops 4, euler -6` ✗ |
  | six holes, **a different inset per row** | `loops 12, euler -22` ✗ |

  Two facts the detail section added, which **narrow the fault and rule out my first guess**:

  - **every reveal is emitted** — the mass holds `2 ×` (two triangles per quad) for all four sides of
    *both* openings, so the defect is not a missing reveal, which is what the "split the jambs at every
    cut" theory assumed;
  - the boundary edges sit on the **inner face** at the *window's* jamb positions (`±0.483`) but at the
    *door's* elevations (`0` and `2.050`), while the outer face's are at the door's own jambs (`±0.553`).
    So two points that should be one are in **different places in plan**: the rim as the wall's bands build
    it and the rim as the reveals build it disagree, and because the crate welds by **authored identity**
    rather than by coordinate, the two edges never meet.

  That points at the jamb-point arithmetic on the **inner** face (`WallFace::inner_at` and the mitre's
  `inner_from`/`inner_to`) rather than at the reveal loop, and the next instrument is to print each
  cut station's outer and inner plan positions beside the reveal corners that use them. Recorded rather
  than guessed: the fix is a slice of its own, and the scene renders today with column-aligned insets.

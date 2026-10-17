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

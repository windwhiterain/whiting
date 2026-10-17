# Volumes: a mass becomes a wall solid and a plate

Started 2026-10-27. Branch `feature/building-volumes` (worktree `.worktrees/building-volumes`), from `main` at
`32cc001`. The decision is
[`agentic-building-generation.md`](../proposals/agentic-building-generation.md), section *The volume decision*.

## The decision this implements

**Walls, floors and roofs become volumes, not faces of one shell.** The project asked for it after the courtyard
slice proved the shell could not express a wall standing in the middle of a plate: the wall's inner base would have
had to pair with both the paving and the courtyard-facing face — three surfaces on one edge, for every winding.

With volumes:

- **a mass is two solids**: its **wall ring** and its **plate**, each closed on its own;
- a courtyard is the wall ring carrying a **void loop**, so the ring is a ring-shaped solid with **Euler 0**;
- the plate is a box under both the rooms and the courtyard, so nothing is split at a wall, and the three-surface
  edge never arises.

## What is already true and must not be redone

- **`SolidId` already supports many solids in one mesh** — masses, roofs, `Wall` runs, `Slab`s and parts are each
  their own solid, and `validate` measures manifold, boundary loops and Euler **per solid**.
- **`Roof`, `Wall` and `Slab` are already volumes.** The change is confined to the mass, which is where the shell's
  complexity lived.
- **`fill::ring_fill(outer, inner)` is tested and exact** (84/84, 92/92, 92/92; a non-convex loop is refused). It is
  what fills a ring-shaped wall's top and bottom — the volume version needs it more, not less.
- **The identity channel names faces per solid.** A wall volume's faces need names just like a shell's did
  (`OuterWall`, `InnerWall`, `WallTop`, and for a courtyard `VoidWall`, `VoidInner`, `VoidTop`, `VoidUnderside`,
  `VoidFloor` — four of those already exist on `main`).

## The order this must go in, and why

1. **`SolidId` gains parts** (`MassWall(MassId)` / `MassPlate(MassId)`, or one kind with a part). This is the
   compiler-enforced audit: every `match` on `SolidId` in `building` and `gallery` will point at the site it needs.
   Nothing else can be done first, because every subsequent measurement is per solid.
2. **`expand_mass` becomes two emitters**: a wall ring (outer loop, inner loop, top ring, bottom ring) and a plate
   box. The shell's `WallFace` / `EdgeNode` / `resolve_holes` machinery is kept **only** where it cuts openings into
   a wall volume — a wall with a window is an ordinary hole problem, not a shell's.
3. **Openings cut the wall volume.** A window is a hole through the ring; a courtyard wall with a window into the
   courtyard becomes a hole through the *inner* ring, which the void loop makes easy.
4. **`validate` keeps its per-solid answers** and its three courtyard faults (`VoidNotInside`, `VoidNotConvex`,
   `VoidOverlaps`) unchanged; `Problem::VoidNotDrawn` is deleted in the commit that draws courtyards.

## Acceptance, in the order it must be measured

1. **The nine scene hashes do not move** — a mass with no void must expand to **exactly** the bytes it does today, or
   the volume change is not a refactor but a new picture. The hashes are in the design document's baseline table.
2. **A plain mass measures two closed solids** (`manifold`, `loops 0`, `euler 2` each) instead of one shell.
3. **A courtyard mass measures `euler 0`** on its wall ring, with `loops 0`, and a plate box under it.
4. **`patio_probe`'s three faults still report** and the good courtyard reports none.

## Traps this project has already paid for

- `Solid::point` interns nothing: a shared corner called twice becomes two vertices and the seam never closes. In the
  volume model every solid owns its corners, so this stops being a hazard **between** solids and stays one **within**
  a solid.
- `cargo fmt` writes the main checkout's tree; run it on the branch.
- Scripted multi-line replacements on Rust **silently match nothing** when the file is CRLF; use `edit`.
- Every slice of this build has needed **one instrument that shows the thing** (a census, a debug dump, a pixel
  count). Do not flip windings by guess — dump the unmatched edges first; that is how the last slice found that every
  open edge lay on the plate plane.
- `OffsetInverted` and `MiterTooLong` are written for **inward** offsets; an outward offset needs the sign-aware
  forms now on this branch's history (a negative wall, `MITER_LIMIT * wall.abs()`, and a side-aware inversion test).

## Status log

- **2026-10-27** — handoff written; the shell slice (`feature/building-patio3`) is deleted, so this plan starts from
  `main` alone. Nothing implemented.

## The blast radius, measured before starting (2026-10-27)

`SolidId::Mass` is referenced **39 times**: **3 in the library** (`mesh.rs` once, `validate.rs` twice) and **36 across
16 probes**. So the audit the compiler enforces is small where it matters and mechanical where it does not — the
refactor is not the monster the phrase "rewrite `expand_mass`" suggests.

**And the recorded volumes survive it.** A shell around a mass closes on: the outer walls, the wall tops, the inner
walls, the floor and the soffit — so the region it encloses is exactly the **wall material plus the plate material**,
which is what the two volumes hold together. Every volume in the acceptance record (the model building scale, the
slab arithmetic, the floor area) is therefore still the number to check against, and a volume that does not match is
a mistake rather than a new normal.

**The shape of the change, then:** `SolidId::Mass(MassId)` stays as **the wall ring** (the name keeps its meaning:
the mass's walls), a **`MassPlate(MassId)`** is added for the plate, and the emitter produces both. The 36 probe
sites need no edit at all, and the plate gets its own probes.

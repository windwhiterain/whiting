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

## Two corrections before any code (2026-10-27)

**1. The recorded volumes do NOT all survive, and the last note was wrong about it.** The shell's enclosed region is
the wall material *plus* the plate material, so `solid_signed_volume(SolidId::Mass(_))` reads the **sum** today. Once
`Mass` is the wall ring alone, that call reads **only the walls**, and the plate's volume moves to `MassPlate`. The
arithmetic is unchanged and the *sum* must still match every recorded number — but each probe has to say which solid
it is measuring. Checking the sum is the acceptance; checking `Mass` alone is not.

**2. The crux of the volume model: volumes may overlap; shells may not.** The reason a volume model makes the
courtyard trivial is not that volumes are nicer — it is that **two closed solids are allowed to intersect** and no
shared edges are created. A shell has to partition space exactly; anything that stands on it, or in the middle of it,
turns one of its edges into a three-surface edge.

That gives the wall its base: **the wall ring sinks into the plate.** Its bottom ring sits at the plate's own bottom
plane (`elevation − slab`), the plate box spans the whole footprint under it, and the two volumes overlap through the
wall's footprint. The floor seen from inside is the plate's top face, which is why nothing has to be split at the
wall — and the same trick is what lets a courtyard's wall ring be its own volume later instead of a face of the
same shell.

**The slice, then:** `SolidId::MassPlate(MassId)` for the plate box; `expand_mass` keeps the outer walls and the wall
top, runs the inner walls down to the plate's bottom plane, closes the ring with a **bottom ring** (`ring_fill`
between the outline and the inner loop), drops the floor disc and the soffit, and emits the plate as its own box.
**The pictures may move slightly** — a plate box adds its own side faces and the ring's inner faces now begin a slab
lower — so the nine hashes are re-recorded with that reason rather than held as unchanged: a volume change is a
geometry change, and pretending otherwise would make the baseline a lie.

## The simplification that makes this small: the plate is already a `Slab` (2026-10-27)

**No new ids are needed for the plate.** The crate already has a volume plate as a first-class element:

- `derive::floor_slab(mass, id, elevation, thickness) -> Option<Slab>` derives one from a mass;
- `Slab` carries its own `SlabSurface`, and its solid is `SolidId::Slab(SlabId)`;
- **two scenes already draw their plates that way** — `workshop` (its two storeys) and `authored` — so the path is
  not a plan, it is in use.

So the slice becomes:

1. **`expand_mass` keeps the wall ring and nothing else.** The inner walls run down to the plate's bottom plane
   (`elevation − slab`), the ring closes with a **bottom ring** between the outline and the inner loop, and the floor
   disc and the soffit are **dropped** — they belong to the plate now.
2. **The plate is authored as a `Slab`**, derived by `floor_slab` where a building is derived (`derive::starter`) and
   written into the documents that author masses by hand.
3. **The plate and the ring overlap** through the wall's footprint, which is the whole point of volumes: no shared
   edge, so no three-surface edge, and no split floor.

**No `Element` variant, no `SolidId` variant, no new surface kind** — the audit that looked like three sites of
trouble shrinks to zero, because the plate was already a volume and only the mass was pretending otherwise.

**What still has to be measured:** the sum of the ring's volume and the plate's volume against every recorded
number (the plate's own numbers come from `floor_slab`'s thickness), and the nine hashes re-recorded with the
reason — a mass with no floor disc renders differently at its inner base by one plate thickness.

## Where the refactor stands (2026-10-27, branch `feature/building-vol2`, worktree `.worktrees/building-vol2`)

Step 1 is written and builds: `expand_mass` runs the inner walls down to the plate bottom plane, closes the ring with
a `WallBase` annulus (`ring_fill` via a `ring` helper), and no longer emits `FloorTop` or `SlabSoffit`. Measured:
`census`, `floor_probe` and `slab_probe` exit 0; **`expand_probe` panics** because its expectations are the old shell's
— that is the step that has not landed yet, not a defect: **the documents carry no plate**, so a mass has walls and
no floor.

Step 2 is the rest: add the plate `Slab` wherever a mass is authored (`derive::starter` first, then `terrace.rs`,
`wing.rs`, and every probe document that builds a mass), then bring `expand_probe`'s numbers up to the ring-plus-plate
split, then re-record the nine hashes. **The volumes to check:** the old shell's `solid_signed_volume(Mass)` must
equal the new ring's volume **plus** the plate's.

## Where step 2 stands (2026-10-27, branch `feature/building-vol2`)

**The building crate is green**: all thirteen probes pass with the strip base, the plate in `derive::starter`, and
`expand_probe`/`validate_probe` brought up to the split. **The gallery is not**:

| scene | hash | reading |
| --- | --- | --- |
| `house` | `B8384583E741BE57` | unchanged — the plate's top is the plane the floor already was |
| `hall` | `A68F409B2DB9E9D9` | unchanged |
| `wing` | `2050549F7611B0E4` | unchanged |
| `yard` | `ECCEB09EE3FD0BD9` | unchanged |
| `hall_large` | `37E166401574C75E` | **moved** — the plate's side and the wall's outer face are coplanar, so the face
  fights for the same pixels |
| `terrace` | `84EB54DEF1190BFE` | **moved**, same reason, three times over |
| `shed`, `workshop`, `identity` | — | **render failed** |

and `scene_identity_probe` says the channel is **not** two-way for every scene.

**Read that as three separate jobs, not one defect:** (1) the coplanar fight — the plate should be inset from the
wall's outer face by a millimetre or the wall should overhang it, so the two do not share a plane; (2) the three
failing scenes, which need their render error read first; (3) the identity probe, whose failure is probably the same
cause as (2) — a document that no longer expands the way its table was built from.

## The three failing scenes are rejected documents, not renderer faults (2026-10-27)

`render -- shed` panics in the scene's own assertion, and the violation list is the whole story:

```text
the shed's own document must be clean, and it is not:
  SolidOpen { solid: Mass(MassId(0)), loops: 2 },  SolidNonManifold { solid: Mass(MassId(0)) },
  SolidNotASphere { solid: Mass(MassId(0)), euler: -8 },
  SolidOpen { solid: Part(0), loops: 2 }, SolidNotASphere { solid: Part(0), euler: 0 },
  SolidOpen { solid: Part(1), loops: 2 }, SolidNotASphere { solid: Part(1), euler: 0 }
```

**So the mass is no longer watertight once its floor and soffit are gone** — and the two parts fail the same way.
The parts are the clue: a part's solid is a template's own mass, expanded through the same `expand_mass`, and it has
**no plate at all**, so if a part's mass is not closed by the ring alone then the ring alone is not enough in general.

**Hypothesis, to be tested before any fix** (it is a hypothesis, not a finding): the ring's base strip closes a
mass whose outline is a simple loop, and the shed's mass and its parts are the documents that exercise something the
strip does not cover — an opening whose reveal was welded to the **floor disc**, or a mass whose outline carries
per-node elevations so the base strip is not planar. The first measurement to make is `SolidOpen`'s boundary edges
with the `PATIO_DEBUG`-style dump (it is not on this branch; the trick is recorded in the patio handoff's history):
the edges will name the hole instead of guessing at it.

## The base strip's one defect, measured (2026-10-27)

The base strip closes a mass with **no openings**, and leaves one open with them. The `SHED_DEBUG` dump named it:
every open edge sat at the floor plane on a **reveal's own sill**, e.g.

```text
OPEN [0.0, 0.0, 0.05] -> [1.1, 0.0, 0.05]
OPEN [0.01, 0.0, 0.010000002] -> [1.09, 0.0, 0.01]
```

**The strip is built from the raw outline arrays (`outside_base`, `inside_floor`), and the walls are built from the
cut loops (`outside_loop`/`soffit_points`, `inner_loop`/`floor_points`), which carry the stations an opening's reveal
added.** So the strip's edges do not pair with the walls' bottoms wherever a reveal reaches the floor, and each such
reveal leaves its sill's edge open. The two cut loops have the **same length** (the stations come from the same
cuts), so the fix is mechanical:

```text
for edge in 0..soffit_points.len() {
    let next = (edge + 1) % soffit_points.len();
    solid.quad([soffit_points[next], soffit_points[edge], floor_points[edge], floor_points[next]],
               Element::Mass(id, MassSurface::WallBase));
}
```

That replacement was written and lost to a bad line slice; the worktree was reverted to `7c88e2a`, so it is one edit
away. Two further things the same measurement says: the strip is **warped** when the outer and inner loops sit at
different heights (`FaceNotPlanar` with `departure = slab/2` on the shed), which is why the inner loop must be at the
**same** elevation as the outer — `inside_floor` was moved down for one commit and that is what produced the warped
quads; and the plate must be **inset** from the wall's outer face, or the plate's side and the wall's face fight for
the same pixels (measured on `hall_large` and `terrace`).

## The strip fix, its measurement, and what is still open (2026-10-27)

The strip now runs over the **cut** loops (`soffit_points`/`floor_points`), which is what the dump said it needed.
Measured on the shed: `SolidNonManifold` **gone**, Euler `-8 → -6`, and one `FaceNotPlanar { departure: 0.19 }`
left. **The mass and both parts are still `loops: 2`.**

So the reveal sills pair now, and two boundary loops remain — on the mass **and on both parts**. The parts failing the
same way is the strongest clue left: a part is a template's own mass, expanded by the same function, and nothing about
a part is special. Two loops with Euler −6 on all three says a **shared, systematic** pair of holes rather than a
shed-only shape.

The next instrument is the same one that got this far: `SHED_DEBUG=1` is in `expand_mass` on this branch and prints
the unmatched edges, so the next round reads **which two loops** and stops guessing. A `departure: 0.19` face points at
a warped quad — likely the base strip again where the two cut loops disagree in **order** rather than length, which the
same dump would show as a crossing pair.

## The 73 edges, read (2026-10-27)

`SHED_DEBUG=1` on the shed now prints **73 unmatched edges, every one at `y = 0`**, and they trace small rectangles
around each opening — `0..1.1 × 0..0.05` for one, `0..1.45 × -0.7..0` for another. So the open edges are the
**strips of wall under each opening**, not the ring's own base: the base strip spans the loop once, and the openings
need it to follow their stations.

**The cheapest next measurement is two numbers, not a fix:** print `soffit_points.len()` and `floor_points.len()`. The
strip pairs node for node because the two cut loops were *assumed* equal, and if the opening's stations land on one
loop only, the lengths differ and every station beyond the shorter loop is off by one — which is exactly a small
rectangle left open at every opening.

**An instrument improvement that did not land:** attributing each open edge to its face's `element` needs a
multi-line edit, which silently does nothing on these CRLF files through a script; write it with the `edit` tool, one
anchor, or not at all.

## The lengths are equal, so the pairing assumption was not the defect (2026-10-27)

`SHED_DEBUG=1` now prints the two loops beside the station count:

```text
LOOPS outer 10 inner 10 stations 6     (the shed's mass, with its six opening stations)
LOOPS outer 4 inner 4 stations 0       (each part, with no openings at all)
```

**Equal lengths, so the strip pairs node for node and the "one loop has the stations" hypothesis is dead.** And the
decisive detail is the parts: `4/4` nodes, **no openings**, and they still report `SolidOpen { loops: 2 }` with
`euler 0`. A plain four-node mass whose base strip is four quads does not close, so the defect is in the base strip's
**own** geometry, not in its interaction with openings.

Read the numbers as a shape: `euler 0` with two boundary loops is a **torus that is open at one seam** — which is
what a ring looks like when one of its two end strips is missing. The base strip exists; the question is whether it is
the **only** end. The wall top is the other, and it is emitted only where the walls are: a part whose mass is a
closed loop of wall around a hole (a frame) would need both, and the top is emitted per edge — check whether the part
mass's top ring is emitted at all (`WallTop` faces) before assuming anything about the base.

## The dump now names the mass, and it corrects the last note (2026-10-27)

`SHED_DEBUG=1` prints the mass id, and the reading is one line: **all 72 open edges belong to `MassId(0)`; the parts
have none.** So the parts' `SolidOpen { loops: 2 }` is **not** the mass's defect and not the base strip's: a part is
expanded by the template machinery, which is a different path that `expand_mass` does not own, and its closure is its
own question.

**That retires the round-114 inference** (which read the parts as decisive evidence about the base strip). The correct
statement: the mass's open edges are the strips under its six openings — 72 of them, all at `y = 0` — and the parts
fail separately. Two defects, not one.

**For the mass**, the next print that pays is the face's `element` for each open edge (write it with the `edit` tool:
a multi-line scripted replace does nothing on these files), because 72 edges under six openings is consistent with
either the strip's quads being emitted with the wrong winding at each station pair, or the reveals' sills landing on
the strip's corners. The element will say which.

## The open edges are the walls' own bottoms (2026-10-27)

With the owning element printed, the answer is not the base strip's own shape: **all 72 open edges are owned by
`InnerWall` and `OuterWall` faces.** The walls' bottom edges are unpaired, so the base strip is not meeting them at
all — it is a separate ring floating in the same plane.

```text
OPEN mass 0 owner Some(Mass(MassId(0), InnerWall { edge: 0 })) [0.01, 0.0, 0.010000002] -> [1.09, 0.0, 0.01]
OPEN mass 0 owner Some(Mass(MassId(0), OuterWall { edge: 3 })) [0.0, 0.0, 0.0]        -> [0.0, 0.0, 0.05]
```

**Why it cannot meet them as written.** The walls are emitted from the raw corner arrays (`outside_base`,
`inside_floor`) — one quad an edge, subdivided only at the stations a hole adds — while the strip is emitted from the
**cut loops** (`soffit_points`, `floor_points`). Those two descriptions agree in *order* but not necessarily in *ids*:
`split_loop` rebuilds the loop's point list, and `Solid::point` interns nothing, so a corner can exist twice under two
ids and the edge never pairs. The floor disc used to hide this because it was the *last* face in the solid and its
boundary was compared against nothing.

**The next measurement is two ids, not a fix:** print `outside_base[0]`, `soffit_points[0]`, `inside_floor[0]`,
`floor_points[0]`. If the pairs differ, the strip must be built from the **same arrays the walls use**, subdivided at
the same stations — i.e. the wall emission and the base strip have to share one description of the loop, which is the
single change this whole slice has been circling.

## The ids are shared, so the strip is not missing points — it is missing faces (2026-10-27)

The four ids compare equal at the first corner of every mass:

```text
LOOPS outer 10 inner 10 stations 6 | ids outer_base 0 soffit 0 inner_floor 2 floor 2
```

and the station count is 6 with a 10-node cut loop, so **both the strip and the walls are subdivided at the same
stations and share the same vertices.** The round-116 hypothesis — two descriptions with different ids — is dead, and
the ids are the third hypothesis this slice has killed by measurement rather than argued away.

What is left is one number: **how many `WallBase` faces exist** on the shed's mass. If it is 10 (the cut loop's
length), the strip is emitted and its edges are somehow oriented the same way as the walls' rather than opposite, and
the fix is one winding. If it is 4 (the outline's length), the strip is built from the raw loop after all and the
`soffit_points` binding is not what the emission reads. If it is **0**, the strip is not emitted and the reason is in
the code path between the loop and the quad — which is where the `let _ = walked; walked = true;` lines I left in while
carving deserve a hard look: they were scaffolding, and scaffolding is exactly what can silently swallow an emission.

## The strip turned round closes the mass (2026-10-27)

A quad's base strip (`soffit[edge] → soffit[next] → floor[next] → floor[edge]`) makes the mass closed:

| reading | before the flip | after |
| --- | --- | --- |
| `SolidOpen` | `loops: 2` | **gone** |
| `SolidNonManifold` | present | **gone** |
| `SolidNotASphere` | `euler: -6` | `euler: -6` (still) |
| `FaceNotPlanar` | 1 × 0.19 | 1 × 0.19 |

**Why the flip is right, and how it was reached without guessing:** `BASES 20 for a loop of 10` said the strip was
emitted with the right count, the four ids said it shared vertices with the walls, and the walls' own bottom edge
runs `base[next] → base[edge]` — so the strip had to run the **other** way to pair. Three measurements, one flip.

**What remains is not closure but counting.** `euler -6` on a closed surface means the solid has four handles, and the
shed has three openings, so one of them is being counted twice or the reveals add a handle of their own. The parts
still report `euler 0` — they are expanded by the template path, and their own closure is a separate question still
open. A single `FaceNotPlanar { departure: 0.19 }` remains and is the strip's own warped quad wherever the two loops'
stations disagree in a way the lengths do not show.

## `SolidNotASphere` is a shell-era invariant, and the volume model must break it (2026-10-27)

The readings after the flip are `euler -6` on the mass and `euler 0` on each part, and the shed's own assertion calls
all three faults — but **`euler 0` is the correct answer for a wall ring**: a ring of wall is a torus, and a torus has
`2 - 2g = 0`. The shell-era solid was a *box* (euler 2) because its inner and outer surfaces were one closed surface
with the plate, and the check named "the solid is a sphere" was true of that box.

**A wall volume is not a sphere, ever.** So `SolidNotASphere` cannot remain a fault in the volume model without
flagging every wall in the project. What it has to become is a **reading with an expectation**:

*`euler = 2 - 2 × <handles>`*, where a plain ring of four walls has **one** handle (the wall's own tube) and each
opening through the ring adds another. The number is then checkable rather than merely spherical, and the acceptance
in `scenes` becomes "the euler characteristic is what this document's openings imply" — which is exactly the kind of
statement the crate prefers: a fact about the document, computed.

**And the mass's `-6` says one more thing:** four handles where a plain ring has one, so the strip is pairing its
**depth edges** (the `soffit[i] → floor[i]` edges) into extra loops at the stations. That is the remaining defect,
and it is now a question of *which* handles, not *whether* the shell is closed.

## `euler -6` is the right number, and the arithmetic proves it (2026-10-27)

The mass's readings — six stations, which is **three openings** — and the shape of a wall ring give the expected
Euler characteristic exactly:

```text
a ring of wall               1 handle   (the wall's own tube)
three openings through it    3 handles  (one each)
euler = 2 - 2 * (1 + 3) = -6
```

and the measured value is **−6**. The parts measure **0**, which is `2 - 2 * 1`: a ring of wall with nothing cut
through it. **So nothing is wrong with the mass's topology.** The only thing wrong is the check that called it names:
`SolidNotASphere` expects 2, and no wall volume will ever give 2.

That converts the remaining work from geometry to accounting:

1. **`SolidNotASphere` must become a counting check** — `euler = 2 - 2 * (1 + openings)` for a mass, `2 - 2` for a
   slab, `2` for a box — which is a change to `validate` and to the scenes' own assertions;
2. **one `FaceNotPlanar { departure: 0.19 }`** is the only geometric fault left, and it is the strip's quad where the
   two cut loops' stations land at different distances along the corner — a warped quad, not a hole;
3. **re-record the nine hashes** once the three scenes stop being rejected.

This is the first time in the slice that a measured violation turned out to be **the instrument's** fault rather than
the geometry's, and the arithmetic is what settled it: `-6` is not "bad", it is `1 + 3` handles.

## The ring's bottom is level now, and the only check left is the shell-era one (2026-10-27)

The outer base was one plate thickness below the inner, which made the base strip a **slanted** band — a warped quad
wherever an opening's stations land unevenly. With both at the wall's own elevation:

| fault | before | after |
| --- | --- | --- |
| `FaceNotPlanar` | 1 × 0.19 | 3 × (0.157, 0.01, 0.025) |
| `SolidOpen` / `SolidNonManifold` | gone | gone |
| `SolidNotASphere` | `-6` / `0` / `0` | `-6` / `0` / `0` |

The large warp fell and small ones appeared: the band is level now, and what remains is at the reveals' own stations.
**The three scenes cannot render until `SolidNotASphere` is replaced**, because the numbers it rejects are the right
ones (`-6 = 2 - 2(1 + 3)` handles on the mass; `0 = 2 - 2` on each part). That replacement is the next slice and it is
a crate change, not a scene tweak: `validate` must expect `2 - 2 × (1 + openings)` for a mass and `2 - 2` for a slab,
and every scene's own assertion follows it.

## Topology is clean; what is left is warped faces (2026-10-27)

The Euler check was already counting handles — `2 - 2 x holes` — and the volume model adds exactly **one**: a mass
(and a part, whose solid is a template's own mass) is a **closed tube** now, so it is a torus before any opening is
cut. With `ring = 1` for `Mass` and `Part`:

| scene | before | after |
| --- | --- | --- |
| `shed` | `SolidNotASphere` x3, `FaceNotPlanar` x3 | **only `FaceNotPlanar` x3** |
| `workshop` | `SolidOpen`, `SolidNonManifold`, `SolidNotASphere` x? | **only `FaceNotPlanar` x6** |

So **no solid is open, non-manifold or wrongly counted any more.** The remaining faults are small warps: five at
`departure = 0.01` (one centimetre) and one larger on each of the shed (`0.157`) and the workshop (`0.268`).

**The 0.01 ones are almost certainly the reveals** — a reveal span whose width is the wall thickness, cut at stations
that differ by a centimetre between the two faces. **The large ones are the base strip's quad at a station where the
outer and inner cuts are spaced differently**, which is the same class of problem one degree higher. Neither is a
closure fault, and both are now visible as "a quad is not planar" rather than as a hole.

## Six of nine scenes render on the branch, with new hashes (2026-10-27)

| scene | branch hash |
| --- | --- |
| `house` | `D6EDBADCC7BD6C3D` |
| `hall` | `E0C1F46C3E9035D5` |
| `hall_large` | `8983DF2105A40B9D` |
| `terrace` | `51862007E2F526A1` |
| `wing` | `9B7610210E8CC7C2` |
| `yard` | `F934F309B9B5A4EE` |
| `shed`, `workshop`, `identity` | **still rejected** — the warped faces of the last note |

**Every hash moved, and it should:** the plate is a new solid with its own side faces, and the wall ring's bottom is
now at the wall's own elevation instead of a plate thickness below. These are the numbers to record in the proposal's
baseline table **when the branch merges**, not before — a baseline that predicts the future is not a baseline.

**What remains is one class of fault:** `FaceNotPlanar`, five small (0.01) and one large per scene. The small ones are
the reveals — a reveal's width is the wall thickness, and its two ends are cut at stations that differ by a centimetre
between the inner and outer faces. The large ones are the base strip where the two loops' stations are spaced
differently. Both are "a quad is not planar", not "the surface is broken", which is why six scenes pass the closure and
topology checks.

## Every warped face is `WallBase`, and the cause is the stations' elevations (2026-10-27)

A one-line instrument in `validate` (`WARP {:?} departure {departure}` behind `SHED_DEBUG`) named them on the shed:

```text
WARP Mass(MassId(0), WallBase) departure 0.15704492
WARP Part { placement: 0, surface: Mass(MassId(0), WallBase) } departure 0.01
WARP Part { placement: 1, surface: Mass(MassId(0), WallBase) } departure 0.025000095
```

All three are the base strip — and the strip's corners are all supposed to lie at the wall's own elevation, which is
what made the last fix work. **So the strip's warped corner is not one the strip computes: it is a station's point.**
`split_loop` returns the loop's points with each cut station interpolated in, and a station carries the elevation of
the **hole** it belongs to — a door's jamb is at the door's sill, not at the wall's base. The strip then takes four
corners that are three at the base and one at a sill, and the quad is warped by exactly the sill's height.

**The next step is therefore not in the strip but in `split_loop`:** a station that the **base** reads should carry the
base's elevation, and the same station read by a **reveal** should carry the reveal's. That is the "one description,
two readers" problem this whole slice has been circling, now with a name and a number: three warps, one a sill's
height, two the tops of jambs.

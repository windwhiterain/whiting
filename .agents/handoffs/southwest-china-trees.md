# Handoff: ten trees of the Hengduan

Status: **in implementation**. Design record:
[proposals/southwest-china-trees.md](../proposals/southwest-china-trees.md). Worktree
`.worktrees/swchina`, branch `feature/southwest-china-trees`, based on `main` at `e8c7b44`.

## What is done

### Arc 1 — the picture can carry a tree (commit `e7cbdf2`, on top of `799af89`)

Vegetation crate:

- `LeafField::cluster` and `LeafField::fan` — `n` leaves at one station, `1/n` of a turn apart,
  fanned through `fan` radians of extra pitch. The fascicle, the palmate leaflet set, the bract pair
  and the flower truss are one dial. Identity: `cluster = 1`, `fan = 0`.
- `Detail::spacing` — rings are `max(stations, ceil(arc_length / spacing))`. `DEFAULT.spacing` is
  0.5 m, which is coarser than every spine the crate's own species make, so every existing plant is
  drawn exactly as before and a 25 m trunk gets the rings its length asks for.
- `decorator::scatter(plant, generation, template, station, count)` — a seat on every live node of a
  generation, count and slot drawn from the node's own **scatter stream** (the node's path plus
  `u32::MAX`), so a seat never draws from the stream a leaf field reads.
- `Species::decorators: Vec<decorator::Template>` — the ornaments a species carries, indexed by a
  `Seat::template`.

Gallery:

- `Skin`, `Palette` (wood / one per leaf template / one per decorator template) on `Planting`,
  defaulting to the two greens every vegetation scene has had.
- `Lighting` — key, fill, floor skin, `Atmosphere`, sky radiance — defaulting to the module constants
  they replace. `plant_scene_lit` is the entry that takes one.
- Every mesh is now keyed by **(geometry, skin)**: one wood mesh per distinct wood skin, one blade
  mesh per distinct (leaf template, skin), one decorator mesh per distinct (decorator template,
  skin) that a seat names. `templates.first()` is gone; every leaf group draws the mesh its own
  `template` index names.
- **Verified byte-identical**: `render -- bamboo out/x` hashes the same before and after
  (`1D533E76…`). The four vegetation probes report 0 failed invariants.

## What is next, in order

1. **Arc 2 — 云南松 *Pinus yunnanensis* at hero grade.** The note is
   [investigate/trees/pinus_yunnanensis.md](../investigate/trees/pinus_yunnanensis.md) and the
   photographs are in `out/refs/pinus_yunnanensis/`. It needs, from the note's modelling table: a
   14 m trunk with a clean bole to ~6 m and a flare at the ground; whorled first-order branches
   (needs `Generation.cluster`, added but not yet used by any species); a needle template 20 cm by
   1.1 mm; a leaf field with `cluster = 3` and a fan, on the outer two thirds of each twig, plus a
   second field for the upright terminal brush; cones by `decorator::scatter` at the twig tips; a
   palette of grey-green needles and fissured grey-brown bark. Render it, look at it against
   `01-habit.jpg` and `04-branch.jpg`, and iterate.
2. Arcs 3–8 as tabled in the proposal.

## Commands

```
# build and render, from the worktree, with its own target
CARGO_TARGET_DIR=<worktree>/target CARGO_INCREMENTAL=0 cargo build -j 3 -p whiting-device-rasterize --example render
cargo run -q -p whiting-device-rasterize --example render -- <scene> <out-stem>
```

A new scene needs two registrations (see
[investigate/vegetation-render-path.md](../investigate/vegetation-render-path.md)): a `"name" => …`
arm in `Gallery::build` (crates/gallery/src/lib.rs) and a `Naming` row in crates/gallery/src/locate.rs.
**Do not add it to `Gallery::names()`** — that is the list the pass-count test holds itself to.

## Verification recipe

- The four vegetation probes: `plant_probe`, `leaf_probe`, `decorator_probe`, `species_probe`; each
  prints `0 of N invariant(s) failed`.
- Render identity: `out/arc0-bamboo.png` is the pre-Arc-1 baseline; any change to the builder must
  re-render `bamboo` to the same SHA-256 until a change is *meant* to move it.
- H3 (crown depth) is not yet a probe. It becomes one in arc 2; until then it is judged by eye
  against the reference photographs and the judgement is written into the species' note.

## Open questions carried from the proposal

1. Whether H3's luminance ratio is set by arc 2's pine or measured first.
2. Bark as texture rather than as section form — deferred to arc 4's oak.
3. Two seasons (kapok flowering leafless) — deferred to arc 6.
4. Whether the rasteriser's shadow map survives a crown of forty thousand leaves — measured in arc 2.

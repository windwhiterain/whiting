# Handoff: ten trees of the Hengduan

Status: **in implementation**, arcs 1–6 landed on `feature/southwest-china-trees`, arc 7 opened.
Design record: [proposals/southwest-china-trees.md](../proposals/southwest-china-trees.md) — read it
first; it carries the bar (H1–H5, E1–E4), the defect list, the gap list §4.1–§4.15 and the decisions
D1–D10. Species facts: [investigate/trees/](../investigate/trees/), one note per species, every
number sourced; photographs in `out/refs/<slug>/`.

## Where each arc stands (2026-10-19)

| arc | state | evidence |
| --- | --- | --- |
| 1 picture can carry a tree | **done** | every template drawn, `Palette` per planting, `Lighting` per scene, haze a dial; bamboo byte-identical across the change |
| 2 云南松 at hero grade | **done, provisional on H1's ten-sheet** | plates `out/arc2-pine24.png` (tree), `arc2-final-yunnan_pine_bough.png`, base pending re-render after the shadow-page fix; H3 measured 0.628 / 0.769 against bars 0.25 / 0.80 |
| 3 spire conifers | **first pass merged, second pass in flight** (`feature/sw-spire2`) | `out/sw-spire/`; second pass fixes the spruce bough framing, the fir's cone cue, the plume dead-give, raking keys, base plates |
| 4 川滇高山栎 | **first pass merged, second pass in flight** (`feature/sw-oak2`) | `out/sw-oak/`; second pass densifies the crown with sprig templates and adds the base plate |
| 5 flagships | **done to second pass** | `out/sw-flag/`; dove bracts as two templates, rhododendron rosettes and truss; base plates delivered |
| 6 valley and fern | **first pass merged, second pass in flight** (`feature/sw-valley2`) | `out/sw-valley/`; second pass makes the kapok's flower a sprig corolla via `scatter_in` and opens the fern's crown |
| 7 the site | **opened** | `southwest_site.rs`, `site_montane`: one slope law, three heroes, eleven companions at `Foliage::Stand` and `Detail` COMPANION, a standing stone for E2; first two frames in `out/arc7-montane{1,2}.png` |
| 8 second wave | not started | notes exist for all seven species |

## Merged system changes, in order

- `LeafField::cluster` / `fan` / `cluster_size`; `Generation::cluster`; `Joint::station`
  (`799af89`, `a2f6de1`) — the fascicle, the whorl, the unequal bract pair, the crown profile.
- `Detail::spacing` (`799af89`) — rings by arc length; a trunk is not a hexagon.
- `decorator::scatter`, `Species::decorators`, `Node::set_section` (`799af89`).
- `Skin` / `Palette` / `Lighting` / `plant_scene_lit`, meshes keyed by (geometry, skin),
  `templates.first()` gone (`e7cbdf2`); `Lighting::key_distance` split from `key_reach`,
  `Lighting::haze` (`225f0d4`).
- `Foliage` LODs: `Needle`, `Spray` (a comb-toothed plume card), `Stand` (a companion's density)
  (`1906a95`, site commit) — D8/D10.
- `PLANTING_STRIDE` 4096 → 65536; `Planting::detail` (`5f5db28`).
- `LeafTemplate::sprig` and `shape::Loft::{moved, joined}` (`b717e98`) — several blades, one instance.
- creased wood normals, `WOOD_CREASE = 0.61` (`sw-crease` merge) — a fluted section reads as facets.
- foliage shadow bias: `shadow_bias_scale` 0.05 on leaves, 1.0 on wood; pine `key_extent` 8
  (`sw-shadow` merge) — leaves shadow leaves.
- emit hoist: per-material bundles, label hash tables, point positions resolved once
  (`sw-emit` merge) — the pine's scene compile 215 s → 22 s.
- shadow page orthographic **half**-width (`sw-baseblack` merge) — the floor no longer occludes the
  bole's own foot; bamboo moved 0.81 % of pixels, all false shadow, recorded in that commit.
- `scatter_in` with a parent-station band; `decorator::Template::sprig`; `shape::Sweep` parts
  (`sw-scatter` merge).

## Instruments

- `audit/frame_stats.py <frame> <authored_id> --crown` — coverage, H3 interior share (bar 0.25),
  H3 interior over lit p90 (bar 0.80), luminance percentiles. The crown box comes from the identity
  mask; the bar's derivation is proposal §8 question 1.
- `audit/silhouette.py <authored_id> <out>` — the H1 cut-out.
- `audit/sheet.py <out> <name>=<silhouette> …` — the H1 contact sheet; first sheet
  `out/h1-sheet1.png` with six species, and it already named the spruce/fir confusion pair.
- The canary: `render -- bamboo` must equal the CURRENT baseline. The baseline moved twice on
  purpose — `arc0-bamboo.png` before the shadow-bias merge, `328B509A…` after it, and the shadow-page
  fix moved it again by measured false-shadow removal. Re-baseline with a stated reason, never
  silently; the current one is in `out/sw-scatter-final-bamboo.png`.

## Commands

```
# from the worktree, with its own target; FOREGROUND — a background job ignores workdir here
CARGO_TARGET_DIR=<wt>/target CARGO_INCREMENTAL=0 cargo build -j 3 -p whiting-device-rasterize --example render
cargo run -q -p whiting-device-rasterize --example render -- <scene> <out-stem>
```

A new scene needs a `"name" => …` arm in `Gallery::build` (crates/gallery/src/lib.rs) and a `Naming`
row in crates/gallery/src/locate.rs. **Never `Gallery::names()`** — `clustered_lights` pins pass
counts over that list.

## Costs to plan around

- A scene compile is ~20 µs per instance row after the emit hoist; the raster is fill-rate bound at
  ~200 kpx/s of shaded foliage. Budget 90 000 rows and 90 s per plate.
- A base plate needs its own low key on the lens's side: a tree's crown shadows its own foot, and a
  key above ~35° puts the flare in that shadow.
- `key_reach` must exceed the floor's diagonal or a light-cluster boundary draws a straight edge
  across the ground; `key_distance` is where the irradiance is stated.

## Open, in order

1. The base-black fix landed after the pine's base plate was last rendered: re-render
   `yunnan_pine_base` and judge H4 on it.
2. H1's ten-silhouette sheet, once arcs 3–6 second passes land.
3. Site art: the slope needs a valley and a ridge that read (iteration 2 in flight), mist that lies
   in the gully (no volumetric channel exists — a crate gap, unassigned), and companions in three
   depth layers.
4. E4 (the two backends) is unverified-with-reason: no Cycles binary on this machine.
5. The nine valley limitations are decided in proposal §4.14; three are deferred and will return.

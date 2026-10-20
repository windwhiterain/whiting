# What every gallery scene costs to build

**Question answered:** "PCG content generation" is the largest term in a plate's cost — but *which
scene*, and is `alpine_oak` really the slow one? No: it is fourth. The long scenes are the three
multi-planting **sites**, and their own split is in [The site's split](#the-sites-split-measured)
below.

**Measured:** one build per name, `--release`, on the 6-core `i5-12400F` the other notes use, with
the same co-tenant caveat. Probes: `crates/gallery/examples/scene_cost.rs` (the ranking) and
`crates/gallery/examples/site_cost_probe.rs` (a site's split), both print and assert nothing. 85
names, `Gallery::build` each, one pass; the whole sweep is 141.5 s of builds.

## The ranking

| | scene | build | what it is |
|---|---|---|---|
| 1 | `site_transect` | **30.7 s** | 918 plantings: 17 trees, 900 tufts, a stone |
| 2 | `site_subalpine` | **24.0 s** | two full-detail trees, companions, 600 tufts |
| 3 | `site_montane` | **19.7 s** | 3 full-detail trees, 11 companions, 900 tufts |
| 4 | `alpine_oak` | **9.67 s** | one oak, decomposed in [where-a-vegetation-plate-spends-its-build](where-a-vegetation-plate-spends-its-build.md) |
| 5 | `alpine_oak_base` | **9.66 s** | the same oak, a different frame |
| 6 | `site_valley` | 4.36 s | 700 tufts and a few trees |
| 7 | `lijian_spruce_base` | 2.81 s | |
| 8 | `evelyn_keteleeria` | 2.78 s | |
| 9 | `armand_pine_base` | 2.76 s | |
| 10 | `evelyn_keteleeria_base` | 2.60 s | |
| 11 | `armand_pine` | 2.57 s | |
| 12 | `lijian_spruce` | 2.50 s | |
| 13 | `dove_tree` | 2.22 s | |
| 14 | `dove_tree_base` | 2.09 s | |
| 15 | `yunnan_pine_base` | 2.05 s | |
| 16 | `yunnan_pine` | 2.02 s | |
| 17 | `southwest_birch_base` | 1.93 s | |
| 18 | `cangshan_fir_base` | 1.80 s | |
| 19 | `cangshan_fir` | 1.70 s | |
| 20 | `terraces` | 1.43 s | a landscape, not a plant |
| 21 | `qiong_bamboo_base` | 1.26 s | |
| 22 | `qiong_bamboo` | 1.22 s | |
| 23 | `southwest_birch` | 1.10 s | |
| 24 | `foldyard` | 1.08 s | a building scene |
| 25 | `kapok` | 0.86 s | |
| 26 | `landscape` | 0.84 s | |

The remaining 59 names are under 800 ms, and **48 of the 85 are under 100 ms**. Every building
scene is milliseconds: `hall` is 1.8 ms. So "content generation time" means **vegetation**, and
within vegetation it means **the full-detail trees**, not the scenery around them.

## Three readings

- **The sites are two to three times the oak.** They are the scenes an editor loop would actually
  be asked to re-render, and no note has decomposed one.
- **The same build is paid more than once across names.** `alpine_oak` and `alpine_oak_base` are the
  same tree at two frames and cost 9.67 s and 9.66 s; so do `yunnan_pine`/`_base`,
  `dove_tree`/`_base`, `armand_pine`/`_base`, `cangshan_fir`/`_base` and `qiong_bamboo`/`_base`.
  A `_base` plate is a second full generation, not a crop of the first.
- **A site's cost is not the sum of its plates.** The sites draw their trees at coarser dials than
  the plates do — `DISTANT` is `stations: 3, around: 6, spacing: 3.0` and `COMPANION` is
  `4 / 8 / 1.5`, against the oak plate's `8 / 24 / 0.45` — so the vertex count shrinks several-fold
  while the per-node work does not. The next section measures what that leaves.

## The site's split, measured

`site_cost_probe` builds `site_transect`'s 917 plantings three ways: the plantings themselves (each
species grown and dressed), their expansions (each element's wood and leaves), and the assembly
(the site's own build minus the two). One pass, release.

| phase | ms | share | what it is |
|---|---|---|---|
| **expansions** | **16 944** | **51 %** | 917 drawings; the seventeen trees are 16.2 s of it, the 900 tufts 0.72 s |
| **assembly** | **8 990** | **27 %** | mesh packing, instance rows, naming |
| **plantings** | **7 084** | **21 %** | grow and dress, per element |

Two readings, and the second is the one that matters:

- **The tufts are not the site.** Nine hundred ground-cover elements cost 0.72 s of expansions and
  of the order of half a second of plantings — some four per cent between them. The cost is the
  seventeen trees: twelve of them expand in 1.1–2.8 s each, and the oak alone is 2.82 s. Anything
  aimed at the ground cover is aimed at nothing.
- **The coarse dial buys almost nothing.** The oak in this site is drawn at `DISTANT` (3 rings, 6
  around) and expands in **2.82 s**; the same species as a plate at 8 rings and 24 around expands in
  **2.38 s** — a different seed, and ten times the vertices, for *less* time. So the expansion is
  not bound by the vertices at all: it is bound by the per-node work (a placed spine, a sweep and a
  loft per node), and the leaves do not respond to the dial in any case, because `Detail` reaches
  the wood's rings and nothing else. **A site's trees are not cheap because they are drawn coarse,
  and no dial will make them cheap.** The lever is the per-node constant the plate's own
  decomposition already pointed at (75–135 µs per node, five operations).

## What this changes

- **The target is the site, not the plate.** The oak's decomposition (trim 39 %, expand 28 %,
  document assembly 24 %) is still the content of the question, but it is paid per tree and, for
  the per-node half, paid in full however coarse the tree is drawn.
- **The sites are not in the lichen vertical.** Only `alpine_oak` and `hall` have stage programs, so
  nothing about a site is cached today, let alone incremental. A site is also the shape a
  per-planting boundary pays on: one tree moves, and per-stage cells over all plantings would
  re-run the whole site, where per-planting cells would cost one tree. Same conclusion as
  [an-incremental-scene-document](../proposals/an-incremental-scene-document.md), one level up.
- **Cross-name duplication is free to notice and not free to fix**: a process building two plates
  of one species generates the tree twice. Nothing caches across scenes today.

## What is not measured

- **The sites' documents.** A site's build is 20–30 s; its document's size, write time and compile
  time are unknown, and on the oak's scaling (481 MB, 2.6 s write, 5.74 s compile) a site's
  document is the next order of magnitude. Anything done to the build leaves the write as the
  largest step unless it is addressed with it.
- **Why the oak is four times the pine.** 9.67 s against 2.02 s is either a heavier tree or a
  heavier rule, and nothing here separates the two.
- **Where inside an expansion's per-node work the time goes.** The probe splits the site by element,
  not an element by its per-node calls; the plate's own decomposition names the candidates
  (a placed spine, a sweep, a loft, a leaf group) without separating them.

## See also

- [where-a-vegetation-plate-spends-its-build](where-a-vegetation-plate-spends-its-build.md) — the oak's own decomposition
- [the-cost-of-the-binary-form-of-a-scene-document](the-cost-of-the-binary-form-of-a-scene-document.md) — what happens after a build
- [an-incremental-scene-document](../proposals/an-incremental-scene-document.md) — why the boundaries after the build are the other half

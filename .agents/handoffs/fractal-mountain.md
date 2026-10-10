# Fractal mountain: the massif as a hierarchy, and the scene rebuilt on it

Branch `feature/fractal-mountain`, worktree `.worktrees/fractal-mountain`. Off `a85b9b7`.

## The ask

The demo scene was ugly. Rebuild it so the massif's **fractal and block-face** character reads, and
say what could improve this PCG system. Narrowed by superior to: the ridge lines must be able to be a
**network** topology, the mountain's shape must express **both network and tree** topology, and the
small-scale relief must come from **more authored skeleton lines**, not from a displacement pass
downstream (which would break R6's "raw output" and the planar-face guarantee).

## Where it stands

| | |
| --- | --- |
| `9ba413b` | **The corner rule.** A belt turns where two ends *weld*, not where an end is *nearer*. |
| `b53e694` | **The side rule.** A flank's side is declared by its rib, not measured from the floor. |
| `5bfae34` | **The massif is a hierarchy and the scene is rebuilt on it.** |
| stashed | The network face walk. Proven correct, rim weaving unfinished. |

The scene validates: 19 crests, 10 basins, `violations: 0`, `euler 1`, `1 boundary loop`,
`manifold true`, plan area within 0.02% of the rim's.

## The finding that unlocked it

`Sampling::path` chose where a belt changes line by measuring which end of the leaving line was
nearer in the plan to the point it was aiming at. That is right for exactly one shape — a fan, where
every crest runs away from the one summit — and it is what made a recorded design note claim that a
spur hung off a parent's flank "cannot be built, and the reason is structural". It can. Asking which
pair of ends **welds** instead, a chain with a spur born at its col expands to a closed manifold
surface. That note is retracted in `SPEC.align/mountain.md`, with what is actually true written in its
place: **a belt changes line only at a line's end**, so a junction is authored by splitting the parent
there.

Two more rules fell out, both from a hierarchy rather than from a fan, and both in the proposal:
`check_coverage` attributed a flank by measuring from a point between two floor nodes that nobody
authored, and a rib must land **exactly** on every junction or the stretch of crest beside it is faced
by nothing.

## What the scene is now

A **tree**: a spine running *through* the summit and bending as it falls, spurs born at its cols,
sub-spurs born at the spurs' cols. Three levels, so the same shape recurs at three sizes. The basins
are **walked out of the graph**, not written by hand — the first attempt wrote them by hand and got
five junctions that were not junctions.

The picture: a long lens at a low eye looking **across** the spine instead of a wide angle standing
inside the corridor; the sun raking so a facet reads as a difference from its neighbour; flat
per-facet normals so the joints are the block boundaries; the stone varying **per block** off the
authored id, which is the one per-face number that is constant across a block and steps only at its
edge. The ground is an alluvial apron keyed to the distance from the nearest range, and the sky's
cirrus is sampled on a cloud deck — elevation alone is a ring, azimuth alone is a wedge.

## To continue

1. **Finish the network.** `git stash pop` — the face walk is written and *proven correct*: with a tie
   ridge closing a ring it returns `face: 37 darts, 8 tip visits` and `face: 3 darts, 0 tip visits`,
   which is exactly the outside and the cirque. What fails is the **rim**: it is woven in basin order,
   which is right for a tree and wrong once a cycle lets a tip be reached from two directions. Weave it
   by walking the outer face's own boundary and inserting each mouth at its tip pair. The three bugs
   already paid for are recorded in
   [proposals/network-derivation.md](../proposals/network-derivation.md) — do not rediscover them.
2. **Facet quality.** Aspect median 6.0 with 856 of 1711 faces over 6, worst 1468, all the worst ones
   in one belt (`ValleyId(4) side Left segment 9`). That is a belt whose two boundaries are walked at
   very different rates — a rib slant, and the lever is the rib spacing against the crest sampling, not
   the geometry.
3. **The picture is pale.** Contrast is low and the whole frame leans to one hue. The tone map and the
   two lights are the places to look; the geometry is not.
4. **`half_width` is authored and never read** by the expansion, so a crest's flank has no width.
   `Starter` still produces a cone. Neither is a scene problem.
5. `WELD` says `1.0e-2` and `weld_key` implements `1/1024`. Both the walk and the derivation now read
   through it.

## Commands

```text
cargo run -p whiting-gallery --example alpine_probe      # validates, and says what it found
BASIN_TRACE=1 cargo run -p whiting-gallery --example alpine_probe   # the basins the walk derived
cargo run -p whiting-device-rasterize --example render -- landscape out/NAME
cargo run -p whiting-device-rasterize --example render -- landscape out/NAME --debug position
```

The doc header on `render.rs` says `-p whiting-device`; that is stale, the example lives in
`whiting-device-rasterize`.

Delete this file when the implementation is finished.

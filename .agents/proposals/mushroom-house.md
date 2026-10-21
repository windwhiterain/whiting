# 蘑菇房 — the Hani house under its straw cap

The region's eighth type, and the first whose **roof is the building**: a square rammed-earth room under a four-slope
straw cap that projects more than a metre past the wall on every side.

## The three facts of the type

| fact | how it is built |
| --- | --- |
| **the cap is a hip roof projecting on all four sides** | one `derive::hip_roof` on the room's own plan, `OVERHANG = 1.15` — the projection *is* the type |
| **the straw is thick** | `ROOF_THICKNESS = 0.42`, more than twice a tile roof's, and a covering of wide shallow bundles (`period 0.55`, `fill 0.34`) rather than courses of tile |
| **the wing is lower and its roof is a lean-to** | a wing mass attached to the room's own wall by `ops::attach_mass`, and `derive::lean_to` from the room's wall down over the wing |

The door is the middle of three bays on the long face, with a window either side; the room stands on a stone foot.

## Two measurements this type cost

- **A hip over a nearly square plan is not planar.** At `7.4 × 7.0` the ridge is almost no length, the four planes
  meet close enough to a point that one of them left its own plane by `1.2e-3 m` — `FaceNotPlanar` at a tolerance of
  `1e-3`, with the cap the only roof in the document. The room is `8.2 × 6.6` now, which is a fact about the
  geometry and not a preference.
- **A wing's station must be off a corner.** Attaching at `0.5` of a rectangle's perimeter lands exactly on a corner,
  and splitting an outline at a corner twice is `OutlineEdgeDegenerate`. The station is computed from the wall the
  wing belongs on: `(2·WIDTH + 3·DEPTH + DEPTH/2) / perimeter`.

## What it does not draw

- **the terraces**: a 蘑菇房 stands in a field of them and this scene's ground is flat;
- **the thatch's laps**: the straw is bound in courses whose laps would need a covering of its own.

Identity: **274 rows**, and twenty-four scenes in the gallery.
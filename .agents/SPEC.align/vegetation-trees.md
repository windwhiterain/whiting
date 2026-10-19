# SPEC.align — the vegetation system as the Hengduan trees left it

An amend, never merged: what the ten-species work (proposal
[proposals/southwest-china-trees.md](../proposals/southwest-china-trees.md), branch
`feature/southwest-china-trees`, checkpoint-merged to `main` at `bfa5c83`) settled about the
`vegetation` crate and the gallery's plant pictures. Each item is a law the code now states; the
proposal carries the measurements and the rejected alternatives.

## A plant is a rule, a seed, and the levels of detail it is drawn at

- A species carries its ornaments: `Species::decorators` is the list a `Seat`'s template index names,
  beside `templates` for blades. A cone, a knot, a corolla and a bract-head are all decorators or all
  leaves, and which is a decision recorded per species (proposal D5).
- A generation may put several children at one station: `Generation::cluster` is a whorl. A leaf
  field may put several leaves at one station: `LeafField::cluster` is a fascicle, a palmate leaf, a
  bract pair. One word, one meaning, two types.
- A cluster's members may differ in size: `LeafField::cluster_size`, a law over the member's rank
  fraction, identity a constant one.
- A cluster's members may fan: `LeafField::fan`, radians of extra pitch centred on the field's pitch.
- A shape function sees where its child hangs: `Joint::station`. A crown profile is a law over it.
- Rings are a spacing law with a floor: `Detail::spacing` with `Detail::stations`; a spine gets the
  finer of the two. A section keeps its authored corners: a wood solid samples at
  `max(detail.around, section.corners())`.
- A template may be a sprig: `LeafTemplate::sprig` and `decorator::Template::sprig` join several
  sweeps into one instanced mesh. Instancing pays per row, so a shoot of nine leaves is one row.
- A scatter may read a band of the parent's station: `decorator::scatter_in`. Fruit on the upper
  crown is a band, not a list of seats.
- A planting carries its own resolution (`Planting::detail`) and its own surfaces
  (`Planting::palette`): a hero and a companion on one ridge are one species at two dials.

## A picture of plants states its light and its air

- `Lighting` carries the key, the fill, the floor's skin, the sky and the grade's haze. A light's
  `distance` is where its irradiance is stated and its `reach` is where it stops; the two were one
  number and a tree thirty metres from its key was lit at three times its own figure.
- A key's reach must cover the floor's diagonal or the light is culled per cluster box and the box
  boundary draws a straight edge across the ground.
- A base plate needs its own low key on the lens's side: a crown shadows its own foot under the
  tree plate's light.
- Every mesh is keyed by (geometry, skin): two plantings that agree on a skin share a mesh, and two
  species in one frame cannot share a tint by accident.

## Foliage is a level of detail, and the frame says which one

- A blade narrower than a pixel at the frame's distance is dropped whole by a point-sampled
  rasteriser; a crown's mass is therefore a budget of blade AREA, and the LOD that pays it is a
  comb-toothed spray card at whole-tree distance while the bough plate draws the true needle. Each
  plate is judged at its own distance and neither serves the other's (proposal D8, D10).
- The scene compile pays per instance row; the emitter resolves a row's material, mesh and label in
  hash tables and a material's schema work once per material (215 s to 22 s on one pine).

## The bar, as measured

- H1 is a contact sheet of cut-out silhouettes (`audit/sheet.py`); eight of ten nameable at a glance
  passes, and the confusions are recorded with the sheet.
- H3 is two numbers over the plant's own pixels in the crown's identity box (`audit/frame_stats.py
  --crown`): interior share at least 0.25 on the tree plate, interior mean over the lit p90 at most
  0.80 on the bough plate, because at whole-tree distance a dense crown's interior is lit leaves and
  only the bough plate resolves self-shadowing.
- A leaf shadows a leaf only where the shadow page is fine enough: plant materials carry a
  `shadow_bias_scale` of 0.05 and wood 1.0, and the page's orthographic width is a HALF width
  (crates/scene/src/standard/math.rs), which once made a floor occlude the bole's own foot.

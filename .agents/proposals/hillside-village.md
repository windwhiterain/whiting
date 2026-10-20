# 山坡上的村寨 — the hillside village now stands on an eroded massif

The hillside scene was merged with its ground written as a closed-form formula, and the proposal said plainly what
that was: *the point of `HeightField` is that the reader is the one a real field would be read by, not that the
field is real.* It is real now.

## The pipeline, in the order the three crates state it

| step | call | what it makes |
| --- | --- | --- |
| **the rock** | `mountain::starter(seed, Starter { ridges: 5, nodes: 6, reach: 65, summit: 9, tip: 0.4, bulge: 4, spread: TAU })` | a fan of five ridges expanded into a `FacetMesh` |
| **the lattice** | `terrain::rasterize(&mesh, 16.0)` | the massif read onto 168 × 186 cells |
| **the erosion** | `terrain::erode_traced(&ErodeParams::default(), &read.field, &Hydraulic)` | the solved field, which is what the village stands on |
| **the village** | `compose(ground, lane, platform)` | the same five buildings, seated on the solved field, with the square cut to a platform at the hill's own elevation there |

This is the first frame in which all three crates are in one picture: `mountain` makes the rock, `terrain` reads
and erodes it, and a building stands on the result.

## Three measurements, all of them refusals of the obvious choice

- **A massif's footprint is not its `reach`.** Five ridges at `reach = 65` with the default spread produced a
  field **1 343 × 1 486 cells at two metres — 2.7 km across** — because a fan spread over less than a full turn
  leaves a closing wedge whose corner stands hundreds of metres out. `spread: TAU` closes the fan and the corner
  goes away; the footprint is still kilometres, because it is the *massif's* and not the hill's.
- **Erosion is priced by the lattice, not by the hill.** At two metres the field is two million cells and one
  build took **~15 s** (28 builds, 428 s of probe). At sixteen metres it is thirty thousand cells and **~0.4 s**.
  The village stands on forty metres of the field, where the solve's detail is below the scale of a house, so the
  coarse lattice is the right lattice here and not a compromise.
- **A camera written for a flat site does not survive a terrain.** The scene's own eye was at `y = 6` and the
  massif's flank is sixteen metres up: the first frame was the village seen through the underside of its own
  hill. The second framing — a high oblique — put the camera above the shoulder but framed a dark plane. What
  works is the one rule a terrain allows: **the eye stands above the ground under its own feet**, and the
  composition is framed from the side the flat scene had already earned.

## What the hillside village still is not

- **one summit, one seed**: the massif is `mountain`'s own starter fan, not an authored skeleton, so it has no
  front and no back and the village sits on whichever flank the offset names;
- **one cut**: the square's platform is cut, and the houses fill under themselves with posts, but each house's own
  pad is not cut and filled as a stair;
- **no lane**: the street is a straight climb and this hill is not one.

Identity: **4 643 rows** (from 4 559: the houses stand on more posts now that the ground has relief), and
twenty-two scenes in the gallery.

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
## The pads: a stone plinth under every house

The hillside village had posts under its houses and nothing else, and posts alone are a house on stilts — a
different building from a house on a pad. Every seated house now gets a **stone plinth**: its own plan, proud by
`0.28 m` on every side, from the lowest ground under that plan up to the sill.

**It has no thickness of its own**, and that is the whole of the design: it is deep where the fill is deep and a
step where the ground is level, because its underside is the slope's own shape. Two consequences follow and both
are in the code rather than in a comment: the outline is inflated **about its own centre**, so a plan that is not
a rectangle still gets a margin on every side; and a house whose sill is already on the ground gets **no plate at
all**, because a plate thinner than two centimetres is a face at a face and the expansion reports that as one
solid or as a crack.

Rows: `village_square` 16 497 → **16 521**, `hillside_village` 4 643 → **4 667** — six rows a house and four
houses, in both villages, because the plinth is the composition's and not the terrain's.
## The stair was under the hill

The lane out of the square is one plate a step, each landing on the grade at its own run — and it was cut from
**`grade`**, which is the *flat* village's arithmetic: level inside the square, falling twelve centimetres a metre
outside it, to a limit of 2.4. In the hillside village, where the terrain is seventeen metres up, that put the
whole stair **nineteen metres under the hill it was cut into**, and nothing in the document said so: the plates
were there, their elevations were finite, and `validate` had no question to ask about a plate that is simply
somewhere else.

It is laid on `seated_ground` now — the scene's own ground, the same one the houses were seated on and the frame
draws — so the stair descends the slope between the earth houses and the log house, which is what a hillside
village's way out of its square is.

**Identity is unchanged (16 521 and 4 667)**, and that is the point of keeping the number: the defect was not a
missing plate but a misplaced one, so a count of the document could not have found it and only the frame could.
## Round 45: the village's sixth building

The 蘑菇房 joins the settlement, down the hill west of the square, seated like the other houses (its own pads, its own
posts, `pad_plinth`). It is the village's **third house type** — a Hani house beside two 土掌房 and two 木楞房 under one
侗族 tower — which is what a 西南 village on a road between peoples actually looks like.

**Two things of its document are dropped, both of them site work**, and both were found by the village refusing to
build:

- **its terrace kerb**, because on a hill the seat gives that kerb's two ends two different grounds;
- **its straw covering**, because one of the two hip rolls leaves its own plane by `1.09e-3` — a millimetre, against a
  tolerance of `1e-3`, on a roof that is planar to `0.9e-3` before the seat rounds its coordinates. **A covering is
  close-scene geometry**; the far view does not need the bundles, and the near scene keeps them.

Identity: `village_square` 18 026 → **18 205 rows**, `hillside_village` 5 572 → **5 775**, and the other twenty-two
scenes unchanged.

**What this round did not do**: the village's own cameras do not frame the new house — it stands 16 m west of the
square, past the western 土掌房, which is where the hill has room for it. The composition now has six buildings and a
frame that shows five.
## Round 46: a view that holds six

`village_west` is the same document as `village_square` seen along the other diagonal — from the south-west, low, so the
蘑菇房 is in the foreground, the two 土掌房 stand behind it, and the tower rises over both. **The gallery's twenty-fifth
scene, and the first frame in which all six buildings of the settlement are visible**: the tower, two earthen houses,
two log houses, the 骑楼 row and the Hani house down the hill.

It is a *view* and not a building, and that is the honest description: the geometry is the square's to the row, and what
this scene adds is the reading — a village of several peoples is only a village when one frame can hold it.

Identity: 25 scenes, **105 876 rows**, of which `village_west` is the same **18 205** as the square's own document.

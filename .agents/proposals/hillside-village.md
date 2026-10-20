# 山坡上的村寨 — a village on a real `terrain::HeightField`

Every village so far has stood on a rule: a plateau, a line, a slope written as arithmetic. This one stands on
**terrain**: a `terrain::HeightField` of 320 × 320 one-metre cells, filled from a closed-form hill, with a
platform cut into it for the square.

## One composition, two grounds

The village's document was already a composition of five buildings; it is now a function of its ground:

| | `document()` | `hillside_document()` |
| --- | --- | --- |
| ground | the terrace's own `grade` — flat inside the square, climbing east along the lane, falling away on the other three sides | the hill, with the square cut to a platform at the hill's own elevation at the square's centre |
| the lane | joined on, because the street's own grade is the one this village continues | **not** joined on: the street is authored on a straight climb and a hill is not one |
| the tower | on the terrace, not seated | on the platform, not seated — a seat would flatten its seven storeys |

`compose(ground, lane, platform)` is the composition; neither variant re-derives a tower or a house.

## The defect this found: a height field read its `base` on a lattice line

The hillside's first frame was a patchwork of plateaus at wildly different heights, and the probe found why in
one line: **`HeightField::sample` answered `0.0` for any plan point exactly on a lattice line.**

`sample_run` took `left = floor(column)` and `right = ceil(column)` and weighted them by `right - column` and
`column - left`. On an integer coordinate the two are the **same number**, so both shares are zero, the weight
total stays zero, and the function returns `None` — which `sample` answers with the field's `base`. Nothing about
that is specific to this scene: the field's own centre, and every whole-metre point in it, was unreadable.

The fix is one line in `crates/terrain/src/heightfield.rs`: the far cell is `floor + 1` and its share is
`column - left`, which is the bilinear interpolation everyone means. Measured after:

| plan point | before | after |
| --- | --- | --- |
| (0, −48) | 0.000 | **8.534** |
| (0, 0) | 0.000 | **5.340** |
| (26, −44) | 0.000 | **9.040** |
| (100, 100) | 0.000 | **−0.452** |

The scene had been cutting its platform from that zero — which is why the square's platform stood five metres
below the hill it was cut into, and the frame showed the step.

## What the hillside village is not

- **no lane**: the street is a straight climb and this hill is not one, so joining it would be the two hills
  meeting that the terrace version was built to avoid;
- **no real terrain data**: the field is filled from a formula, not from `mountain`'s massif or from `erode`. The
  point of using `HeightField` is that the *reader* is the same one a real field would be read by;
- **one platform, one cut**: the houses are seated on the hill and fill under themselves with posts, but nothing
  cuts a pad for each of them, and a real hillside village is a stair of pads.

Identity: **4 559 rows**, and twenty-two scenes in the gallery.

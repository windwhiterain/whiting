# The massif is two long ridges with a corridor between them

## What this is for

`landscape` was reading as **two mountains standing on a plain**: a cone each, a green sheet
between them, and a river thread that stopped being a river. The reference is a **corridor** — the
eye inside a valley, the ranges running away on both sides, the floor a river has cut.

Three separate things had to change, and only the first one was in the plan.

## 1. The plan has to be long, which means the reaches and not the crests

The plan's extent is `mouth = 1.28 x max(adjacent reach) / cos(half-angle)`, so it is set by the
**longest reach beside each wedge**. With ten reaches evenly spread the plan is round whatever the
bearings say — the ceiling is about 1.65:1 and no arrangement of crests gets past it.

Past it by **putting the long reaches in pairs**, so that a wedge's mouth is only large where two
long crests already agree:

| | |
|---|---|
| long reaches (2200) | bearings 78, 102, 258, 282 — two pairs straddling the valley axis |
| short reaches (400-500) | bearings 0, 44, 136, 175, 224, 316 |

The long pairs' mouths all land near the axis, so the plan goes to **2.02:1** and the range sits
with its long axis along the corridor instead of across it. `RANGES` moved from `+/-0.62` to
`+/-0.58` of a radius, which narrows the gap between the two ranges from `0.36R` to `0.19R`.

## 2. A crest's profile has to be flat, or a long reach is still a cone

A single summit forces every crest to fall monotonically from it, so a massif of ten crests is a
fan of ten cones no matter how it is arranged. What makes it read as a **wall** is the rate of the
fall: the four long crests went from "1860 -> 800 over the first 44%" to **1860 -> 1240 over the
whole run**, a 16-degree skyline instead of a 45-degree one. That is the difference between two
horns and two ranges, and it costs nothing in the skeleton.

It also moved the framing. The summit's elevation angle from the eye is what the eye is set from,
and flattening the profile put the far ends of the ranges back inside the frame:

```
framing from the eye, half frame 35.5 degrees off the axis:
  west: summit at 26.1 degrees, nearest point at 19.6 - inside
  east: summit at 26.1 degrees, nearest point at 19.6 - inside
```

## 3. The surface was reading as straw, and not for the reason it looked like

A flank of the west range was covered in fine stripes fanning down from the summit, and the
obvious reading — a smoothing group that creases at every rib — turned out to account for **3.7%
of the crease** (see `.agents/SPEC.align/mountain.md`, "A flank is one surface"). Three things
looked identical in the render and had to be told apart by rendering each out:

| candidate | verdict |
|---|---|
| `ruggedness` in the albedo | ruled out — the stripes survive with the term removed |
| a fifty-unit world-space noise | ruled out the same way |
| the smoothing group | ruled out by **rendering the normal as a colour**: a smooth gradient |
| **`dip_angle` read as a per-vertex quantity** | the stripes are all here |

`dip_angle` is a property of a face, written at all three of its corners, so it is constant across
each triangle and steps at every triangle edge — two thousand triangles each carrying their own
constant. The body now reads steepness off `normal.y`, which is the same angle and is interpolated.

## What it costs

- **A crate contract widened, not changed.** `CornerNormals::of` is unchanged in behaviour; it is
  now `of_grouped` with the block as one particular grouping, and `faceted` takes the grouping as
  an argument because "which edges are creases" is a scene's statement. No other scene moves.
- **Facet aspect stays poor and is now measured against.** Worst aspect on this massif is **259.9**
  (median 5.5, 988 of 2100 faces over 6), concentrated in the wedges where a long crest stands
  beside a short one and `valley_radius`'s mean leaves the floor far inside the long crest's
  stations. This is the cost of elongating the plan and it was accepted with it; the probe reports
  it every run so it cannot drift unnoticed.
- **The river is left as a thread.** It is a braided river at the right width and the wrong
  texture — the noise lattice that cuts the channels is nearly square in world space, so the bars
  read as blocks rather than as gravel. A dedicated river module owns this next; the floodplain's
  benches and dry fields are in place underneath it.

## Verification

`cargo run -p whiting-gallery --example alpine_probe`

```
violations: 0
mesh: 2100 faces, 1224 points, shortest edge 2.648
plan area 7748688.0 against footprint 7748677.0, difference 11.000
surface: 1 boundary loop(s), euler 1, manifold true
bounds x [-1427.6, 1427.6], y [0.0, 1860.0], z [-2878.9, 2878.9]
faces: 2100 | aspect: median 5.5, worst 259.9, 988 of 2100 over 6
creases across a flank: 1508 of 2977 interior edges, worst 158.1 degrees
creases across a belt:  1684 of 2977 interior edges, worst 158.1 degrees
```

Tests: `whiting-mountain` (unit, integration, doctests), `whiting-scene` `scale_notes` (6) and
`clustered_lights` (13), `whiting-device` `gallery_render` (2).
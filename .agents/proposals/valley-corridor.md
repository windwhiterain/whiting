# A valley the eye stands inside, rather than a massif it looks at

## What the scene was asked for

A high valley corridor: look **along** the valley's axis, the two ranges closing in from either
side, a braided river on the valley floor with green and gold floodplain terraces beside it, four
or five layers of blue receding ranges at the far end, low-angle light through gaps in the cloud,
and no single subject in the frame.

Three of the choices below were the upper's and are recorded as such. The rest were mine, and the
ones I could not resolve on my own are in the last section rather than buried here.

## What the scene was, and what had to change

It was a **massif photographed from outside**: a single radially symmetric range on a plain, one
camera looking at its best face, the sun a quarter turn round so that one flank was lit and its
neighbour was not. Every part of that reads as a specimen, and none of it survives contact with a
valley.

The work is in four groups, in the order the picture forced them.

### 1. One massif, stood twice

The range is authored once and **translated** into two positions — `stood_at` is a pure
translation, so the plan area, the winding and every topological number are unchanged. Two
separately authored ranges would be two things to keep in step, and the valley would stop being
a valley the first time one of them moved.

A placement names a mesh by name, so two ranges drawn from one geometry are still **two meshes and
two passes in every stage**. `identity::Identity` gained a `first_id` for the same reason: the
belt-path table has to start at a different number for each range or a flank on the west wall and
a flank at the same station on the east wall answer to one id, and a pixel resolves to whichever
one a reader asked about first. `clustered_lights` records the pass count and its `landscape` row
moved from `2/2/2/1` to `4/3/3/1` — the scene's own shape, not a change in how a pass is counted.

### 2. The eye, the aim and the sun all moved onto the valley's axis

The camera looks **along** `x = 0`, which is where the river runs and where both the eye and the
aim stand. Aiming across the valley would put one wall in the middle of the frame and the other
off the edge, which is a mountain shot again.

The sun moved further than anything else and it is the single change that decided whether this
read as a valley at all. It used to stand a little over a quarter turn off the camera's bearing,
which put it **behind the eye**: every flank that faced the camera was lit, the two ranges came
out as bright brown cones, and the brightest thing in the frame was whichever facet happened to
face the sun. It now stands a quarter of a turn off the axis and low, so the camera is looking
into it, both walls are on their shadow side, and the brightest thing left in the picture is the
water between them. That is the reading the whole scene is for, and nothing else in it substitutes
for it.

### 3. The floor carries a river, and the cloud is pushed to where the valley closes

The valley floor was a cloud sea over the whole plain. It is now a **braided river on a floodplain**,
with the cloud keyed to a **distance down the valley** rather than to a height — the floor is flat,
so a cloud keyed on height either covers all of it or none of it, and a sea over the whole floor
is a lake with a mountain round it. The cloud code stayed; only the boundary moved.

The river is the one thing in the frame that carries a colour of its own, which is what makes it
water: a term that returns sky *alone* paints the channel the colour of the sky and the river
becomes a strip of white paper down the middle of a valley.

### 4. Four things the pipeline decided, none of which were available as a knob

- **The sky a face stands in is decided in the surface body.** The pipeline carries one
  directional light, and the second light is a point light in a scene sixty times its own size —
  what reaches a surface is a fact about how far away the lamp stands. A backlit wall is therefore
  black no matter how the lamp is placed, so `MASSIF` and `GROUND` each add their own share of the
  sky, weighted by how upward-facing the face is, using a `SUN_AHEAD` constant the scene already
  computed. This is the difference between a range that is a dark shape and a range that is rock.
- **`ridges` takes a layer count.** How deep a horizon recedes is a fact about the land in front of
  the eye; a constant forces a scene to lie about either how far away its horizon is or how far
  the air has closed over it. `landscape` takes five, the other two callers three.
- **A creased noise with no envelope is a row of tents.** Peaks need to stand *on* something: a
  broad massif term at a third of the crease's frequency is what makes two peaks a massif and six
  peaks a chain, and it is the difference between a skyline and a row of identical pyramids.
- **A world-space pattern on a floor the eye stands on has to fold back into its own mean.** A
  value-noise cell is a fixed length of ground, so under the eye it is a cell across a third of the
  frame and at the far end of the corridor it is a pixel. Neither reads. The braided channels are
  warped before they are read — an unwarped lattice gives a river cut out of axis-aligned
  rectangles — and are damped toward their mean with distance.

## Measurements

Geometry, unchanged except for the relief below. `alpine_probe`:

```
ridge lines: 1 connected piece, violations: 0
mesh: 1005 faces, 567 points, shortest edge 1.901
plan area 1210939.6 against footprint 1210939.2, difference 0.375
surface: 1 boundary loop, euler 1, manifold true
```

The plan area is **bit-identical** to the pre-valley massif: the crests moved up, not out.

Facet aspect is where the relief cost something:

| | median | worst |
|---|---|---|
| summit 720 | 5.2 | 125.4 |
| summit 1044 | 6.7 | 374.7 |

Raising the relief without changing the plan makes the flanks steeper, and a flank's cells are
bounded across by a rib and along by the crest step, so the same step gives a taller triangle for
the same width. 545 of 1005 facets are now over 6. This is a real cost and it is the reason the
relief was not pushed further.

Tests: `scale_notes` 6, `clustered_lights` 13, `gallery_render` 2 — all green, with the one
deliberate change to `landscape`'s pass count described above.

## Open, and not mine to decide

**The ranges are cones and the reference has walls.** Not a shading problem and not a camera
problem; it is structural, and `alpine_probe` now measures it:

```
framing from the eye, half frame 35.5 degrees off the axis:
  west: summit at 37.6 degrees, nearest point at 35.8
  east: summit at 28.8 degrees, nearest point at 26.4
```

A range's nearest point and its summit are both on the ray from the eye through the range's own
centre, so the two agree to within a degree or two and the whole range occupies **one narrow slice
of azimuth** — two degrees against a half-frame of thirty-five. A two-degree object is a thing
standing in the frame, not a wall of it: a wall has to reach from the axis out past the edge, and
having its summit inside means staying well short of it. No relief and no focal length changes
that, because both numbers are the same ray.

What *was* traded against what, and it is the right trade: the summit went from 720 to 1044, the
lens from 1.12 to 1.24 rad, the ranges from `±1.32R` to `±1.20R`, and that is the most a single
blob can do. The cost is in the facet table above.

Getting walls needs the massif **elongated along the valley axis** — long reaches on the crests
whose bearings run up and down the valley, short ones across it. That is a rewrite of
`CRESTS[].reach`, and it is not free:

- The plan grows by about half, so `radius` grows with it and **every other number in the scene is
  a multiple of `radius`** and moves at once — eye, aim, ranges, plain reach, haze density,
  shadow extent.
- The summit has to go up again by the same factor to keep its angle from the eye, so the facet
  aspect degrades a second time on top of the table above.
- The two ranges would then overlap along the axis, and a massif's foot is at the plain's height,
  so they interpenetrate. The two walls would have to be authored to leave a floor between them
  rather than to meet at a summit, which is a different skeleton from the one built here.

**A second pair of ranges up the valley** is the cheaper half of the same idea and needs no new
code: `RANGES` is already a list, and `instance_buckets`, the id numbering, the mesh list and the
reverse lookup all walk it. Two more massifs at `z ≈ +2R` would put four walls in the corridor
instead of two and give the far half of the picture something other than haze. It costs two more
meshes, so the `clustered_lights` pin moves again, and the two up-valley ranges would need to be
sized so they close the corridor rather than block it.

# The village on a terrace, and the rule the seat taught it

The village from the previous round stood on level earth. A village square in this country is not level ground
that happened: it is **a terrace cut into a hillside** — the one piece of flat ground a slope can be made to give
— and the houses stand on the hill around it, each on its own pad.

## What the scene now does

- **the ground is a grade function**: level inside the square's own 16 × 12, and falling `0.12` per metre
  outside it, to a limit of 2.4 m. The same function seats the houses and draws the ground mesh, so a house
  cannot float and the hill cannot disagree with the building standing on it;
- **each house is seated on its own ground before it is merged**, with the `site::seat` every single-building
  scene already calls — read once per house, because a village of five documents seats five times;
- **the tower is not seated.** A seat moves every mass's nodes to the ground under them, which is exactly what a
  tower of stacked storeys must not have: seven floors would be flattened onto one elevation. It stands on the
  terrace, which is level, so nothing moves;
- **the terrace is held by a wall**, one run to a side, standing at the foot of the drop: a plateau cut into a
  hill is a cut face, and that face is what a person on the square sees when they look over the edge.

Identity: **4 253 rows** (from 4 229: four walls).

## The rule the seat taught, and the crate change it needed

Seating a house *after* its document has authored its door and windows was **refused**, four times, with
`GridOutsideFace`. A grid's `levels` are the face's own elevations, so a mass that the seat lowers keeps its
bays but leaves its storey lines where they were: the rhythm no longer spans the face, and the validator says so.

This is not a scene bug and it was fixed where it belongs — **in `site::seat`**: a rhythm is authored against a
face, and a face that moves takes its rhythm with it, so the seat now shifts each of the moved mass's grids by
the same `delta` it shifts its nodes. Measured after the fix: the village's four houses seat cleanly on the
slope, and **all twenty other scenes are unchanged**, because every one of them either seats before it authors a
rhythm or seats with a delta of zero (the courtyards are built flat on purpose, and the street seats first, which
its own comment states as "seating first is the whole of the order").

## What is still missing at this scale

- **the lanes**: the square is bounded on four sides and open at two, and the streets that run out of it are what
  `street` already knows how to build — the two scenes have still not been joined;
- **the valley**: this grade is a rule about a plateau, not a terrain. `mountain`, `terrain` and `vegetation` can
  make a real hillside, and no village has yet stood on one;
- **the pads**: each house is seated flat on the ground under its own outline, which on a slope means its uphill
  side is cut in and its downhill side is filled — the fill is not drawn and a real one stands on posts.

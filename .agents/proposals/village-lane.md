# The lane: the street scene joined onto the square

The village square has had an opening on its east side since it was laid out, and nothing behind the opening. It
has a lane now: **the street scene, merged on**, five plots a side climbing east — the composition the two
operations from two rounds ago were built for, used for the second time.

## The one thing that had to agree first: the grade

The street is authored on **its own** grade, `0.055 * x` from its own origin, and seats its five plots on it. The
village had a grade of its own: level inside the square, falling `0.12` a metre on all four sides.

Two grades that disagree at a seam are two hills meeting, not one village. So the village's grade was rewritten to
**continue the street's**: flat inside the square, rising `0.055` a metre past the square's east edge, and still
falling away on the other three sides. The two halves meet at `x = SQUARE[2]`, where both are `0.0` — the street's
`0.055 * x` from its own origin is this document's `0.055 * (x - 8)` from the square's edge, and they are one
line.

That is the whole of the join: `LANE_SLOPE` is the street scene's own number, `LANE_AT` is where its origin goes,
and the street's skeleton is translated and merged with no seating of its own, because its own document already
seated it on the grade this village continues.

## Two things measured on the way

- **`WallTooLow` on the east kerb.** The terrace's retaining walls had their tops at absolute elevation `0.0`,
  which was the square's level until the ground past it began to climb: at the east wall's own foot the ground is
  `0.0165` above it. The wall's top is now a **kerb** `0.35` above the terrace, which is what a kerb is.
- **the second earth house stood in the lane.** Left at `(13, 0)` its plan overlapped two of the street's plots —
  `UnattachedOverlap`, and the plans were right to disagree. It moved to the north-west `(-13, -13)`, where it
  forms a corner with the west house and the tower.

Identity: **16 497 rows** for the village and the lane together (from 4 559), and **76 224** across the gallery's
twenty-one scenes.

## What the village still is not

- **one street, not a network**: the lane runs east and stops at its fifth plot; there is no second lane, no
  cross-lane, and no way for the square to be the middle of anything but one arm;
- **no terrain under any of it**: the grade is a rule about a plateau and a line, not a hillside;
- **the seam is a line and not a joint**: the lane's paving and the square's paving meet at the square's own edge
  with nothing between them — no threshold, no step, no grate.

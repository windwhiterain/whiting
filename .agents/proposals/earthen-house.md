# 土掌房 — the flat-roofed earth house

The fifth type, and the first one in this gallery whose roof is **not a roof**: a 土掌房's roof is a slab of
earth on joists, and it is the household's **晒场** — the drying floor for grain, chillies and maize. The family
lives under it and works on it.

It is drawn as `crates/gallery/src/earthen.rs`, a scene of one mass:

| fact | how it is built |
| --- | --- |
| **earth walls 0.55 m thick** | one mass, `wall = 0.55`, `slab = 0.40` — four times a timber frame's wall, which is what makes its windows holes a metre deep |
| **the roof is a floor** | `Slab::roof` on the mass's own top, `0.35` deep, hosting the mass; there is **no `Roof` in the document at all** |
| **a parapet round it** | four low members on the slab's edge, `0.28` thick and `0.55` tall: a wall standing on another wall's top, which is why they are members and not a second mass — a mass is a closed volume with a floor of its own |
| **the stair is outside** | fourteen treads climbing the gable end from the yard to the roof, a stringer under them, and a gap in the parapet where they land |
| **a door and three windows** | `facade::open_court_face` on the roof-facing wall: two storeys, three bays, a door in the middle and a 直棂窗 in each upper bay |

Identity: **178 rows**, and nineteen scenes in the gallery.

## Two things measured while drawing it

- **A section laid along its own axis has no width.** The parapet's four sides run two ways, and a member whose
  `across` is parallel to its axis expands to `MemberDegenerate { length: 6.5 }` — a number that reads like a
  degenerate *length* and is the member's own length. `across_of` now reads the perpendicular from the pair.
- **A stair that runs the wrong way reads as a fan.** The first treads ran through the whole depth of the house
  and climbed in `x`, so the frame showed twelve fins radiating off the gable end. A tread runs *across* the
  stair and steps *along* the wall.

## The porch, the stringer and the landing's gap

The type was finished in the round after it was drawn, and the three pieces are the ones that make it read as a
house rather than as a box with a stair:

- **the 厦子** — a lean-to over the door, shed away from the wall and standing on two posts at the yard. A porch
  that hangs off a wall with nothing under it is a canopy, and the type's porch is where the year's work is done
  out of the rain. Its roof is the document's only `Roof`;
- **the stair's stringer** — one sloped beam under the treads' outer edge, so the run carries something: a set
  of evenly spaced treads reads as steps only because it is evenly spaced, which is a ladder lying down;
- **the landing's gap** — the parapet's stair side is two segments with the stair's own width between them,
  because a parapet with no gap is a parapet you cannot step over. `stair_width` is one number read by the
  treads, their stringer and the gap, so a stair narrowed in one place cannot leave a gap that no longer fits.

Identity: **208 rows** (from 178).

**A scene with no roof in it needs a roof mesh before it can have a porch.** The scene's meshes were walls,
slabs, members and ground — and the porch's `Roof` element was drawn by nothing at all, which is the same
mistake the 一颗印's window bars made in an earlier round: an element the document carries and the frame does
not pack is an element nobody sees.

## What the type still wants

- **batter**: a rammed-earth wall leans in as it rises, and `Mass` has no field for a lean. The thickness
  carries the type's mass and the lean is missing;
- **the joists under the slab** and the earth on it are materials here, not structure;
- **the 厦子**, the lean-to porch most of these houses carry on one side; and
- **a stringer under the stair**, because a run of floating treads reads as steps only because they are evenly
  spaced.
## Round 48: a setback band closes, and the 土掌房's failure is the cut

**The 土掌房 was to be a two-tier house** — a lower wall with every opening in it, a **晒场** left open round the room
above, and that room standing back from the edge. Three attempts, all refused by the document, and the probe settled
which of them was the geometry and which was the cut:

| the probe's mass | unbalanced | violations |
| --- | --- | --- |
| plain, four sections | 0 | 0 |
| with a shopfront and a window cut in | 16 holes | 3 |
| **with the plan set back** | **0** | **0** |

- **A setback band closes.** The narrowing band — a wide section below, a narrow one above — walks, seals and validates
  exactly as a projecting one does, so the earlier reading of `SolidNonManifold` (that a narrowing band's inner loop
  leaves its own solid) was **wrong**, and it is corrected here.
- **The 土掌房 failed on its openings**, twice, in two different ways, and both are the cut and not the wall:
  - with the batter **below** them, a cut whose lower edge lands on a section line subdivides that section's loop, and
    the band below has no crown to seal the stations it makes — `SolidOpen { loops: 1 }`;
  - with the batter **above** them (the setback), the same shell came back `SolidOpen { loops: 2 }`.

  Raising the door's sill clear of the section line did not close it. **A cut in a banded mass is still broken**, and
  the probe's `cut` mode is where it is measured.

So the tiered 土掌房 waits on the cut, and the record now says which wall is the wall: **the band walk is not the
obstacle — the cut is.** That is a narrower question than the one this round opened with, and the probe answers it in one
run in three modes.

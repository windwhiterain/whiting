# 木楞房 — the log house

The sixth type, and the first whose walls are **built by stacking** rather than by cutting: a 木楞房 is a pile of
logs, course on course, each course laid the other way round from the one below so the ends cross at the corners
and lock the building together without a joint cut. It is drawn as `crates/gallery/src/log_house.rs`.

| fact | how it is built |
| --- | --- |
| **every course is two logs, and the courses alternate** | one course lies along `x` on the two `z` walls, the next along `z` on the two `x` walls; each end runs `LOG_PROJECT` (0.38 m) past the corner, which is the crossing |
| **the shell is not the wall** | a mass with `wall = 0.16` and `slab = 0.30` carries the volume, and the logs are laid on its outer face: the log is the wall a viewer sees, and the shell is what closes the building |
| **the front logs stop at the openings** | the door and the window cut through the mass, and the logs that would have crossed them are laid in pieces: `log_spans` reads the same two rectangles the openings were cut with, so a log ends at the jamb the wall ends at |
| **the roof is board and it is low** | a gable with `rise = 1.35`, no 举架 ladder and **no covering at all** — the type wears boards or shingles, and a 木楞房's roof is not much more than the logs continued upward |

Identity: **168 rows**, and twenty scenes in the gallery.

## Two things measured while drawing it

- **A log that passes over a door is a door with a log in it.** The first frame showed the door and the window as
  dark slots with two log courses running straight across them, which is exactly what the document said: the
  mass was cut and the members were not. `log_spans` now leaves the openings open, and the door reads as a hole
  in a stack rather than as a hole behind one.
- **A mesh with no points stops the renderer rather than drawing nothing.** The scene packed `Element::Slab` for
  a floor mesh, and this document has no slabs at all — the mass's own plate is a `Mass` face. Cycles answered
  `mesh 'floors' has no points` and refused the whole document. A mesh a scene declares must have geometry in
  *this* document, not in the last one that copied it.

## What the type still wants

- **the chinking**: the moss and clay between the logs is what a 木楞房 is pointed with, and it is a material the
  frame cannot show;
- **the notches**: a real log is notched half through at each end where it crosses; a member here is a square
  box that simply stops;
- **the log floor**: the upper floor of a storeyed 木楞房 is more logs, laid across the span.
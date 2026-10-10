# A basin floor is a fan, and a dissected upland has none

Branch `feature/dissected-plateau`, worktree `.worktrees/dissected`. Off `77b3fc2`.
**Not merged**: the scene does not build cleanly. This records why, so the next session starts where
this one stopped rather than where it started.

## The plan view, which answers the question the ground view cannot

`LANDSCAPE_VIEW=overhead` compiles the scene with a near-vertical eye — same sun, same aim, same haze,
so it is the same geometry under the same light and not a second setup to keep in step. Seen from the
ground three ridges one behind another are one ridge, and a range grown badly looks exactly like one
grown well until you walk around it.

It says at once what no ground render did:

- **The generator's tree is a blade, not an area.** A long divide with a knot of spurs at its middle.
  Its plan is 1685 by 3222, and the geometry that reads as a mountain from the ground is a thin dark
  sliver. The reference is an *area covered in ridges*; this is a spine with tufts.
- **The ground's painted background ranges are the hard white scratches.** They are a screen-space
  ridge texture, and from above they read as exactly what they are. This is the "硬裂缝" in the frame
  and it is not the massif at all.
- **The summit was a degree-one tip**, because the divide began at it. A pocket between two tips is
  drained from the highest vertex *between* them; when that is one of the two there is none, `split`
  returns nothing, and the pocket was dropped **in silence** — taking its tips off the rim and leaving
  a crest end with no footprint vertex to weld to. The divide now runs *through* the summit, in two
  arms.

### Where the blade comes from, and what would fix it

The tree is grown by one rule — a child takes a slice of the half of its parent's angular interval on
its own side — and that rule bounds a subtree's *direction*, not its *position*. Two crests whose
directions are in disjoint intervals still cross when their origins are far enough apart and their
directions converge, which is why lengthening the spurs (`SHRINK` 0.46 → 0.60) lost planarity outright.
So the shape is stuck between "spurs too short to cover the area" and "spurs long enough but crossing".

**What to do instead:** stop confining direction and confine **area**. Give each branch a convex region
of the plan — a wedge *from the summit*, with its own origin, not merely its own bearing — and require
children to live inside their parent's region. Then non-crossing is a fact about regions rather than
about angles, and spurs can be as long as the region allows. The other route, likelier to look right, is
to grow the network from the **drainage** instead: scatter seeds on the plateau, let them cut downhill
and merge into trunks, and take the interfluves as the ridges. That is how the reference's pattern is
actually made, and it gives an area-covering network by construction rather than by parameter search.

## What works, and is on the branch

- **A massif is a range on a plateau.** `RIM` was zero, so authored relief was measured from sea level
  and the picture was a spire on nothing. It is now the height the land already stands at.
  `massif::shelf` tapers the **relief above** it rather than the whole elevation — a bug fix, not a
  preference: the old form forces every far end to zero and is correct only when the ground happens to
  be at zero. It then clamps at zero relief, which is the plateau, so a crest descends until it meets
  the land and runs along it. **Worst facet aspect 583 → 206**, and the folded belt in frame centre is
  gone.
- **The crests are grown, not authored.** `landscape/massif/network.rs` grows a main divide through the
  summit with spurs off both flanks, recursively, from a seed. It is planar by construction and checks
  itself. Reaches **146 crests and 74 valleys** at depth four, a closed manifold with one boundary loop.
- **The plan view**, `LANDSCAPE_VIEW=overhead`.
- **The network.** Face walk, col-to-col tie, ring split, and a `debug_assert!` that a wall is a path.

## What does not, and why

**`drain` lays a basin floor out as a fan from the basin's head** — the floor nodes run along the head's
bisector at the average of the two walls' distances. That is right for a mountain that widens away from
its summit, which is what the last three versions of this scene were, and wrong for the long narrow
troughs a dissected upland is made of. The checker names it directly:

```text
RibCrossesLine           a rib crosses a crest, because the fan does not fit the trough
SelfIntersects
Unwalkable(Disjoint { from: Ridge(RidgeId(27)), to: Footprint })
```

This is not a placement problem and moving the floor does not fix it: the floor's plan position was
tried at the midpoint of its two stations (better — 1249 violations to 984 — and it puts belts on the
wrong sides, because that position is what decides which side a belt lies on) and the ribs were tried
at equal arc distance (worse). **The shape of the floor is wrong, not its coordinates.**

## What to do next

1. **Derive the floor along the valley, not around the head.** A basin between two long ridges has a
   medial line, and the floor is a walk along it: at each step take the pair of stations on the two
   walls that are nearest each other, or the midpoint of the pair at equal distance from the foot, and
   step outward. Ribs are then the joins between consecutive floor nodes and are perpendicular to the
   trough by construction. The head-to-mouth fan survives as the special case where the two walls
   diverge from one point — which is what a summit basin is, and it is worth keeping as the case that
   already works rather than replacing.
2. **Then re-frame the region**, which is a separate task from the floor: the generator has to cover an
   area before the camera can be asked to show one.
3. **Re-measure the coverage.** The `NotCovered` reports on the generated network are *not* all real:
   the mesh is a closed manifold with the right area and **every one of the 78 crests bounds exactly two
   basins**, so the derivation is complete and the checker's side attribution is what disagrees.
   `check_coverage` decides a rib's side by the nearest station on a crest, which misfires on a curved
   crest.
4. **The ground's painted ranges want deleting.** They are the hard scratches, and a scene whose
   geometry is a real network does not need a texture pretending to be one behind it.

## The generator's wrong versions, so they are not paid for twice

Each produced a crossing, or a network one generation deep, and each is a plausible thing to write:

- **A splay measured from the parent's own bearing accumulates.** Four children turning the same way
  come round three radians and curl back.
- **A symmetric bound on the accumulated lean still fails**, because a descendant can swing to the
  other side of its own parent's spine.
- **A fixed angular margin is larger than the room left after one halving**, so the recursion stops one
  generation deep. The margin has to be a fraction of the interval.
- **A wander that is a fixed fraction of a crest's length is wider than the wedge a deep crest owns.**
- **A generated name is a key, not a label.** `ridges()` reads every crest's end point by looking its
  name up, so two crests called the same thing have their junctions computed from one of them.

The version that holds: a child takes a **slice of the half of its parent's interval on its own side**,
the margin is a fraction of the interval, and the wander is scaled by the interval.


Branch `feature/dissected-plateau`, worktree `.worktrees/dissected`. Off `77b3fc2`.
**Not merged**: the scene does not build. This records why, so the next session starts where this one
stopped rather than where it started.

## What was asked

Make the massif look like a real mountain group — the reference being a hillshade of 四姑娘山: a
**dissected upland**, ridges branching and rejoining at four or five scales, valleys cut into high
ground — and finish the defects left from the last round.

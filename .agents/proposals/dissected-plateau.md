# A basin floor is a fan, and a dissected upland has none

Branch `feature/dissected-plateau`, worktree `.worktrees/dissected`. Off `77b3fc2`.
**Not merged**: the scene does not build. This records why, so the next session starts where this one
stopped rather than where it started.

## What was asked

Make the massif look like a real mountain group — the reference being a hillshade of 四姑娘山: a
**dissected upland**, ridges branching and rejoining at four or five scales, valleys cut into high
ground — and finish the defects left from the last round.

## What works, and is on the branch

- **A massif is a range on a plateau.** `RIM` was zero, so authored relief was measured from sea level
  and the picture was a spire on nothing. It is now the height the land already stands at.
  `massif::shelf` tapers the **relief above** it rather than the whole elevation — a bug fix, not a
  preference: the old form forces every far end to zero and is correct only when the ground happens to
  be at zero. It then clamps at zero relief, which is the plateau, so a crest descends until it meets
  the land and runs along it. **Worst facet aspect 583 → 206**, and the folded belt in frame centre is
  gone.
- **The crests are grown, not authored.** `landscape/massif/network.rs` grows a main divide across the
  plateau with spurs off both flanks, recursively, from a seed. Planar by construction and it checks
  itself.
- **The network.** Face walk, col-to-col tie, ring split, and a `debug_assert!` that a wall is a path.
  Two-way junctions and the doubled cirque lip are fixed; the tree on `main` is unchanged by them.

## What does not, and why

**`drain` lays a basin floor out as a fan from the basin's head** — the floor nodes run along the head's
bisector at the average of the two walls' distances. That is right for a mountain that widens away from
its summit, which is what the last three versions of this scene were, and wrong for the long narrow
troughs a dissected upland is made of. The checker now names it directly:

```text
RibCrossesLine   x9      a rib crosses a crest, because the fan does not fit the trough
SelfIntersects   x1
Unwalkable(Disjoint { from: Ridge(RidgeId(27)), to: Footprint })
```

This is not a placement problem and moving the floor does not fix it: the floor's plan position was
tried at the midpoint of its two stations (better — 1249 violations to 984 — and it puts belts on the
wrong sides, because that position is what decides which side a belt lies on) and the ribs were tried
at equal arc distance (worse). **The shape of the floor is wrong, not its coordinates.**

## What to do next

1. **Derive the floor along the valley, not around the head.** A basin between two long ridges has a
   medial line, and the floor is a walk along it: at each step, take the pair of stations on the two
   walls that are nearest each other, or the midpoint of the pair at equal distance from the foot, and
   step outward. Ribs are then the joins between consecutive floor nodes and are perpendicular to the
   trough by construction. The head-to-mouth fan survives as the special case where the two walls
   diverge from one point — which is what a summit basin is, and it is worth keeping as the case that
   already works rather than replacing.
2. **Then re-measure.** The `NotCovered` reports on the generated network are *not* all real: the mesh
   is a closed manifold with the right area and **every one of the 78 crests bounds exactly two basins**,
   so the derivation is complete and the checker's side attribution is what disagrees. `check_coverage`
   decides a rib's side by the nearest station on a crest, which misfires on a curved crest; it wants to
   read the side the way the sampler does, or to be checked against the mesh.
3. **`Ridge(27)` does not weld to the footprint.** One tip in the generated network has a footprint
   vertex it cannot reach, and the mesh refuses to build. Look at `is_junction` first: the tip is
   skipped from levelling if the end names, or is named by, another crest, and a tip that is neither
   still has to land on the rim's vertex.
4. **Relief, not density, is what is missing from the picture.** With the main divide spanning the
   plateau, the fall from summit to plateau happens over the whole of it and the valleys come out
   shallow — floor within a few units of ceiling in the probe's own table. The reference's valleys cut
   a third of the relief. Whatever the floor walks along, its `depth` has to be read against the
   *basin's* fall and not the crest's.

## The generator's three wrong versions, so they are not paid for twice

Each produced a ridge that ran back across the one it grew from, and each is a plausible thing to
write:

- **A splay measured from the parent's own bearing accumulates.** Four children each turning the same
  way come round three radians and curl back.
- **A symmetric bound on the accumulated lean still fails**, because a descendant can swing to the
  *other* side of its own parent's spine — the same crossing one generation further down.
- **A wander that is a fixed fraction of a crest's length is wider than the wedge a deep crest owns.**
  The wander has to be scaled by the angular interval the crest is confined to.

The version that holds: a child takes **half of its parent's angular interval, strictly on its own side
of the parent's line**, so the parent's continuation down the middle is bounded away from every child by
construction. Worth keeping that shape if the generator is rewritten.

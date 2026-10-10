# A massif is a network, and the walk that could not turn

Two changes to `mountain`, found by trying to build a hierarchy and failing, and the second one only
survives because the first made the failure legible. Both are in `9ba413b` and the commit after it.

## 1. A corner is where two ends weld, not where an end is nearer

`Sampling::path` walks a belt's upper boundary between two stations. When they name different lines
it must turn where the lines meet, and it chose the turn with `nearest_end`: the end of the leaving
line that is nearer, in the plan, to the point being aimed at.

That is right for exactly one shape — a fan, where every crest runs away from the one summit, so the
end nearer the aim is always the tip — and wrong for everything else. On a crest that is one segment
of a longer chain, the walk often has to leave by the end *further* from its aim; the measurement
turned it back onto the summit and answered `Disjoint`. `corner` now asks which pair of ends welds,
which is a fact about the document, and keeps the nearer-end measurement only as the tie-break when
both ends weld — the rim's case, where a loop meets a line at one point and the seam makes that point
both ends.

`validate::check_coverage` carried its own copy of the measurement (`end_nearer`, `parameter_at`). It
reads the sampler's rule now, because two rules about where two lines meet is two rules that can
disagree, and the disagreement shows up as a skeleton that builds and is reported broken, or the
other way round.

### The amendment this retires

`SPEC.align/mountain.md`, "A spur hung off a parent's flank cannot be built, and the reason is
structural", is **wrong**, and the scene's `massif()` doc repeats it. Its two reasons:

- *"Non-increasing along every root-to-leaf path means a connected skeleton has exactly one summit,
  and a parent and a spur at `t > 0` do not share it."* A non-sequitur: the spur's first node **is**
  the parent's station, so the root-to-tip path through it is non-increasing and there is still one
  summit.
- *"The wedge doubles on itself."* The image is real — a surface folded along the parent's upper
  segment — but its cause was the walk turning at the wrong corner, not the correspondence. The wedge
  between a parent above the junction and the spur does not close onto the summit; its first row
  collapses onto **the junction**, because both of its stations name that one point, which is the same
  collapse a summit already uses.

**What is true, and is the residue worth keeping:** *a belt changes line only at a line's end.*
`corner` inspects the leaving line's two ends and nothing else, so a child named at a parent's
interior welds geometrically but is walkable in one direction only. A junction must be authored as an
end — split the parent there. `attach_ridge(parent, 0.5, ..)` is accepted by the ops and expands to
`Disjoint`, and that is the honest, narrow statement of the old claim.

## 2. A flank's side is declared by its rib, not measured from the floor

`check_coverage` filed each covered span under the side of the line it thought the belt was on,
measured with `side_of(line, t, middle)` where `middle` is the midpoint of the two valley nodes either
side of the rib. That point is a construction nobody authored, and on a wide basin — or one whose two
crests part at a shallow angle — it crosses the crest from one segment to the next. The spans then
land on alternating sides and the report is a crowd of gaps in a flank the mesh covers once.

It now reads the declaration that already exists: a rib joins a valley node to the station it names,
and the node stands on one side of that line, measured at the rib's own station. A node standing
exactly on its station — a head or a corner collapsed to a point — declares nothing, and its belt
takes the side the other rib declared.

No `Side` field was added to `ValleyNode`. That would be a second copy of a fact the geometry already
states, free to disagree with it silently, which is the failure the corner change exists to remove.

## 3. What the topology checks do not see

`euler_characteristic`, `boundary_loops` and `is_manifold` certify **closure**. Two flanks doubled onto
one crest are two discs glued along an edge, which is a disc: `euler 1, 1 boundary loop, manifold
true` while the plan area is ten times the rim's. Only the area comparison and the coverage check see
a double cover. A probe that reports the first three and not the second two is not a check.

## Evidence

A hand-bound hierarchy — a crest chain `S → J → T` with a spur born at `J`, both ends naming the
junction — expands to `301 faces, 196 points, euler 1, 1 boundary loop, manifold true`, with faces
hanging off all four ridge segments. The probe that showed it was deleted rather than shipped: its
floors were hand-placed and did not validate, and the claim it proves is now carried by the scene,
which has to validate to build at all.

The fan is untouched: `alpine_probe` before and after both give 2100 faces, 1224 points, plan area
7748688.0 against footprint 7748677.0, difference 11.000, 1 boundary loop, euler 1, manifold true.

## Still open, and not solved here

- **Deriving valleys for a network.** `derive::Fan` reads a fan in closed form; nothing derives a
  hierarchy, so an author hand-binds every station. The design is worked out and the hard parts are
  known: the faces of a tree are not the sectors at its vertices (a summit's sector contains every
  spur that branches below it), so they have to come out of a half-edge walk of the ridge graph; the
  walk needs no rim, because a tree's outer face splits into pockets between its degree-one vertices
  and the rim's *topology* is known before its geometry; a mouth must be bisected by **rotating one
  chain through half the sector at the head**, not by summing the two tip directions, because the
  vector-sum bisector degenerates at 180° and picks the wrong side past it — which is exactly how a
  range's wraparound pocket behaves, and what made a derived rim wind backwards with a negative area.
  A pocket whose highest vertex *is* one of its tips has one empty chain and must bind that side to
  the rim. A cycle closes a face that never reaches the rim, so it drains to a corner rather than a
  mouth. Worth a session of its own; none of it blocks the demo.

## The face walk, tried and measured (2026-10-08)

The scene's massif was rebuilt on the tree walk first, then the half-edge walk was written over it to
test the network. **The walk itself works and the network is reachable; the rim is what is not done.**

With a tie ridge added — one crest whose *both* ends stand on other crests, closing a ring between the
north spur's shoulder and a gully's mouth — the walk returns exactly the two faces the graph has:

```text
face: 37 darts, 8 tip visits     # the outside, which is where the eight pockets are
face:  3 darts,  0 tip visits    # the cirque: a closed ring that reaches no rim
```

Three findings came out of it, each one a bug that was easy to write and hard to see:

- **A dart's slot has to be one function.** The `seen` index and the test for the walk's own closing
  are the same number written twice; written twice, one was inverted, and every face came back one
  crest long — which does not crash, it just quietly produces twenty basins for twenty crests.
- **A branch already knows which way its crest runs from the vertex it stands at.** Building the next
  dart as the *negation* of a branch's direction makes the walk step straight back along the crest it
  came in on. Same silent symptom: faces the length of one crest.
- **Which neighbour of the arrival is "next" decides only the winding, not the answer.** Taking the
  branch clockwise of the arrival and taking the one counterclockwise both yield the same two faces
  with the same sizes, mirrored. Worth knowing, because it is the first thing to suspect when a face
  looks wrong.

**What is left, and it is one thing.** The rim is woven from each basin's left tip and its mouth, in
the order the basins were walked — which is correct for a tree, where the pockets tile the outside and
each tip is the end of exactly one pocket's left wall. With a cycle, a pocket's boundary can run
*around* the closed face and a tip can be reached from two directions, so the sequence
`tip, mouth, tip, mouth …` stops being a simple loop and the mesh comes apart: `9 boundary loops,
euler 40, manifold false`. The fix is to weave the rim by walking the **outer face's own boundary** and
inserting each pocket's mouth at the tip pair it belongs to, rather than by trusting the basin order —
which is the same lesson the tree already taught, that the rim's order is a fact about the walk and not
something to recover by sorting afterwards.

- **`WELD` says `1.0e-2` and `weld_key` implements `1/1024`.** Both the walk and the derivation now
  read through it, so the mismatch is more load-bearing than when it was recorded.
- **Confluences.** Every basin drains to the rim on its own floor, so a deep hierarchy combs into
  parallel gullies instead of a tree of valleys. `Station` can name a valley, so a valley-into-valley
  junction is expressible and has never been built. The cirque above is the near case: its floor
  reaches a col and stops, and what it should do then is *continue down the valley that owns the col*,
  which is a junction between two valleys and has the same one problem the ridge junction had — the
  walk has to be able to turn at it.

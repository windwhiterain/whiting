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

## A head fold's two valleys share their head crests, and the coverage rule cannot see it (2026-10-10)

Found while authoring `crates/gallery/src/landscape/massif/authored.rs` as a dissected upland: one
long divide with spurs at four levels, thirty-two crests and seventeen valleys, written down by hand.

### What is built and what it does

The divide is a chain of twelve segments, each descending from a shoulder or the summit into the col
beside it; the spurs and the spurs' own spurs hang off the shoulders. The **ridge** checks pass:
`ridge lines: 1 connected piece(s)`, every crest descends from its peak, every junction is an end
that names the crest owning it, and the stations are written as `(crest, node)` pairs so each one
lands exactly on its node.

The latest run, with `side_foot` in `validate` and the parameter fix below:

```text
32 ridges, 17 valleys, 34 footprint nodes, step 279.50012
ridge lines: 1 connected piece(s)
violations: 66
mesh: 203 faces, 132 points, shortest edge 5.099
plan area 3483590.5 against footprint 3887049.5, difference -403459.000
surface: 3 boundary loop(s), euler -1, manifold true
```

### The parameter bug that held the file at 114 violations

`parameter_of_node` returned `0.0` for every node. It walked `nodes.windows(2)` and tested
`index + 1 == node`, which is right, but the loop it sat in had been shortened to a single pass over
pairs while the check was left reading the pair's **second** node against the loop index — so no
node ever matched and every station resolved to `t = 0`. Every rib was then a point at its crest's
start, which reads as a mesh that builds and leaves a hundred spans bare. Fixing it to accumulate a
running length per node dropped the violations by half (`114 -> 66`) and took the surface from six
boundary loops to three. **A station that resolves to the wrong parameter is a whole mountain of
`NotCovered` and nothing else.**

### The rule that still does not hold

Two valleys share the crest a head stands on — `w0` at the summit carries the north valley's left
wall and the south valley's right wall. Both write the head belt as `(crest, head-end node)` and the
station after it as the same crest's next node, so **both cover the same stretch of that crest** and
the other half of it is bare:

```text
SAME valley 4  Left  segment 0  Ridge(7) 0.0..0.5004  side Left
SAME valley 13 Right segment 0  Ridge(7) 0.0..0.5004  side Left
SAME valley 4  Right segment 0  Ridge(0) 0.0..0.5067  side Right
SAME valley 13 Left  segment 0  Ridge(0) 0.0..0.5067  side Right
```

It is not a matter of moving the floor: both valleys' first floor node stands in the same wedge, on
the same side of the shared crest, so `cross(direction, foot - station)` is equal and opposite for
the two crests and **equal** for the same crest in both valleys. No plan position of a floor node
makes them differ, and making the head rib a point does not either — `side_foot` then reads the side
from the next node, which is the same point in both valleys for the same reason.

### What would fix it

- **A side declared on the rib rather than measured from the foot.** `ValleyNode` would carry which
  flank of the crest it is on and the check would read it.
- **A head that stands on a crest the two valleys do not share**, so the two head ribs are not the
  two flanks of one line.

Neither is in this change. Nothing else about the tree is refused.

## The face walk, tried and measured (2026-10-08)


The walk itself works and the network is reachable. What is not finished is the **composition of a
wall** for a cyclic graph. The state of that attempt is kept beside this proposal as
[`network-face-walk.rs.txt`](network-face-walk.rs.txt) — the scene's `landscape/massif.rs` with the
half-edge walk, the col-to-col tie and the ring split already in it, reaching **2 violations from
2128** and a plan area exact to 6 units in 5.35 million. A reference, not a patch.

### What works

With a tie ridge whose **both** ends stand on cols — `spur_s2`'s shoulder and `gully_s`'s — the walk
returns exactly the two faces the graph has:

```text
face: 39 darts, 10 tip visits, [shoulder_top spur_w spur_w … spine_w]   # the outside, 10 pockets
face:  5 darts,  0 tip visits, [spine_m spur_s1 gully_s tie_n spur_s2]  # the cirque
```

The second is a genuine five-crest ring reaching no tip, and it validates as `simple true`. That is a
**cirque**, and a tree cannot make one. The rim comes out the right size — plan area 5353259 against a
footprint of 5353265 — and the second boundary loop is gone.

### What is left, and it is one thing

`ElevationRises` in one valley, plus `MeshOpen { boundaries: 2 }`. The trace is unambiguous:

```text
rib 0.641: L 802 on Ridge(13) t 0.641 / R    0 on Ridge(21) t 1.000   # R ran out to a levelled tip
rib 0.667: L 744 on Ridge(13) t 0.667 / R 1021 on Ridge(20) t 0.072   # R jumped to a different crest
```

The right wall walks one crest out to `t = 1.0` — a tip the shelf took down to the plain, so its height
is 0 — and then continues on **another crest** whose start is 1021. Those two crest ends are not the
same point, so the wall is not a path: `Chain::at` interpolates across a jump the geometry does not
have, the floor is written against a wall that dips to zero and climbs again, and the belt between them
is a hole.

The wall is built from `basin.left`/`basin.right`, halves of a **mirrored** stretch — each pocket is
reversed and every dart turned around before the walls are cut, because the rim must be wound opposite
to the walk's own sense. Mirroring a dart list is continuous in principle: reversing `d0…dn` and
flipping each dart gives a path from `end(dn)` back to `start(d0)`. So the fault is in how the halves
are read, and **the next session should begin by asserting continuity outright** — that
`start(dart[i + 1]) == end(dart[i])` for every wall — which turns a silently wrong wall into a message
naming the crest ends that disagree. That check is missing, and its absence is most of why this took as
long as it did. Read `Graph::faces` and `split` before writing either of them again.

### What the attempt cost, in bugs that do not crash

Each produced a *plausible* mesh rather than an error, which is the theme:

- **A dart's slot has to be one function.** The `seen` index and the test for the walk's own closing
  are the same number; written twice, one was inverted, and every face came back one crest long.
- **A branch already knows which way its crest runs from the vertex it stands at.** Building the next
  dart as the negation of a branch's direction makes the walk step back the way it came. Same symptom.
- **A closed stretch's simplicity is not the open one's.** An open run of `n` darts stands on `n + 1`
  vertices; a ring of `n` stands on `n`, because its last vertex is its first. Comparing the distinct
  count to `n + 1` refuses the cirque — the very face the check exists to protect.
- **A ring has to be split twice.** Splitting a cirque at its head alone gives two walls that both end
  at the head: `foot == head`, fall zero, floor written flat at summit height with a mouth inside the
  mountain. The arcs run between the ring's **highest and lowest** vertices.
- **The rim is wound opposite to the walk.** A rim written in the walk's own sense comes out a correct
  simple outline of exactly the right area and a *negative* one, which the plan comparison reads as a
  massif that is not there. The tip that starts a basin is the **right** wall's end, not the left's.
- **A tie between two tips is not a tie.** A lollipop hanging off the graph by one vertex makes the
  pocket around it an **annulus**, for which "two walls, a head, a bisector, a mouth" has no meaning.
  Its mouth lands inside the massif, and because welding is by position a misplaced rim vertex glues
  the outline to whatever sample it fell on and shatters the mesh. Tie two **cols**. The `debug_assert!`
  on stretch simplicity now says so in as many words.
- **Several valleys must be split at the col a tie is born at**, for the same reason junctions are: a
  belt changes line only at a line's end.
- **`WELD` says `1.0e-2` and `weld_key` implemented `1/1024`.** Fixed: `WELD` is the one tolerance, the
  cells are that wide, and a lookup measures the 3×3 neighbourhood.

### Still open beside it

- **Confluences.** Every basin drains to the rim on its own floor, so a deep hierarchy combs into
  parallel gullies instead of a tree of valleys. `Station` can name a valley, so a valley-into-valley
  junction is expressible and has never been built. The cirque above is the near case: its floor reaches
  a col and stops, and what it should do then is *continue down the valley that owns the col*. The data
  model already allows it; the walk cannot build it, because at the confluence the trunk's flanking side
  must switch crests and the two do not weld anywhere. The belt straddling it needs **one jump edge** —
  walk the leaving line to its welded end, then a single edge to the to-station's vertex — which is the
  same edge the ending valley's terminal belt already builds, so the surfaces agree.
- **A tie between two tips** (rather than two cols) is where this leaves the scene and `basin.rs`
  begins: cutting an annulus with a second valley has a real design space — which attachment drains,
  where the cut goes — and belongs in the crate where the general half-edge walk will live. Do not build
  annulus cutting scene-side.

# Handoff: the objects the three bamboo worktrees found, without the scenes

Worktree: `./.worktrees/objects`, branch `feature/objects`, on top of `0a7dfe4`.

Second half of the work begun at `a1cd39e`, which took the *framework* the three parallel
attempts at one brief found. This takes the **objects** they found — the land, the walls, the
roofs, the turned vessels — and again none of the scenes.

## What landed

| module | what | from |
| --- | --- | --- |
| `gallery::land` | heights: crest profile, ridge mass, box distance, bench, terrace, slope, a composed valley, an edge taper, a quarter gate | all three |
| `gallery::built` | a wall run, a wall ring, a roof profile, a swept pole, three vessel profiles | m3.1 and dsf |
| `gallery::geometry` | `smoothstep`, a public `Shaded`, a new `quad_normal` | all three |

`geometry` was the prerequisite and is its own commit. Three agents had each written a private
`smoothstep`, which is the evidence it belongs in the gallery; and a composed form is not one
primitive, so it needs to append into a shared run and needs a quad whose normal it states
rather than one derived from its corner order.

## The reasoning is most of what was rescued

The geometry is ordinary. The prose is not, and it is what these modules are mostly made of.
The three that would have died with the branches:

- **A skyline is a line of crests and saddles, not a wander** — and the sawtooth is smoothed
  *before* it is folded, because a bare fold is a triangle wave and a triangle wave along a
  skyline reads as saw teeth rather than as rock. (`crest_profile`)
- **A heightfield's silhouette has to die down to nothing before the mesh stops**, or the far
  ground is a cliff of cut earth standing against the sky, and the only thing that can hide a
  cliff is what is drawn beyond it. Every scene that ends in weather needs this, and a scene
  without it has a stage set standing on a table. (`edge_taper`)
- **High ground in a quarter is a valley; high ground all the way round is a bowl**, and a bowl
  puts its own wall between a camera and its subject on every side. (`quarter`)
- **How far out a mass stands is the whole of what makes it a range rather than a wall** — a
  mass rising from fifty units fills half the frame above the roofline at forty degrees of
  elevation, and forty degrees is a cliff three hundred metres high with a house at its foot.
- **The blend of a cut platform is a smoothstep of the distance, not a mask on the rectangle.**
  A mask stops at the edge, the bank begins as a discontinuity, and a heightfield takes its
  normal from its own height function, so a discontinuity has no normal to give. (`bench`)
- **Five metres of wrinkle is nothing on a hundred-metre face**, which is why the undulation
  is gated by the height already reached. Leaving it in is what makes a mountain read as noise.
- **Tile channels are not geometry.** A tile is a quarter of a metre and the mesh cell is a
  twentieth of the run, so a modelled ripple is sampled twice a cycle exactly at the distances
  a roof is looked at from, and aliases into a moiré. They belong in the surface body, where
  they can be faded out as a tile drops below a pixel. (`roof_ridge`)
- **A lintel must run slightly past its posts**, or two boxes stop on the same plane, leave two
  coincident faces, and the gate pier shows it as a bright vertical line up the corner of the
  opening. **A coping must be wider than its wall**, because a cap flush with its wall is a lid
  and a lid throws no shadow line along the top of the band it caps.
- **A gate is a gap in the run, not a boolean cut**, so there is no interior face to get the
  winding of wrong. The two runs' end faces *are* the reveal.
- **A turned vessel's profile is closed**, so the inside is a real surface and the rim is a lip
  rather than an edge.

## Three defects found, two in the sources and one in the extraction

1. **The roof was upside down.** dsf's `roof_tile` measured the fall as `RISE * (run/length)`,
   which grows *away* from the ridge, and placed the mesh's origin at the ridge height
   (`ROOF_RIDGE` is `PLINTH + WALL_HEIGHT + ROOF_RISE`). So the eaves stood one rise **above**
   the ridge: a trough, not a roof. A heightfield has no way to say which way is up and a roof
   profile naturally reads as "distance from the ridge", so the trap is the framework's and not
   only his. `roof_ridge` measures the fall down from the ridge and says why.
2. **The basket profile was open.** dsf's `BASKET` ended at radius 0.240 rather than on the
   axis, so the doc claiming it was closed was describing something the numbers did not do. The
   landed profile closes with a floor, which is what the doc was about.
3. **Both side walls on both sides.** Found in review of the extraction, not in a source:
   `Valley::height_at` gated `walls[0]` and `walls[1]` on the same `x.abs()` and added them, so
   a point on the west side stood under the east wall too and the two differed only in the
   shape of their noise. Each wall now gathers outward into its own half.

## Deliberately not taken

- **All three scenes**, still. k3 and m3.1 both register the gallery name `bamboo`; dsf
  registers six `courtyard-*`.
- **The WGSL ridge backdrop**, because it is already in `main` — `RIDGE_WGSL`, `ridge_at`,
  `ridges` and `ridge_noise` at `terraces.rs:941`, which all three copied out of `terraces`.
- **dsf's `ground`**, which *multiplies* the land by a ramp rather than easing it to a level.
  Multiplication can only lower land toward zero, so it cannot lift a pad to a height above the
  ground; `bench` eases to a level and can do either.
- **The paddy band** in dsf's `terraced`, which is policy about where a valley is farmed rather
  than a generator, and **m3.1's yard and rough masks**, whose apron width is a scene decision.
- **The veranda, the gate door and the stone bench** as generators. They are one scene's plan
  rather than a form, and `wall_ring` with a gap and a lintel is the whole of what they were.

## State

- [x] `land.rs` and `built.rs`, both compiling and registered
- [x] the wall-gating bug above, found in review and fixed
- [x] `cargo fmt --all` clean
- [x] `cargo test --workspace --features whiting-device/wgpu` — 34 suites, 183 tests, 0
      failures, the two adapter tests included. The count is unchanged from the baseline,
      which is the point: nothing that existed before was altered, and nothing new is called
      by anything.

## Not verified, and this is the real limitation

**Nothing in the tree calls either module.** They compile and are type-checked, and not one of
them has ever been executed: there is no scene in the gallery that builds a valley, a wall or a
roof, because the three scenes that would are not here. A generator that compiles and produces
a degenerate mesh is a generator that compiles.

Two of the three are the better-evidenced half of their merge — the ridge idiom was arrived at
twice independently, and the sources rendered — but the wall run, the ring with its gaps, the
sweep and the roof profile rest on one implementation each, with no frame to check them
against. **Landing one scene is what would exercise all of this**, and it is now much cheaper
than it was: the terrain, the walls and the vessels are generators, so a scene is a placement
document rather than four thousand lines of geometry. This is the strongest argument for
revisiting the "no scenes" ruling, and it is the superior's to make.

The narrower point worth taking from it: a generator with no caller is a generator with no test,
and the framework has no way to notice that either.

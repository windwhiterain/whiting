# Shadow probes: measuring what a per-object shadow density actually buys

Follows [virtual-shadow-maps.md](../proposals/virtual-shadow-maps.md), which carries the design, and
[virtual-shadow-maps.md](../handoffs/virtual-shadow-maps.md), which records what landed.

Status: **all six probes landed and have been run.** `crates/device/examples/shadow_probe.rs` builds
a scene per probe, renders it on a real gpu, and prints the pages, the pass count, the shadow's
extent, its soft share, its leaks and a fingerprint of its pixels. Nothing else in the tree was
touched but one re-export.

## What the probes found

**The density knob reaches the shading, and that had never been shown before.** One rod, four
densities, four different fingerprints: `801fa34b` at 5 and 29 (the same four page grid, so the same
picture, as it must be), `d9fcb271` at 232, `35fc5fd0` at 464. The shadow's **extent is 312 pixels in
all four**, which is the invariant the design promises and the reason the density is safe to expose:
the chain changes how sharp a shadow is and never whether there is one.

**A caster shorter than the receiver's normal offset casts nothing, and the offset is 1.6 world
units.** `shadowed_pages` samples the shadow at `world_position + normal * 1.6`, so on a flat floor
the lookup point is in the air a unit and a half above the ground, and a caster below that height is
not between the lamp and the point being sampled. Measured on one box at one standing, one density
and one grid, varying only its height:

| height | shadow px | soft | pixels |
| --- | --- | --- | --- |
| 1.3 | 665 | 387.1% | `a6babaef` |
| 1.7 | 912 | 417.2% | `b53a77ec` |
| 2.1 | 749 | 627.4% | `cc7f62da` |
| 3.4 | **3024** | **76.5%** | `c1b9a4bc` |

The break sits where the number sits, and the 3.4 frame has a shadow a person sees at a glance
(`out/probe/probe6-wall-height-34.png`) while the 1.3 frame has none at all.

**`spotlight` itself has no cast shadow, and never did.** Rendered by `render` on this machine
(`out/probe/spotlight-check.png`, 251 passes), its three boxes stand in the pool and cast nothing.
This is the scene the virtual shadow map was verified against, and it matched the pre-work frame to
a mean of 0.009 of 255 — which is what a scene with no cast shadow in it matches at. The work
faithfully reproduced a frame that was already missing its shadows, and no control noticed, because
every control asked whether two paths **agree** and none asked whether either one draws a shadow.
`spotlight`'s own boxes are 1.3, 3.4 and 4.6 units tall, so two of them clear the offset and still
cast nothing: the height is a real part of the mechanism and it is **not the whole of it**, and the
difference between a box that casts in this probe's scene and a box that does not in `spotlight` is
still open. The two scenes differ in their caster set and in their floor's density, and that is
where the next session should look.

**A caster's declared level does not change its own shadow, and this is the finding that matters
most.** With the grid held at thirty two pages a side by a caster standing four units away, the wall
walked 232 → 116 → 58 → 29 — every level of the pyramid — and the pass count fell from **661 to 471**
and the page count from **593 to 439**, while the four frames came back **byte for byte identical**
under both cameras (`a2d56fd4` four times, `d7d30b02` four times). The allocation is doing exactly
what `caster_level` says and the picture is not moving.

The likely reason is the resolution rule rather than an arithmetic slip: a fragment walks finest to
coarsest and stops at the first level that has a page, and a page is drawn with **every** caster
whose projection covers it. A fine page allocated for one caster therefore also carries its
neighbours, and a neighbour's shadow is found at the neighbour's resolution whatever the neighbour
asked for. **A density decides the finest page a caster asks for; it does not decide how finely its
own shadow is recorded wherever a finer page already exists.** That is the same measurement with the
holder standing close enough to reach, and it came back identical, so the confound was removed before
the claim was made — but removing the confound did not change the answer, and a probe that reports
"no difference" when there is none is the only kind worth having.

**Pages follow the camera, and that half works.** The camera turned away from the whole scene
allocated **no pages at all** and fell to 8 passes; the camera that sees a corner of it allocated 50
to 69 pages against the square-on camera's 63. The offstage caster could not be put out of frame by
any of the three cameras tried, and the probe says so on every line rather than reporting a number
about a setup it never achieved.

**No leaks.** Zero at every frame, at every density, square on and at a grazing angle. The `viewport`
and the per-page PCF clamp both hold.

**The budget cliff is where the handoff said, and it is a cliff.** With the ceiling left at its
default of sixteen pages a side, a single caster is refused above a hundred and sixteen: 352 pages
against 256 slots. Raised to sixty four, the ladder runs to 464 — 1526 pages, an atlas forty to a
side, **1568 passes for one lamp** — and 928 is refused at 5446 pages. The cost of the finest
setting is a thousand and a half passes for one light, and that number is the honest answer to
"what does the resolution cost".

## The control every number is read against

`spotlight`, compiled and not drawn, reports **8 pages a side, 4 levels, 85 pages allocated, an atlas
10 to a side, 252 passes** — the numbers the landed work recorded. A probe that cannot be reconciled
with those is measuring something other than what it says, and one run of this harness could not be.

## The three ways this probe was wrong before it was right

Recorded because each cost a full run and each is a mistake a probe can make quietly.

1. **The receiver was sizing the map.** A caster naming no density is given the scene's default, and
   the default is high, so a floor eighteen units across demanded a grid sized for *its* shadow
   resolution and every density above 58 was refused for pages the floor had asked for. The fix is
   the documented one: `shadow_density: 0` puts a caster in the coarsest level and out of
   `max_density`.
2. **The soft share was measuring the lamp.** Normalising a whole row of floor against its own
   darkest and brightest put the cone's falloff where a shadow edge should have been, and every run
   came back above a hundred percent. A shadow's contrast has to be taken against the lit floor
   *beside* it, and the shadow's own region has to be found by arithmetic — a box's top face
   projected from the lamp — rather than by a threshold.
3. **The casters were hiding their own shadows.** The lamp throws shadows away from this camera, so
   a wide caster standing where a thin one would stand hides all of it, and the probe measured a
   thousand dark pixels in a frame with no shadow in it: the dark pixels were the far rim of the
   cone. Every caster now stands where one of `spotlight`'s own boxes stands, because that scene's
   casters are the only ones in the tree known to be visible from this camera.

## Files this work owns

- `crates/device/examples/shadow_probe.rs` — new, the whole harness.
- `crates/device/Cargo.toml` — the `[[example]]` entry with `required-features = ["wgpu"]`.
- `crates/scene/src/standard/mod.rs` — one word, re-exporting `SHADOW_DENSITY` so the probe does not
  write the field's name a second time.
- `out/probe/**` — the frames, which `out` already ignores.

## What the next session has to decide

- **Whether the level result is a defect or a decision that was never written down.** If a caster's
  level is not meant to decide its own shadow's resolution, then §3 of the proposal is claiming
  something the design does not do, and the proposal wants rewriting rather than the allocator
  wanting fixing. If it is meant to, the resolution rule wants changing — a fragment should prefer
  the level its **caster** was recorded at over the finest page that happens to be there.
- **Why `spotlight` casts nothing from boxes that clear the offset.** The height is one mechanism and
  this scene says it is not the only one. Its casters and its floor's density are the difference from
  the probe's scene, and the frame is the evidence.
- **Whether the harness earns its place in the tree.** Six probes that report are a report; six that
  hold a line are a test. The natural next step is a `crates/device/tests/` file that runs the two
  claims with hard thresholds — the extent not moving as density moves, and a caster's shadow
  appearing at all — and skips with a printed note on a machine with no gpu.

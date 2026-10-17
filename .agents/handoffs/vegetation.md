# Handoff: `vegetation` — P1 landed, P2's hand in flight

Started 2026-10-17. Worktree `.worktrees/vegetation`, branch `feature/vegetation-p1`. The design is
[proposals/agentic-vegetation-generation.md](../proposals/agentic-vegetation-generation.md); this file is only
the state a next session needs, and it is deleted when the crate's design phases are done.

## Where the tree stands

- **P1 is on `main`** (`140af41` and later): `handle`, `rng`, `node`, `plant`, `leaf`, `expand`, `derive`, and
  three probes — `tree_probe` (5 rows), `plant_probe` (8), `leaf_probe` (8). One plant renders: `gallery`'s
  `bamboo` scene, 13 passes, every blade buffer 24 rows of 168 bytes, so the leaves are one instanced pass.
- **P2's first slice is on this branch**: `ops` — the edit table of V5 with four of its six kinds — the brush
  (`selector` + `ops::paint`), and their probes: `edit_probe` (11 rows) and `select_probe` (8). `shape` gained
  `Scalar::offset`, with its own row in `scalar_probe`.
- **P2's second slice is on this branch**: `species` — a species as a rule and the seed that fills it in — and
  `species_probe` (6 rows). `derive::bamboo` is now `grow(&bamboo_rule(), seed)`, **and every P1 probe passes
  unchanged against the grown tree**, which is the reading that the rule says what the author typed; `derive::shrub`
  is the rule's control, with computed limbs and radii that are fractions of what carries them.
- **In flight**: nothing. The brush (`selector.rs` + `select_probe`) and the generator (`species.rs` +
  `species_probe`, with `derive::shrub` as its control) are here too.
- **The generator changed the authorship and not the plant**: the `bamboo` scene still renders to the same
  sha256 after `derive::bamboo` became `grow(&bamboo_rule(), seed)`.

## What a consumer has

```rust
let species = derive::bamboo(seed);             // Species { plant, templates, fields }
let species = derive::shrub(seed);              // the same, grown from a rule that leaves numbers to the seed
let mut plant = species.plant;

for (handle, node) in plant.walk() { }          // every node, tombstones included: addressing
for (handle, node) in plant.live() { }          // what the plant draws
plant.get(&handle);  plant.get_mut(&handle);    // Ok(&Node) | Err(HandleOrphaned { from, index })
plant.placement(&handle);                       // the world frame, composed from the root down
plant.surface(&handle, attachment);             // a frame on the node's own skin
plant.spine(&handle);                           // the node's spine in the world

// A species is a rule: the trunk, and one generation of children below it.
let plant = species::grow(&rule, seed);         // deterministic in (rule, seed), node for node
let grafted = species::graft(&mut plant, &derive::bamboo_rule(), &bough, 0.2, 0.75);  // one more of the rule's

// The crown a plant may reach, and the trim that fits it there without losing a number.
let crown = envelope::Envelope::radial(&[(0.0, 0.55), (0.5, 0.45), (1.0, 0.35)])
    .with_section(shape::Section::ellipse(1.3, 1.0));
crown.outside(&plant, [x, y, z]);                  // 0 inside, the radial excess beyond it
let trim = envelope::trim(&mut plant, &crown);     // Trim { trimmed: Vec<(Handle, f32)>, pressed: Vec<(Handle, f32)> }

// What the plant arrived at: counts, and the crown against the envelope in the trunk's own fractions.
let report = report::report(&plant, Detail::DEFAULT, &species.fields, Some(&crown));
report.worst_exceedance();                         // zero or negative when the plant fits

let expansion = expand::expand(&plant, Detail::DEFAULT, &species.fields);
expansion.wood;                                 // one Wood { handle, loft } per live node
expansion.leaves;                               // one LeafGroup { node, template, leaves } per node

// Small authored things on a node's surface: a knot, a thorn, a fruit. No angle is expressible anywhere.
let knot = decorator::Template { sweep, stations: 7, around: 12 };
let seat = decorator::Seat::new(bough, 0, 0.1, slot).sunk(0.01).scaled(1.0);
let placed = decorator::placed(&plant, &[seat]);   // Placed { node, template, placement, scale }

// The forces that shape a grown plant: gravity, a light, a wind. `amount` is a fraction of each spine's length.
let plant = plant.with_fields(vec![field::Field::gravity(0.25), field::Field::light([1.0, 0.0, 0.0], 0.15)]);

// An obstacle is a signed distance in the world: NEGATIVE INSIDE. `1.0` puts a spine on its surface.
let wall = |point: [f32; 3]| 0.55 - point[0];     // solid where x > 0.55
let plant = plant.with_fields(vec![field::Field::obstacle(wall, 1.0)]);
let contacts = contact::collide(&plant, wall, Detail::DEFAULT);   // deepest first, spine samples only

// What a crown can afford: a sibling whose tip is closer than `spacing` to one already kept is pruned.
let taken = thin::thin(&mut plant, 0.08);         // Thin { pruned: Vec<Handle> }
// …or the same species under a scene's weather, which changes nothing but the fields.
let indoors = derive::bamboo(seed).with_fields(vec![field::Field::light([1.0, 0.4, 0.0], 0.1)]);

let report = ops::apply(&mut plant, &[
    Edit::new(bough(2), Change::Offset { property: Property::Station, by: 0.1 }),
    Edit::new(bough(1), Change::Spine { jets }),
    Edit::new(bough(0), Change::Prune { reason: Prune::ByAuthor }),
]);
report.applied; report.problems;                // what landed, and per edit that did not, why

// The brush writes that list: one edit per handle a selector names, in walk order.
let edits = ops::paint(&plant, &Selector::Generation(2), Change::Offset { property: Property::Radius, by: 0.001 });
```

## What P2 still owes

Nothing. `reference` on a leaf field landed in round 16 (`LeafField::reference`, SpeedTree's five anchor levels as one
integer), and the list this section used to carry was closed item by item: `Change::Graft` became `species::graft` (2),
`Change::Curve` (15), decorators and their sockets (5), the gravity/light/obstacle fields (4 and 6), the envelope as a
trimming law (4), the collision report and `derive::thin` (6), `--region` (12), the segmentation scene (13),
`report()` (4), seating (10), and the promotion of the five gallery generators (14).

**The implementation is finished.** P1, P2 and P3 are all landed; the next step is the overall review — every probe,
every scene, the SPEC's alignment — and then this handoff is deleted, as it should be.

## Traps

- **A scene's sky must be an image and one `environment_light`, not a shader.** A path tracer has no shader stage:
  the plant scenes drew black backgrounds until they adopted `air::Atmosphere` (one struct emits the grade's helpers
  and the 512x256 latlong map, and the light's radiance is the raster ambient *and* Cycles' `background` strength). A
  hand-written `sky_at` is a sky only one backend can see.

- **The ground is one function**: the scene draws its floor from it *and* seats every planting on it (Ground::Fielded /
  egetation::ground::seat). A second ground function is the defect a seated plant exists to rule out. Framing::ground is
  Ground::Plane for the five single-plant scenes, so a plane's seating is the identity and their hashes do not move.

- **The wood is one mesh per scene, not per planting.** A mesh per plant is a pass per plant (the emitter groups by
  mesh) and the ids live in the corners, so merging the geometry does not merge the names. A planting with **no**
  template is legal — a tuft's blades are its wood — and the builder's lade_of is an Option because of it.

- **A picture's hash is a pixel-scale instrument.** Rewriting the field's normalisation from *rotate then normalise*
  to *normalise then rotate* — the same law to a rounding — changed the windy frame's hash (`1A3ACC93…` →
  `D7488DF2…`) while every probe stayed green. Use a probe for meaning and a hash for *"did anything move at all"*.
- **A planting's ids are `base + node`, and planting 0's base is 1.** A resolver that refuses ids below
  `PLANTING_STRIDE` refuses every node of the first plant; only `NOT_AUTHORED` (zero) is not a node.
- **A plant is not instanced; its leaves are.** A grove is one wood mesh per plant and one blade mesh per distinct
  template, so six plantings draw their leaves in two instanced passes per stage, not six.

- **The occupancy's sign is the whole contract: negative inside.** The first version of `obstacle_probe` wrote
  `x − face`, which is positive inside, so the field pushed the boughs *deeper into the wall* and the row passed
  anyway ("47 → 35 contacts"). A scene that passes the inverted function pushes plants into walls and nothing
  complains.
- **One push is not a solver.** The weight is `t²`, so a knot mid-spine is pushed a quarter of what a tip is, and
  the contacts that remain after a field are the author's to read (`contact::collide`), not a failure of the push.
- **`thin` compares siblings only**, keeps the first of a crowding group, decides on every tip before any prune
  lands, and prunes with `Prune::Thinned` through `ops` — so a second call finds nothing to do.

- **A decorator's frame is the node's surface frame taken whole**, and `Seat` carries no angle: a decorator that
  needs a roll is a decorator whose *template* should be authored rolled. A seat on a tombstone draws nothing,
  which is the one place a decorator can be absent without an error.
- **`Curve::closest` pins the distance exactly and the fraction only as well as the distance is flat.** Near a foot
  the distance is quadratic in `t`, so `f32` stops resolving the fraction at about `1e-4` on a metre-scale curve.
  An envelope's `outside` therefore reads a few millionths on its own boundary, and a probe that demanded an exact
  zero there would be demanding something `f32` cannot give.
- **The trim writes length nudges through `ops`**, so a trimmed plant is an edited plant: every amount is a
  `Change::Offset` at a handle. It runs on a **linear** model of the tip, which is exact without fields and
  approximate with them, and then re-reads every tip — that is why `Trim::pressed` exists.
- **A trim cannot make a branch vanish.** V4's sentence taken literally took a bough from 1.054910 m to
  0.000007 m; a node that would have to go below a tenth of its length is pressed and reported instead.
- **The trunk is not trimmed**: it is the envelope's own axis, so shortening it would move the crown rather than
  fit it.
- **A field is applied where a spine is drawn, and only there.** `Plant::shape` folds the plant's `fields` over
  `Node::spine_now`, and `spine`, `surface` and `placement` all read it — so a field bends a parent and its
  children follow. A new reader that reaches for `Node::spine_now` directly bypasses every field.
- **Only the component across a spine bends it.** A push along a trunk is a *compression*: the first version
  displaced every knot along the field and moved an upright trunk's tip 15% of its reach downward instead of
  bending it. The amount is a **fraction of the spine's own length**, so a droop is the same shape at every
  scale — an absolute push folded a 16 cm twig and broke a closed solid.
- **A baked spine is immune to fields**, and `Plant::shape` is the one place that reads `Node::baked`.
- **The shrub's rule carries `Field::gravity(0.25)`**; the bamboo's carries none, so the bamboo's picture is
  bit-identical to before fields existed.
- **A species is a rule, and the seed's draw order is data.** `species::grow` draws, per parent: the count (only
  when its range can vary), then per child the station jitter and the slot jitter — **unconditionally, even when
  the jitter is zero** — then whatever the generation's shape function draws. Reordering those draws changes
  every plant a rule makes.
- **A rule may leave nothing to chance**: bamboo's counts, bands and jitters are fixed and its shapes draw no
  numbers, so its seed reaches the leaf field and nothing else. Two bamboo seeds are the same tree with two leaf
  layouts; two shrub seeds are two shrubs.
- **A graft is the generator's operation, not an edit.** `species::graft(plant, &rule, at, station, slot)` needs the
  rule that made the tree, so it cannot be a `Change` — an edit is a handle and a change, and carrying a species in
  every edit is what the layering exists to avoid. The generation is the parent's own depth, and the new node's
  shape draws from **its own** forked stream, so a graft is as deterministic and as local as a grown child. The
  leaf field sees it at once: a grafted branch took **9** leaves with no extra call.
- **A prune is a tombstone, and both walks exist for a reason.** `walk` names tombstones so a handle goes on
  naming what it named; `live` skips them *and their subtrees*. `expand::wood` reads `live`. A new reader that
  takes `walk` will draw deleted branches.
- **Prunes outrank offsets in the same batch.** The order is enforced in `ops::apply`, not left to the caller's
  list order: an offset aimed at a node the same batch prunes is reported, not written.
- **An offset is added at use, never written into the authored data.** That is why `expand` reads
  `taper_now`, `Plant::spine` reads `spine_now`, and `Plant::placement` reads `effective_attachment`. A new
  reader that reaches for `taper`, `spine` or `attachment` bypasses every edit silently — that was a real
  defect: the first `expand::wood` read `taper` and a radius nudge moved nothing.
- **A shift is stored in the node's own frame.** `Frame::Local` is that frame already; `Parent` and `World` are
  undone by one rotation each. For a child of the root, `Parent` and `World` are the same frame, because the
  root's placement *is* the identity — a reading, not a defect.
- **Only `Station`, `Slot` and `Length` are range-checked.** A radius offset can fold a section through its
  spine; nothing refuses it yet, and a validation pass is where that belongs.
- **A bake replaces; a nudge adds on top.** The stages run prunes, then bakes, then nudges, so a batch may bake
  a spine and thin it in one go. `Node::baked` is the marker a field must read before reshaping anything, and it
  is set only by `Change::Spine`.
- **A brush edits nothing.** `Selector` answers `matches` and `handles`; `ops::paint` is the only bridge, and it
  goes one way — a selector writes the edit list that `apply` reads. A predicate that tried to change a plant
  would be a second editing path, which is what the design exists to avoid.
- **The design's vocabulary is a contract**: depth 0 trunk, 1 bough, 2 branch, 3 twig, 4 sprig (`Handle`'s
  `NAMES`). `derive::bamboo`'s second level is a *branch*, and the code said "twig" until a probe printed
  `trunk/bough 0/branch 1` beside prose that disagreed with it.
- **`Scalar::offset` keeps every slope, to a rounding.** The slope is evaluated from the shifted knots, so it
  agrees to ~4e-7 rather than exactly; `scalar_probe`'s row says so.
- **A template's frame is its midrib's initial normal**, and a flat section's thin axis is the frame's
  *binormal*: the leaf template states it once (`+y`), and the gallery's `sweep` scene hit the same rule from
  the other side.
- **A leaf's `scale` is in the template's own frame**: `placement.place(point · scale)`. The other order moves
  the anchor by `(1 − scale) · |origin|` — 1.362 m on this plant's topmost leaf.
- **The emitter groups a pass by shader, mesh and images — not by material.** Six `LeafGroup`s of one template
  are one pass over one instance buffer.

## How to verify

```text
cargo run -p whiting-shape --example curve_probe           # 12 invariants, closest included
cargo run -p whiting-shape --example section_probe         # 8 invariants, 
each included
cargo run -p whiting-shape --example scalar_probe          # 8 invariants, the offset row included
cargo run -p whiting-vegetation --example tree_probe       # 5
cargo run -p whiting-vegetation --example plant_probe      # 8
cargo run -p whiting-vegetation --example leaf_probe       # 8
cargo run -p whiting-vegetation --example edit_probe       # 8, the hand
cargo run -p whiting-vegetation --example select_probe     # 8, the brush
cargo run -p whiting-vegetation --example species_probe    # 10, the generator and the tuft
cargo run -p whiting-vegetation --example field_probe      # 8, the fields
cargo run -p whiting-vegetation --example envelope_probe   # 6, the envelope, the trim and the report
cargo run -p whiting-vegetation --example decorator_probe  # 6, the decorators
cargo run -p whiting-vegetation --example obstacle_probe   # 6, the obstacle, the contacts and the thinning
cargo run -p whiting-vegetation --example sculpt_probe     # 7, the hand's whole vocabulary on one plant
cargo run -p whiting-vegetation --example ground_probe     # 6, seating and the ground
cargo run -p whiting-device-rasterize --example render -- bamboo out/bamboo
cargo run -p whiting-device-rasterize --example render -- bamboo_wind out/bamboo-wind
cargo run -p whiting-device-rasterize --example render -- bamboo_crown out/bamboo-crown
cargo run -p whiting-device-rasterize --example render -- bamboo_knots out/bamboo-knots
cargo run -p whiting-device-rasterize --example render -- grove out/grove
cargo run -p whiting-device-rasterize --example render -- sculpted out/sculpted
# Which node a pixel is, and which nodes a rectangle covers: both ask `gallery::locate::NAMINGS`.
cargo run -p whiting-device-rasterize --example render -- bamboo out/located --locate 600,300
cargo run -p whiting-device-rasterize --example render -- grove out/selected --region 400,200,1000,900
# The segmentation view: one colour per element, and --region names what you see.
cargo run -p whiting-device-rasterize --example render -- segmentation out/segmentation --region 300,200,900,1200
# The same document through the path tracer: the sky comes from the environment map, not a shader.
cargo run -p whiting-device-cycles --example render_cycles -- bamboo out/cycles-bamboo.png 64
cargo run -p whiting-device-rasterize --example render -- sweep out/sweep
```

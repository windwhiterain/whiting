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
- **In flight**: nothing. `selector.rs` and `select_probe` are here too (merged from `feature/selector`).

## What a consumer has

```rust
let species = derive::bamboo(seed);             // Species { plant, templates, fields }
let mut plant = species.plant;

for (handle, node) in plant.walk() { }          // every node, tombstones included: addressing
for (handle, node) in plant.live() { }          // what the plant draws
plant.get(&handle);  plant.get_mut(&handle);    // Ok(&Node) | Err(HandleOrphaned { from, index })
plant.placement(&handle);                       // the world frame, composed from the root down
plant.surface(&handle, attachment);             // a frame on the node's own skin
plant.spine(&handle);                           // the node's spine in the world

let expansion = expand::expand(&plant, Detail::DEFAULT, &species.fields);
expansion.wood;                                 // one Wood { handle, loft } per live node
expansion.leaves;                               // one LeafGroup { node, template, leaves } per node

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

- **`Change::Curve`** (a law added to a law) and **`Change::Graft`** — the last needs `GenerationId`, which
  arrives with the generator.
- **Decorators and their sockets**, the four fields (gravity, ancestor, light, obstacle), and the envelope as a
  trimming law.
- **The collision report** and `derive::thin`.
- **`--region`** in the render harness, and the segmentation scene.
- **`report()`** — the crown against the envelope's own radius curve, the counts, the instance count.
- **`reference` on a leaf field** — needs a closest-point query (`shape`'s open question 4).
- The promotion of `gallery::geometry::{culm, blade, double_blade, leaf_spray, tuft}` into `derive`.

## Traps

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
cargo run -p whiting-shape --example scalar_probe          # 8 invariants, the offset row included
cargo run -p whiting-vegetation --example tree_probe       # 5
cargo run -p whiting-vegetation --example plant_probe      # 8
cargo run -p whiting-vegetation --example leaf_probe       # 8
cargo run -p whiting-vegetation --example edit_probe       # 8, the hand
cargo run -p whiting-vegetation --example select_probe     # the brush, once it lands
cargo run -p whiting-device-rasterize --example render -- bamboo out/bamboo
cargo run -p whiting-device-rasterize --example render -- sweep out/sweep
```

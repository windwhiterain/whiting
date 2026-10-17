# Handoff: `vegetation` P1 — the tree, its wood, and the leaf field in flight

Started 2026-10-17. Branch `feature/vegetation-p1`, worktree `.worktrees/vegetation`, from `main` at
`997b45e`. The design is
[proposals/agentic-vegetation-generation.md](../proposals/agentic-vegetation-generation.md); this file is
only the state a next session needs, and it is deleted when P1 is finished.

## Where the tree stands

- **`shape` P1 is on `main`** (`81358f0`): `math`, `curve`, `scalar`, `section`, `sweep`, `loft`, `mesh`, and
  five probes reading 38 invariants. Everything this crate draws with is there.
- **On this branch**: `handle` (a node's derivation path), `rng` (one stream per node, forked by path),
  `node` (a spine, a taper, a section, and three numbers that attach it), `plant` (the root, the walk, the
  placement, the world spine), `expand` (one closed solid per node, one resolution dial), and two probes:
  `tree_probe` (5 rows) and `plant_probe` (5 rows, wood only).
- **In flight**: `feature/vegetation-leaves`, worktree `.worktrees/leaf`, branched from `6addeb3` —
  `leaf.rs` (`LeafTemplate`, `LeafField`, `LeafGroup`, `LeafInstance`) and `leaf_probe`.

## What a consumer has

```rust
let mut root = Node::root(culm_spine, taper_law);
root.push(Node::attached(bough_spine, bough_taper, Attachment::new(0.55, 0.0).sunk(0.004)));
let plant = Plant::new(root, seed);

for (handle, node) in plant.walk() { /* … */ }        // depth first, children in order
plant.get(&handle);                                   // Ok(&Node) | Err(HandleOrphaned { from, index })
plant.rng(&handle).unit();                            // the node's own numbers
plant.placement(&handle);                             // the node's world frame (Placement { origin, x, y, z })
plant.surface(&handle, attachment);                   // a frame on the node's own skin
plant.spine(&handle);                                 // the node's spine in the world

let solids = expand::wood(&plant, Detail::DEFAULT);   // one Wood { handle, loft } per node
mesh::signed_volume(solids[0].loft.view());           // every loft is a closed solid
```

`Attachment { station, slot, sink }` is the whole of a joint: where along the parent, where around its
section, and how far the child's origin sinks **inward along the parent's outward direction**.

## What P1 still owes

- **The leaf rows in `plant_probe`** — the field's groups, the instance count, and the statement that one
  group is one instanced draw. The leaf module is in flight; the rows land when it merges.
- **A rendered plant.** P1's exit asks for *one rendered plant whose leaves are instances of one template
  drawn in one pass*. Two gallery pictures exist or are in flight: `sweep_picture` (a tube, a blade and an
  envelope from one `Sweep` call, which is `shape`'s exit) and, after that, a plant.
- **`report()`** — P1 names it (the crown against the envelope's own radius curve, the counts, the instance
  count). The probes print counts; there is no `report()` type yet, and the envelope is P2's field.
- **P2's edits.** A node's attachment is set when it is built and cannot be moved afterwards, which is
  deliberate — the design's edits are *replacements* (D10) and a moved bough will be a rebuilt one. If a
  selector has to retarget an existing node, it needs `Plant::get_mut` plus a stated replace semantics, and
  that is P2's decision rather than a convenience to add here.

## Traps

- **A node's coordinates are its parent's attachment frame**: `x` outward, `y` along the parent toward its
  tip, `z = x × y`. World-space authoring is deliberately not expressible; a child of the root is the only
  thing authored in world coordinates, and only because the root's placement is the identity.
- **`Plant::placement` is `O(depth²)`** and `expand` asks for `plant.spine` per node, which recomputes it.
  Ten nodes do not notice; a plant of hundreds wants a placement stack walked once, and that is a change to
  make with a measurement rather than in advance.
- **A zero taper leaves the outward direction undefined.** `Plant::surface` falls back to the section's own
  `+x` (the frame's normal) and its doc says so; a child attached at a tip therefore faces the same way the
  spine does.
- **The rng draw order is data**: the node's phase is drawn first, then one jitter per leaf in index order.
  Reordering the draws moves every leaf in the plant, which is why it is stated here and in `leaf`.
- **`Detail` is the only resolution.** `Detail::scaled` is a probe's convenience; a scene that wants a
  coarser preview expands the same plant at a smaller dial.

## How to verify

```text
cargo run -p whiting-vegetation --example tree_probe     # 5 invariants
cargo run -p whiting-vegetation --example plant_probe    # 5 invariants, wood
cargo run -p whiting-vegetation --example leaf_probe     # the leaf field, once it lands
cargo check -p whiting-vegetation --all-targets
```

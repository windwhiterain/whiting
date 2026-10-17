# Handoff: `vegetation` P1 — the tree, its wood and its leaf field

Started 2026-10-17. Branch `feature/vegetation-p1`, worktree `.worktrees/vegetation`, from `main` at
`9b860e1`. The design is
[proposals/agentic-vegetation-generation.md](../proposals/agentic-vegetation-generation.md); this file is
only the state a next session needs, and it is deleted when P1 is finished.

## Where the tree stands

- **`shape` P1 is on `main`** (`81358f0`): `math`, `curve`, `scalar`, `section`, `sweep`, `loft`, `mesh`, and
  five probes reading 38 invariants. Everything this crate draws with is there.
- **On this branch**: `handle` (a node's derivation path), `rng` (one stream per node, forked by path),
  `node` (a spine, a taper, a section, and three numbers that attach it), `plant` (the root, the walk, the
  placement, the world spine), `expand` (`wood` and `expand`, one closed solid per node, one resolution dial),
  `leaf` (`LeafTemplate`, `LeafField`, `LeafGroup`, `LeafInstance`), and three probes: `tree_probe` (5 rows),
  `plant_probe` (7 rows, wood and leaves) and `leaf_probe` (8 rows).

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

let template = LeafTemplate::blade(midrib, width_law, thickness, 7, 8);
let mut field = LeafField::new(2, 0);                 // generation 2, template 0
field.band = (0.25, 0.9); field.jitter = 0.15;

let plant_geometry = expand::expand(&plant, Detail::DEFAULT, &[field]);
plant_geometry.wood;                                  // one Wood { handle, loft } per node
plant_geometry.leaves;                                // one LeafGroup { node, template, leaves } per node
mesh::signed_volume(plant_geometry.wood[0].loft.view());   // every wood loft is a closed solid
```

`Attachment { station, slot, sink }` is the whole of a joint: where along the parent, where around its
section, and how far the child's origin sinks **inward along the parent's outward direction**.

## What P1 still owes

- **`report()`** — P1 names it (the crown against the envelope's own radius curve, the counts, the instance
  count). The probes print counts; there is no `report()` type yet, and the envelope is P2's field.
- **`reference` on a leaf field** — the one orientation term P1 did not implement, because it needs a
  closest-point query on an ancestor and that is the query `shape` has not built (its open question 4).
- **P2's edits.** A node's attachment is set when it is built and cannot be moved afterwards, which is
  deliberate — the design's edits are *replacements* (D10) and a moved bough will be a rebuilt one. If a
  selector has to retarget an existing node, it needs `Plant::get_mut` plus a stated replace semantics, and
  that is P2's decision rather than a convenience to add here.

**The rendered plant is done**: `gallery`'s `bamboo` scene draws `derive::bamboo` — the wood as one mesh
carrying per-point `authored_id`s, the blade as one mesh instanced once per leaf — and

```text
cargo run -p whiting-device-rasterize --example render -- bamboo out/bamboo
```

writes a 1200×1400 frame in 13 passes. **The leaves are one pass over one instance buffer**, and the evidence
is the emitter's own documents: every blade buffer is 4032 bytes of instance rows, which is 24 rows of 168 — so
`k` leaves cost one buffer and one lighting pass, not `k` draws. `vegetation_mesh.rs` is the wood's mesh and its
identity, `vegetation_instancing.rs` the leaf frame's conversion, `vegetation_picture.rs` the scene.

The crown is **sparse on purpose at this size**: three boughs, two twigs each, nine 11 cm blades a twig. A fuller
crown is a species edit — `derive::bamboo`'s twig count and `leaf::LeafField::density` are ordinary numbers —
and not a mechanism the picture is missing.

## The gallery's pattern, for the plant picture

`gallery` builds **documents**; it does not render them. The one renderer is
`crates\device-rasterize\examples\render.rs`:

```text
cargo run -p whiting-device-rasterize --example render -- <scene> <out> [--locate x,y] [--where name] [--debug channel] [--census]
```

which draws `Gallery::build(name)` and writes `<out>.png`, `<out>-manifest.json` and the id sidecar. A scene is
a module in `crates\gallery\src\` with a `pub fn <name>() -> Value`, registered in `Gallery::build` — follow
`sweep_picture.rs`, which is the smallest complete one.

- **`Gallery::build` and `Gallery::names` are two lists, and the second is not a shorter first.** `names()` is
  what `wgsl_validation`, `gallery_render` and `scene`'s `clustered_lights` hold themselves to, and
  `clustered_lights` pins a **pass count per scene**, so adding a name there is a change to three suites.
  Unlisted scenes are the house pattern (`workshop` and `yard` are buildable and unlisted), and the harness's
  usage text does not enumerate names.
- **That pass-count test is already red on `main`**, for the seven scenes `882c0d8` added and the table never
  learned (`hall`, `hall_large`, `house`, `identity`, `shed`, `terrace`, `wing`). It is not any one slice's
  doing and it is not P1's to fix; know it before reading a red suite as your own.
- **`gallery` now depends on `shape` as a normal dependency**, because a module in `src\` is the library and a
  library cannot reach a dev-dependency. That is the design's migration row 3 (*`gallery` depends on `shape`
  for the curve*) and it is met; `vegetation` will be the next such edge, for the leaf instances.
- **A `Triangles` mesh is a flat soup** — three shaded points per triangle, no indices — so `Loft`'s indices
  are consumed at the boundary and not passed on.
- **The `image` field is a marker, not a row field.** The shader's declared `custom_schema` carries it; the
  vertex attribute list must come from `gallery_custom_schema()` *without* it, or every later field shifts a
  location and the frame is refused with a message about `metallic` at vertex location 6.

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
- **The emitter groups a pass by shader, mesh and images — not by material.** So six `LeafGroup`s of one
  template draw in **one** pass over **one** instance buffer, and their six materials are the *rows'*
  `authored_offset`, not six buffers. Giving each group its own mesh "to match the brief" would have made six
  meshes and six passes, which is the cost the design exists to avoid.
- **A leaf's `scale` is in the template's own frame**: `placement.place(point · scale)`. The other order moves
  the anchor by `(1 − scale) · |origin|`, measured at **1.362 m** on this plant's topmost leaf, and a picture
  cannot show it because the blade stays the right size while sitting in the wrong place.
- **A template's length is in its own coordinates**, so a species' `size` law multiplies it: `0.05` on a
  0.31 m blade is a 1.5 cm fleck. The first species asked for exactly that.
- **A template's frame is its midrib's initial normal.** A blade's width must land on the template's `y` and
  its thickness on `z`, so the midrib's initial normal is `+y`; a `+x` midrib's default is `−z`, which stands
  the blade on its edge. `LeafTemplate` states it once, and `leaf_probe` measures `|y| 0.494932` against
  `|z| 0.009924` — a transposed section would swap two numbers that both look plausible. **The gallery's
  `sweep` scene hit the same contract from the other side**: a flat section's thin axis is the frame's
  *binormal*, so its first attempt with `+z` drew a green sliver lying on the floor. Two independent slices,
  one rule — which is why it is written here rather than in either slice's own notes.
- **One twist is a quarter turn of the *face* onto the negated side**, not onto the side: the right-handed
  formulas `side' = side·cos + face·sin`, `face' = face·cos − side·sin` are what keep `(forward, side, face)`
  right-handed, and `leaf_probe` reads `4.425e-8` and `4.284e-8` for the two halves of that statement.
- **`Detail` is the only resolution.** `Detail::scaled` is a probe's convenience; a scene that wants a
  coarser preview expands the same plant at a smaller dial.

## How to verify

```text
cargo run -p whiting-vegetation --example tree_probe     # 5 invariants
cargo run -p whiting-vegetation --example plant_probe    # 7 invariants: wood and leaves
cargo run -p whiting-vegetation --example leaf_probe     # 8 invariants of the leaf field
cargo check -p whiting-vegetation --all-targets
```

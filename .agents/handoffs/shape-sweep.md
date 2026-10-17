# Handoff: the `shape` crate — P1 landed, and what a consumer can now ask it

Started 2026-10-17. Branch `feature/shape-sweep`, worktree `.worktrees/shape`, from `main` at `7a76d62`.
The design is [proposals/shape-primitives.md](../proposals/shape-primitives.md); this file is only the state
a next session needs, and it is deleted when the crate is finished.

## Where the tree stands

**P0 is on `main`** (`57b1184`, with `mesh` merged as `7a76d62`): `math`, `curve` and `mesh` with their two
probes. **P1 lands in this checkpoint**: `hermite`, `scalar`, `section`, `sweep`, `loft` and
`sweep_probe`; `curve_probe` gained a *closed ends* row; `building` now names `shape` for `math` instead of
`mountain`.

Five probes run as examples, each gated on its own readings:

```text
cargo run -p whiting-shape --example curve_probe      # 8 invariants: arc length, stations, insert, frame, inflection, holonomy, closed ends, Bezier
cargo run -p whiting-shape --example scalar_probe     # 7: constant, two knots, limiter, C1, quadratic, clamps, refusals
cargo run -p whiting-shape --example mesh_probe       # 8: cube, reversed winding, open cube, open prism, closed prism, three-way edge, repeated corner, unused position
cargo run -p whiting-shape --example section_probe    # the ring's arc-length spacing, the outline's winding, and the refusals
cargo run -p whiting-shape --example sweep_probe      # 8: frustum volume, solid vs tube, u and v, ring plane, twist, section frame, blade tip, envelope refine
```

## What a consumer can ask for now

```rust
let mut sweep = Sweep::new(spine, Section::circle());
sweep.scale = Scalar::new(&[(0.0, 1.0), (1.0, 0.4)])?;  // the taper, in the spine's own `t`
sweep.twist = Scalar::constant(0.0);                    // radians in the section's plane
let loft = sweep.sample(stations, around);
let surface = loft.view();                              // mesh::Triangles
mesh::boundary_loops(surface);                          // and is_manifold, euler_characteristic,
mesh::signed_volume(surface);                           // winding_is_consistent, same_direction_edges
```

- **`Section`** — `circle()`, `flat(half_width, thickness)`, `ellipse(x, y)`, `outline(&[Vec2])`, and
  `ring(around)`. `Outline::new` refuses fewer than three points, a non-finite point, and two consecutive
  points that are equal *including the closing pair*; it normalises the winding to counter-clockwise
  keeping `points[0]` first.
- **`Scalar`** — `constant(v)`, `new(&[(t, v)])` with limited slopes, `with_slopes(&[(t, v, slope)])`,
  `at(t)`, `slope(t)`. Knots must start at `0`, end at `1` and increase.
- **`Loft`** — `positions()`, `uvs()`, `triangles()`, `rings()`, `around()`, `vertex(ring, slot)`,
  `vertices()`, `view()`.
- **`mesh`** — six predicates over `Triangles { positions, corners }`, each one pass and one table.
  `signed_volume` is positive for an outward winding and only meaningful for a closed surface.

## What is owed, and is not this file's to do

- **The gallery picture** — the P1 exit's last item: draw a tube, a blade and an envelope from one call.
  `shape` has no scene; the gallery has.
- **`building`'s predicates** could read `shape::mesh` now. Its probes still read their own copies and pass,
  so nothing forces the switch; do it when one of its own checks is the same check.
- **`vegetation` P1** starts on this API, and is the reason `scale`, `twist` and `(u, v)` are shaped as they
  are.

## Traps a next session should not re-learn

- **`t` is an arc-length fraction**, so a `Scalar`'s knots are fractions of the spine's *length* — the same
  convention as `Curve::at`. A knot's own fraction is `knot_parameters()`, not its index.
- **The frame's initial normal is the orientation statement.** A section's `x` is `frame.normal` and its `y`
  is `frame.binormal`, so two spines given different initial normals face their sections differently. Say it
  once, on each spine, with `Curve::set_initial_normal`.
- **`scale` must stay positive.** A negative one folds the section through the spine. A *zero* one is a
  point: the caps are still built, so the surface stays closed and `boundary_loops` stays `0` — the leaf's
  tip is exactly that case, and it needs no special handling anywhere.
- **The cap fans reuse the end rings' own vertices**, so `Loft::positions().len() == rings * around` always.
  A consumer that assumes an extra centre vertex per cap will read the wrong index.
- **`boundary_loops` counts connected components of the once-used edges.** A collapsed ring is closed
  because the fan shares its edges, not because it collapsed; with `capped: false` the same ring *is* two
  boundary loops, which is right.
- **A Hermite circle is `5.2e-4` short of a true one**, while a closed loop's *ends* are exact. So "the seam
  closes" and "the loop is a circle" are separate claims with separate readings.
- **Do not add `closest(point)` yet**, and do not raise the ellipse's arc-length table resolution without
  measuring: `section_probe` prints the inversion's error and the table's step count is a stated cost.

## How to verify

```text
cargo run -p whiting-shape --example curve_probe
cargo run -p whiting-shape --example scalar_probe
cargo run -p whiting-shape --example mesh_probe
cargo run -p whiting-shape --example section_probe
cargo run -p whiting-shape --example sweep_probe
cargo check -p whiting-mountain -p whiting-terrain -p whiting-building -p whiting-gallery --all-targets
```

Every probe prints one row per invariant with the threshold it is compared against and exits non-zero if any
row leaves it, so a green run is a reading and not a claim.

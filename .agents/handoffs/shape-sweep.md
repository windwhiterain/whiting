# Handoff: the `shape` crate — P0 landed, P1 (the sweep) is next

Started 2026-10-17. Branch `feature/shape-primitives`, worktree `.worktrees/shape`, from `main` at
`8a80b20`. The design is [proposals/shape-primitives.md](../proposals/shape-primitives.md); this file is
only the state a next session needs, and it is deleted when the crate is finished.

## Where the tree stands

**P0 is landed and green.** `crates/shape` exists with `math` and `curve`:

- `math` is `mountain::math` **moved**, not copied: `crates/mountain/src/math.rs` was `git mv`-ed to
  `crates/shape/src/math.rs`, `mountain` re-exports it (`pub use shape::math;`), and every
  `mountain::math::…` path in `mountain`, `terrain` and `gallery` resolves unchanged. `building`'s
  `src/math.rs` re-export still points at `mountain`, which still works and can be switched later.
- `curve` is a cubic Hermite polyline: `Jet`, `Curve`, `with_result`-style validation (`CurveError`),
  arc-length `t`, `at`/`tangent`/`frame`/`curvature`/`stations`, `segments` (Bezier), `jets`,
  `knot_parameters`, `split`/`insert`/`sub`, and a transported frame by double reflection.
- `crates/shape/examples/curve_probe.rs` measures seven invariants with a control each and **exits
  non-zero if any leaves its threshold**. `cargo check` is green for `whiting-shape`,
  `whiting-mountain`, `whiting-terrain`, `whiting-building` and `whiting-gallery --all-targets`.

The measured numbers are in the proposal's *What P0 measured*, including the two boundaries a consumer
will meet: `Curve::new(curve.jets())` is **not** a round trip after a split (a split rescales the
parameter, so the two sides' tangent magnitudes differ and a `Jet` holds one number), and a Hermite arc is
5.2e-4 short of a circle's length.

## What P1 is

`sweep`, and then the first consumer:

1. **`Section`** — `Circle { radius: Curve }`, `Ellipse { radius, squash, roll }`,
   `Outline { outline: Vec<Vec2>, scale: Curve }`, `Flat { half_width, thickness }` — and **`Sweep {
   spine, section, closed }`**, whose `sample(stations, around)` walks the spine's `stations`, asks each
   for its frame, evaluates the section in that plane, and zips consecutive rings into a `Loft`.
2. **`Loft` owns the `(u, v)` definition**: `u` is the station's arc-length fraction and `v` the fraction
   around the section, returned **with** the position so no consumer writes a second table. Triangles
   only: the scene layer's `Topology` has no quads.
3. **Three sections, one primitive**: a branch is `Circle` with a radius curve (the taper), a leaf is
   `Flat`/`Outline` with `closed: false`… **or `closed: true` for a thin solid**, which is
   [agentic-vegetation-generation.md](../proposals/agentic-vegetation-generation.md) D18's answer to
   two-sided leaves — and the envelope is `Circle` refined to `Outline`. If any of those three needs a
   different code path, the unification is wrong and that is worth re-measuring before writing it.
4. **The `mesh` predicates** (`boundary_loops`, `is_manifold`, `euler_characteristic`,
   `signed_volume`, `winding_is_consistent`) land with the first consumer that calls them, not before.

## Traps a next session should not re-learn

- **`t` is an arc-length fraction**, so a station is a fraction of length and an edit that changes a
  segment's length moves the physical point. That is deliberate (`shape` D2) and it is why a knot's own
  fraction is `knot_parameters()`, not an index.
- **The frame's initial normal is state.** `set_initial_normal` exists because a sweep of two curves that
  met at a point must agree about which way their sections face. `split` already carries the parent's
  normal into the right half; a `Sweep` over a `sub`-curve that loses it is the defect this is for.
- **A `Section::Outline` needs a stated `x`**: the transported frame's normal is an arbitrary but
  continuous choice, so two sections on one plant that start at different angles rotate against each
  other. That is the proposal's open question 2 and P1 is where it gets answered.
- **`frame(t)` interpolates from a table of eight samples per segment**; `stations(k)` transports across
  the `k` stations themselves, which is exact at those stations. A sweep should use `stations`, not
  `frame(t)`, for its rings.
- **Do not create `closest(point)` yet.** The envelope's trimming law is its first caller and nothing
  before that needs it.

## How to verify, and what to run

```text
cargo run -p whiting-shape --example curve_probe          # 7 invariants, exit 0 required
cargo check -p whiting-mountain -p whiting-terrain -p whiting-building --all-targets
cargo check -p whiting-gallery --all-targets
```

A sweep probe should print, for a tube, a blade and an envelope built from **one** call: the ring count,
the two vertices that disagree about `u` (**zero**), the signed volume of a closed sweep against the
analytic frustum sum, and the `(u, v)` of a known corner. A picture is the gallery's job and not this
crate's: `shape` has no scene and no renderer.

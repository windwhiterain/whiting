# Why is the lower bole black in a base plate, and what makes it lit?

**Date of every measurement below: 2026-10-19.** Worktree `.worktrees/sw-baseblack`, branch
`feature/sw-baseblack` off `feature/southwest-china-trees` at `ee7f258`. Every render is
`yunnan_pine_base` unless the table says otherwise, `cargo run -p whiting-device-rasterize
--example render`, one variable at a time, the rest of the document untouched. Renders cost
~16 s, so the bisect was run in full rather than sampled.

## The question, and the short answer

The lower bole was black because **the floor's own geometry was recorded into the point light's
shadow page in the same column as the bole, at a smaller depth than the bole's own surface** —
and because that column's page was two times wider than the box the page was fitted to, the bole
was inside the page to be shadow-tested at all. The floor is not between the key and the bole;
the ray from `[7, 6, 8]` to a bole point at `y = 0.92` never reaches `y = 0`. The renderer
invented that occlusion. What makes it lit is one expression in
[`crates/scene/src/standard/math.rs`](../../../.worktrees/sw-baseblack/crates/scene/src/standard/math.rs):
`projection_of` was passing **twice** the box's half width into `orthographic`, whose first
parameter is a *half* width, so every point light's page rasterised a region 2.2× the
`Bounds::around(scene.centre(), shadow_extent)` it was fitted to.

## The bisect

The "dark" value is not black: the unlit bole reads `rgb(51, 50, 51)` on the trunk centre line and
the lit bole `rgb(96, 87, 79)`. `authored_id` is a single id over the whole bole, so one material and
one node — a lighting effect, not a material or geometry one. "Boundary row" is the first row down
the trunk centre line where the value steps down by ≥ 6; "world Y" converts it through the
`--debug position` sidecar, and is what makes the rows comparable across cameras.

| # | variable | setting | boundary row | world Y of boundary | verdict |
| --- | --- | --- | --- | --- | --- |
| 0 | baseline | key 29.4°, `key_extent` 4.0, floor 10, cam y 1.3, bias 1.0, crown on | 774 | 0.921 m | defect present; pixel-identical to `out/arc2-base6.png` (`da4c7646…`) |
| 1 | key elevation | 60° | none | — | whole bole uniformly `(63,59,57)`: lit but grazing. Boundary gone |
| 2 | key elevation | 14° | 474 | 1.689 m | boundary **moved up 0.77 m**. It is not world-constant under the key |
| 3 | wood `shadow_bias_scale` | 0.0 | ~795 | 0.865 m | acne appears at rows 245–350 (the bias was doing its job) and the dark region **stays**. Not a bias artifact |
| 4 | crown presence | leaf fields cleared | 791 | 0.900 m | dark region **stays**. Not the crown's shadow |
| 5 | `key_extent` | 40.0 | none | — | dark region **gone**, whole bole `(95,86,78)` |
| 6 | floor extent | 100.0 | 774 | 0.921 m | byte-identical boundary. The floor's *size* is irrelevant |
| 7 | camera height | eye y 6.0 | 785 | 0.906 m | boundary **stays at the same world height**. World-locked, not screen-locked |
| 8 | **floor removed** (extra) | `floor-{cast,depth,shade}` instances deleted | none | — | dark region **gone**. The floor is the occluder |

Row 8 is the one that names the caster, and it is a clean control: removing the floor's three
instances barely moves `bounds.centre()` (the floor is ±10 m and the tree is 16 m tall, so the
centre is ~`(0, 8.25, 0)` either way), which means the page's matrices are the same before and
after. Only the **casters** changed.

Rows 1 and 2 kill the brief's premise that the boundary does not move with the key's elevation. It
does, and it moves the way the mechanism below predicts: fit a line through (key y, boundary Y) and
it gives `Y ≈ 0.92 − 0.23 · (key_y − 6)`, which puts the boundary below ground by the time the key
is at 60°, which is exactly row 1.

## The cause, at file:line

`crates/scene/src/standard/math.rs:242` (pre-fix), in `projection_of`:

```rust
let ortho = orthographic(
    2.0 * box_of.largest_half_width() * padding,   // ← a HALF width, doubled
    depth / 4.0,
    depth,
);
```

`orthographic(half_width, near, far)` documents its first parameter as "an orthographic projection
of **half width** `half_width`". `projection_of` is called from exactly one place —
`crates/scene/src/standard/emit.rs:4194`, inside `light_projection`, which is the **point** light's
pool page and the only caller of this function. So every point light's page was 2·`shadow_extent`·1.1
wide where the box it was fitted to was `shadow_extent`. (`directional_projection` at
`emit.rs:4865` has its own implementation and is unaffected; spot lights use the perspective
`spot_matrices`; nothing in the camera path uses it.)

Why that blackens a bole. `light_projection` builds `Bounds::around(bounds.centre(), shadow_extent)`
and looks at it from the light. For the base plate that box is `y ∈ [4.25, 12.25]` — it does not
contain the subject, which is the trunk's foot at `y ∈ [0, 3]`, because `bounds` is the union of every
instance's box and the scene carries a 16 m tree. The contract (`.agents/SPEC.align.md:2052-2068`)
says geometry outside that box reads **unshadowed**. The factor of two is what stopped that from
happening: the page was wide enough to contain the trunk's foot, so it was depth-tested instead.

The occluder, computed rather than guessed. With the key at `[7, 6, 8]`, `centre ≈ (0, 8.25, 0)`,
`d = |eye − centre| = 10.866`, so `near = (d + 2·4)/4 = 4.72` and `far = 18.87`. The view direction
is `f = (centre − eye)/d = (−0.644, +0.207, −0.736)` — **`f.y > 0`, because the key at 6 m sits below
the scene's centre at 8.25 m, so the page's axis tilts upward.** A bole point `P = (0.2, Y, 0.2)` is
at depth `8.881 + 0.207·Y` along the page axis. Walking backward along the column (toward the light)
to `y = 0` gives `s = −Y/0.207` and a floor point at `(0.2 + 3.12·Y, 0, 0.2 + 3.56·Y)` whose depth is
`8.881 − 4.62·Y` — **always nearer than `P` along the page's axis**, and inside ±10 m of floor for
every `Y` in the plate. The floor is recorded there and reads as an occluder.

The sharp horizontal edge is that floor point hitting the page's **near plane**: `8.881 − 4.62·Y ≥ 4.72`
gives `Y ≤ 0.90 m`, measured `0.921 m`. Above that line the floor is clipped out of the page and the
bole is lit; below it the floor is recorded and the bole answers occluded. That is the "clean
horizontal line at a fixed world height", and it is why it moved with the key (row 2), why
`key_extent = 40` removed it (row 5 — `near = 22.7` puts the near plane in front of the subject, so
**nothing at all is recorded and the key casts no shadow at all**; that is an accident, not a fix),
and why the floor's own extent was irrelevant (row 6).

Geometrically the shadow is simply false: the segment from the key to `(0.2, 0.92, 0.2)` runs from
`y = 6` to `y = 0.92` and never meets the ground plane, and the recorded floor point is not on it
(its ray ratios are 0.579 and 1.181, not parallel). The root cause underneath is that a point light's
real visibility needs the six-faced cube grid `virtual_volume` documents as *"not a grid this
compiler can build yet"*; the single orthographic pool page is the stand-in, and a parallel-axis page
cannot represent a point light over a scene deeper than the page's own depth range. The factor of two
is what turned a documented approximation into a wrong picture with a hard edge.

## The fix

One expression, in `projection_of`:

```rust
let ortho = orthographic(box_of.largest_half_width() * padding, depth / 4.0, depth);
```

plus the invariant it violates, written into the same function's doc comment. Committed as `f5ed72d`
on `feature/sw-baseblack`. Nothing else changed: no gallery file, no scene number, no shader.

**The renderer was not innocent.** The page contradicted `projection_of`'s own docstring ("the box is
fitted to the view … `padding` widens the box"), `SPEC.align.md:2049` ("the half width is
`shadow_extent` and nothing else"), and `pine_lighting`'s comment ("the floor beyond eight metres of
the tree now falls outside the key's shadow" — with the factor of two it fell outside 17.6 m, not 8 m).

## Verdicts

- `cargo test -p whiting-scene`: **15 binaries, 0 failed** (`cargo fmt` applied to `whiting-scene`,
  CRLF restored; the diff is one expression plus its doc comment).
- `cargo test -p whiting-gallery`: **4 binaries, 0 failed, 1 ignored** (pre-existing).
- Build: clean, no warnings.
- **`yunnan_pine_base`** → [`out/sw-baseblack/base-fixed.png`](../../out/sw-baseblack/base-fixed.png),
  `3954468534CCD86B03B7724EBA342D040C65A33830259A1972743E487E3AE6A2`. The flare and the lower bole are
  lit, the flare's creased facets read, and the boundary is gone. Judged against
  `out/refs/pinus_yunnanensis/03-bark.jpg`: the value and the light direction now read as a photograph's
  (warm key side, cool shadow side, the flare's facets catching the light), but the reference's bark is
  **deep fissured plates** and this is a near-smooth bole. That is a modelling gap in `bark()` and is
  **not** this defect — the defect was the lighting, and it is gone.
- **Bamboo moved, and not only plant pixels.** Baseline reproduced first:
  `bamboo-base.png` = `328B509A56E82973EA45F5B94AF56D195381C46E77EDD53976F799A828B2C81F`, exactly the SHA
  the branch promises. With the fix, `bamboo-fixed.png` = `44F79219E076F44B1975C49679FBEB66676E562742135A8DFD3524063684B789`.

  | scene | pixels | changed | share | plant-id px | floor (reserved-zero) px |
  | --- | --- | --- | --- | --- | --- |
  | `bamboo` | 1 680 000 | 107 573 | **6.4032 %** | 12 399 | 95 174 |
  | `yunnan_pine` | 2 380 000 | 361 527 | **15.1902 %** | 201 420 | 160 107 |

  Classified by the scene's own `authored_id` channel: plant pixels carry an id, the floor carries the
  reserved zero and writes black in that image. **The brief's expectation that the delta be plant-only
  does not hold, and I am not claiming it does.** Both numbers are the same defect being removed, not a
  regression:

  - The **leaf pixels** were falsely shadowed by the same mechanism and are now lit; the leaves are
    *brighter* and better defined in `bamboo-fixed.png`, and the culm's own cast shadow on the floor is
    still present and correctly placed.
  - The **floor pixels** are one large false shadow being removed. In `bamboo-base.png` the far floor
    is uniformly dark from the same "floor in the floor's own columns" error; the difference image
    ([`out/sw-baseblack/bamboo-diff.png`](../../out/sw-baseblack/bamboo-diff.png)) shows a bright wedge
    at the floor's horizon plus the soft edge of the cast shadow.
  - `yunnan_pine` is the strongest evidence the fix is right rather than narrow: in `pine-base.png`
    the **whole visible bole is near-black**, and in `pine-fixed.png` it is lit brown with a lit side.
    That plate never showed a hard horizontal boundary (its key at `y = 14` is above the scene's centre,
    so `f.y < 0` and the floor cannot enter the bole's columns from above), which is why the defect hid
    there — but it was black for the same reason. **Its crown did not regress**: the whorls, the needle
    detail, the bough silhouettes and the ground shadow are all intact and, where they differ, better.

  The honest cost: a smaller page makes the page's own lateral boundary visible where a wide floor
  crosses it (a faint straight edge on the right of `bamboo-fixed.png`). That edge is inherent to
  "a page fitted to a box smaller than the scene, and unshadowed outside it" — the contract this fix
  restores — and it disappears when a point light gets the cube grid the code already says is owed.

## Sources

`.worktrees/sw-baseblack/crates/scene/src/standard/math.rs` (`projection_of`, `orthographic`),
`crates/scene/src/standard/emit.rs` (`light_projection:4191`, `projection_of:4194`,
`directional_projection:4865`, `virtual_volume:557`), `crates/gallery/src/shaders.rs` (`shadowed`,
`shadow_term`), `crates/scene/src/vshadow.rs` (the spot-only virtual path),
`.agents/SPEC.align.md:1038-1052, 2040-2068`, and the renders listed in the table.

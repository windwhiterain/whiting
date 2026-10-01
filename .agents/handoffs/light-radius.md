# A light that cannot reach is a light that is not there

Worktree: `./.worktrees/light-radius`, branch `feature/light-radius`, on top of `main` at `a752bd4`.

## The defect

`dunes` renders a staircase of hard-edged, screen-axis-aligned rectangles across the field
instead of a smooth terminator. The edges land on multiples of 64 px across and of
`1024/9` down, which is the cluster grid: `width_extents` 16 by `height_extents` 9
(`DEFAULT_WIDTH_EXTENTS` / `DEFAULT_HEIGHT_EXTENTS` in `lighting/assignment.rs`).

A light is culled per cluster by its own `radius` (`ClusterLight::radius`, tested in
`assignment.rs` by `nearest_squared > volume.radius_squared`). `radius` was an **optional**
field of a `Point` and a light that named none was given `bounds.largest_half_width()`. That
default is right for exactly one kind of light — one standing **inside** the scene it lights —
and wrong for a sun standing outside a valley, which is culled short of its own reach, dropped
from every cluster past that, and leaves the boundary between lit and unlit quantised to the
cluster grid.

This is not a new disease and the repository already knows it, **twice**:

- `emit.rs` records the identical symptom for a **spot** light — *"what the author sees is a
  grid-aligned rectangle where the cone's own circle should be"* — and answers it by giving the
  spot the reach of its cone. The point light half of the same sentence was never done.
- `proposals/photographic-stilllife.md` §2 is the same diagnosis reached from the other end, on
  `stilllife`: *"rectangular patches about a cluster wide, each a flat tone, and the patch size
  moved with `clustering.width_extents`"*, taken all the way to a cluster count dump
  (`73 clusters lit by nothing`) and worked around by tightening the camera's near and far
  planes. It names the real cost itself: *"the failure mode is a picture rather than an error.
  Nothing in the dump above is visible from the scene's own source."*

Both prior answers were scene-level workarounds. `crates/device/tests/clustered_shadow_render.rs`
was already working around it too, with a `reaching()` helper that inserted the radius by hand;
that helper is now unnecessary and is gone, because `kit::light` takes the number.

## Ruled out before this was accepted

- **Not the shadow.** `shadow_extent` 175 → 400 produces a **byte identical** frame.
  `shadow_extent` → 10, which leaves the page covering a 44 by 44 patch and the whole field
  unshadowed, leaves the rectangles untouched. The page is 1024 by 1024, so it is not the
  resolution either.
- **Not the depth test.** That landed at 19:47 and moved `dunes` by a mean of 0.010; the
  rectangles are in the 19:29 frame, before it.
- **Not the cloud shadow or the geometry.** The scene's own time is 0; at t=200 the dunes move
  and the rectangles do not.
- **Not a screen-space shader artifact.** `sunlight` and `studio` are clean; the scenes with a
  large terrain and a light standing outside it are not.

## What landed

**`radius` is a required field of a `Point` and a parameter of `kit::light`.** There is no
default behind it, so a scene can no longer state a light whose reach is a number the compiler
picked.

- `scene_schema.rs`: `radius()` loses `.optional()`; a sibling `optional_radius()` is added and
  given to the **`Spot`** variant, which had no radius field at all.
- `emit.rs`: `LightRead::radius` loses the `bounds.largest_half_width()` arm. It now answers a
  point light from the document, a spot from `SpotCone::reach`, and a kind that takes no cluster
  slot with zero.
- `kit.rs`: `light()` takes a `radius`; `light_with_radius()` is deleted, since it was the same
  builder with the field bolted on afterwards.

**`studio` is the purest case, and it is a scene that changed.** `example_standard_scene()`'s
light stood 7 to 8 units off a triangle two units across, so the old default — one box radius
of the triangle, which is `1.0` — culled it out of every cluster. It has been rendering as two
**unlit** triangles on black, with no error and no note. It now says `10.0` and is lit.

**Two scenes are corrected and four are held at exactly what they rendered before**, so that this
change is a schema change and not a relight:

| scene | radius now | the picture |
| --- | --- | --- |
| `dunes` | 430.0, 430.0 | **corrected** — was 165, reaching 427 and 420 |
| `terraces` | 700.0, 700.0 | **corrected** — was 190, reaching 692 |
| `lanterns` | 300.0 × 3 | unchanged; 300 is what the convention gave it |
| `tidewater` | 1400.0 × 2 | unchanged; 1400 is what the convention gave it |
| `monoliths` | 340.0 | unchanged; the terrain's own half width |
| `tiles` | 12.0 | unchanged; the terrain's own half width |
| `studio` | 10.0 | **changed** — the light was reaching nothing |

## What is verified, and how

Measured on the adapter, all ten gallery scenes rendered before and after at the scene's own
time, compared channel by channel:

| scene | mean absolute difference | largest | channels differing by 8 or more |
| --- | --- | --- | --- |
| `dunes` | 20.500 | 80 | 49.22% |
| `terraces` | 2.761 | 64 | 14.20% |
| `studio` | 5.217 | 239 | 2.57% |
| `lanterns`, `monoliths`, `spotlight`, `stilllife`, `sunlight`, `tidewater` ×2, `tiles` | **0.000** | **0** | **0.00% — byte identical** |

Eight byte-identical frames is the measurement that says the schema change is faithful: making
`radius` required touches every light document in the gallery, and everything that already said
one says the same one it used to get from the convention. The three that moved are the three
that were wrong.

The cluster-aligned hard edge is gone from `dunes`: the long horizontal edge at y = 682.5, which
is `1024 - 7 * (1024/9) = 682.7` to within half a pixel, is present before and absent after.

- `whiting-scene`: 56 pass (13 + 8 + 3 + 6 + 11 + 7 + 3 + 5), doctests 5 pass.
- `whiting-device` with `wgpu`: 22 pass across six integration targets, doctest 1 pass.
  **The doctest fails in a release build and that is pre-existing**: `lib.rs` skips the
  cross-check outside debug and test mode, and the doctest asserts it ran. Unmodified `main`
  fails it the same way at the same line.
- No clippy warnings in the files this branch touches.

## Still owed

- **`tidewater` and `lanterns` reach short of their own scenes and are deliberately left so.**
  - `tidewater` is 1400 against a far corner at 2610 (sun) and 2413 (sky). The number is what
    its current look is tuned around — raising it relights the whole lake — and the handoff it
    already owes three loose ends says a scene-level look is owed there.
  - `lanterns` is a night scene whose water is *meant* to go dark away from the field, so its
    300 may be the intent rather than the defect. It needs a look, not a number.
  Both are now **visible** in the documents rather than implicit, which is the point of the
  change: the next person to look at a number that does not reach can see that it does not.
- **A spot light is still not required to say its radius**, and that is a judgement call made
  here rather than an oversight. A cone already determines how far a spot carries, and the
  schema's `radius` is optional for a `Spot` precisely so a cone is never culled shorter than
  the light it is drawing — which is the very rectangle `emit.rs` already documents. Requiring
  it there would make every spot restate a number its own document implies. Override this if the
  ruling was that no kind may default.
- **A `Directional` light declares no `radius` and takes no cluster slot**, so there is nothing
  for the number to cull. `LightRead::radius` answers it with zero, which is a number nothing
  reads.
- **The framework note is not attempted.** The compiler can see a light standing outside the
  bounds its own reach was measured against, and could say so. The superior's ruling was to keep
  `radius` and write it down, so this is recorded rather than proposed.
- Deriving the radius from `intensity` was considered and **does not work**, so it is not owed:
  `intensity` is folded into `colour` at read time (`emit.rs`) and the falloff is
  `1 / (1 + 0.0035r + 0.00002r²)` rather than inverse-square. A cutoff read as a fraction of the
  light's own peak cancels the intensity out entirely — `A(r) = ε` gives 153 at ε=0.5 and 589 at
  ε=0.1, the same number for a one unit still life and a 400 unit valley. A cutoff read as an
  absolute radiance does make the intensity count, and then every light reaches 589 to 5740 units
  and the culling stops culling.

# Photographic scenes: the framework findings, implemented

Worktree: `./.worktrees/photographic`, branch `feature/photographic`, on top of `main` at
`5a50461`.

A lake at first light, and the write-up of what the framework could not say while building it:
[proposals/photographic-scenes.md](../proposals/photographic-scenes.md). That proposal is now
the record of what was implemented, what was held back and why, and it carries two
corrections to its own original claims.

```text
cargo run --release -p whiting-device --features wgpu --example render -- tidewater out/tidewater 0
```

Both cameras are rendered; the wide one is the picture and the long lens is the treeline.

## What this stretch did

Four of the seven findings, and the two that land in files `feature/lighting` is rewriting were
held back by the superior's ruling and are recorded against their items in the proposal.

| finding | state |
| --- | --- |
| 1. a surface body cannot displace the normal | implemented (earlier stretch) |
| 2. no value reaches two stages | **the gallery half** — `air::Atmosphere`; `dunes` migrated and its seam is closed |
| 3. a mirror in a lighting pass is added once per light | **the gallery half** — `kit::instances` takes a `Stage`, and the rule is stated on it |
| 4. the specular is not normalised and the Fresnel goes to one | implemented — and measured to be nearly invisible on this gallery |
| 5. MSAA is unreachable | not implemented; the size was re-measured and it is a renderer change, not a scene field |
| 6. no exposure, no falloff, no lens | **the lens** — `focal_length`, `sensor_width`, `roll`; exposure and falloff held back |
| 7. the harness wrote both cameras to one file | implemented (earlier stretch) |

## The two corrections, because both invert a claim

- **Finding 4 does not improve every frame in the gallery.** The original proposal said it did.
  Measured across the gallery: a mean absolute difference of 0.0009 to 0.026 per channel out
  of 255, the largest single-channel difference anywhere being 20. The reason is not that the
  change is wrong — it is that **the gallery no longer contains the case that motivated it**.
  The 340-reed bed of `tidewater` was removed once it was diagnosed as the bug. The fix is
  kept because the class of scene it fixes is large and it costs one multiply and one divide,
  but anyone who builds a scene to *see* it will conclude it does nothing unless the scene
  contains a thin edge-on object at a grazing angle.
- **Finding 3's second half is not settled.** The proposal said a directional light "is already
  in the schema", which reads as though the question were closed. It is in the schema and
  `read_lights` (`emit.rs:623`) **refuses** it with `UnsupportedLightKind`. `tidewater` still
  writes the sun into its own `WGSL` as a constant to get a glitter path at all.

## What is verified, and how

Measured on the adapter, not by reading:

- All seven gallery scenes render with every change applied.
- `whiting-scene`: 12 + 6 + 7 (1 ignored) + 3 + 5 pass, doctests 5 pass.
- `gallery_render`: 2 pass on the adapter.
- No clippy warnings in `air.rs`, `kit.rs`, `shaders.rs`, `dunes.rs`, `tidewater.rs`,
  `camera.rs` or `scene_schema.rs`. The warnings that remain are pre-existing, in files this
  branch does not touch.
- **The lens is live**: 35 mm and 85 mm on the same scene give different frames, and 0.25 rad
  of roll turns the horizon, the treeline and the sun's reflection column together with the
  geometry standing still. Checked by rendering, because an earlier build was stale and a
  field that does nothing is indistinguishable from a field that is right.
- **The seam is closed**: `dunes` changed by a mean of 1.728 per channel with 6.2% of channels
  differing by 8 or more — the largest change anything in this branch made to any frame.

## Still owed

- **`stilllife` was not started.** Approved at the outset and nothing of it exists. It is the
  scene that would actually exercise the fixed specular and the new lens fields, and it is
  still the better half of the pair.
- **`tidewater`'s three loose ends**, unchanged: the sun's reflection breaks into hard
  horizontal steps where the waves line up rather than sparkling; the near bank reads as a
  sandbar rather than as the corner it was placed to be; a bright sliver at the left edge of
  the long-lens frame, undiagnosed. The sun's disc also shows through the treeline in the wide
  frame.
- **`tidewater`, `terraces` and `monoliths` each carry the duplicate air pair that `dunes`
  had.** `tidewater` got it right by hand, which is what `air::Atmosphere` is that code
  promoted; migrating the other two is mechanical and is the obvious next piece of work here.
- **The framework half of finding 2** — a compiler-supplied atmosphere uniform spanning the
  stages — needs a field in the per-light uniform, which is what lighting stage 1/2 replaces.
- **Finding 5** is a renderer change and `feature/lighting` is editing `raster.rs` right now.
  The three defects are written up in the proposal; the smallest of them,
  `PipelineState::read` returning before it reads `samples`, is a one-line fix in `graphics`
  that collides with nothing and can land whenever.

## Do not overlap `feature/lighting`

`.worktrees/lighting` is mid-way through a five stage effort that owns the lighting path. This
branch does not touch the per light pass generation, clustered assignment, the shadow page pool
or IBL, and it does not touch `emit.rs`, `pass_schemas.rs` or anything under
`crates/scene/src/lighting/`. The proposal's *Touched* section lists every file changed, and
the last line of that table is the list of files deliberately not touched and why.

# Agent debug outputs

Worktree: `./.worktrees/debug-outputs`, branch `feature/debug-outputs`, on `main` at `e1b29c2`.

Design and reasoning: [proposals/agent-debug-scenes.md](../proposals/agent-debug-scenes.md). That
proposal is the record of what was decided, what was built, and — in *What the first render
found* — the one thing the building turned up that the design had not anticipated.

The feature: **a scene compiled with a channel asked for renders one extra image, a world
position per pixel, so an agent can tell where in the scene a pixel of the picture is.**

## The decisions, restated so this file stands alone

1. The switch is the caller's, not the document's: `StandardScene::with_debug([Position])`. No
   scene-schema field.
2. One channel in the first landing: `Position`.
3. A position image is `rgba16float`, not a value squeezed into 8 bits.
4. The graphics document declares its outputs in a new optional `outputs` field.

## Checkpoints — all landed

| # | what | state |
| --- | --- | --- |
| 0 | **Risk check**: `rgba16float` renders and reads back on this adapter at all | **done, yes** |
| 1 | `graphics`: the `outputs` field, and a reader that falls back to today's rule | **done** |
| 2 | `scene`: `DebugChannel`, the position shader, the pass, `with_debug` | **done** |
| 3 | `device`: the renderer returns declared outputs, `read_back` format aware | **done** |
| 4 | `harness`: `--debug position`, the extra image beside the picture, a sidecar | **done** |

## Where things are, for the next session

- `crates/graphics/src/read.rs` — `Output`, `outputs()`; `crates/graphics/src/lib.rs` —
  `OUTPUTS_FIELD`, `PRESENT_OUTPUT`, and the schema field.
- `crates/scene/src/standard/debug.rs` — **the channel vocabulary**: the `DebugChannel` enum, the
  position WGSL, and the pass interface. Read the module doc first; it states the coverage
  contract.
- `crates/scene/src/standard/emit.rs` — `debug_passes()` and `debug_outputs()`, and the one hook
  in `emit` itself. This is the only file the parallel `global-lights` session also edits, and the
  hook was kept to the signature, one note and one call for that reason.
- `crates/device/src/render.rs` — `RenderedImage`, `RenderedFrame`, and the `Renderer` trait:
  `render_outputs` is required and `render` is a provided convenience, which is what kept the four
  existing render tests unchanged.
- `crates/device/src/wgpu_renderer/raster.rs` — `read_back(name, target)`, now format aware.
- `crates/device/examples/render.rs` — `--debug`, `write_channels`, `viewable`, `probe`.

## The measurements, so a later session does not have to re-make them

- `rgba16float` builds a pipeline with `wgpu::BlendState::REPLACE` and does **not** clamp: a
  fragment writing `(1.5, 0.25, -2.75, 1.0)` reads back as `[0x3E00, 0x3400, 0xC180, 0x3C00]`.
- The beauty image is **byte for byte identical** with a channel and without one, on `studio` and
  on `tidewater` (post chain, two cameras, perspective).
- A channel covers **what the `depthing` bucket drew**, because it reads the depth buffer. The
  example scene disagrees with itself — one instance under `depthing`, two under `lighting` — and
  the position image faithfully showed the one. A scene with no `depthing` instance at all gets a
  compiler **note** at `instances.depthing`.
- Workspace suite green; device suite green on the adapter; `clippy` clean in everything this
  change adds.

## Still owed

- **A permanent test.** The two invariants worth holding are that a channel does not move the
  picture and that a background pixel reports none, and both were checked by hand and by scratch
  programs that were then deleted. The proposal lists them under *Still owed*.
- **A G-buffer**, and with it `Normal` and `Identifier` channels — the change that removes the
  coverage contract, because a channel that ran its own geometry pass would cover exactly what it
  drew.
- **A channel over RPC.** The device and the RPC already return every declared output; what is
  missing is an argument that makes the compiler declare one.

## How to see it

```text
cargo run -p whiting-device --features wgpu --example render -- dunes out/dunes --debug position
```

writes `out/dunes.png`, `out/dunes-position.png` and `out/dunes-position.txt`.

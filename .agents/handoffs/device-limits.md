# Adapter limits reach the compile, and three counts are bounded by the machine they are read for

Worktree: `./.worktrees/device-limits`, branch `feature/device-limits`, on top of `main` at
`7aeaae9`.

The page pool, and the two page dimensions, and the frame's own dimensions, are now held to the
limits of the device the compile is made against. The design and the argument are in
[proposals/adapter-limits-and-the-page-pool.md](../proposals/adapter-limits-and-the-page-pool.md);
that file is now marked implemented and its open question is recorded as decided.

## What changed

| file | what it does now |
| --- | --- |
| `crates/scene/src/limits.rs` | new. `DeviceLimits` and `clamp_to_ceiling`, the one place a count is bounded and named |
| `crates/scene/src/lib.rs` | `CompileContext::limits`, `device_limits()`, `CompileError::PoolTooDeep` removed |
| `crates/scene/src/lighting/buffers.rs` | `read_light_pool` takes limits and returns notes; `MAX_TEXTURE_ARRAY_LAYERS` removed |
| `crates/scene/src/standard/frame.rs` | `Frame::read` takes limits, returns notes |
| `crates/device/src/render.rs` | `Renderer::limits()` |
| `crates/device/src/wgpu_renderer/mod.rs` | answers from `device.limits()`, not from an adapter |

`MAX_TEXTURE_ARRAY_LAYERS` and `CompileError::PoolTooDeep` are gone rather than deprecated, and
nothing else in the workspace constructed either.

## The two numbers the constant held

`DeviceLimits::downlevel()` is 2048 and 256, which is exactly what `MAX_TEXTURE_ARRAY_LAYERS` and
the downlevel floor held. **The number survived; its meaning did not.** It was a threshold above
which a document was refused, and it is now the answer to "no machine behind this compile". A
floor is the smallest answer true everywhere, which is the only honest thing to bound by when
nothing better is known.

`WgpuRenderer` answers `limits()` from `self.device.limits()` rather than from the adapter. wgpu
creates a device with the lower of a request and an adapter's numbers and validates every
resource against the device's, so the device is the authority. The two are equal today for these
two fields because the request asks for the adapter's own — they would part company the day an
override joined them, which is the day reading the adapter would be the wrong answer.

## The line between a bound and a refusal

A count **above** what the machine has is bounded and named. A count **below one** is refused.
The first is a fact about this run, the second is a wrong image on every machine. Every clamped
count in the project goes through `clamp_to_ceiling`, so all of them name the field, the number
asked for, the number given and the ceiling, in that order.

The consequence sentence is the caller's. A narrower frame is a crop of the picture asked for; a
smaller page is a coarser shadow for **every** light rather than a missing one; a shallower pool
is a light lit by the approximation. The page sentence says "coarser" on purpose — that was the
contested point, and a page drawn at the size given is not a smaller frame in the way a light that
lost the ranking is.

## What is verified, and how

- `whiting-scene`: 13 + 8 + 3 + 6 + 11 + 7 + 3 pass, doctests 5 pass.
- `whiting-device`: the tests that run without an adapter pass; the rest are `wgpu` gated.
- `cargo check --workspace --all-targets` is clean, and the wgpu gated render tests **compile**
  with the new `CompileContext` field. They were **not run**: this machine has no adapter, so
  saying they pass would be saying nothing.
- No gallery scene declares a shadow pool, and the two that declare a frame ask for 1536 by 1024.
  Both are under the downlevel floor, so **no gallery frame changes**. That is a reading of the
  documents, not a rendered comparison.

## Still owed

- **`Frame::read` still falls back silently** for a dimension that is not a positive integer,
  while `pool_count` refuses the same value as `MalformedScene`. Left alone deliberately: one is
  a field with a convention, the other a field whose whole content is a budget, and unifying
  them is a design call rather than a missing line.
- **A total-bytes ceiling does not exist.** Bounding the three counts still permits a pool the
  machine cannot allocate — 16384 square pages at 2048 layers is terabytes — and that fails at
  allocation rather than at validation. There is no limit in `wgpu` to bound it with, so any
  answer is a project policy rather than a hardware fact. Not asked for, not invented.
- **The headless server reports no limits it was not given.** `NoopRenderer` answers `None`, and
  a reload through it is made against the floor. Correct, since it draws nothing — but a host
  that wanted the real numbers for a proxying compile still has nowhere to read them from.

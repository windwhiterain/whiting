# Handsoff: the sampled texture channel, device half

Status: **implementation complete**, awaiting merge. Branch `texture-channel`, worktree
`.worktrees/texture-channel`, four commits on top of `d013f85`.

Read [proposals/texture-channel.md](../proposals/texture-channel.md) for the design and
[proposals/texture-channel-plan.md](../proposals/texture-channel-plan.md) for the survey it was
planned from. This file records what was built, what was decided while building it, and what is
still open.

## What landed

| commit | what |
| --- | --- |
| `a040136` | the payload: `data`, `filter`, `address`, `mip_filter`, `SamplerSettings`, and `sampled` as an optional unit key |
| `2c3255e` | four doc examples the camera and bucket migrations had left broken (**pre-existing, not this work**) |
| `ec690dd` | the numbering rule, in `Bindings::binding_numbers`, read by both the layout and the bind group |
| `7ac3215` | the upload, the level chain, `is_filterable`, and six tests on a real adapter |

## Decisions taken while building

Each of these was an open call in the plan, and each was resolved as the plan recommended.

1. **A sampled texture that cannot be filtered is an error**, naming the texture and the
   format. The device requests `Features::empty`, so `R32Float` and `Rgba32Float` genuinely
   cannot be filtered here. The alternative — silently binding `NonFiltering` — hands a
   document that asked for a linear filter a blocky one.
2. **`sampled` on a depth format is an error.** A filtered depth comparison is a comparison
   sampler, which is a different resource from the one this model has.
3. **`data` is validated against the format and the size.** Four bytes per texel, so
   `rgba8unorm`, `rgba8unorm-srgb` and `bgra8unorm` accept it and a length mismatch is an
   error naming both numbers rather than a garbage frame.
4. **`sampled` on a texture that is also a target is allowed**, and the attachment is made
   unfilterable whatever the payload said, because a backend forbids filtering a texture a
   pass is writing.
5. **`write_texture` needs no row padding.** The plan flagged this as needing confirmation
   against wgpu 30; `wgpu-core-30.0.1/src/device/queue.rs:892` passes
   `alignment not required for buffer offset or bytes per row`, so a 300, 100 or 90 wide
   texture is an ordinary shape.
6. **`textureNumLevels` is not usable as a check.** naga returns 255 for it rather than the
   level count, so the tests read a level with `textureLoad` and assert on the texel instead.
   This is recorded because it cost time and would cost it again.

## What the tests do, and what they deliberately do not

`crates/device/tests/texture_channel_render.rs`, six cases, `required-features = ["wgpu"]`,
each skipping with a printed note when no adapter is present:

- a sampled texture reaches the fragment stage and the whole frame is the uploaded texel;
- a texture that declares nothing still renders, read by `textureLoad` — the numbering every
  existing scene relies on;
- an uploaded texture carries the level a box filter derives, read back by integer coordinate;
- the same four texels average to 112 in a linear format and 81 in an sRGB one, which is the
  pair that shows the filter linearises;
- a shader that names the wrong binding is refused with a binding message naming the pass and
  the shader's own variable.

They are not registered in the default workspace run, because they need a device. Run them
with `cargo test -p whiting-device --features wgpu --test texture_channel_render`.

## The gate that matters, and its result

The proposal's sharpest sentence is that the build it replaced never re-rendered an existing
scene. So the acceptance gate is the five gallery scenes, not a new one. All five render **byte
for byte identical** to the `out/HEALTH-*.png` baselines on `main`, checked after the last
edit: `dunes`, `lanterns`, `monoliths`, `studio`, `terraces`.

`cargo test --workspace` passes, and so does `cargo test --workspace --features
whiting-device/wgpu`.

## What is still open

- **The scene-side half.** A standard scene still cannot name a texture. `MeshMaterial` carries
  one `custom_data`, which is a `Value` and not a resource, and every mesh stage's binder set
  is a closed contract shared by every material and shader in the gallery. This needs its own
  proposal, written against the numbering that now exists. See the align note's gallery
  section for the statement of the gap.
- **`data` is inline bytes only.** The `reference()` path exists in `definition` and is unused
  everywhere. Nothing reads the filesystem, and a scene compiler can generate pixels
  procedurally, so this is not needed yet. It becomes needed the moment a real photograph has
  to travel to the device, because the definition is sent as json over rpc.
- **No `encoding` on `data`.** The proposal said "RGBA8 bytes" and the format field decides,
  which is self-consistent today. If a compressed encoding is ever wanted, `data` needs a tag,
  because a flat `Bytes` cannot say what it holds.

## Before merging

`main` is `d013f85`. The branch is four commits and touches `crates/graphics` (the payload) and
`crates/device` (the rule, the upload, the tests). Nothing in `crates/scene` changed except the
doc example fix, and nothing in the gallery changed at all — which is what the byte-identical
renders say.

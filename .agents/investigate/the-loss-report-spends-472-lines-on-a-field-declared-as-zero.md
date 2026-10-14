# The loss report spends 472 lines on a field the scenes declare as zero

Measured 2026-10-14, Cycles 5.3.0 at `C:\resource\cycles\install\cycles.exe`, at `4cd7d0c` in the
tree, by counting the emitter's report lines for each scene:

```text
cargo run --release -p whiting-device-cycles --example render_cycles -- <scene> out/rl-<scene>.png 4
```

Every scene's `custom data` lines were counted and compared against its `reflect` lines. **In all
eleven scenes the two counts are equal**: every custom-data drop the emitter reports, gallery-wide,
is `reflect`.

## The question

`crates/gallery/src/kit.rs:663-665` builds the common case by delegating to the reflecting case:

```rust
pub fn surface(tint: [f64; 3], metallic: f64, roughness: f64, emissive: [f64; 3]) -> Value {
    surface_reflecting(tint, metallic, roughness, emissive, 0.0)
}
```

and `surface_reflecting` writes the field unconditionally (`kit.rs:684`). So **every gallery
material written through `surface()` declares `reflect = 0.0`** — a field that says "this surface
does not reflect", said by a material that never asked to reflect and could not have reflected.

## What the report spends on it

| scene | `reflect` lines | provably zero |
|---|---|---|
| `terraces` | **405** | 405 |
| `foldyard` | 39 | 39 |
| `lanterns` | 10 | not measured |
| `monoliths` | 8 | not measured |
| `stilllife` | 8 | not measured |
| `sunlight` | 7 | 7 |
| `spotlight` | 6 | 6 |
| `landscape` | 5 | 5 |
| `tidewater` | 4 | 4 |
| `dunes` | 3 | 3 |
| `tiles` | 3 | 3 |
| **total** | **498** | **472** |

The right-hand column is counted from source, not inferred: `terraces`, `foldyard`, `landscape`,
`tidewater`, `dunes`, `tiles`, `spotlight` and `sunlight` either never call `surface_reflecting` at
all, or — for `foldyard`, whose thirteen `Group::new` calls all pass `0.0`, including `far-range`
at `foldyard.rs:1249` — never pass anything else. **472 of 498 lines describe a loss of nothing.**

`lanterns` (1.0 at `lanterns.rs:138`), `monoliths` (1.0 at `monoliths.rs:31`) and `stilllife`
(0.55, 0.22, 1.0, 0.85, 1.0 at `stilllife.rs:219-251`) do declare reflection, and their 26 lines are
not broken down here.

The counts are internally consistent, which is the check that they are counting materials and not
lines: `terraces` reports 270 shader-stage lines against 405 `reflect` lines, and 405 is exactly
135 materials at three passes each; `foldyard`'s 39 is exactly 13 groups at three passes, against
26 stage lines for those same 13.

## What it costs beyond the log line

**Under the emitter's default mode, this field is not dropped. It refuses the export.**

`crates/scene/src/cycles_export/mod.rs:87-91` marks `Reject` as `#[default]`, and
`crates/device-cycles/src/render.rs:67` takes `lossy: LossyMode::default()`. The `other` arm at
`material.rs:162-167` **returns `Err` in `Reject`**. So a material whose custom data contains
`reflect` is an export failure under the mode a caller gets by not choosing.

`render_cycles` is the one caller that chooses `Degrade`, and it says why
(`crates/device-cycles/examples/render_cycles.rs:53-56`):

> The gallery's point schemas carry fields Cycles' loader has no socket for. Degrade drops them and
> says so, which is what makes a path-traced gallery render possible at all; Reject would refuse
> every scene in it.

**That comment attributes the refusal to point schemas and does not mention custom data, but
custom data refuses for the same reason at the same match.** A scene with perfectly clean point
schemas, built out of `kit::surface()`, would still be refused — for a field that holds `0.0` and
describes nothing. That is the whole cost of `kit.rs:684` writing the field unconditionally.

Read from source rather than measured: the three sites above are quoted and the control flow is a
`match` on the field's name, so there is no branch in which a declared `reflect` survives `Reject`.
Confirming it on a real document takes one `emit_cycles` run with a `surface()` material.

## Why this is a defect and not a verbose report

**The emitter never reads the value.** `crates/scene/src/cycles_export/material.rs:162-173` puts
`reflect` in the `other` arm of a match on the custom-data **name**, reports it, and discards it —
so a declared `0.0` and a declared `1.0` produce identical text. `principled_bsdf`'s own default is
no reflection, so dropping a declared zero is exactly carrying it.

**The field has a meaning, and a zero-valued declaration spends it.**
`kit.rs:667-671` says the field exists because "a wet floor and a stone are drawn by the same pass
with the same shader, and the only thing that tells the shading which one it is on is the row it
was drawn from". Under that meaning, `reflect` present should mean *this surface reflects*. As
written it means *some materials reflect*.

**This repository has already named the pattern.** `cycles_export/settings.rs:17-18`, on why the
document's own `exposure` wins over the caller's:

> Taking the caller's number first would be the same mistake the camera made: **a field the
> emitter never read, which a scene could add and see nothing happen.**

`reflect` as written by `surface()` is that sentence, applied to 472 materials.

**And the report it drowns out is the one arranged to catch a silent wrong image.**
`cycles_export/error.rs:79-83` gives the `CustomData` message its authority by naming what the
material could not say. On `terraces`, 405 of the 412 lines a reader has to read are the same
non-event — including on a scene whose *real* loss, a terrain that declares no albedo, is recorded
in [terraces-terrain-is-a-wgsl-material-the-document-cannot-carry](terraces-terrain-is-a-wgsl-material-the-document-cannot-carry.md).

## Not measured

**The split of the 26 lines in `lanterns`, `monoliths` and `stilllife`** between zero and non-zero
declarations. Those three scenes mix `surface()` and `surface_reflecting()` call sites, so their
counts cannot be read off the source the way the other eight were.

**Whether the emitter's behaviour is wrong or only the gallery's.** Two changes are defensible and
they answer different questions — see the proposal
[report-a-dropped-field-only-when-dropping-it-loses-something](../proposals/report-a-dropped-field-only-when-dropping-it-loses-something.md).
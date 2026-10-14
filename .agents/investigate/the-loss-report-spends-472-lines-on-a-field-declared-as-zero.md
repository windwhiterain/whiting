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
(0.55, 0.22, 1.0, 0.85, 1.0 at `stilllife.rs:219-251`) do declare reflection, so their 26 lines were
expected to be real. **They are not: 15 of the 26 are zero-valued as well.** Those three scenes mix
`surface()` and `surface_reflecting()` call sites, so their counts could not be read off the source
the way the other eight were — that was the open item, and the exporter now answers it by reporting
3, 3 and 5.

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

## What was done about it, and what that measured

**Option A — stop writing the field — is not implementable, and the reason is in the packer.**
`gallery_custom_schema()` declares `reflect` as a required `Float` (`kit.rs:642-649`), so
`validate_fields` (`schema.rs:248-258`) answers an absent one with `MissingField`. A schema `default`
does not rescue it, because `pack_struct` (`packing.rs:114-120`) walks the schema's field order and
answers an absent one with *"the data is missing a field its own schema declares"* — and
`Value::defaulted()` (`value.rs:353-372`), which would fill it, is reached only through
`defaulted_value()`, **which has no production caller**. The compile path never substitutes a
default. Removing the field breaks the rasterised compile outright, which is not a trade 472 lines of
noise is worth.

**Option B — the exporter stops reporting a loss it did not take — is what shipped.** The guard in
`cycles_export/material.rs` reads the value through the same `parameter` reader every other
parameter uses, and skips both the record and the refusal when the value is the node's own default.

| | before | after |
|---|---|---|
| `reflect` lines, gallery-wide | 498 | **11** |
| `terraces` | 405 | **0** |
| `foldyard` | 39 | **0** |
| `monoliths` | 8 | 3 |
| `lanterns` | 10 | 3 |
| `stilllife` | 8 | 5 |
| the other five scenes | 28 | **0** |

**All eleven rendered frames are byte-identical before and after**, compared by SHA256 over
`out/rl-<scene>.png`. That is the check the change had to pass: dropping a declared zero cannot move
a pixel, and none moved. The stage-line counts are unchanged in every scene and `custom_data` still
equals `reflect` in every scene, so nothing else was silenced.

The 11 that remain are measured rather than inferred, and **15 of those three scenes' original 26
lines were zero-valued too**: `monoliths` reports one `surface_reflecting` call site at 1.0
(`monoliths.rs:31`) and `lanterns` one at 1.0 (`lanterns.rs:138`), against five non-zero call sites
in `stilllife`, so 8, 10 and 8 became 3, 3 and 5.

### What the `Reject` claim now rests on

**The guard sits ahead of the `match mode` and reads no mode.** The 472 lines that disappeared were
disappearing under `LossyMode::Degrade`, which is what `render_cycles` sets; that same `continue`
gates the `Err` in the `Reject` arm of the same `match`, with nothing between them that could tell
the two apart. **`Reject` itself was not run** — `emit_cycles` builds its own scene and writes no
`reflect`, so it never exercised this path, and no example selects `Reject`. The claim is measured
for `Degrade` and follows for `Reject` from the guard's position; it has not been observed.

## Why this was a defect and not a verbose report

**The emitter never read the value.** `crates/scene/src/cycles_export/material.rs` put `reflect` in
the `other` arm of a match on the custom-data **name**, reported it, and discarded it — so a declared
`0.0` and a declared `1.0` produced identical text. `principled_bsdf`'s own default is no reflection,
so dropping a declared zero was exactly carrying it. That is the arm `loses_nothing_when_dropped`
now stands in front of.

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

**`LossyMode::Reject` against a real gallery scene**, for the reason given above. The 11 remaining
lines are the shape of that unrun case: under `Reject` those eleven materials would be refusals, and
under `Degrade` they are eleven lines. No example selects `Reject` over a gallery scene.

**Whether the 11 remaining lines are the right 11.** Each is a material declaring a non-zero
`reflect`, which the node cannot carry, so each is a genuine loss — but this records the count and
not a per-material audit of the three scenes.

**Making the field absent from the document at all.** That is the version that would give the field
its stated meaning back, and it is refused by the packer rather than by this change; see
[report-a-reflect-only-when-dropping-it-loses-something](../proposals/report-a-reflect-only-when-dropping-it-loses-something.md).
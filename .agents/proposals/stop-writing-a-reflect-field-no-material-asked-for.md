# Stop writing a `reflect` field no material asked for

## The question

`crates/gallery/src/kit.rs:672-686` writes `reflect` into every gallery material's custom data,
including the ones that go through `surface()` and can only ever hold `0.0`. The emitter cannot
carry the field, so it reports it once per material per pass — **472 of 498 report lines
gallery-wide describe a loss of nothing** — and under `LossyMode`, which is the `#[default]`, it
refuses the export instead.

The counts, the `Reject` path, and the sources are in
[the-loss-report-spends-472-lines-on-a-field-declared-as-zero](../investigate/the-loss-report-spends-472-lines-on-a-field-declared-as-zero.md).

## Three ways to do it

### A — write the field only when it means something (gallery side)

One condition in `surface_reflecting`, where the field is built:

```rust
let mut fields = vec![
    (TINT.to_owned(), triple(tint)),
    (METALLIC.to_owned(), Value::Float(metallic)),
    (ROUGHNESS.to_owned(), Value::Float(roughness)),
    (EMISSIVE.to_owned(), triple(emissive)),
];
// The field's meaning is "this surface shows the sky at a grazing angle". A material that
// does not is not saying so with a zero; it is saying it has nothing to say.
if reflect != 0.0 {
    fields.push((REFLECT.to_owned(), Value::Float(reflect)));
}
Value::structure(fields)
```

**For:** fixes the source rather than the symptom, so both the report and the `Reject` refusal go at
once. It is **one rule, not two** — it covers `surface()` (always `0.0`), `foldyard` (all thirteen
groups pass `0.0`) and `stilllife`/`lanterns`/`monoliths` (which genuinely reflect) in the same
place, and it leaves `reflect` present exactly when a surface reflects, which is the meaning
`kit.rs:667-671` gives it. Smallest change in the repository that fixes both symptoms.

**Against:** does nothing for a scene outside this gallery that writes an explicit `0.0`.

### B — report a drop only when dropping it loses something (emitter side)

`material.rs` reads the value and skips the report, and the refusal, when it is the one the emitter
would have written anyway.

**For:** fixes every scene, this gallery and any other. Makes the report honest as a general
contract rather than by this repository's cooperation.

**Against:** it needs a **per-field table** saying which fields have a "dropping this changes
nothing" default, and this repository has already been bitten by exactly that shape of bug:
`attribute_map::is_mappable` and `geometry.rs`'s `UV_FIELDS` were two lists answering one question
and disagreeing, which put a false claim in the report on every mesh of every scene until they
were merged (recorded in
[cycles-emitter-dropped-material-fields](../investigate/cycles-emitter-dropped-material-fields.md)).
Adding a second such table invites the same failure. It also cuts against the stated reason
`Reject` exists at all — "a silently missing attribute produces a plausible image" — by teaching
the emitter to let a field through without carrying it.

### C — both

**For:** A fixes the documents, B makes the report right for documents A did not and will not
touch.

**Against:** two mechanisms for one fact, and the census of which is responsible becomes a question
with two answers. Given that this repository's own history is a story of two lists disagreeing,
one mechanism is worth more than two that agree today.

## Recommendation

**A.** The root cause is that a material is made to declare a field it does not have. That is a
fact about the document, and a fact about the document should be fixed where the document is built.

## Not settled here

**Whether `foldyard` should stop declaring `reflect` at all.** All thirteen of its groups pass
`0.0` through `Group::new`, so under A it stops declaring it and the 39 lines go. If a group ever
needs a non-zero value the field reappears on that material alone, which is the behaviour the
field's documentation already describes.

**Whether the `Reject` refusal is separately worth fixing.** Under A, a scene built entirely from
`surface()` has no custom-data loss left, but the gallery's point schemas still refuse every scene
under `Reject` — which is what `render_cycles.rs:53-56` already says. A is not what makes the
gallery strict-mode-exportable; it removes one reason from the list.

**What happens to the `reflect`-declaring scenes under `Reject` afterwards.** `lanterns`,
`monoliths` and `stilllife` declare a real value, and refusing them is correct: the picture would
otherwise come back without the mirror that the document asked for.

## Verification

1. `cargo run --release -p whiting-device-cycles --example render_cycles -- terraces out/rl-terraces.png 4`
   — the report for the worst scene falls from **405 `reflect` lines to 0**, with every other line
   unchanged.
2. The same eleven-scene count as the investigation, to confirm 472 lines are gone and the 26 in
   `lanterns`/`monoliths`/`stilllife` are untouched.
3. **The rendered frames must not move**, and that is the check that matters: dropping a declared
   zero cannot change a pixel. `out/rl-foldyard.png` and `out/rl-terraces.png` before and after are
   compared by hash.
4. `cargo test --release -p whiting-scene`.

**No new test is proposed.** A test asserting "a zero `reflect` is not written" tests the gallery's
kit against itself; the frames not moving is the evidence, and the report count is the visible one.

## Not included

**Carrying `reflect`.** Separately measured and separately wrong: the document's field is grazing-
angle sky reflectivity and `principled_bsdf`'s `Specular IOR Level` is normal-incidence
reflectance, and writing it cost `landscape` agreement on both bands. That is a mapping question
and this is not it.